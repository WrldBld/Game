# Abstraction Review Remediation Plan

Status: Active
Owner: TBD
Last updated: 2026-01-17

## Current Status

**Build Status:** PASSING
- `cargo check --workspace` - passes
- `cargo clippy --workspace -- -D warnings` - passes

**Migration Status:** Phase 3 substantially complete
- 10 repository files remain as deprecated facades pending full migration
- Documentation (AGENTS.md, ADR-009) updated to reflect full elimination as target architecture

## Summary

This plan addresses findings from a combined architecture review focused on:
1. **Abstraction assessment** - Is the codebase over-abstracted?
2. **Concrete issues** - Security, error handling, naming violations

**Overall verdict:** The codebase is **not in abstraction hell**. Domain, use cases, and player crate are well-architected. The **repository layer is the primary problem area** - it adds unnecessary indirection and contains misplaced business logic.

## Severity Summary

| Severity | Count | Key Issues |
|----------|-------|------------|
| CRITICAL | 1 | JSON escaping vulnerability |
| HIGH | 5 | Business logic in repositories, string error codes |
| MEDIUM | 8 | Missing error context, consistency issues |
| LOW | 6 | Naming, minor type improvements |
| ARCHITECTURAL | 1 | Repository layer over-abstraction |

---

## Phase 1: Immediate (Before Next PR)

### 1.1 Fix Clippy Dead Code Errors

**Severity:** HIGH (blocks CI)

| File | Issue |
|------|-------|
| `shared/src/game_systems/dnd5e.rs:1406` | `default_allocation_system` never used |
| `shared/src/game_systems/pbta.rs:550` | `stat_array_allocation` never used |

**Remediation:**
- Remove dead code or add `#[allow(dead_code)]` with justification comment
- Run `cargo clippy --workspace -- -D warnings` to verify

**Acceptance criteria:**
- `cargo clippy --workspace -- -D warnings` passes

---

### 1.2 Fix JSON Escaping in Tag Search

**Severity:** CRITICAL (data integrity/potential injection)

**File:** `crates/engine/src/infrastructure/neo4j/lore_repo.rs:476`

```rust
// CURRENT: Vulnerable to malformed JSON if tag contains quotes
q = q.param(&format!("tag{}", i), format!("\"{}\"", tag));

// FIX: Use proper JSON escaping
q = q.param(&format!("tag{}", i), serde_json::to_string(tag).unwrap_or_default());
```

**Acceptance criteria:**
- Tags containing quotes (`foo"bar`) are properly escaped
- Unit test added for tag search with special characters

---

### 1.3 Replace String Error Codes with Typed ErrorCode

**Severity:** HIGH (inconsistent error handling)

**File:** `crates/engine/src/api/websocket/mod.rs`

| Line | Current | Should Be |
|------|---------|-----------|
| 289 | `"PARSE_ERROR"` | `ErrorCode::BadRequest` |
| 695 | `"UNKNOWN_MESSAGE"` | `ErrorCode::BadRequest` |
| 704 | `"NOT_IMPLEMENTED"` | Add `ErrorCode::NotImplemented` to enum |

**Remediation:**
1. Add `NotImplemented` variant to `ErrorCode` enum in `shared/src/lib.rs` (or appropriate location)
2. Replace all string literals with typed enum usage
3. Search for other string error codes: `rg '"[A-Z_]+"' crates/engine/src/api/`

**Acceptance criteria:**
- No hardcoded string error codes in `api/websocket/`
- All error responses use `ErrorCode` enum

---

## Phase 2: Short-term (This Sprint)

### 2.1 Move Business Logic from Repositories to Use Cases

**Severity:** HIGH (architecture violation)

This is the **most impactful change** - repositories currently contain use-case-level orchestration.

#### 2.1.1 Scene Resolution

**Source:** `crates/engine/src/repositories/scene.rs`
**Methods:** `resolve_scene()`, `check_time_context()`, `evaluate_conditions()` (lines 237-380)

**Target:** `crates/engine/src/use_cases/scene/resolve_scene.rs`

**Remediation:**
1. Create `use_cases/scene/resolve_scene.rs` with `ResolveScene` use case
2. Move `SceneResolutionContext`, `SceneResolutionResult`, `SceneConsideration` types
3. Move resolution logic, inject `SceneRepository` for data access only
4. Update callers (likely `EnterRegion` use case)

