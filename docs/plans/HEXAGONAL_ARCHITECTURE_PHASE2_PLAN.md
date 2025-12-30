# Hexagonal Architecture Phase 2 - Final Cleanup Plan

**Status**: ACTIVE  
**Created**: 2025-12-29  
**Goal**: Achieve 100% hexagonal architecture compliance with correct dependency graph  
**Priority**: HIGH - These are structural issues that affect the entire codebase  

---

## Executive Summary

Phase 1 of hexagonal remediation achieved 100% compliance at the **code level** (zero warnings, zero arch-check violations). However, a subsequent review revealed **structural issues in the dependency graph** that undermine the architecture at the crate level.

### Current Score: 92/100 (Updated 2025-12-30)

**Completed - All Critical Issues + M1 Resolved**:
- ✅ C1+C2: AdapterState eliminated, AppStatePort created
- ✅ C3: PlatformPort created, player-ui → player-adapters dependency removed
- ✅ C4: FixedRandomPort moved to adapters layer
- ✅ M1: Unused business logic removed from protocol crate
- ✅ No backwards dependencies in any layer
- ✅ arch-check passes with zero violations

**Remaining Issues (identified in 2025-12-30 review)**:
- **C5**: Protocol → Domain dependency via From impls (CRITICAL)
- **C6**: workflow_service_port.rs has 270 lines of impl code in ports (HIGH)
- M2: God traits still present (7 with 15+ methods)
- M3: God object (request_handler.rs at 3,497 lines)

---

## Conceptual Model Update (2025-12-30)

### Terminology Change: "Shared Kernel" → "Shared Vocabulary" + "API Contract"

The previous model incorrectly used DDD's "shared kernel" terminology for protocol. 
Protocol is NOT a shared kernel (which implies shared domain logic). Instead:

- **domain-types**: "Shared Vocabulary" - pure data definitions (enums, simple structs)
- **protocol**: "API Contract" - wire format for engine↔player communication

### Target Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│  SHARED VOCABULARY (domain-types)                           │
│  - Pure data definitions with no identity or logic          │
│  - Depends on: serde only                                   │
└─────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┴───────────────────┐
          ▼                                       ▼
┌─────────────────────────┐          ┌─────────────────────────┐
│  DOMAIN LAYER           │          │  API CONTRACT (protocol)│
│  - Entities with IDs    │          │  - Wire-format DTOs     │
│  - Business logic       │          │  - Message enums        │
│  Depends on:            │          │  Depends on:            │
│  - domain-types         │          │  - domain-types ONLY    │
│  NO protocol imports    │          │  NO domain imports      │
└─────────────────────────┘          └─────────────────────────┘
          │                                       │
          └───────────────────┬───────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  ADAPTERS LAYER                                             │
│  - Domain ↔ Protocol conversions (From impls)               │
│  - Depends on: domain, protocol, ports                      │
└─────────────────────────────────────────────────────────────┘
```

---

## Critical Issues (Must Fix)

### C1+C2: Eliminate AdapterState - Use AppState Directly

**Current Broken State**:
```
engine-adapters ──depends on──► engine-composition (BACKWARDS!)
                                      │
                                      └── AppState (via AdapterState)
```

**Root Cause Analysis**:
- `AdapterState` in engine-adapters wraps `AppState` from engine-composition
- `AdapterState` adds 4 infrastructure fields: `config`, `connection_manager`, `comfyui_client`, `region_repo`
- This creates a backwards dependency: adapters → composition

**Analysis of AdapterState Fields**:

| Field | Current Location | Handler Usage | Solution |
|-------|------------------|---------------|----------|
| `app: AppState` | composition | All handlers | Use `AppState` directly |
| `config: AppConfig` | adapters | **Not used by handlers** | Keep in runner only |
| `connection_manager` | adapters | `get_connection()`, `broadcast_to_world()` | Add to `WorldConnectionManagerPort` |
| `comfyui_client` | adapters | `health_check()` | Use `AppState.comfyui` (already a port) |
| `region_repo` | adapters | `get_region_items()` | Use `AppState.region_repo` (already there) |

**Solution: Eliminate AdapterState Entirely**

Since handlers only need:
1. App-layer services via ports (already in `AppState`)
2. `get_connection()` - add to `WorldConnectionManagerPort`
3. `broadcast_to_world()` - use existing `BroadcastPort`
4. `health_check()` - use `AppState.comfyui` (already `ComfyUIPort`)

We can **eliminate `AdapterState`** and have handlers use `AppState` directly.

**Implementation Steps**:

**Step 1: Extend WorldConnectionManagerPort**

Add missing methods to `crates/engine-ports/src/outbound/world_connection_manager_port.rs`:

```rust
/// Get connection info by connection ID
async fn get_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo>;

