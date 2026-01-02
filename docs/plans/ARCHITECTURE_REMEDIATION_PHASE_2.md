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

## Phase 3: Use Case Inbound Port Fix (1 hour)

### 3.1 Problem

`PlayerActionUseCase` depends on `MovementUseCasePort` (an inbound port), violating:
> "Use cases MUST depend only on outbound ports, never on inbound ports."
> — hexagonal-architecture.md:543

### 3.2 Solution

1. **Create outbound port** `MovementOperationsPort` in `crates/engine-ports/src/outbound/movement_operations_port.rs`:
   ```rust
   #[async_trait]
   pub trait MovementOperationsPort: Send + Sync {
       async fn move_to_region(&self, ctx: &UseCaseContext, input: MoveToRegionInput) -> Result<MovementResult, MovementError>;
       async fn exit_to_location(&self, ctx: &UseCaseContext, input: ExitToLocationInput) -> Result<MovementResult, MovementError>;
   }
   ```

2. **Implement on MovementUseCase** in `crates/engine-app/src/application/use_cases/movement.rs`

3. **Update PlayerActionUseCase** to depend on `Arc<dyn MovementOperationsPort>`

4. **Update wiring** in `engine-runner`

---

## Phase 4: Documentation (15 minutes)

### 4.1 Known Tech Debt to Document

Add to `AGENTS.md` or create tech debt tracking:

| Item | Description | Future Action |
|------|-------------|---------------|
| 39 `*_port.rs` files in `engine-app/services/internal/` | Confusing naming (these are internal service traits, not ports) | Rename to `*_service.rs` in dedicated refactor |
| `{:?}` formatting for Neo4j storage | 15+ locations use Debug formatting for enum storage | Implement Display traits before production |
| `engine-dto` imports in `queue_use_case_port.rs` | Ports importing from engine-dto | Analyze proper DTO placement |

---

## Deferred Items

| # | Issue | Reason |
|---|-------|--------|
| 6 | engine-dto imports in ports | Needs deeper analysis |
| 7 | Debug formatting for DB | No production data yet |
| 10 | Naming conventions (39 files) | High churn, low immediate value |

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

- [ ] No orphan `mod.rs` file in player-ports
- [ ] No default implementations in port traits
- [ ] No duplicate mock implementations across crates
- [ ] No concrete types in handlers (all trait objects)
- [ ] No `panic!()` in production code paths
- [ ] PlayerActionUseCase depends only on outbound ports
- [ ] All changes documented
- [ ] `cargo xtask arch-check` passes