**Acceptance criteria:**
- `SceneRepository` has only CRUD methods
- Scene resolution logic lives in `use_cases/scene/`

#### 2.1.2 Inventory Operations

**Source:** `crates/engine/src/repositories/inventory.rs`
**Methods:** `equip_item()`, `unequip_item()`, `drop_item()`, `pickup_item()`, `give_item_to_pc()` (lines 92-280)

**Target:** `crates/engine/src/use_cases/inventory/`

**Remediation:**
1. Create `use_cases/inventory/` module with individual use cases:
   - `equip_item.rs`
   - `drop_item.rs`
   - `pickup_item.rs`
   - `transfer_item.rs`
2. Move `InventoryActionResult`, `InventoryError` types
3. `InventoryRepository` becomes pure data access (or eliminated - see Phase 3)

**Acceptance criteria:**
- `InventoryRepository` has no methods with business logic
- Inventory operations are proper use cases with clear inputs/outputs

#### 2.1.3 Exit Resolution

**Source:** `crates/engine/src/repositories/location.rs`
**Methods:** `get_exits()`, `can_move_to()` (lines 181-276)

**Target:** `crates/engine/src/use_cases/movement/get_region_exits.rs`

**Remediation:**
1. Create `GetRegionExits` use case
2. Move `RegionExit`, `RegionExitsResult`, `SkippedExit` types
3. Location repository becomes pure CRUD

#### 2.1.4 Time Operations (Duplicate Logic)

**Source:** `crates/engine/src/repositories/world.rs`
**Methods:** `advance_time()`, `set_time()`, `set_time_mode()` (lines 63-107)

**Note:** These duplicate logic already in `use_cases/time/`. 

**Remediation:**
1. Remove duplicate methods from `WorldRepository`
2. Ensure `use_cases/time/` is the single source of truth
3. Update any callers to use the use case

---

### 2.2 Add Error Context to Use Case Errors

**Severity:** MEDIUM (debugging difficulty)

Multiple error types lack entity IDs for debugging:

| File | Error Type | Missing Context |
|------|------------|-----------------|
| `use_cases/staging/mod.rs:87-96` | `StagingError::WorldNotFound` | `WorldId` |
| `use_cases/conversation/start.rs:169-190` | `ConversationError::NpcNotFound` | `CharacterId` |
| `use_cases/challenge/mod.rs:767-786` | `ChallengeError::NotFound` | `ChallengeId` |
| `use_cases/movement/enter_region.rs:271-286` | `EnterRegionError::RegionNotFound` | `RegionId` |
| `use_cases/management/mod.rs:33-42` | `ManagementError::NotFound` | Entity type AND ID |

**Remediation pattern:**
```rust
// BEFORE
#[error("World not found")]
WorldNotFound,

// AFTER
#[error("World not found: {0}")]
WorldNotFound(WorldId),
```

**Acceptance criteria:**
- All `*NotFound` error variants include the ID that wasn't found
- `ManagementError::NotFound` includes entity type and ID

---

---

## Phase 3: Repository Layer Elimination (ADR-009)

**Decision:** APPROVED - Eliminate the repository wrapper layer.

See [ADR-009](../architecture/ADR-009-repository-layer-elimination.md) for full rationale.

### 3.1 Rename In-Memory Stores

Move in-memory state management out of `repositories/` before deletion:

```
repositories/session.rs        → stores/session.rs
repositories/pending_staging.rs → stores/pending_staging.rs
repositories/directorial.rs    → stores/directorial.rs
repositories/time_suggestion.rs → stores/time_suggestion.rs
```

**Acceptance criteria:**
- `stores/` directory exists with all 4 files
- Files renamed to `*Store` pattern where needed
- Old locations deleted

### 3.2 Update Use Cases to Inject Ports Directly

For each use case file, change:

```rust
// BEFORE
use crate::repositories::CharacterRepository;

pub struct MyUseCase {
    character: Arc<CharacterRepository>,
}

impl MyUseCase {
    pub fn new(character: Arc<CharacterRepository>) -> Self { ... }
}

// AFTER
use crate::infrastructure::ports::CharacterRepo;

pub struct MyUseCase {
    character: Arc<dyn CharacterRepo>,
}

impl MyUseCase {
    pub fn new(character: Arc<dyn CharacterRepo>) -> Self { ... }
}
```

