# Architecture Violation Remediation Plan

> **Status**: COMPLETE
> **Created**: 2026-01-02
> **Completed**: 2026-01-02
> **Type**: Critical architecture fix + pre-existing tech debt

## Executive Summary

An agent code review identified architectural violations, including one **caused by our recent hexagonal refactor** (commits `2bbb3e7`, `0f6c56d`, `cd69808`). This plan addresses:

1. **CRITICAL (our refactor)**: `engine-adapters` → `engine-app` dependency violation - **COMPLETE**
2. **CRITICAL (pre-existing)**: DTO duplication between `protocol` and `engine-dto` - **COMPLETE**
3. **HIGH (pre-existing)**: Missing `#[serde(other)]` for forward compatibility - **COMPLETE**
4. **HIGH (pre-existing)**: `SuggestionContext` missing `world_id` in protocol - **COMPLETE**
5. **MEDIUM (pre-existing)**: `DmActionQueuePlaceholder` no-op implementation - **COMPLETE**

---

## Phase 1: Fix engine-adapters → engine-app Dependency (CRITICAL) - COMPLETE

> **Status**: COMPLETE
> **Completed**: 2026-01-02

### Summary of Changes Made

All architectural violations in `engine-adapters` have been fixed. The crate no longer depends on `engine-app`.

### Step 1.1: Fix `StagingServiceAdapter` - COMPLETE

**Solution Applied**: Made `StagingService` directly implement `StagingUseCaseServiceExtPort`, eliminating the need for the wrapper adapter.

**Files Changed**:
- Deleted `crates/engine-adapters/src/infrastructure/staging_port_adapters.rs`
- Updated `engine-composition` and `engine-runner` references

---

### Step 1.2: Fix `SuggestionEnqueueAdapter` - COMPLETE

**Solution Applied**: Created a proper port abstraction with a bridge adapter in the composition layer.

**Files Created**:
- `crates/engine-ports/src/outbound/llm_suggestion_queue_port.rs` - New outbound port with:
  - `LlmSuggestionQueuePort` trait
  - `LlmSuggestionQueueRequest` DTO
  - Re-exports `SuggestionContext` from `engine-dto`
- `crates/engine-composition/src/llm_suggestion_queue_adapter.rs` - Bridge adapter that:
  - Implements `LlmSuggestionQueuePort`
  - Wraps internal `LlmQueueServicePort` from `engine-app`
  - Handles DTO conversion between port types and internal types

**Files Modified**:
- `crates/engine-ports/src/outbound/mod.rs` - Added module declaration and exports
- `crates/engine-adapters/src/infrastructure/suggestion_enqueue_adapter.rs` - Updated to use:
  - `LlmSuggestionQueuePort` (outbound port) instead of `LlmQueueServicePort` (internal)
  - `SuggestionContext` from `engine-dto`
- `crates/engine-composition/src/lib.rs` - Added module and export for `LlmSuggestionQueueAdapter`
- `crates/engine-runner/src/composition/app_state.rs` - Updated wiring to create `LlmSuggestionQueueAdapter` and pass to `SuggestionEnqueueAdapter`

