# Hexagonal Architecture Final Remediation Plan

**Created:** 2024-12-30
**Status:** In Progress

## Overview

This plan addresses all remaining hexagonal architecture violations and design concerns identified in the code review.

---

## Priority 1: Architecture Violations

### 1.1 Refactor `workflow_routes.rs` - Move entity mutation to app layer

**Problem:** HTTP handler directly manipulates `WorkflowConfiguration` entity instead of delegating to an application service.

**Current State:**
- `WorkflowServicePort` exists with basic CRUD methods
- `WorkflowConfigService` implements `WorkflowServicePort`
- HTTP handlers in `workflow_routes.rs` contain entity mutation logic (lines 129-158, 219-229, 284-303)

**Solution:** Add new methods to `WorkflowServicePort` and `WorkflowConfigService`:

```rust
// New methods for WorkflowServicePort:
async fn create_or_update(
    &self,
    slot: WorkflowSlot,
    name: String,
    workflow_json: serde_json::Value,
    prompt_mappings: Vec<PromptMapping>,
    input_defaults: Vec<InputDefault>,
    locked_inputs: Vec<String>,
) -> Result<WorkflowConfiguration>;

async fn update_defaults(
    &self,
    slot: WorkflowSlot,
    input_defaults: Vec<InputDefault>,
    locked_inputs: Option<Vec<String>>,
) -> Result<WorkflowConfiguration>;

async fn import_configs(
    &self,
    configs: Vec<WorkflowConfiguration>,
    replace_existing: bool,
) -> Result<ImportResult>;
```

**Files to modify:**
1. `crates/engine-ports/src/outbound/workflow_service_port.rs` - Add new methods to trait
2. `crates/engine-app/src/application/services/workflow_config_service.rs` - Implement new methods
3. `crates/engine-adapters/src/infrastructure/http/workflow_routes.rs` - Simplify handlers to delegate

**Estimated changes:** ~150 lines moved, ~100 new lines in service

---

### 1.2 Move `MockGameConnectionPort` to player-ports

**Problem:** Test code in `player-app` imports mock from `player-adapters`, violating layer boundaries.

**Files with violations:**
- `crates/player-app/src/application/services/action_service.rs:85`
- `crates/player-app/src/application/services/narrative_event_service.rs:115`
- `crates/player-app/src/application/services/challenge_service.rs:139`

**Solution:** Move mock to player-ports behind `testing` feature flag.

**Files to modify:**
1. `crates/player-ports/Cargo.toml` - Add mockall as optional dependency with `testing` feature
2. `crates/player-ports/src/outbound/game_connection/mod.rs` - Add mock behind feature flag
3. `crates/player-app/Cargo.toml` - Change dev-dependency from player-adapters to player-ports/testing
4. `crates/player-app/src/application/services/action_service.rs` - Update import
5. `crates/player-app/src/application/services/narrative_event_service.rs` - Update import
6. `crates/player-app/src/application/services/challenge_service.rs` - Update import

**Estimated changes:** ~50 lines

---

## Priority 2: Design Improvements

### 2.1 Align PromptContextService interfaces

**Problem:** `PromptContextServicePortAdapter` shim exists because:
- Port trait: 6 individual parameters + `PromptContextError`
- App trait: 2 parameters (`world_id`, `&PlayerActionData`) + `QueueError`
- Shim hardcodes `player_id` and generates `timestamp` (data loss)

**Key Finding:** The port trait is never actually called - all usage goes through app-layer trait.

**Solution:** Align port to app-layer signature, eliminate shim.

**Files to modify:**
1. `crates/engine-ports/src/outbound/prompt_context_service_port.rs` - Change signature to take `&PlayerActionData`
2. `crates/engine-app/src/application/services/prompt_context_service.rs` - Have impl implement port trait directly
3. `crates/engine-runner/src/composition/app_state.rs` - Remove adapter shim, use impl directly
4. `crates/engine-composition/src/app_state.rs` - Update if needed

**Estimated changes:** ~100 lines removed (shim), ~20 lines modified

---

### 2.2 Move config types from player-ports to player-runner

**Problem:** `ShellKind` and `RunnerConfig` in `player-ports/src/lib.rs` are configuration types, not port interfaces.

**Solution:** Move to player-runner where they belong.

**Files to modify:**
1. `crates/player-ports/src/lib.rs` - Remove `ShellKind`, `RunnerConfig`
2. `crates/player-runner/src/lib.rs` - Add config types
3. Any files importing these - Update imports

**Estimated changes:** ~50 lines moved

---

### 2.3 Consider builder pattern for AppState (OPTIONAL)

**Problem:** `AppState::new()` has 19 parameters.

**Decision:** Skip for now - the constructor works and is type-safe. Builder pattern would add complexity without significant benefit. Document as accepted technical debt.

---

## Execution Order

| Step | Task | Dependencies |
|------|------|--------------|
| 1 | Move MockGameConnectionPort to player-ports | None |
| 2 | Refactor workflow_routes.rs | None |
| 3 | Align PromptContextService interfaces | None |
| 4 | Move config types to player-runner | None |
| 5 | Run tests + arch-check | After 1-4 |
| 6 | Single commit | After 5 |

---

## Commit Message (Draft)

```
refactor: complete hexagonal architecture remediation

Priority 1 - Architecture Violations:
- Move MockGameConnectionPort from player-adapters to player-ports
  with testing feature flag (fixes test import violations)
- Move entity mutation logic from workflow_routes.rs to
  WorkflowConfigService (fixes business logic in adapter)

Priority 2 - Design Improvements:
- Align PromptContextService port/app interfaces, remove adapter shim
- Move ShellKind/RunnerConfig from player-ports to player-runner

All layers now properly respect hexagonal boundaries:
- Domain: pure business logic, no I/O
- Ports: trait definitions only (+ test mocks behind feature)
- Adapters: infrastructure only, delegate to services
- App: orchestration using ports
- Composition: wiring using trait objects
- Runner: concrete instantiation
```

---

## Verification

After all changes:
1. `cargo check --workspace` - Compilation
2. `cargo test --workspace` - All tests pass
3. `cargo xtask arch-check` - Architecture validation
4. Manual review of changed files
