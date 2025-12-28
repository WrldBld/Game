# Hexagonal Architecture Gap Remediation Plan

**Status**: Ready for Implementation  
**Created**: 2025-12-28  
**Updated**: 2025-12-28 (Post-validation corrections)  
**Priority**: High - Architecture violations identified  
**Estimated Total Effort**: 6-10 hours (excluding deferred items)

## Executive Summary

Validation of the Hexagonal Enforcement Master Plan revealed gaps in implementation. This unified plan addresses all remaining architecture violations and incomplete work, incorporating detailed analysis from multiple validation passes.

**Important Update**: Deep validation revealed that **Phase G1 describes issues that don't exist**. The `DirectorialContextData` and use-case-specific `DirectorialContextRepositoryPort` in `scene.rs` are **proper hexagonal architecture patterns** (use-case-specific DTOs and ports), not violations. The adapters are **required bridge adapters**. G1 has been marked as INVALID.

### Gaps Identified

| ID | Severity | Issue | Location | Status |
|----|----------|-------|----------|--------|
| G1 | ~~Critical~~ | ~~Duplicate DirectorialContextRepositoryPort~~ | `scene.rs`, adapters | **INVALID** - Not a violation |
| G2 | **High** | `app_event_repository_port.rs` imports protocol `AppEvent` | `engine-ports` | Active |
| G3 | **High** | Player-app services import protocol `Create*Data` types | 25+ imports across 13 services | Active |
| G4 | **Low** | 2 handlers not using `IntoServerError` | `movement.rs`, `narrative.rs` | Active |
| G5 | **Low** | Player-app dto/mod.rs re-exports protocol types | `dto/mod.rs` | Active |
| G6 | **Medium** | Phase E5 handler split incomplete (Deferred) | `request_handler.rs` (3,058 lines) | Deferred |
| G7 | **Medium** | Protocol re-exports domain types | `protocol/types.rs`, `protocol/rule_system.rs` | **NEW** |
| G8 | **Low** | Player-app DTO re-exports domain/ports | `world_snapshot.rs`, `session_types.rs` | **NEW** |

---

## Phase G1: DirectorialContext Duplication - INVALID

**Status**: **INVALID - No changes needed**  
**Resolution**: Deep validation confirmed this is NOT a violation

### Why G1 is Invalid

Initial analysis incorrectly identified the `DirectorialContextData` and use-case-specific `DirectorialContextRepositoryPort` in `scene.rs` as "duplicate" violations. However, deeper validation revealed:

1. **`DirectorialContextData` is a proper use-case-specific DTO** - It serves a different purpose than `DirectorialNotes`:
   - `DirectorialNotes` (domain): Rich domain model with enums (`ToneGuidance`, `PacingGuidance`)
   - `DirectorialContextData` (app): Simplified DTO for scene use-case operations

2. **The use-case-specific `DirectorialContextRepositoryPort` is a valid pattern** - Having a use-case-specific port in the application layer that gets adapted to the infrastructure port is a **correct hexagonal architecture pattern**, not a violation.

3. **The bridge adapters are required** - `DirectorialContextAdapter` and `ConnectionDirectorialContextAdapter` perform the legitimate function of converting between application-layer DTOs and infrastructure-layer domain types.

4. **`DirectorialNotes` already has `Serialize`/`Deserialize`** - Confirmed present in domain.

### Architecture Pattern Explanation

```
┌─────────────────────────────────────────────────────────────────┐
│  Use-Case Layer (engine-app)                                    │
│  - DirectorialContextData (use-case DTO)                        │
│  - DirectorialContextRepositoryPort (use-case contract)         │
│       ↓ (adapted by)                                            │
├─────────────────────────────────────────────────────────────────┤
│  Adapter Layer (engine-adapters)                                │
│  - DirectorialContextAdapter (converts DTO ↔ domain)            │
│       ↓ (delegates to)                                          │
├─────────────────────────────────────────────────────────────────┤
│  Ports Layer (engine-ports)                                     │
│  - DirectorialContextRepositoryPort (infrastructure contract)   │
│       ↓ (implemented by)                                        │
├─────────────────────────────────────────────────────────────────┤
│  Infrastructure (engine-adapters)                               │
│  - SqliteDirectorialContextRepository                           │
└─────────────────────────────────────────────────────────────────┘
```

