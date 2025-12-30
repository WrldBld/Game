# Plan: Remove engine-adapters -> engine-app Dependency

## Status: COMPLETED

**Completed:** 2024-12-30

**Commits:**
- `e4a8c85` - Move pure workflow analysis functions to domain-types
- `39aa965` - Update engine-app WorkflowService to delegate to domain-types
- `4d6840c` - Update engine-adapters to use domain-types for workflow analysis
- `8001ddb` - Remove engine-adapters -> engine-app dependency
- `d60a772` - Update dto/workflow.rs to use domain-types directly

---

## Problem Statement

`engine-adapters` currently depends on `engine-app` to access `WorkflowService` static utility functions. This violates hexagonal architecture principles where adapters should only depend on ports, not the application layer.

**Previous violation:**
```
engine-adapters/Cargo.toml:17
wrldbldr-engine-app = { workspace = true }
```

**Used in:** `workflow_routes.rs` for workflow JSON analysis functions.

## Root Cause Analysis

`WorkflowService` in engine-app contains two types of methods:

### Pure Functions (No I/O, No State)
These should live in `domain-types`:
- `analyze_workflow()` - Parses ComfyUI workflow JSON
- `validate_workflow()` - Validates workflow structure
- `find_nodes_by_type()` - Finds nodes by class_type
- `auto_detect_prompt_mappings()` - Detects prompt mappings

### Application Logic (Uses rand, DTOs)
These were moved to `engine-adapters/src/infrastructure/http/workflow_helpers.rs`:
- `prepare_workflow()` - Uses `rand::thread_rng()` for seed randomization
- `export_configs()` / `import_configs()` - Uses DTO conversions
- `set_input()` / `randomize_seeds()` - Private helpers

## Solution Implemented

1. **Pure functions moved to `domain-types/src/workflow.rs`** as free functions
2. **Orchestration functions moved to `engine-adapters/src/infrastructure/http/workflow_helpers.rs`**
3. **engine-app WorkflowService now delegates** to domain-types for pure functions
4. **engine-adapters no longer depends on engine-app**

## Implementation Steps

### Phase 1: Add serde_json dependency to domain-types

- [x] Already had `serde_json = { workspace = true }` - No changes needed

### Phase 2: Move pure functions to domain-types

- [x] Added the following functions to `domain-types/src/workflow.rs`:
  - `pub fn analyze_workflow(workflow_json: &serde_json::Value) -> WorkflowAnalysis`
  - `pub fn validate_workflow(workflow_json: &serde_json::Value) -> Result<(), String>`
  - `pub fn find_nodes_by_type(workflow: &serde_json::Value, class_type: &str) -> Vec<(String, serde_json::Value)>`
  - `pub fn auto_detect_prompt_mappings(workflow: &serde_json::Value) -> Vec<PromptMapping>`

- [x] Exported these functions from `domain-types/src/lib.rs`

### Phase 3: Update engine-app WorkflowService

- [x] Replaced implementations with thin wrappers that delegate to domain-types
- [x] Updated imports to use `wrldbldr_domain_types`

### Phase 4: Update engine-adapters

- [x] Updated `workflow_routes.rs` to import pure functions from `wrldbldr_domain_types`

### Phase 5: Remove engine-app dependency

- [x] Created `workflow_helpers.rs` in engine-adapters with:
  - `prepare_workflow` (uses rand)
  - `export_configs` / `import_configs` (uses DTOs)
  - Private helpers `set_input`, `randomize_seeds`
- [x] Added `workflow_config_from_export_dto` to dto_conversions
- [x] Removed `wrldbldr-engine-app` from `engine-adapters/Cargo.toml`
- [x] Updated arch-check rules in `xtask/src/main.rs`

### Phase 6: Update engine-app/dto/workflow.rs

- [x] Updated to use `wrldbldr_domain_types::analyze_workflow()` directly
- [x] Removed `WorkflowService` import

### Phase 7: Move tests

- [x] Tests remain in engine-app as integration tests (they exercise the delegation)
- [x] All 4 workflow_service tests pass

### Phase 8: Verification

- [x] `cargo check --workspace` - PASS
- [x] `cargo xtask arch-check` - PASS
- [x] `cargo test -p wrldbldr-domain-types` - PASS
- [x] `cargo test -p wrldbldr-engine-app` - 70 tests PASS

## Files Modified

| File | Action |
|------|--------|
| `domain-types/src/workflow.rs` | Added 4 pure functions (~150 lines) |
| `domain-types/src/lib.rs` | Exported new functions |
| `engine-app/Cargo.toml` | Added domain-types dependency |
| `engine-app/src/application/services/workflow_service.rs` | Replaced with thin wrappers |
| `engine-app/src/application/dto/workflow.rs` | Updated to use domain-types |
| `engine-adapters/src/infrastructure/http/workflow_routes.rs` | Updated imports |
| `engine-adapters/src/infrastructure/http/workflow_helpers.rs` | NEW - orchestration functions |
| `engine-adapters/src/infrastructure/http/mod.rs` | Added workflow_helpers module |
| `engine-adapters/src/infrastructure/dto_conversions/workflow_conversions.rs` | Added from_export_dto |
| `engine-adapters/src/infrastructure/dto_conversions/mod.rs` | Exported new function |
| `engine-adapters/Cargo.toml` | Removed engine-app dependency |
| `xtask/src/main.rs` | Updated arch-check rules |

## Final Architecture

```
BEFORE:
engine-adapters -> engine-app -> engine-ports -> domain
                                              -> domain-types

AFTER:
engine-adapters -> engine-ports -> domain
                -> domain-types (for pure workflow analysis)
                
engine-app -> engine-ports -> domain
                           -> domain-types
```

## Success Criteria - ALL MET

1. ✅ `cargo xtask arch-check` passes
2. ✅ No `wrldbldr-engine-app` in `engine-adapters/Cargo.toml`
3. ✅ Pure workflow analysis functions accessible from domain-types
4. ✅ All existing tests pass
