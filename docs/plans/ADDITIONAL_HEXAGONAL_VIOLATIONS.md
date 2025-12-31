# Additional Hexagonal Architecture Violations

## Executive Summary

This document identifies additional hexagonal architecture violations and tech debt issues beyond those documented in `PORT_ADAPTER_TECH_DEBT_REMEDIATION.md`.

**Total Additional Issues Found**: 3 new issues (after validation)

**Priority Breakdown**:
- P2: 1 issue
- P3: 2 issues

**Last Updated**: Validated by codebase analysis - most issues were in test code

> **Spot-check validation (gemini-refactor / 2025-12-31)**
>
> In this session we re-verified Issue 16 directly (duplicate `StagingProposal` types and the misleading comment in `world_state_manager.rs`).
> Issues 14 and 15 were not re-opened line-by-line in this pass.

---

## Issue 14: Test Code Using unwrap/expect/panic (P3)

### The Problem

Several locations in **test code** use `.unwrap()`, `.expect()`, or `panic!()`. While this is generally acceptable in test code, it can make tests less informative when they fail.

### Validation Results

**Status**: ✅ **VALIDATED - All examples are in test code**

After validation, all reported locations are in test code (`#[cfg(test)]` or `#[test]` blocks):

1. **World Export** (`crates/engine-adapters/src/infrastructure/export/world_snapshot.rs`):
   - Lines 228, 232, 248, 287, 293 - All in `#[cfg(test)]` block
   - These are test assertions, not production code

2. **World Connection Manager** (`crates/engine-adapters/src/infrastructure/world_connection_manager.rs`):
   - Lines 1105, 1132, 1267, 1288, 1293 - All in `#[cfg(test)]` block
   - These are test assertions, not production code

3. **HTTP Middleware** (`crates/engine-adapters/src/infrastructure/http/middleware/auth.rs`):
   - Lines 152, 154 - Both in `#[test]` function
   - These are test assertions, not production code

4. **Other Test Files**:
   - `crates/engine-adapters/src/infrastructure/websocket/context.rs`: Multiple `panic!()` calls in tests
   - `crates/engine-adapters/src/infrastructure/websocket/error_conversion.rs`: `panic!()` in tests
   - `crates/engine-adapters/src/infrastructure/testing/mock_clock.rs`: `.unwrap()` in test helpers

**Note**: The ComfyUI client example (`unwrap_or_else(|p| p.into_inner())`) is a standard Rust pattern for handling lock poisoning and is acceptable.

### Impact

- **Low impact**: Test code using `unwrap()` is acceptable practice in Rust
- **Test quality**: Could be improved to provide better error messages, but not a critical issue
- **No production risk**: These are not in production code paths

### Fix (Optional)

If desired, improve test error messages:
```rust
// Current (acceptable)
let result = operation().await.unwrap();

// Improved (optional)
let result = operation().await
    .expect("Operation should succeed in test");
```

### Priority

P3 - Low priority. Test code using `unwrap()` is acceptable practice. This is a code quality improvement, not a critical issue.

---

## Issue 15: Utc::now() in Application Layer (P3)

### The Problem

The application layer uses `Utc::now()` directly instead of injecting `ClockPort`, making code non-deterministic and harder to test.

### Validation Results

**Status**: ✅ **VALIDATED - Mostly test code, 1 production location (Default impl)**

After validation, most reported locations are in test code:

1. **Prompt Context Service** (`crates/engine-app/src/application/services/prompt_context_service.rs`):
   - Lines 579, 592, 602, 611 - All in `#[test]` functions
   - These are test code, not production code

2. **Trigger Evaluation Service** (`crates/engine-app/src/application/services/trigger_evaluation_service.rs`):
   - Line 801 - In `#[test]` function
   - This is test code, not production code

3. **World Snapshot DTO** (`crates/engine-app/src/application/dto/world_snapshot.rs:74`):
   ```rust
   world: World::new("Empty World", "A placeholder world", Utc::now()),
   ```
   **Status**: This IS in production code (Default implementation)
   **Assessment**: Using `Utc::now()` in Default impls is **acceptable** (though not ideal)

### Impact

- **Test code**: Non-deterministic tests (low impact, acceptable)
- **Production code**: Only 1 location in Default impl, which is acceptable practice
- **Low priority**: Not a critical architectural violation

### Fix (Optional)

1. **Test code**: Inject `MockClockPort` for deterministic tests (optional improvement)
2. **Production code**: The Default impl is acceptable, but could be improved:
   - Document that default uses current time
   - Consider making time optional in Default
   - Using a sentinel value is not recommended (breaks invariants)

### Priority

P3 - Low priority. Test code is acceptable. The production Default implementation is acceptable practice, though could be improved for better testability.

---

## Issue 16: Misleading Comment and Confirmed Type Duplication (P2)

### The Problem

A comment in `world_state_manager.rs` incorrectly states that `StagingProposal` comes from `engine-app`, but it actually comes from `engine-dto`. Additionally, there are **confirmed duplicate** `StagingProposal` types.

### Location

`crates/engine-adapters/src/infrastructure/world_state_manager.rs:8,22`:
```rust
use wrldbldr_engine_dto::StagingProposal;
// ...
/// the world state port traits because they depend on `StagingProposal` from engine-app.
```

### Validation Results

**Status**: ✅ **VALIDATED - Confirmed duplication**

