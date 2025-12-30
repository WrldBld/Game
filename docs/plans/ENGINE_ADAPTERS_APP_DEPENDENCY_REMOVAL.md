# Plan: Remove engine-adapters -> engine-app Dependency

## Problem Statement

`engine-adapters` currently depends on `engine-app` to access `WorkflowService` static utility functions. This violates hexagonal architecture principles where adapters should only depend on ports, not the application layer.

**Current violation:**
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
These should stay in `engine-app`:
- `prepare_workflow()` - Uses `rand::thread_rng()` for seed randomization
- `export_configs()` / `import_configs()` - Uses DTO conversions
- `set_input()` / `randomize_seeds()` - Private helpers

## Solution

Move pure functions to `domain-types/src/workflow.rs` as free functions, keeping application logic in `engine-app`.

## Implementation Steps

### Phase 1: Add serde_json dependency to domain-types

- [ ] Update `domain-types/Cargo.toml` to add `serde_json = { workspace = true }`

### Phase 2: Move pure functions to domain-types

- [ ] Add the following functions to `domain-types/src/workflow.rs`:
  - `pub fn analyze_workflow(workflow_json: &serde_json::Value) -> WorkflowAnalysis`
  - `pub fn validate_workflow(workflow_json: &serde_json::Value) -> Result<(), String>`
  - `pub fn find_nodes_by_type(workflow: &serde_json::Value, class_type: &str) -> Vec<(String, serde_json::Value)>`
  - `pub fn auto_detect_prompt_mappings(workflow: &serde_json::Value) -> Vec<PromptMapping>`

- [ ] Export these functions from `domain-types/src/lib.rs`

### Phase 3: Update engine-app WorkflowService

- [ ] Remove moved functions from `engine-app/src/application/services/workflow_service.rs`
- [ ] Import and delegate to domain-types functions where needed internally
- [ ] Keep `prepare_workflow`, `export_configs`, `import_configs`, and private helpers

### Phase 4: Update engine-adapters

- [ ] Update `workflow_routes.rs` imports:
  - Import pure functions from `wrldbldr_domain_types`
  - Keep importing `WorkflowService` from `engine-app` ONLY for `prepare_workflow`, `export_configs`, `import_configs`

- [ ] Check if `engine-app` dependency can be removed entirely
  - If `prepare_workflow`/`export_configs`/`import_configs` are only used in workflow_routes.rs, consider moving them to adapters or creating a port

### Phase 5: Remove engine-app dependency (if possible)

- [ ] If all usages can be satisfied by domain-types functions, remove from `engine-adapters/Cargo.toml`
- [ ] Update `xtask/src/main.rs` arch-check allowed dependencies

### Phase 6: Update engine-app/dto/workflow.rs

- [ ] Update imports to use domain-types functions instead of WorkflowService
- [ ] `workflow_config_to_response_dto()` and `workflow_config_to_full_response_dto()` currently call `WorkflowService::analyze_workflow()` - change to `wrldbldr_domain_types::analyze_workflow()`

### Phase 7: Move tests

- [ ] Move relevant tests from `engine-app/services/workflow_service.rs` to `domain-types`
- [ ] Keep tests for application logic in engine-app

### Phase 8: Verification

- [ ] `cargo check --workspace`
- [ ] `cargo xtask arch-check`
- [ ] `cargo test -p wrldbldr-domain-types`
- [ ] `cargo test -p wrldbldr-engine-app`

## Files to Modify

| File | Action |
|------|--------|
| `domain-types/Cargo.toml` | Add serde_json dependency |
| `domain-types/src/workflow.rs` | Add pure functions |
| `domain-types/src/lib.rs` | Export new functions |
| `engine-app/src/application/services/workflow_service.rs` | Remove moved functions, update imports |
| `engine-app/src/application/dto/workflow.rs` | Update to use domain-types |
| `engine-adapters/src/infrastructure/http/workflow_routes.rs` | Update imports |
| `engine-adapters/Cargo.toml` | Remove engine-app dependency (if possible) |
| `xtask/src/main.rs` | Update arch-check rules |

## Expected Outcome

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

## Risk Assessment

- **LOW RISK**: Pure function extraction is straightforward
- **MEDIUM RISK**: May need to keep engine-app dependency if `prepare_workflow` is used in adapters
  - Mitigation: Create `WorkflowPreparationPort` trait if needed

## Fallback

If complete removal of engine-app dependency is not feasible, document the remaining usage and add proper comments explaining the architectural exception.

## Success Criteria

1. `cargo xtask arch-check` passes
2. No `wrldbldr-engine-app` in `engine-adapters/Cargo.toml` (or documented exception)
3. Pure workflow analysis functions accessible from domain-types
4. All existing tests pass
