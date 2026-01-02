# Known Architecture Issues (For Later Remediation)

> **Status**: Tracking document for pre-existing architecture issues
> **Created**: 2026-01-02
> **Priority**: Low (technical debt, not blocking)

This document tracks known architecture issues that are pre-existing (not introduced by recent
refactoring) and should be addressed in future cleanup work when time permits.

## Issues in Ports Layer

### 1. ~~Blanket Implementation in player-ports~~ (RESOLVED - Intentional Pattern)

**Location**: `crates/player-ports/src/outbound/game_connection/*.rs`

**Status**: ACCEPTED AS INTENTIONAL

**Rationale**: The 6 blanket impls (`GameRequestPort`, `PlayerActionPort`, `NavigationPort`,
`DmControlPort`, `SessionCommandPort`, `ConnectionLifecyclePort`) enable the ISP (Interface
Segregation Principle) pattern for the player-side WebSocket connection:

- `GameConnectionPort` is the "god trait" that adapters implement (50+ methods)
- The 6 sub-traits provide focused interfaces that services can depend on
- Blanket impls ensure any `GameConnectionPort` implementation automatically provides all sub-traits
- This is pure delegation with no business logic

Moving these to adapters would break encapsulation and add complexity without benefit.
This is an acceptable exception documented here.

---

### 2. Mock Implementation in player-ports

**Location**: `crates/player-ports/src/outbound/testing/mock_game_connection.rs` (413 lines)

**Issue**: Full mock implementation exists in ports layer. Mocks belong in adapters or test
utilities, not in ports.

**History**: Was moved back from adapters in commit `ccc901e` (likely for practical reasons).

**Impact**: Low - test infrastructure only.

**Recommended Fix**: Move to `player-adapters/infrastructure/testing/` or a dedicated test
utilities crate.

---

### 3. ~~Large File with Implementations~~ (RESOLVED - Acceptable Pattern)

**Location**: `crates/engine-ports/src/outbound/use_case_types.rs` (1,283 lines)

**Status**: ACCEPTED AS INTENTIONAL

**Analysis**:
- The `From<StagingApprovalSource> for StagingSource` impl cannot be moved due to Rust's orphan
  rules - it must be in the crate that owns one of the types (engine-ports owns `StagingApprovalSource`)
- The `ErrorCode` trait impls are trivial match statements with no business logic
- The file size (1,283 lines) is large but contains many small related DTOs

**Future consideration**: The file could be split into focused modules (movement_types.rs,
staging_types.rs, etc.) but this is a low-priority refactoring task.

---

## Issues in Adapters Layer

### 4. `Utc::now()` Fallback in Repositories

**Locations**:
- `crates/engine-adapters/src/infrastructure/persistence/event_chain_repository.rs:405`
- `crates/engine-adapters/src/infrastructure/persistence/narrative_event_repository/common.rs:52`
- `crates/engine-adapters/src/infrastructure/persistence/story_event_repository/common.rs:22`

**Issue**: Uses `Utc::now()` as fallback for datetime parsing instead of injecting via ClockPort.

**Impact**: Low - only affects fallback paths when parsing fails; primary timestamp creation
correctly uses ClockPort.

**Recommended Fix**: Pass `&dyn ClockPort` to helper functions or convert to repository methods.

---

### 5. Crate Aliasing

**Locations**:
- `crates/player-adapters/src/infrastructure/session_type_converters.rs:10`
- `crates/engine-adapters/src/infrastructure/websocket/approval_converters.rs:14`

**Issue**: Uses `use wrldbldr_protocol as proto;` which violates the "no crate aliasing" rule.

**Impact**: Low - purely stylistic, arch-check would flag this.

**Recommended Fix**: Replace with direct imports: `use wrldbldr_protocol::{...};`

---

## Naming Inconsistencies

### 6. Internal Traits Named `*ServicePort`

**Location**: 28 traits in `crates/engine-app/src/application/services/internal/`

**Issue**: Traits like `CharacterServicePort` use `Port` suffix but are NOT hexagonal ports -
they're internal app-layer contracts.

**Impact**: Medium - can cause confusion with actual ports in `engine-ports`.

**Status**: Documented as intentional in `internal/mod.rs`. The `Port` suffix means "injectable
dependency" in this context.

**Recommended Fix** (later): Consider renaming to `*Service` (drop `Port` suffix) for clarity.

---

### 7. ~~Mixed Naming: `StagingUseCaseServicePort`~~ (RESOLVED)

**Location**: `crates/engine-ports/src/outbound/staging_use_case_service_ports.rs`

**Status**: FIXED

**Resolution**: Renamed to clearer names:
- `StagingUseCaseServicePort` → `StagingQueryPort` (query operations)
- `StagingUseCaseServiceExtPort` → `StagingMutationPort` (mutation operations)

Backwards compatibility aliases maintained in `mod.rs` for gradual migration.

---

## Summary

| Issue | Location | Impact | Priority |
|-------|----------|--------|----------|
| Blanket impl in ports | player-ports | Low | Later |
| Mock impl in ports | player-ports | Low | Later |
| Large file + impls | engine-ports | Low | Later |
| `Utc::now()` fallback | engine-adapters | Low | Later |
| Crate aliasing | adapters | Low | Later |
| Internal `*ServicePort` naming | engine-app | Medium | Later |
| Mixed naming | engine-ports | Low | Later |

None of these issues are blocking or cause runtime problems. They represent technical debt
that can be addressed during future cleanup sprints.
