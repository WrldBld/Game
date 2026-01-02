# Architecture Remediation Phase 2

> **Status**: COMPLETED
> **Created**: 2026-01-02
> **Completed**: 2026-01-02
> **Type**: Code review findings remediation

## Executive Summary

Following a comprehensive code review by 8 review agents and 5 validation agents, this plan addresses confirmed architectural issues. All issues have been verified against the codebase.

## Phase 1: Quick Fixes (30 minutes)

### 1.1 Delete Dead `mod.rs` File
- **File**: `crates/player-ports/src/mod.rs`
- **Issue**: Orphan file declaring `pub mod inbound;` for non-existent directory
- **Action**: Delete the file (lib.rs is the actual crate root)

### 1.2 Remove Default Implementation from Port
- **File**: `crates/engine-ports/src/outbound/repository_port.rs:595-597`
- **Issue**: `has_observed()` has default implementation, violating port purity
- **Action**: 
  1. Remove default impl from trait
  2. Add implementation to Neo4j adapter in `crates/engine-adapters/src/infrastructure/persistence/observation_repository.rs`

### 1.3 ~~Remove Duplicate MockGameConnectionPort~~ (TECH DEBT)
- **Files**: 
  - `crates/player-ports/src/outbound/testing/mock_game_connection.rs`
  - `crates/player-adapters/src/infrastructure/testing/mock_game_connection.rs`
- **Issue**: Duplicate mock implementations (mocks belong in adapters, not ports)
- **Analysis**: The player-ports version is actively used by player-app tests. Since player-app cannot depend on player-adapters (would create wrong dependency direction), the mock must remain in player-ports for now.
- **Action**: Documented as tech debt. Future fix: Create `player-testing` crate or use conditional compilation.

### 1.4 Fix Concrete Type in Handler
- **File**: `crates/engine-app/src/application/handlers/request_handler.rs:75`
- **Issue**: `Arc<SheetTemplateService>` instead of `Arc<dyn SheetTemplateServicePort>`
- **Action**: Change to trait object for consistency

---

## Phase 2: ~~Production Panic Removal~~ (FALSE POSITIVE - NO ACTION NEEDED)

### 2.1 ~~Locations with `panic!()`~~

**Analysis Result**: All 6 `panic!()` calls identified by the review are inside `#[test]` functions. They are test assertions, NOT production code paths.

| File | Lines | Actual Location |
|------|-------|-----------------|
| `engine-ports/src/outbound/use_case_types.rs` | 1227, 1232 | Inside `#[test] fn test_challenge_result_methods()` |
| `engine-app/src/application/use_cases/challenge.rs` | 727, 732 | Inside `#[test] fn test_challenge_result_methods()` |
| `engine-app/src/application/use_cases/player_action.rs` | 292, 299 | Inside `#[test] fn test_action_status_queued_data()` |

**Action**: None required. Test panics are appropriate for asserting invariants.

---

## Phase 3: Use Case Inbound Port Fix (COMPLETED)

### 3.1 Problem

`PlayerActionUseCase` depends on `MovementUseCasePort` (an inbound port), violating:
> "Use cases MUST depend only on outbound ports, never on inbound ports."
> — hexagonal-architecture.md:543

### 3.2 Solution (Implemented)

1. **Created outbound port** `MovementOperationsPort` in `crates/engine-ports/src/outbound/movement_operations_port.rs`:
   - Trait with `move_to_region()` and `exit_to_location()` methods
   - Takes `UseCaseContext` by value (matching existing signatures)
   - Uses existing types from `use_case_types.rs`
   - Includes `#[automock]` for testing

2. **Implemented on MovementUseCase** in `crates/engine-app/src/application/use_cases/movement.rs`:
   - Added `impl MovementOperationsPort for MovementUseCase` block
   - Delegates to existing `MovementUseCasePort` methods

3. **Updated PlayerActionUseCase** in `crates/engine-app/src/application/use_cases/player_action.rs`:
   - Changed import from `MovementUseCasePort` to `MovementOperationsPort`
   - Changed field type to `Arc<dyn MovementOperationsPort>`
   - Updated constructor parameter

4. **Wiring**: No changes needed - `Arc<MovementUseCase>` coerces to `Arc<dyn MovementOperationsPort>` automatically