/// Broadcast a message to all connections in a world
async fn broadcast_to_world(&self, world_id: Uuid, message: ServerMessage);

/// Broadcast to all worlds
async fn broadcast_to_all_worlds(&self, message: ServerMessage);
```

**Step 2: Add ConnectionInfo to ports layer**

Move/duplicate `ConnectionInfo` struct to engine-ports (or use existing types).

**Step 3: Update WorldConnectionManager implementation**

Implement the new port methods in `crates/engine-adapters/src/infrastructure/world_connection_manager.rs`.

**Step 4: Update all handlers to use AppState**

Change all handlers from:
```rust
async fn handle_foo(state: &AdapterState, ...) {
    state.connection_manager.get_connection(id);  // concrete type
    state.comfyui_client.health_check();          // concrete type
    state.app.use_cases.foo.execute(...);
}
```

To:
```rust
async fn handle_foo(state: &AppState, ...) {
    state.world_connection_manager.get_connection(id);  // via port
    state.comfyui.health_check();                       // via port
    state.use_cases.foo.execute(...);
}
```

**Step 5: Delete AdapterState**

- Delete `crates/engine-composition/src/adapter_state.rs`
- Remove from `crates/engine-composition/src/lib.rs`
- Update engine-runner to use `AppState` directly

**Step 6: Fix Cargo.toml dependencies**

- Remove `wrldbldr-engine-composition` from `engine-adapters/Cargo.toml`
- Verify correct dependency flow:
  ```
  domain → ports → adapters → app → composition → runner
  ```

**Files to modify**:
- `crates/engine-ports/src/outbound/world_connection_manager_port.rs` - Add methods
- `crates/engine-adapters/src/infrastructure/world_connection_manager.rs` - Implement methods
- `crates/engine-adapters/src/infrastructure/websocket/**/*.rs` - Update ~25 handlers
- `crates/engine-adapters/src/infrastructure/http/**/*.rs` - Update HTTP handlers
- `crates/engine-composition/src/adapter_state.rs` - DELETE
- `crates/engine-composition/src/lib.rs` - Remove export
- `crates/engine-adapters/Cargo.toml` - Remove composition dependency
- `crates/engine-runner/src/**/*.rs` - Use AppState directly

**Corrected Dependency Flow After Fix**:
```
┌─────────────┐
│   domain    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  protocol   │
└──────┬──────┘
       │
       ▼
┌──────────────┐
│ engine-ports │  ◄── WorldConnectionManagerPort with all needed methods
└──────┬───────┘
       │
       ├───────────────────┐
       ▼                   ▼
┌─────────────┐    ┌────────────────┐
│ engine-app  │    │engine-adapters │  ◄── NO composition dependency
└──────┬──────┘    └───────┬────────┘
       │                   │
       └─────────┬─────────┘
                 ▼
       ┌───────────────────┐
       │engine-composition │  ◄── Only AppState, no AdapterState
       └─────────┬─────────┘
                 │
                 ▼
       ┌───────────────────┐
       │  engine-runner    │
       └───────────────────┘
