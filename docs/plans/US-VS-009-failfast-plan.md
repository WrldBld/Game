**Created:** January 22, 2026
**Status:** 🟢 COMPLETED
**Owner:** OpenCode
**Scope:** US-VS-009 fail-fast visual state flow

---

## Goal

Implement a fail-fast, no-backcompat visual state flow so players always receive an approved visual state (or the action fails with a clear error). Remove silent fallbacks and deprecated defaults.

## Progress Tracking

- [x] P1: Enforce strict visual_state_source parsing (no defaults)
- [x] P2: Reject invalid visual state IDs at API boundary
- [x] P3: Resolve and persist default visual state IDs
- [x] P4: Make staging + active-state updates atomic
- [x] P5: Fail-fast visual state lookup in movement
- [x] P6: Wire DM staging UI to actual state options
- [x] P7: Add tests for fail-fast behavior
- [x] P8: Re-run checks/tests

---

## Implementation Details

### P1: Enforce strict visual_state_source parsing (no defaults)

**Why:** Remove legacy/backcompat defaults and fail on incomplete staging data.

**Files:**
- `crates/engine/src/infrastructure/neo4j/staging_repo.rs`

**Changes:**
- In `row_to_staging` and `node_to_staging`, remove any fallback defaults for `visual_state_source`.
- If `visual_state_source` is missing or invalid, return a `RepoError::database` with a clear message.

**Acceptance:**
- Missing `visual_state_source` causes staging load to fail (InternalError at API layer).

---

### P2: Reject invalid visual state IDs at API boundary

**Why:** Fail fast for malformed IDs; don't silently drop DM overrides.

**Files:**
- `crates/engine/src/api/websocket/ws_staging.rs`
- `crates/engine/src/use_cases/staging/approve.rs`

**Changes:**
- Parse `location_state_id` / `region_state_id` as UUIDs in the handler.
- Return `ErrorCode::ValidationError` to DM on invalid IDs.
- Use case accepts typed IDs or validated strings only.

**Acceptance:**
- Invalid IDs never reach the use case and produce ValidationError.

---

### P3: Resolve and persist default visual state IDs

**Why:** "Default" must produce deterministic visual state for players.

**Files:**
- `crates/engine/src/use_cases/staging/approve.rs`

**Changes:**
- When no IDs are provided, resolve active location/region states.
- Persist resolved IDs into the staging record.
- If no active states exist, return `StagingError::Validation` (ValidationError to DM).

**Acceptance:**
- "Default" approvals always store valid IDs or fail validation.

---

### P4: Make staging + active-state updates atomic

**Why:** Avoid partial state updates when saving staging fails.

**Files:**
- `crates/engine/src/use_cases/staging/approve.rs`
- `crates/engine/src/infrastructure/neo4j/staging_repo.rs`
- `crates/engine/src/infrastructure/neo4j/location_state_repo.rs`
- `crates/engine/src/infrastructure/neo4j/region_state_repo.rs`

**Changes:**
- Move location/region `set_active` calls into the same Neo4j transaction as `save_and_activate_pending_staging`.
- Add a new repo method if needed to update active states within the same transaction.

**Acceptance:**
- If any operation fails, no staging or active state changes persist.

---

### P5: Fail-fast visual state lookup in movement

**Why:** Missing/invalid state IDs should block movement instead of silently clearing visual state.

**Files:**
- `crates/engine/src/use_cases/movement/mod.rs`

**Changes:**
- `resolve_visual_state_from_staging` returns `Result<Option<ResolvedVisualState>, RepoError>`.
- Propagate errors through `resolve_staging_for_region` so movement fails with InternalError.

**Acceptance:**
- Movement fails when visual state lookup fails.

---

### P6: Wire DM staging UI to actual state options

**Why:** Ensure DM selections are real and validated.

**Files:**
- `crates/player/src/ui/presentation/components/dm_panel/staging_approval.rs`

**Changes:**
- Populate dropdowns from `available_location_states` / `available_region_states`.
- Preselect `resolved_visual_state` where applicable.
- Disable approve if selection invalid or missing when required.

**Acceptance:**
- UI selection uses actual options and matches server validation.

---

### P7: Add tests for fail-fast behavior

**Why:** Prevent regressions for critical error paths.