This is a **valid hexagonal layering pattern** where each layer has appropriate types.

### No Action Required

G1 has been marked **INVALID**. No code changes are needed.

---

## Phase G2: Fix app_event_repository_port Protocol Import (High)

**Estimated Time**: 1-2 hours  
**Priority**: High - Ports layer should not import protocol

### Problem

`crates/engine-ports/src/outbound/app_event_repository_port.rs` imports `wrldbldr_protocol::AppEvent` directly. Ports should only use domain types.

### Solution Options

**Option A (Recommended): Deprecate and Remove**

Since `DomainEventRepositoryPort` already exists and uses the correct domain type:
1. Migrate all usages to `DomainEventRepositoryPort`
2. Delete `app_event_repository_port.rs`

**Option B: Keep as Adapter-Layer Storage Format**

If `AppEvent` is the persistence format:
1. Move `AppEventRepositoryPort` to adapters layer (it's a storage concern)
2. Have adapters convert `DomainEvent` → `AppEvent` before storage

### Implementation (Option A)

#### Step 1: Find usages

```bash
grep -r "AppEventRepositoryPort" crates/
```

#### Step 2: Update usages to DomainEventRepositoryPort

Update any files importing `AppEventRepositoryPort` to use `DomainEventRepositoryPort` instead.

#### Step 3: Delete app_event_repository_port.rs

```bash
rm crates/engine-ports/src/outbound/app_event_repository_port.rs
```

#### Step 4: Update mod.rs exports

Remove the export from `crates/engine-ports/src/outbound/mod.rs`.

### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-ports/src/outbound/app_event_repository_port.rs` | Delete |
| `crates/engine-ports/src/outbound/mod.rs` | Remove export |
| Any files importing `AppEventRepositoryPort` | Update to use `DomainEventRepositoryPort` |

### Verification

```bash
# Should return NO results after completion
grep -r "AppEventRepositoryPort" crates/
grep -r "app_event_repository_port" crates/
```

---

## Phase G3: Create Player-App Request DTOs (High)

**Estimated Time**: 4-6 hours  
**Priority**: High - Application layer should not import protocol types

### Problem

Player-app services directly import protocol `Create*Data` and `Update*Data` types:
- `CreateWorldData`, `CreateChallengeData`, `CreateSkillData`, etc.
- `UpdateChallengeData`, `UpdateSkillData`, `UpdateEventChainData`, etc.

This violates hexagonal architecture - application layer should use its own DTOs.

### Affected Services

| Service | Protocol Types Imported |
|---------|------------------------|
| `world_service.rs` | `CreateWorldData` |
| `challenge_service.rs` | `CreateChallengeData`, `UpdateChallengeData` |
| `skill_service.rs` | `CreateSkillData`, `UpdateSkillData` |
| `character_service.rs` | `CreateCharacterData`, `UpdateCharacterData` |
| `location_service.rs` | `CreateLocationData`, `UpdateLocationData` |
| `event_chain_service.rs` | `CreateEventChainData`, `UpdateEventChainData` |
| `narrative_event_service.rs` | `CreateNarrativeEventData` |
| `actantial_service.rs` | `CreateGoalData`, `CreateWantData`, `UpdateGoalData`, `UpdateWantData` |
| `session_service.rs` | `ServerMessage` |

### Solution

Create player-app owned request DTOs and add converters in player-adapters.

### Implementation

#### Step 1: Create player-app request DTOs

**File:** `crates/player-app/src/application/dto/requests.rs`

```rust
//! Request DTOs for player-app services
//!
//! These mirror protocol Create*/Update* types but are owned by the
//! application layer. Conversion to protocol types happens in adapters.

use serde::{Deserialize, Serialize};

// === World ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorldRequest {
    pub name: String,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub setting: Option<String>,
}

// === Challenge ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChallengeRequest {
    pub name: String,
    pub description: Option<String>,
    pub skill_id: Option<String>,
    pub difficulty: Option<String>,
    pub success_description: Option<String>,
    pub failure_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChallengeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub skill_id: Option<String>,
    pub difficulty: Option<String>,
}

// === Skill ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: Option<String>,
    pub base_stat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub base_stat: Option<String>,
}

// === Character ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCharacterRequest {
    pub name: String,
    pub description: Option<String>,
    pub archetype_id: Option<String>,
    pub location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCharacterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub archetype_id: Option<String>,
}

// === Location ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocationRequest {
    pub name: String,
    pub description: Option<String>,
    pub region_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

// === EventChain ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventChainRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEventChainRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

// === NarrativeEvent ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNarrativeEventRequest {
    pub name: String,
    pub description: Option<String>,
    pub event_chain_id: Option<String>,
}

// === Actantial (Goal/Want) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoalRequest {
    pub character_id: String,
    pub description: String,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGoalRequest {
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub completed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWantRequest {
    pub character_id: String,
    pub description: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWantRequest {
    pub description: Option<String>,
    pub satisfied: Option<bool>,
}
```

#### Step 2: Create converters in player-adapters

**File:** `crates/player-adapters/src/infrastructure/request_converters.rs`

```rust
//! Converters from player-app request DTOs to protocol types
//!
//! These conversions happen at the adapter boundary when making
//! requests to the engine via GameConnectionPort.

use wrldbldr_player_app::application::dto::requests::*;
use wrldbldr_protocol::*;

// === World ===

impl From<CreateWorldRequest> for CreateWorldData {
    fn from(req: CreateWorldRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            genre: req.genre,
            setting: req.setting,
        }
    }
}

// === Challenge ===

impl From<CreateChallengeRequest> for CreateChallengeData {
    fn from(req: CreateChallengeRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            skill_id: req.skill_id,
            difficulty: req.difficulty,
            success_description: req.success_description,
            failure_description: req.failure_description,
        }
    }
}

impl From<UpdateChallengeRequest> for UpdateChallengeData {
    fn from(req: UpdateChallengeRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            skill_id: req.skill_id,
            difficulty: req.difficulty,
        }
    }
}

// ... Similar for all other types
```

#### Step 3: Update player-app services

Replace protocol imports with app-layer imports in each service file.

#### Step 4: Export from dto/mod.rs

Add `pub mod requests;` to `crates/player-app/src/application/dto/mod.rs`.

### Files to Create

| File | Content |
|------|---------|
| `crates/player-app/src/application/dto/requests.rs` | Request DTOs |
| `crates/player-adapters/src/infrastructure/request_converters.rs` | DTO → Protocol converters |

### Files to Modify

| File | Action |
|------|--------|
| `crates/player-app/src/application/dto/mod.rs` | Add `pub mod requests` |
| `crates/player-app/src/application/services/world_service.rs` | Update imports |
| `crates/player-app/src/application/services/challenge_service.rs` | Update imports |
| `crates/player-app/src/application/services/skill_service.rs` | Update imports |
| `crates/player-app/src/application/services/character_service.rs` | Update imports |
| `crates/player-app/src/application/services/location_service.rs` | Update imports |
| `crates/player-app/src/application/services/event_chain_service.rs` | Update imports |
| `crates/player-app/src/application/services/narrative_event_service.rs` | Update imports |
| `crates/player-app/src/application/services/actantial_service.rs` | Update imports |

### Verification

```bash
# Should return NO results (excluding documented exemptions for RequestPayload)
grep -r "use wrldbldr_protocol::Create" crates/player-app/src/application/services/
grep -r "use wrldbldr_protocol::Update" crates/player-app/src/application/services/
```

---

## Phase G4: Migrate Remaining Handlers to IntoServerError (Low)

**Estimated Time**: 30 minutes  
**Priority**: Low - Consistency improvement, not architecture violation

### Problem

Two handlers use different error handling patterns:
- `movement.rs` - uses local `movement_error_to_message()` function
- `narrative.rs` - uses `error_msg()` helper

### Solution

Migrate both to use `IntoServerError` trait for consistency.

### Implementation

#### Step 1: Update movement.rs

**File:** `crates/engine-adapters/src/infrastructure/websocket/handlers/movement.rs`

```rust
// ADD import:
use crate::infrastructure::websocket::IntoServerError;

// REMOVE function movement_error_to_message() (lines 182-195)

// REPLACE calls:
// FROM: movement_error_to_message(e)
// TO: e.into_server_error()
```

#### Step 2: Update narrative.rs

**File:** `crates/engine-adapters/src/infrastructure/websocket/handlers/narrative.rs`

```rust
// ADD import:
use crate::infrastructure::websocket::IntoServerError;

// REPLACE:
// FROM: error_msg("NARRATIVE_EVENT_ERROR", &e.to_string())
// TO: e.into_server_error()
```

### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-adapters/src/infrastructure/websocket/handlers/movement.rs` | Migrate error handling |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/narrative.rs` | Migrate error handling |

---

## Phase G5: Remove Protocol Re-exports from Player DTO (Low)

**Estimated Time**: 1 hour  
**Priority**: Low - API hygiene improvement

### Problem

`crates/player-app/src/application/dto/mod.rs` re-exports protocol actantial types:

```rust
pub use wrldbldr_protocol::{
    WantVisibilityData, ActantialRoleData, WantTargetTypeData,
    NpcActantialContextData, WantData, GoalData,
    ActantialActorData, ActorTypeData, SocialRelationData,
};
```

This leaks protocol types through player-app's public API.

### Solution

**Option A (Recommended): Private imports only**

Remove the `pub use` and update files that need these types to import from protocol directly. This is acceptable for adapter-boundary code where protocol types are legitimately needed.

**Option B: Create app-layer types**

Create equivalents in player-app/dto and add converters. This is more work but provides better isolation.

### Implementation (Option A)

1. Remove `pub use wrldbldr_protocol::{...}` block from `dto/mod.rs`
2. Update any files that were using these re-exports to import directly from protocol

---

## Phase G6: Complete Handler Split (Medium - DEFERRED)

**Estimated Time**: 8-12 hours  
**Priority**: Medium - Maintainability improvement, not architecture violation  
**Status**: DEFERRED

### Problem

`request_handler.rs` is 3,058 lines with 134 request variants.

### Current State

This was Phase E5 of the original plan. Only `common.rs` (helper extraction) was completed (~20% of the work).

### Recommendation

**Defer this phase** - it's a maintainability improvement, not an architecture violation. The current code works and passes arch-check.

If implemented later, follow the original Phase E5 plan:
1. Create 8 domain-specific handlers
2. Create RequestRouter for dispatch
3. Split the monolithic handler

---

## Phase G7: Protocol Re-exports Domain Types (Medium - NEW)

**Estimated Time**: 1-2 hours  
**Priority**: Medium - Cross-layer type leakage  
**Status**: Active (newly discovered)

### Problem

The `protocol` crate re-exports types from `domain`, violating the architecture rule that each layer should own its types:

**File:** `crates/protocol/src/types.rs:8-9`
```rust
pub use wrldbldr_domain::value_objects::npc_context::CharacterMood;
pub use wrldbldr_domain::value_objects::npc_context::RelationshipModifier;
```

**File:** `crates/protocol/src/rule_system.rs:7-10`
```rust
pub use wrldbldr_domain::value_objects::archetype::ArchetypeData;
pub use wrldbldr_domain::value_objects::archetype::ArchetypeRelation;
pub use wrldbldr_domain::value_objects::archetype::RelationType;
pub use wrldbldr_domain::entities::skill::SkillData;
```

### Analysis

This is a gray area:
- **Argument for keeping**: Protocol is the wire format, domain types are stable
- **Argument for removing**: Violates strict hexagonal layering; consumers should import from domain directly

### Solution Options

**Option A (Recommended): Document as Approved Exception**

These are stable domain types used for serialization. Adding re-exports to protocol simplifies consumer imports. Document this as an approved architectural exception rather than fixing it.

**Option B: Remove Re-exports**

Remove the `pub use` statements and have consumers import from `wrldbldr_domain` directly. This is more work and may break downstream code.

### Implementation (Option A)

Add documentation comment to `protocol/src/types.rs` and `protocol/src/rule_system.rs`:

```rust
// Re-exports from domain for convenience. These are stable types used
// in protocol serialization. Approved exception to strict layering rules.
```

### Files to Modify

| File | Action |
|------|--------|
| `crates/protocol/src/types.rs` | Add documentation comment |
| `crates/protocol/src/rule_system.rs` | Add documentation comment |

---

## Phase G8: Player-App DTO Re-exports (Low - NEW)

**Estimated Time**: 30 minutes  
**Priority**: Low - Minor type leakage  
**Status**: Active (newly discovered)

### Problem

Player-app DTO files re-export types from domain and ports:

**File:** `crates/player-app/src/application/dto/world_snapshot.rs`
- Re-exports domain types

**File:** `crates/player-app/src/application/dto/session_types.rs`  
- Re-exports from `wrldbldr_player_ports::session_types`

### Analysis

Similar to G7, this is convenience re-exporting. The severity is lower because:
- Player-app is the boundary layer where protocol/domain types meet
- These re-exports simplify the public API

### Solution Options

**Option A (Recommended): Document as Approved Exception**

These serve a legitimate API simplification purpose. Document rather than fix.

**Option B: Remove Re-exports**

Have consumers import directly from source crates.

### Implementation (Option A)

Add documentation comment explaining the re-exports.

---

## Implementation Order

### Phase 1: High Priority (Do First)
1. **G2**: Fix app_event_repository_port (1-2 hours)
2. **G3**: Create player-app request DTOs (4-6 hours)

### Phase 2: Low Priority (Do When Time Permits)
3. **G4**: Migrate handlers to IntoServerError (30 minutes)
4. **G5**: Remove protocol re-exports from player DTO (1 hour)
5. **G7**: Document protocol re-exports as approved exception (30 minutes)
6. **G8**: Document player-app DTO re-exports (15 minutes)

### Deferred
7. **G6**: Handler split (8-12 hours) - Optional maintainability improvement

### Invalid (No Action)
- **G1**: DirectorialContext duplication - INVALID (not a violation)

---

## Verification

After completing each phase:

```bash
# Build check
cargo check --workspace

# Architecture check
cargo run -p xtask -- arch-check

# Phase G2 verification
grep -r "AppEventRepositoryPort" crates/  # Should be 0

# Phase G3 verification
grep -r "use wrldbldr_protocol::Create" crates/player-app/src/application/services/  # Should be 0
grep -r "use wrldbldr_protocol::Update" crates/player-app/src/application/services/  # Should be 0
```

---

## Success Criteria

| Metric | Before | After |
|--------|--------|-------|
| AppEventRepositoryPort usages | >0 | 0 |
| Protocol imports in player-app services | 25+ | 0 (excluding documented exemptions) |
| Handlers using IntoServerError | 7/9 | 9/9 |
| Protocol re-exports documented | No | Yes (G7) |
| arch-check | Pass | Pass |

---

## Appendix A: Files Affected Summary

### To Delete
- `crates/engine-ports/src/outbound/app_event_repository_port.rs` (after G2)

### To Create
- `crates/player-app/src/application/dto/requests.rs` (G3)
- `crates/player-adapters/src/infrastructure/request_converters.rs` (G3)

### To Modify (High - G2)
- `crates/engine-ports/src/outbound/mod.rs`
- Any files importing `AppEventRepositoryPort`

### To Modify (High - G3)
- `crates/player-app/src/application/dto/mod.rs`
- 13 player-app service files

### To Modify (Low - G4)
- `crates/engine-adapters/src/infrastructure/websocket/handlers/movement.rs`
- `crates/engine-adapters/src/infrastructure/websocket/handlers/narrative.rs`

### To Modify (Low - G5)
- `crates/player-app/src/application/dto/mod.rs`

### To Modify (Medium - G7)
- `crates/protocol/src/types.rs` (add documentation)
- `crates/protocol/src/rule_system.rs` (add documentation)

### To Modify (Low - G8)
- `crates/player-app/src/application/dto/world_snapshot.rs` (add documentation)
- `crates/player-app/src/application/dto/session_types.rs` (add documentation)

---

## Appendix B: Handler Protocol Usage Clarification

The `engine-app/handlers/request_handler.rs` and `common.rs` files use protocol types. This is **acceptable** because:

1. These handlers are the **boundary layer** where protocol↔domain conversion happens
2. The Cargo.toml explicitly allows `engine-app → protocol` dependency  
3. The documented rule is only that `use_cases` must NOT import `ServerMessage`

This is **not** a violation to fix.

---

## Appendix C: Related Documentation

- Original plan: `docs/plans/HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md`
- Cleanup plan: `docs/plans/HEXAGONAL_CLEANUP_PLAN.md`
- Architecture documentation: `docs/architecture/hexagonal-architecture.md`