**Files to update (~50 use case files):**
- All files in `use_cases/movement/`
- All files in `use_cases/conversation/`
- All files in `use_cases/challenge/`
- All files in `use_cases/staging/`
- All files in `use_cases/narrative/`
- All files in `use_cases/management/`
- All files in `use_cases/time/`
- All files in `use_cases/lore/`
- All files in `use_cases/queues/`
- All files in `use_cases/visual_state/`
- All files in `use_cases/assets/`
- All files in `use_cases/world/`
- And others...

### 3.3 Update App Composition Root

**File:** `crates/engine/src/app.rs`

Change from creating repository wrappers to passing ports directly:

```rust
// BEFORE
let character = Arc::new(repositories::CharacterRepository::new(repos.character.clone()));
let movement = MovementUseCases::new(EnterRegion::new(character.clone(), ...));

// AFTER
let character: Arc<dyn CharacterRepo> = repos.character.clone();
let movement = MovementUseCases::new(EnterRegion::new(character.clone(), ...));
```

### 3.4 Delete Repository Files

After all use cases updated, delete pure-delegation repositories:

**Delete these files:**
- `repositories/character.rs` (313 lines)
- `repositories/player_character.rs` (95 lines)
- `repositories/act.rs` (34 lines)
- `repositories/content.rs` (40 lines)
- `repositories/interaction.rs` (40 lines)
- `repositories/goal.rs` (44 lines)
- `repositories/challenge.rs` (69 lines)
- `repositories/staging.rs` (219 lines)
- `repositories/region_state.rs` (68 lines)
- `repositories/location_state.rs` (71 lines)
- `repositories/clock.rs` (20 lines)
- `repositories/random.rs` (24 lines)
- `repositories/llm.rs` (28 lines)
- `repositories/queue.rs` (125 lines)
- `repositories/lore.rs` (126 lines) - after logic extracted
- `repositories/narrative.rs` (291 lines) - after logic extracted
- `repositories/scene.rs` (381 lines) - after logic extracted (Phase 2)
- `repositories/location.rs` (277 lines) - after logic extracted (Phase 2)
- `repositories/inventory.rs` (358 lines) - after logic extracted (Phase 2)
- `repositories/observation.rs` (133 lines) - after logic extracted (Phase 2)
- `repositories/world.rs` (108 lines) - after logic extracted (Phase 2)
- `repositories/flag.rs` (105 lines) - after logic extracted

**Keep these files:**
- `repositories/settings.rs` - Has caching logic
- `repositories/assets.rs` - Coordinates 2 ports (AssetRepo + ImageGenPort)

**Update `repositories/mod.rs`** to only export remaining items.

### 3.5 Update Tests

Tests that mock repositories need to mock port traits instead:

```rust
// BEFORE
let mut mock_repo = MockCharacterRepository::new();

// AFTER  
let mut mock_repo = MockCharacterRepo::new();
```

**Acceptance criteria:**
- All tests pass
- No references to deleted repository types

---

## Phase 4: Polish (Backlog)

### 4.1 Standardize Aggregate Mutation Return Types

**Severity:** MEDIUM (consistency)

**Issue:** Inconsistent patterns across aggregates:

| Aggregate | Mutation Pattern |
|-----------|------------------|
| `Character` | Returns domain events (correct) |
| `Location` | Setters return `()` |
| `World` | Mixed |

**Remediation:**
- Setters for simple properties can return `()` (per ADR-008)
- State transitions and complex mutations should return events
- Document the decision criteria in AGENTS.md

---

### 4.2 Fix Lost Error Context Patterns

**Severity:** MEDIUM (debugging difficulty)

| File | Line | Pattern |
|------|------|---------|
| `api/websocket/mod.rs` | 854 | `Uuid::parse_str(id_str).map_err(|_| ...)` |
| `api/websocket/ws_creator.rs` | 592, 606 | `.map_err(|_| ServerMessage::Response...` |
| `api/websocket/ws_core.rs` | 1013 | `.map_err(|_| ServerMessage::Response...` |

**Remediation:**
```rust
// BEFORE: Lost context
Uuid::parse_str(id_str).map_err(|_| SomeError::InvalidId)?;

// AFTER: Preserved context
Uuid::parse_str(id_str).map_err(|e| SomeError::InvalidId { 
    value: id_str.to_string(), 
    source: e 
})?;
```

---

### 4.3 Domain Type Consistency

**Severity:** LOW