**Architecture**: This follows correct hexagonal patterns:
- Outbound adapters can depend on other outbound ports
- `engine-composition` can depend on `engine-app` (it's the DI wiring layer)
- `engine-adapters` has NO dependency on `engine-app`

---

### Step 1.3: Fix `world_state_manager.rs` - COMPLETE

**Solution Applied**: Created local adapter-internal types instead of importing from engine-app.

**Files Modified**:
- `crates/engine-adapters/src/infrastructure/world_state_manager.rs`:
  - Created local `StagingProposal` and `StagedNpcProposal` structs (lines 66-125)
  - These are marked as adapter-internal types with documentation
  - They don't cross layer boundaries - used only for in-memory storage

---

### Step 1.4: Remove engine-app dependency - COMPLETE

**Files Modified**:
- `crates/engine-adapters/Cargo.toml` - Removed `wrldbldr-engine-app` dependency
- Added comment: "NOTE: engine-adapters must NOT depend on engine-app (hexagonal architecture rule)"

**Verification**:
```bash
$ grep -r "wrldbldr_engine_app" crates/engine-adapters/src/
# Returns empty - no imports found

$ cargo xtask arch-check
arch-check OK (16 workspace crates checked)
```

---

## Phase 2: Fix DTO Duplication (CRITICAL) - COMPLETE

### Problem

`PromptMappingDto` and `InputDefaultDto` defined in both:
- `protocol/src/dto.rs` (wire format - canonical)
- `engine-dto/src/persistence.rs` (duplicate)

### Solution Applied

1. Added `#[serde(other)]` Unknown variant to `PromptMappingTypeDto` in protocol for forward compatibility
2. Deleted duplicate struct definitions from `engine-dto/src/persistence.rs`
3. Added re-exports from protocol:
   ```rust
   pub use wrldbldr_protocol::dto::{InputDefaultDto, PromptMappingDto, PromptMappingTypeDto};
   ```

---

## Phase 3: Add Forward Compatibility (HIGH) - COMPLETE

### Problem

Protocol enums lack `#[serde(other)]` variants.

### Solution Applied

Added `#[serde(other)]` to:
- `PromptMappingTypeDto` - Added `Unknown` variant with `#[serde(other)]`
- `InputTypeDto` - Already had `Unknown`, added `#[serde(other)]` attribute

Note: `SectionLayoutDto` and `ItemListTypeDto` in engine-dto already had `#[serde(other)]` variants.

---

## Phase 4: Fix SuggestionContext world_id (HIGH) - COMPLETE

### Problem

`SuggestionContext` in domain has `world_id: Option<WorldId>`, but `SuggestionContextData` in protocol is missing this field.

### Solution Applied

1. Added `world_id: Option<Uuid>` to `protocol::SuggestionContextData`
2. Added `uuid::Uuid` import to `protocol/src/requests.rs`
3. Updated `player-app` `SuggestionContext` DTO to include `world_id`
4. Updated all struct initializers in `player-ui` to include `world_id: None`

---

## Phase 5: Implement DmActionQueueAdapter (MEDIUM) - COMPLETE

### Problem

`DmActionQueuePlaceholder` was a no-op that logs but doesn't persist.

### Solution Applied

Created proper hexagonal adapter chain:

1. **New outbound port** `DmActionEnqueuePort` in `engine-ports/outbound/`:
   - `DmActionEnqueueRequest` DTO
   - `DmActionEnqueueType` enum (with `ApprovalDecision` variant)
   - `DmEnqueueDecision` enum (Approve, Reject, ApproveWithEdits)

2. **Bridge adapter** `DmActionEnqueueAdapter` in `engine-composition/`:
   - Implements `DmActionEnqueuePort`
   - Wraps internal `DmActionQueueServicePort` from engine-app
   - Converts port DTOs to internal DTOs

3. **Real adapter** `SceneDmActionQueueAdapter` in `engine-adapters/`:
   - Replaces `DmActionQueuePlaceholder`
   - Implements `SceneDmActionQueuePort`
   - Uses `DmActionEnqueuePort` and `ClockPort`
   - Converts `SceneDmAction` to `DmActionEnqueueRequest`

4. **Updated wiring** in `engine-runner/`:
   - Added `dm_action_queue_service_port` to `UseCaseDependencies`
   - Creates adapter chain: DmActionEnqueueAdapter → SceneDmActionQueueAdapter
   - Passes to SceneUseCase

---

## Verification

After all phases:

```bash
cargo xtask arch-check      # Must pass
cargo check --workspace     # Must compile
cargo test --workspace      # Must pass

# Verify no engine-app imports in adapters
grep -r "wrldbldr_engine_app" crates/engine-adapters/
# Should return empty
```

---

## Execution Order

| Phase | Priority | Effort | Dependencies |
|-------|----------|--------|--------------|
| 1.1 | CRITICAL | 2-3h | None |
| 1.2 | CRITICAL | 1-2h | None |
| 1.3 | CRITICAL | 1h | None |
| 1.4 | CRITICAL | 30m | 1.1, 1.2, 1.3 |
| 2 | CRITICAL | 30m | None |
| 3 | HIGH | 30m | None |
| 4 | HIGH | 15m | None |
| 5 | MEDIUM | 1h | None |

**Total estimated effort**: 6-8 hours

---

## Success Criteria

- [x] `engine-adapters/Cargo.toml` has no `wrldbldr-engine-app` dependency - **COMPLETE**
- [x] No `use wrldbldr_engine_app` in any adapter file - **COMPLETE**
- [x] `PromptMappingDto`, `InputDefaultDto` have single source in protocol - **COMPLETE**
- [x] All protocol/engine-dto enums have `#[serde(other)]` Unknown variant - **COMPLETE**
- [x] `SuggestionContextData` has `world_id` field - **COMPLETE**
- [x] `DmActionQueuePlaceholder` replaced with real adapter - **COMPLETE**
- [x] `cargo xtask arch-check` passes - **COMPLETE**
- [x] All tests pass - **COMPLETE**