**Files:**
- `crates/engine/src/use_cases/staging/tests.rs` (or module-specific tests)
- `crates/engine/src/use_cases/movement/tests.rs`

**Tests:**
- Invalid visual state IDs return ValidationError.
- Default visual state with no active states fails validation.
- Missing `visual_state_source` fails staging load.

---

### P8: Re-run checks/tests

**Commands:**
- `cargo check -p wrldbldr-engine`
- `cargo test -p wrldbldr-engine --lib use_cases::movement`
- `cargo check -p wrldbldr-player`

---

## Post-Review Fixes

These fixes address feedback from the code review to ensure fail-fast behavior is complete.

### Task 1: Ensure activation queries verify state nodes exist
**Issue:** Active state updates could be silent no-ops when state nodes are missing.

**Files:**
- `crates/engine/src/infrastructure/neo4j/staging_repo.rs`

**Changes:**
- Modified `save_and_activate_pending_staging_with_states` to use `txn.execute()` instead of `txn.run()`.
- Added `RETURN count(*) as rows_affected` to all activation query variants.
- Check row count after execution and return `RepoError::database` if zero rows affected.
- Error message includes both `location_state_id` and `region_state_id` for debugging.

**Acceptance:**
- [x] Activation queries return row count
- [x] Zero row count returns RepoError with clear message
- [x] Both state IDs included in error for debugging

### Task 2: Treat active staging with no visual state IDs as data integrity error
**Issue:** Active staging without visual state IDs returned `Ok(None)` instead of failing.

**Files:**
- `crates/engine/src/use_cases/movement/mod.rs`

**Changes:**
- Added data integrity check in `resolve_staging_for_region` when active staging exists.
- If both `location_state_id` and `region_state_id` are `None`, return `RepoError::database`.
- Error message clearly states this is a data integrity issue - staging was approved without resolving visual state IDs.
- Includes staging ID in error message for debugging.

**Acceptance:**
- [x] Active staging with no visual state IDs returns error
- [x] Error message indicates data integrity issue
- [x] Staging ID included in error for debugging
- [x] Tests added for this behavior

### Task 3: Build visual state response from resolved IDs directly
**Issue:** `build_visual_state_for_staging` used `get_active()` which could return different states than what was approved.

**Files:**
- `crates/engine/src/use_cases/staging/approve.rs`

**Changes:**
- Changed `build_visual_state_for_staging` to accept `resolved_location_state_id` and `resolved_region_state_id` as parameters.
- Fetch states by ID using `location_state_repo.get(id)` and `region_state_repo.get(id)`.
- Return `RepoError::not_found` if resolved ID's entity doesn't exist (InternalError, not ValidationError).
- Updated caller in `execute()` to pass resolved IDs from `resolve_visual_state_ids()`.
- If both IDs are None, return `StagingError::Repo(RepoError::database(...))` - data integrity error.

**Acceptance:**
- [x] Method accepts resolved IDs as parameters
- [x] Fetches states by ID, not via get_active()
- [x] Returns RepoError::not_found if entity missing (InternalError)
- [x] Returns error when both IDs are None (fail-fast, no backcompat)
- [x] Tests added for this behavior

### Tests Added

**Movement tests (`crates/engine/src/use_cases/movement/tests/failfast_tests.rs`):**
- [x] `test_active_staging_no_visual_state_ids_returns_error` - Validates data integrity error for active staging without visual state IDs

**Staging tests (`crates/engine/src/use_cases/staging/tests/failfast_tests.rs`):**
- [x] `test_visual_state_fetch_by_id_returns_not_found_on_missing` - Validates RepoError::not_found when resolved state ID's entity doesn't exist
- [x] `test_visual_state_fetch_validates_both_states_exist` - Validates both location and region states are checked

---

## Notes

- No backward compatibility is required. Missing/invalid data is treated as error.
- All visual state flows fail-fast: at least one visual state ID must be present for all approvals (manual and auto).
- Error mapping: validation issues → `ValidationError`; all other failures → `InternalError`.
- Post-review fixes ensure fail-fast behavior is complete at all layers (repo, use case, API).

## Implementation Summary

See [US-VS-009-implementation-summary.md](./US-VS-009-implementation-summary.md) for detailed changes made to each task.