| Issue | Location | Recommendation |
|-------|----------|----------------|
| `PlayerCharacter.description` is `Option<String>` | `domain/src/aggregates/player_character.rs:99` | Change to `Option<Description>` |
| Boolean flags in entities | Various | Evaluate case-by-case per ADR-008 |

**Note:** Per ADR-008 (Tiered Encapsulation), not all fields need newtypes. Only change where validation adds value.

---

### 4.4 Clean Up Player Crate Dead Code

**Severity:** LOW

**File:** `crates/player/src/` - `apply_generation_read_state` marked `#[allow(dead_code)]`

**Remediation:** Remove if unused, or document why it's kept.

---

### 4.5 Consider Splitting `ports.rs`

**Severity:** LOW (maintainability)

**File:** `crates/engine/src/infrastructure/ports.rs` (1448 lines)

**Suggested structure:**
```
infrastructure/
  ports/
    mod.rs           # Re-exports
    repos.rs         # 20 repository traits
    services.rs      # LlmPort, ImageGenPort, QueuePort
    testability.rs   # ClockPort, RandomPort
```

---

## Excluded Items (Explicitly Accepted)

### E1: Port Trait Count (25 vs ~10 guideline)

**Reason:** The 20 repository traits map 1:1 to domain entities. The 5 non-repository ports (LLM, ImageGen, Queue, Clock, Random) align with the ~10 guideline. This is appropriate for the domain model size.

### E2: `SheetValue` Import in Use Cases

**File:** `use_cases/narrative/execute_effects.rs:12`

**Reason:** Per AGENTS.md, `shared` crate contains game system types needed by both Player and Engine. This is documented and intentional.

### E3: Boolean Flags in Simple Entities

**Examples:** `is_unique`, `is_favorite`, `is_locked`

**Reason:** Per ADR-008, simple binary states without invalid combinations are acceptable as booleans. Enums would add ceremony without value.

---

## Verification Commands

```bash
# Phase 1 verification
cargo clippy --workspace -- -D warnings
cargo test --workspace

# Search for string error codes
rg '"[A-Z_]+"' crates/engine/src/api/websocket/

# Phase 2 verification - business logic moved out of repositories
rg -l "pub async fn" crates/engine/src/repositories/ | xargs wc -l
# Should be significantly smaller after Phase 2

# Phase 3 verification - repository elimination complete
# Should show NO imports from repositories (except settings, assets, stores)
rg "use crate::repositories::" crates/engine/src/use_cases/ --type rust

# Use cases should import from ports directly
rg "Arc<dyn.*Repo>" crates/engine/src/use_cases/ --type rust
# Should show many matches (port traits injected directly)

# No repository wrapper types in use cases
rg "Repository>" crates/engine/src/use_cases/ --type rust
# Should show zero matches after Phase 3

# E2E tests
E2E_LLM_MODE=playback cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
```

---

## Progress Tracking

| Phase | Item | Status | Owner | Notes |
|-------|------|--------|-------|-------|
| 1.1 | Clippy dead code | DONE | | All errors fixed, clippy passes |
| 1.2 | JSON escaping | DONE | | Was already fixed previously |
| 1.3 | Typed error codes | DONE | | Was already fixed previously |
| 2.1.1 | Scene resolution → use case | PARTIAL | | Logic moved, facade file remains |
| 2.1.2 | Inventory operations → use cases | PARTIAL | | Logic moved, facade file remains |
| 2.1.3 | Exit resolution → use case | PARTIAL | | Logic moved, facade file remains |
| 2.1.4 | Time operations (remove dupes) | PARTIAL | | Logic moved, facade file remains |
| 2.2 | Error context in use cases | DONE | | Added IDs to all NotFound variants |
| 3.1 | Rename in-memory stores | DONE | | stores/ directory created |
| 3.2 | Update use cases (port injection) | DONE | | All use cases now inject Arc<dyn *Repo> |
| 3.3 | Update App composition | DONE | | |
| 3.4 | Delete repository files | PARTIAL | | 4 store re-exports deleted, 10 deprecated facades remain |
| 3.5 | Update tests | DONE | | Tests mock port traits directly |
| 4.1 | Mutation return types | TODO | | |
| 4.2 | Lost error context | TODO | | |
| 4.3 | Domain type consistency | TODO | | |
| 4.4 | Player dead code | TODO | | |
| 4.5 | Split ports.rs | TODO | | |
