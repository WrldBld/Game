# Hexagonal Architecture Phase 2 - Final Cleanup Plan

**Status**: ACTIVE  
**Created**: 2025-12-29  
**Goal**: Achieve 100% hexagonal architecture compliance with correct dependency graph  
**Priority**: HIGH - These are structural issues that affect the entire codebase  

---

## Executive Summary

Phase 1 of hexagonal remediation achieved 100% compliance at the **code level** (zero warnings, zero arch-check violations). However, a subsequent review revealed **structural issues in the dependency graph** that undermine the architecture at the crate level.

### Current Score: 76/100

Key issues:
- Backwards dependencies in Cargo.toml files
- Concrete implementations in wrong layers
- Business logic leakage into protocol crate
- God traits still present (7 with 15+ methods)
- God object (request_handler.rs at 3,497 lines)

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

### C3: player-ui → player-adapters Dependency

**Location**: `crates/player-ui/Cargo.toml:10`

**Root Cause**: player-ui imports `Platform` type from player-adapters for DI.

**Impact**: UI layer depends on infrastructure layer - violates hexagonal boundaries.

**Fix Strategy**:
1. Create `PlatformPort` trait in player-ports with required methods (storage, logging, time, connection)
2. Have `Platform` implement `PlatformPort` in player-adapters
3. Update player-ui to depend on `Arc<dyn PlatformPort>` instead of concrete `Platform`
4. player-runner creates Platform and provides as `Arc<dyn PlatformPort>`

**Files to modify**:
- `crates/player-ports/src/outbound/platform_port.rs` → NEW: Define trait
- `crates/player-adapters/src/infrastructure/platform.rs` → Implement trait
- `crates/player-ui/src/**/*.rs` → Update 23 imports to use trait
- `crates/player-runner/src/main.rs` → Provide Arc<dyn PlatformPort>
- `crates/player-ui/Cargo.toml` → Remove player-adapters dependency

---

### C4: FixedRandomPort in Wrong Layer - COMPLETED

**Status**: ✅ DONE

- Moved `FixedRandomPort` from `engine-ports` to `engine-adapters/src/infrastructure/testing/`
- Removed `rand` dependency from engine-ports
- Only `RandomPort` trait remains in engine-ports

---

## Medium Priority Issues

### M1: Business Logic in Protocol Crate

**Locations**:
| File | Line | Method | Fix Location |
|------|------|--------|--------------|
| responses.rs | 125 | `ErrorCode::to_http_status()` | Move to engine-adapters HTTP layer |
| responses.rs | 310 | `WorldRole::can_modify()` | Move to domain layer |
| responses.rs | 315 | `WorldRole::is_dm()` | Move to domain layer |
| responses.rs | 320 | `WorldRole::is_spectator()` | Move to domain layer |
| types.rs | 177 | `GameTime::from_domain()` | Move to engine-app (conversion belongs in app layer) |
| dto.rs | 65 | `NpcDispositionStateDto::to_domain()` | Move to engine-app |

**Fix Strategy**:
1. Create domain WorldRole enum with permission methods
2. Move HTTP status mapping to adapter layer
3. Move DTO conversion methods to application layer
4. Protocol should only have struct definitions + serde

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

### Phase 2.1: Critical Dependency Fixes (6-8 hours)

1. [x] C4: Move FixedRandomPort to engine-adapters
2. [ ] C1+C2: Eliminate AdapterState
   - [ ] Add `get_connection`, `broadcast_to_world` to WorldConnectionManagerPort
   - [ ] Add `ConnectionInfo` to engine-ports
   - [ ] Implement new methods in WorldConnectionManager
   - [ ] Update all handlers to use AppState
   - [ ] Delete AdapterState from engine-composition
   - [ ] Remove engine-composition dependency from engine-adapters
3. [ ] C3: Create PlatformPort trait, update player-ui

### Phase 2.2: Protocol Cleanup (2-3 hours)
1. [ ] M1: Move WorldRole methods to domain
2. [ ] M1: Move ErrorCode::to_http_status to adapters
3. [ ] M1: Move DTO conversion methods to app layer

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

- [ ] `cargo check --workspace` - Zero errors
- [ ] `cargo test --workspace` - All tests pass
- [ ] `cargo xtask arch-check` - Zero violations
- [ ] No backwards dependencies in Cargo.toml files
- [ ] No concrete implementations in ports layer
- [ ] No business logic in protocol layer
- [ ] No traits with more than 12 methods
- [ ] No files over 500 lines (except generated code)
- [ ] Proper dependency flow: domain → ports → adapters → app → composition → runner

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