**Confirmed Findings**:
1. `world_state_manager.rs` imports `StagingProposal` from `wrldbldr_engine_dto` (line 8)
2. Comment incorrectly says it's from `engine-app` (line 22) - **MISLEADING**
3. **Confirmed duplication**: Two `StagingProposal` types exist with **identical structure**:
   - `engine-dto::StagingProposal` (defined in `crates/engine-dto/src/staging.rs`)
   - `engine-ports::outbound::StagingProposal` (defined in `crates/engine-ports/src/outbound/staging_service_port.rs`)

**Type Structure Comparison**:
Both types have identical fields:
- `request_id: String`
- `region_id: String`
- `location_id: String`
- `world_id: String`
- `rule_based_npcs: Vec<StagedNpcProposal>`
- `llm_based_npcs: Vec<StagedNpcProposal>`
- `default_ttl_hours: i32`
- `context: StagingContext`

**Usage**:
- `engine-dto::StagingProposal` - Used by `world_state_manager.rs` and `staging_service.rs`
- `engine-ports::outbound::StagingProposal` - Used in port trait definitions

### Impact

- **Misleading documentation**: Comment is incorrect
- **Confirmed duplication**: Two identical `StagingProposal` types exist
- **Confusion**: Developers may be confused about which type to use
- **Maintenance burden**: Changes must be made in two places
- **Type safety**: Rust treats these as different types, preventing interoperability

### Fix

1. **Update comment** to reflect actual import:
   ```rust
   /// the world state port traits because they depend on `StagingProposal` from engine-dto.
   ```

2. **Consolidate types**:
   - Choose canonical location: `engine-ports::outbound::StagingProposal` (preferred, as it's in the ports layer)
   - Remove `engine-dto::StagingProposal`
   - Update all imports:
     - `world_state_manager.rs`: Change to `use wrldbldr_engine_ports::outbound::StagingProposal;`
     - `staging_service.rs`: Change to use port type
   - Update re-exports in `engine-app/src/application/services/mod.rs`

3. **Verify no breaking changes**: Ensure all usages are updated before removing the duplicate

### Priority

P2 - Misleading documentation and confirmed type duplication should be fixed to avoid confusion and maintenance issues.

---

## Issue 17: Protocol→Domain Dependency (Already Documented)

### Status

This issue is already documented in `ARCHITECTURE_GAP_REMEDIATION_PLAN.md` (Issue C5).

**Summary**: Protocol crate depends on `wrldbldr-domain` for 16 `From<DomainEntity>` implementations, forcing player WASM to compile the entire domain crate.

**Status**: PENDING  
**Effort**: 8-12 hours

**Reference**: See `docs/plans/ARCHITECTURE_GAP_REMEDIATION_PLAN.md` for full details.

---

## Issue 18: Workflow Service Port Implementation Code (Already Documented)

### Status

This issue is already documented in `ARCHITECTURE_GAP_REMEDIATION_PLAN.md` (Issue C6).

**Summary**: `workflow_service_port.rs` was reported to contain ~270 lines of implementation code, but current file only contains the trait definition.

**Status**: ✅ **LIKELY ALREADY FIXED** - The file now only contains the trait definition.

**Verification**: Current `workflow_service_port.rs` (93 lines) contains only:
- Trait definition (`WorkflowServicePort`)
- Method signatures
- Documentation

**Reference**: See `docs/plans/ARCHITECTURE_GAP_REMEDIATION_PLAN.md` for original issue. This may have been resolved.

---

## Summary of All Additional Issues

| Priority | Issue | Count | Status |
|----------|-------|-------|--------|
| **P2** | Type duplication (StagingProposal) | 2 types | 🆕 **NEW** (validated) |
| **P3** | Test code using unwrap/expect/panic | 10+ locations | 🆕 **NEW** (validated - test code) |
| **P3** | Utc::now() in application layer | 6 locations | 🆕 **NEW** (validated - mostly test code) |
| **P2** | Protocol→domain dependency | 16 From impls | ✅ Already documented |
| **P2** | Workflow service port implementation | 1 file | ✅ Likely already fixed |

---

## Recommendations

1. **Prioritize Issue 16** - Confirmed type duplication should be fixed to avoid confusion and maintenance issues.
2. **Optional: Issue 14** - Test code improvements are optional (low priority).
3. **Optional: Issue 15** - Test code improvements are optional (low priority). The Default impl is acceptable.
4. **Track Issue 17** - Already in remediation plan, no action needed here.
5. **Verify Issue 18** - Confirm if workflow service port issue was already fixed.

---

## Integration with Existing Plans

These issues should be integrated into the remediation workflow:

- **Issue 16**: Add to Phase 8 (Fix Other Anti-Patterns) or create new phase for type consolidation
- **Issue 14**: Optional - test code improvements (low priority)
- **Issue 15**: Optional - test code improvements (low priority)
- **Issue 17**: Already tracked in `ARCHITECTURE_GAP_REMEDIATION_PLAN.md`
- **Issue 18**: Verify and close if already fixed

---

## Updated Effort Estimate

**Additional Effort**: +0.25 day (reduced after validation)

**Breakdown**:
- Issue 16 (Type duplication fix): +0.25 day (consolidate types, update imports, fix comment)
- Issue 14 (Test code improvements): Optional, not included in estimate
- Issue 15 (Test code improvements): Optional, not included in estimate

**Total Project Effort**: 6.25-7.25 days (6-7 days from PORT_ADAPTER_TECH_DEBT_REMEDIATION.md + 0.25 day from this document)

**Note**: Issues 14 and 15 are optional test code improvements and don't need to be included in the main remediation effort.