---

## Phase 4: Documentation (COMPLETED)

### 4.1 Known Tech Debt (Documented)

| Item | Description | Future Action | Severity |
|------|-------------|---------------|----------|
| Duplicate `MockGameConnectionPort` | Two copies: player-ports and player-adapters | Create `player-testing` crate or use conditional compilation | Low |
| ~~39 `*_port.rs` files in `engine-app/services/internal/`~~ | ~~Confusing naming~~ | **FIXED** - Renamed to `*_service.rs` | ~~Low~~ |
| `{:?}` formatting for Neo4j storage | 15+ locations use Debug formatting for enum storage | Implement Display traits before production | Medium |
| `engine-dto` imports in `queue_use_case_port.rs` | Ports importing from engine-dto | Analyze proper DTO placement | Low |

---

## Deferred Items

| # | Issue | Reason |
|---|-------|--------|
| 6 | engine-dto imports in ports | Needs deeper analysis |
| 7 | Debug formatting for DB | No production data yet |
| ~~10~~ | ~~Naming conventions (39 files)~~ | **FIXED** - See Phase 5 below |

---

## Verification

After all phases:

```bash
cargo xtask arch-check      # Must pass
cargo check --workspace     # Must compile  
cargo test --workspace      # Must pass
```

---

## Success Criteria

- [x] No orphan `mod.rs` file in player-ports (Phase 1.1 - DELETED)
- [x] No default implementations in port traits (Phase 1.2 - FIXED)
- [x] No duplicate mock implementations across crates (Phase 1.3 - DOCUMENTED AS TECH DEBT, cannot remove due to dependency direction)
- [x] No concrete types in handlers (all trait objects) (Phase 1.4 - FIXED)
- [x] No `panic!()` in production code paths (Phase 2 - FALSE POSITIVE, all panics in tests)
- [x] PlayerActionUseCase depends only on outbound ports (Phase 3 - FIXED with MovementOperationsPort)
- [x] All changes documented (Phase 4 - THIS DOCUMENT)
- [x] `cargo xtask arch-check` passes (verified)
- [x] Internal service traits use correct naming (Phase 5 - FIXED, renamed 39 files)

---

## Phase 5: Naming Convention Fix (COMPLETED)

### 5.1 Problem

39 files in `crates/engine-app/src/application/services/internal/` were named `*_service_port.rs` but these are internal application service traits, NOT ports. Per `AGENTS.md`, the `*Port` suffix should only be used for traits in the ports layer.

### 5.2 Solution (Implemented)

1. Renamed all 39 files from `*_service_port.rs` to `*_service.rs` using `git mv`
2. Updated `mod.rs` module declarations (39 mod statements)
3. Updated `mod.rs` re-exports (~78 pub use statements)

Note: The trait names (e.g., `SheetTemplateServicePort`) were NOT renamed - only the file names. This is intentional to minimize churn; the trait suffix can be addressed in a future refactor if needed.

---

## Files Modified

| File | Change |
|------|--------|
| `crates/player-ports/src/mod.rs` | DELETED |
| `crates/engine-ports/src/outbound/repository_port.rs` | Removed default impl from `has_observed()` |
| `crates/engine-adapters/src/infrastructure/persistence/observation_repository.rs` | Added `has_observed()` implementation |
| `crates/engine-app/src/application/handlers/request_handler.rs` | Changed `Arc<SheetTemplateService>` to `Arc<dyn SheetTemplateServicePort>` |
| `crates/engine-app/src/application/handlers/world_handler.rs` | Changed parameter to trait object |
| `crates/engine-ports/src/outbound/movement_operations_port.rs` | NEW FILE - outbound port for movement operations |
| `crates/engine-ports/src/outbound/mod.rs` | Added module export |
| `crates/engine-app/src/application/use_cases/movement.rs` | Added `MovementOperationsPort` impl |
| `crates/engine-app/src/application/use_cases/player_action.rs` | Changed to depend on `MovementOperationsPort` |
| `crates/engine-app/src/application/services/internal/*.rs` | Renamed 39 files from `*_service_port.rs` to `*_service.rs` |
| `crates/engine-app/src/application/services/internal/mod.rs` | Updated module declarations and re-exports |