```

---

### C3: player-ui → player-adapters Dependency - COMPLETED

**Status**: ✅ DONE (2025-12-30)

**Solution implemented**:
1. Created `PlatformPort` trait in `player-ports/src/outbound/platform_port.rs`
   - Defines all platform methods: time, sleep, random, storage, logging, document, engine config, connection factory
   - Uses `Send + Sync` bounds for Dioxus context compatibility

2. Implemented `PlatformPort` for `Platform` in `player-adapters/src/state/platform.rs`

3. Added to `player-ui/src/lib.rs`:
   - `pub type Platform = Arc<dyn PlatformPort>` - Type alias for convenience
   - `pub fn use_platform() -> Platform` - Hook for Dioxus context access

4. Updated all 23 player-ui files to use:
   - `use crate::use_platform` instead of `use wrldbldr_player_adapters::Platform`
   - `use_platform()` instead of `use_context::<Platform>()`
   - `platform.as_ref()` when passing to functions expecting `&dyn PlatformPort`

5. Updated `player-runner/src/lib.rs` to wrap Platform in Arc<dyn PlatformPort>

6. Removed `wrldbldr-player-adapters` dependency from `player-ui/Cargo.toml`

**Result**: Player UI layer now properly depends only on ports, not adapters

---

### C4: FixedRandomPort in Wrong Layer - COMPLETED

**Status**: ✅ DONE (2025-12-30)

- Moved `FixedRandomPort` from `engine-ports` to `engine-adapters/src/infrastructure/testing/`
- Removed `rand` dependency from engine-ports
- Only `RandomPort` trait remains in engine-ports

---

### C1+C2: AdapterState Eliminated - COMPLETED

**Status**: ✅ DONE (2025-12-30)

**Solution implemented**:
1. Created `AppStatePort` trait in `engine-ports/src/inbound/app_state_port.rs`
   - Provides access to all use cases, services, and infrastructure via trait methods
   - All handlers now use `&dyn AppStatePort` instead of concrete `&AdapterState`

2. Extended `WorldConnectionManagerPort` with:
   - `get_connection_context(connection_id)` - Returns port-safe `ConnectionContext`
   - `get_connection_by_client_id(client_id)` - Lookup by client ID
   - `is_spectator_by_client_id(client_id)` - Check spectator status
   - `get_pc_id_by_client_id(client_id)` - Get player character ID
   - `broadcast_to_world(world_id, message)` - Broadcast to world
   - `broadcast_to_dms(world_id, message)` - Broadcast to DMs
   - `broadcast_to_players(world_id, message)` - Broadcast to players
   - `broadcast_to_all_worlds(message)` - Global broadcast
   - `unregister_connection(connection_id)` - Remove connection

3. Added `ConnectionContext` struct to ports layer (port-safe version of `ConnectionInfo`)

4. Implemented `AppStatePort` for `AppState` in engine-composition

5. Updated all ~23 HTTP/WebSocket handlers to use `&dyn AppStatePort`

6. Added `WorkerServices` struct in engine-runner for background workers needing concrete types

7. Deleted `AdapterState` and removed engine-composition dependency from engine-adapters

**Result**: arch-check passes with zero violations

---

## New Critical Issues (Identified 2025-12-30)

### C5: Protocol → Domain Dependency (CRITICAL)

**Status**: ⏳ IN PROGRESS

**Problem**: Protocol crate depends on `wrldbldr-domain` for `From<DomainEntity>` trait implementations. This:
- Forces player WASM to compile entire domain crate
- Violates API contract principle (protocol should only know wire format)
- Creates tight coupling between wire format and domain internals

**Current protocol→domain imports (16 From impls)**:
- `GalleryAsset` → `GalleryAssetResponseDto`
- `GenerationBatch` → `GenerationBatchResponseDto`
- `NpcDispositionState` → `NpcDispositionStateDto`
- `WorkflowConfiguration` → `WorkflowConfigExportDto`
- `PromptMapping` ↔ `PromptMappingDto` (bidirectional)
- `PromptMappingType` ↔ `PromptMappingTypeDto` (bidirectional)
- `InputDefault` ↔ `InputDefaultDto` (bidirectional)
- `InputType` ↔ `InputTypeDto` (bidirectional)
- `WorkflowInput` → `WorkflowInputDto`
- `WorkflowAnalysis` → `WorkflowAnalysisDto`
- `AdHocOutcomes` (bidirectional in messages.rs)

**Solution**:
1. Unify EntityType enum in domain-types (extend from 3 to 20 variants)
2. Move GameTime to domain-types as canonical source
3. Update protocol imports to use domain-types for shared vocabulary
4. Move all From<DomainEntity> implementations to engine-adapters
5. Remove `wrldbldr-domain` dependency from protocol/Cargo.toml

**Files to modify**:
- `crates/domain-types/src/asset_types.rs` - Extend EntityType
- `crates/domain-types/src/lib.rs` - Add new exports
- `crates/protocol/src/responses.rs` - Re-export EntityType from domain-types
- `crates/protocol/src/dto.rs` - Move From impls out
- `crates/protocol/src/messages.rs` - Move AdHocOutcomes From impls out
- `crates/protocol/Cargo.toml` - Remove domain dependency
- `crates/engine-adapters/src/infrastructure/dto_conversions/` - New module for From impls

---

### C6: Implementation Code in Ports Layer (HIGH)

**Status**: ⏳ PENDING

**Problem**: `crates/engine-ports/src/outbound/workflow_service_port.rs` contains ~270 lines of implementation code:
- `analyze_workflow()` - Parses ComfyUI workflow JSON
- `validate_workflow()` - Validates workflow format
- `prepare_workflow()` - Prepares workflow with prompts (uses `rand`)
- `auto_detect_prompt_mappings()` - Auto-detects CLIPTextEncode nodes
- `export_workflow_configs()` - Exports to JSON
- `import_workflow_configs()` - Imports from JSON

**Additional Issue**: `rand` dependency in engine-ports (acknowledged TODO)

**Solution**:
These functions are already duplicated in `engine-app/services/workflow_service.rs`.
1. Delete implementation code from workflow_service_port.rs (keep only trait)
2. Remove `rand` dependency from engine-ports/Cargo.toml
3. Update engine-adapters/http/workflow_routes.rs to use WorkflowService directly
4. Add engine-app dependency to engine-adapters (acceptable in hexagonal)

**Files to modify**:
- `crates/engine-ports/src/outbound/workflow_service_port.rs` - Delete lines 59-337
- `crates/engine-ports/src/outbound/mod.rs` - Remove function re-exports
- `crates/engine-ports/Cargo.toml` - Remove `rand` dependency
- `crates/engine-adapters/Cargo.toml` - Add `wrldbldr-engine-app`
- `crates/engine-adapters/src/infrastructure/http/workflow_routes.rs` - Update imports

---

## Medium Priority Issues

### M1: Business Logic in Protocol Crate - COMPLETED

**Status**: ✅ DONE (2025-12-30)

**Analysis Results**:
- `ErrorCode::to_http_status()` - **REMOVED** (was unused, HTTP is adapter concern)
- `NpcDispositionStateDto::to_domain()` - **REMOVED** (was unused)
- `WorldRole::can_modify()/is_dm()/is_spectator()` - **KEPT** (acceptable as simple enum predicates)
- `GameTime::from_domain()` - **KEPT** (converts TO wire format, which is protocol's responsibility)

**Changes Made**:
- Removed `ErrorCode::to_http_status()` from responses.rs
- Removed `NpcDispositionStateDto::to_domain()` from dto.rs
- Removed unused `CharacterId`/`PlayerCharacterId` imports

---

### M2: Remaining God Traits (7 with 15+ methods)

These need Interface Segregation Principle (ISP) splitting:

| Trait | Methods | Suggested Split |
|-------|---------|-----------------|
| LocationRepositoryPort | 27 | CrudPort, ConnectionPort, NavigationPort, GridPort |
| RegionRepositoryPort | 19 | CrudPort, LocationPort, NavigationPort |
| PlayerCharacterRepositoryPort | 17 | CrudPort, CharacterPort, WorldPort |
| SceneRepositoryPort | 17 | CrudPort, CharacterPort, DirectorialPort |
| InteractionRepositoryPort | 17 | CrudPort, AvailabilityPort, TriggerPort |
| AssetRepositoryPort | 17 | CrudPort, GalleryPort, GenerationPort |
| EventChainRepositoryPort | 16 | CrudPort, EdgePort, QueryPort |

**Fix Strategy**: Apply same pattern as Character/Challenge/StoryEvent repos that were already split.

---

### M3: request_handler.rs God Object (3,497 lines)

**Location**: `crates/engine-app/src/application/handlers/request_handler.rs`

**Fix Strategy**: Split into domain-specific handler modules:
```
handlers/
├── mod.rs (dispatcher)
├── world_handler.rs
├── character_handler.rs
├── location_handler.rs
├── scene_handler.rs
├── challenge_handler.rs
├── narrative_handler.rs
├── inventory_handler.rs
├── generation_handler.rs
└── admin_handler.rs
```

---

### M4: Additional Findings

| Issue | Location | Fix |
|-------|----------|-----|
| Magic strings (localhost URLs) | 6+ locations | Externalize to config |
| Missing error context | 30+ `.map_err(\|e\| e.to_string())` | Add proper error types/context |
| Excessive `.unwrap()` | 100+ uses | Replace with proper error handling |
| Dead code | 21+ `#[allow(dead_code)]` | Remove or justify |

---

## Implementation Order

### Phase 2.1: Critical Dependency Fixes (6-8 hours) - COMPLETED

1. [x] C4: Move FixedRandomPort to engine-adapters
2. [x] C1+C2: Eliminate AdapterState - COMPLETED 2025-12-30
   - [x] Add `get_connection_context`, `broadcast_to_world` etc to WorldConnectionManagerPort
   - [x] Add `ConnectionContext` struct to engine-ports
   - [x] Implement new methods in WorldConnectionManager
   - [x] Create AppStatePort trait in engine-ports
   - [x] Implement AppStatePort for AppState in engine-composition
   - [x] Update all ~23 handlers to use &dyn AppStatePort
   - [x] Delete AdapterState from engine-composition
   - [x] Remove engine-composition dependency from engine-adapters
   - [x] Remove unused engine-dto dependency from engine-composition
   - [x] Add WorkerServices for background workers needing concrete types
3. [x] C3: Create PlatformPort trait, update player-ui - COMPLETED 2025-12-30
   - [x] Create PlatformPort trait in player-ports
   - [x] Implement PlatformPort for Platform in player-adapters
   - [x] Add use_platform() hook and Platform type alias in player-ui
   - [x] Update all 23 player-ui files to use Arc<dyn PlatformPort>
   - [x] Remove player-adapters dependency from player-ui

### Phase 2.2: Protocol Cleanup (2-3 hours) - COMPLETED
1. [x] M1: Removed unused ErrorCode::to_http_status (was dead code)
2. [x] M1: Removed unused NpcDispositionStateDto::to_domain (was dead code)
3. [x] M1: Kept WorldRole predicates and GameTime::from_domain (acceptable in protocol)

### Phase 2.6: Protocol Architecture Fix (NEW - 8-12 hours)

**Goal**: Remove `wrldbldr-domain` dependency from protocol crate

1. [ ] C5.1: Unify EntityType enum in domain-types
   - [ ] Extend EntityType from 3 to 20 variants
   - [ ] Add `has_assets()` helper method
   - [ ] Add `Unknown` variant for forward compatibility
   - [ ] Use snake_case serde with camelCase aliases for backward compat

2. [ ] C5.2: Move GameTime to domain-types
   - [ ] Create `game_time.rs` in domain-types
   - [ ] Update domain to re-export from domain-types
   - [ ] Update protocol to re-export from domain-types

3. [ ] C5.3: Update protocol to use domain-types imports
   - [ ] Change imports in dto.rs from domain to domain-types
   - [ ] Change imports in responses.rs (EntityType re-export)
   - [ ] Change imports in types.rs if any

4. [ ] C5.4: Create dto_conversions module in engine-adapters
   - [ ] Create `crates/engine-adapters/src/infrastructure/dto_conversions/mod.rs`
   - [ ] Move GalleryAsset → GalleryAssetResponseDto
   - [ ] Move GenerationBatch → GenerationBatchResponseDto
   - [ ] Move NpcDispositionState → NpcDispositionStateDto
   - [ ] Move WorkflowConfiguration → WorkflowConfigExportDto
   - [ ] Move PromptMapping ↔ PromptMappingDto
   - [ ] Move InputDefault ↔ InputDefaultDto
   - [ ] Move InputType ↔ InputTypeDto
   - [ ] Move WorkflowInput → WorkflowInputDto
   - [ ] Move WorkflowAnalysis → WorkflowAnalysisDto
   - [ ] Move helper functions (workflow_config_to_response_dto, etc.)

5. [ ] C5.5: Move AdHocOutcomes conversions from messages.rs

6. [ ] C5.6: Update call sites in engine-adapters and engine-app
   - [ ] Update asset_routes.rs
   - [ ] Update workflow_routes.rs
   - [ ] Update request_handler.rs
   - [ ] Update generation_queue_projection_service.rs

7. [ ] C5.7: Remove domain dependency from protocol
   - [ ] Remove `wrldbldr-domain = { workspace = true }` from Cargo.toml
   - [ ] Verify protocol compiles with domain-types only

8. [ ] C6: Fix workflow_service_port.rs
   - [ ] Delete implementation code (lines 59-337)
   - [ ] Remove function re-exports from mod.rs
   - [ ] Remove `rand` from engine-ports/Cargo.toml
   - [ ] Add engine-app to engine-adapters dependencies
   - [ ] Update workflow_routes.rs to use WorkflowService

### Phase 2.3: God Trait Splitting (8-12 hours)
1. [ ] M2: Split LocationRepositoryPort (27 methods)
2. [ ] M2: Split RegionRepositoryPort (19 methods)
3. [ ] M2: Split remaining 5 god traits

### Phase 2.4: God Object Splitting (4-6 hours)
1. [ ] M3: Split request_handler.rs into domain modules
2. [ ] Update imports across codebase

### Phase 2.5: Code Quality (2-3 hours)
1. [ ] M4: Externalize hardcoded URLs to config
2. [ ] M4: Improve error handling patterns
3. [ ] M4: Clean up dead code

---

## Verification Checklist

After all phases complete:

- [x] `cargo check --workspace` - Zero errors ✅
- [x] `cargo test --workspace` - All tests pass ✅
- [x] `cargo xtask arch-check` - Zero violations ✅ (as of 2025-12-30)
- [x] No backwards dependencies in Cargo.toml files ✅ (engine layer)
- [x] No concrete implementations in ports layer ✅
- [ ] No business logic in protocol layer (M1 pending)
- [ ] No traits with more than 12 methods (M2 pending)
- [ ] No files over 500 lines (M3 pending)
- [x] Proper dependency flow: domain → ports → adapters → app → composition → runner ✅

---

## Architecture Target State

```
┌─────────────────────────────────────────────────────────────────┐
│                     CORRECT DEPENDENCY FLOW                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  domain ─────────────────────────────────────────────────────┐  │
│     │                                                        │  │
│     ▼                                                        │  │
│  protocol (shared kernel - wire types only, NO logic)        │  │
│     │                                                        │  │
│     ▼                                                        │  │
│  ports (traits only, NO implementations)                     │  │
│     │                                                        │  │
│     ▼                                                        │  │
│  adapters (implements ports, NO composition deps)            │  │
│     │                                                        │  │
│     ▼                                                        │  │
│  app (orchestrates domain via ports)                         │  │
│     │                                                        │  │
│     ▼                                                        │  │
│  composition (wires adapters to app, creates AppState)       │  │
│     │                                                        │  │
│     ▼                                                        │  │
│  runner (entry point)                                        │  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Principles

1. **Adapters never depend on composition**: Adapters implement port traits, they don't need to know how services are composed.

2. **Handlers use ports, not concrete types**: WebSocket/HTTP handlers receive `AppState` which contains `Arc<dyn Port>` for all services.

3. **No AdapterState needed**: All infrastructure capabilities are exposed through ports in AppState.

4. **Protocol is wire-format only**: Just serde structs for serialization, no business logic methods.

5. **Ports are pure trait definitions**: No concrete implementations, no test utilities (those go in adapters).

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| C1+C2 | High - 100+ handler changes | Systematic find/replace, preserve function signatures |
| C3 | Medium - 23 import sites | Mechanical change, trait signature matches current usage |
| C4 | Low - Already complete | ✅ Done |
| M1 | Low - Isolated methods | Move one at a time |
| M2 | Medium - Large refactor | Follow established pattern from Phase 1 |
| M3 | Medium - Large file split | Preserve all functionality, add tests |
