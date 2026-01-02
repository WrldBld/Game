# ServicePort Purist Refactor Plan (Option B / Big Bang)

> **Status**: PLANNED
> **Created**: 2026-01-02
> **Type**: Big Bang Refactor (no backwards compatibility required)

## Executive Summary

The engine currently defines **39** `*ServicePort` traits under `crates/engine-ports/src/outbound/`. Most of these traits are **implemented by `engine-app`**, and several are **called by adapters** (primarily via `AppStatePort` getters or via wrapper-forwarder adapters).

This plan pursues **Option B (purist)**:

1. Adapters call **inbound use case ports**, not "service" ports.
2. Application-internal service traits live in `engine-app` (or become concrete dependencies), not in `engine-ports`.
3. Outbound ports remain adapter-implemented; wrapper-forwarder "adapters that call back into app services" are removed (required).

This is a big-bang refactor; no backwards compatibility is expected.

---

## Problem Statement

### Current State (Incorrect for Option B)

```
engine-ports/src/outbound/
├── *_service_port.rs              # many traits implemented by engine-app
└── ... (39 total `*ServicePort` traits)
```

### Why This Is Wrong

Two distinct issues are conflated today:

1. **Adapters directly calling application services** (usually through `AppStatePort` getters returning `Arc<dyn *ServicePort>`). In a purist hexagonal model, adapters call **inbound use cases**, not a service layer.
2. **Wrapper-forwarder adapters** that implement outbound ports by calling back into application services (anti-pattern). This inverts the "outbound implemented by adapters" rule.

### Target State (Correct for Option B)

```
engine-ports/src/inbound/
├── *_use_case_port.rs               # adapters call these
├── request_handler.rs               # boundary handler API (existing)
├── app_state_port.rs                # returns inbound use cases + true outbound infra only
└── (NEW) admin/maintenance use cases as needed by HTTP routes

engine-app/src/
└── application/
   ├── use_cases/                   # implement inbound ports
   └── services/
       ├── internal/                # internal service traits (NOT in engine-ports)
       │   └── mod.rs               # re-exports internal traits
       └── ...                      # service implementations

engine-ports/src/outbound/
└── repositories, clocks, queues, storage, external clients, etc. (implemented by adapters)
```

---

## Trait Classification (Re-Audit Results)

Based on the mechanical rubric from `hexagonal-architecture.md`:

1. If adapters/handlers **call** the trait AND app **implements** it → **INBOUND**
2. If app **depends on** the trait AND adapters **implement** it → **OUTBOUND**
3. If app both **calls and implements** it → **NOT A PORT** (internal trait)

### INBOUND (11 traits) - Replace with Use Case Ports

These are exposed via `AppStatePort` and called by HTTP route handlers:

| Trait | AppStatePort Method | Called by Adapter |
|-------|---------------------|-------------------|
| `SettingsServicePort` | `settings_service()` | `settings_routes.rs` |
| `PromptTemplateServicePort` | `prompt_template_service()` | `prompt_template_routes.rs` |
| `AssetServicePort` | `asset_service()` | `asset_routes.rs` |
| `GenerationServicePort` | `generation_service()` | `asset_routes.rs` |
| `AssetGenerationQueueServicePort` | `asset_generation_queue_service()` | `queue_routes.rs` |
| `WorkflowServicePort` | `workflow_service()` | `workflow_routes.rs` |
| `GenerationQueueProjectionServicePort` | `generation_queue_projection_service()` | `queue_routes.rs` |
| `PlayerActionQueueServicePort` | `player_action_queue_service()` | `queue_routes.rs` |
| `LlmQueueServicePort` | `llm_queue_service()` | `queue_routes.rs` |
| `DmApprovalQueueServicePort` | `dm_approval_queue_service()` | `queue_routes.rs` |
| `DmActionQueueServicePort` | `dm_action_queue_service()` | `queue_routes.rs` |

**Action**: Create new inbound `*UseCasePort` traits and update HTTP handlers to call them.

### OUTBOUND (8 traits) - Keep in `engine-ports/src/outbound/`

These are either implemented by adapters OR wrapped by adapters to implement other ports:

| Trait | Reason | Keep/Delete |
|-------|--------|-------------|
| `StagingUseCaseServicePort` | Implemented by `StagingServiceAdapter` | **KEEP** |
| `StagingUseCaseServiceExtPort` | Implemented by `StagingServiceAdapter` | **KEEP** |
| `PlayerCharacterServicePort` | Wrapped by `PlayerCharacterServiceAdapter` | DELETE (anti-pattern) |
| `WorldServicePort` | Wrapped by `WorldServiceAdapter` | DELETE (anti-pattern) |
| `SceneServicePort` | Wrapped by `SceneServiceAdapter` | DELETE (anti-pattern) |
| `InteractionServicePort` | Wrapped by `InteractionServiceAdapter` | DELETE (anti-pattern) |
| `StagingServicePort` | Wrapped by `StagingServiceAdapter` | DELETE (anti-pattern) |
| `ChallengeOutcomeApprovalServicePort` | Wrapped for approval flow | DELETE (anti-pattern) |

**Action**: Keep only `StagingUseCaseServicePort` and `StagingUseCaseServiceExtPort`. Delete the others after removing wrapper-forwarder adapters.

### NOT A PORT (20 traits) - Internalize to `engine-app`

These are purely app-internal: implemented by app, called by app, never touched by adapters:

| Trait | Current Caller | Target Location |
|-------|---------------|-----------------|
| `ChallengeServicePort` | `ChallengeResolutionService` | `engine-app/services/internal/` |
| `SkillServicePort` | `ChallengeResolutionService` | `engine-app/services/internal/` |
| `NarrativeEventServicePort` | `NarrativeEventApprovalService` | `engine-app/services/internal/` |
| `StoryEventRecordingServicePort` | `NarrativeEventApprovalService` | `engine-app/services/internal/` |
| `DialogueContextServicePort` | `DmApprovalQueueService` | `engine-app/services/internal/` |
| `OutcomeTriggerServicePort` | `ChallengeOutcomeApprovalService` | `engine-app/services/internal/` |
| `NarrativeEventApprovalServicePort` | `NarrativeEventUseCase` | `engine-app/services/internal/` |
| `EventChainServicePort` | Composition only | `engine-app/services/internal/` |
| `StoryEventServicePort` | Composition only | `engine-app/services/internal/` |
| `StoryEventQueryServicePort` | Composition only | `engine-app/services/internal/` |
| `StoryEventAdminServicePort` | Composition only | `engine-app/services/internal/` |
| `ActantialContextServicePort` | Composition only | `engine-app/services/internal/` |
| `DispositionServicePort` | Composition only | `engine-app/services/internal/` |
| `RelationshipServicePort` | Composition only | `engine-app/services/internal/` |
| `LocationServicePort` | Composition only | `engine-app/services/internal/` |
| `RegionServicePort` | Composition only | `engine-app/services/internal/` |
| `ItemServicePort` | Composition only | `engine-app/services/internal/` |
| `SceneResolutionServicePort` | Composition only | `engine-app/services/internal/` |
| `SheetTemplateServicePort` | Composition only | `engine-app/services/internal/` |
| `TriggerEvaluationServicePort` | Composition only | `engine-app/services/internal/` |
| `PromptContextServicePort` | Composition only | `engine-app/services/internal/` |

**Action**: Move these traits to `crates/engine-app/src/application/services/internal/` and remove from `engine-ports`.

---

## Scope

### 1) Replace adapter access to "service ports" with inbound use cases (REQUIRED)

`AppStatePort` currently exposes 11 `Arc<dyn *ServicePort>` getters called directly by HTTP routes.

Option B replaces these with inbound use case ports and updates adapters to call use cases.

### 2) Internalize app-only `*ServicePort` traits (REQUIRED)

The 20 traits classified as "NOT A PORT" must be:

1. Moved to `crates/engine-app/src/application/services/internal/`
2. Removed from `engine-ports`
3. Imports updated across `engine-app`

### 3) Keep only adapter-implemented outbound ports (REQUIRED)

After this refactor, only 2 `*ServicePort` traits remain in `engine-ports/src/outbound/`:

- `StagingUseCaseServicePort`
- `StagingUseCaseServiceExtPort`

### 4) Remove wrapper-forwarder adapters (REQUIRED)

Wrapper-forwarder adapters violate strict hexagonal direction:

```rust
// ANTI-PATTERN: Adapter wrapping app service to implement another port
pub struct PlayerCharacterServiceAdapter {
    service: Arc<dyn PlayerCharacterServicePort>,  // depends on "outbound" port
}

impl PlayerCharacterDtoPort for PlayerCharacterServiceAdapter { ... }
```

Known modules participating in this anti-pattern:

- `crates/engine-adapters/src/infrastructure/connection_port_adapters.rs`
- `crates/engine-adapters/src/infrastructure/scene_port_adapters.rs`
- `crates/engine-adapters/src/infrastructure/challenge_port_adapters.rs`
- `crates/engine-adapters/src/infrastructure/suggestion_enqueue_adapter.rs`
- `crates/engine-adapters/src/infrastructure/player_action_port_adapters.rs`

This is mandatory in the Option B big-bang; it cannot be a follow-up.

---

## Concrete Execution Checklist (Big Bang / Option B)

### Step 0: Pre-flight

1. Ensure a clean working tree.
2. Record baseline:

```bash
cargo xtask arch-check
cargo check --workspace
```

### Step 1: Create internal traits directory in `engine-app`

1. Create `crates/engine-app/src/application/services/internal/`
2. Create `mod.rs` with re-exports

### Step 2: Internalize the 20 "NOT A PORT" traits

For each of the 20 traits:

1. Move trait file from `engine-ports/src/outbound/` to `engine-app/src/application/services/internal/`
2. Update imports in `engine-app`
3. Remove export from `engine-ports/src/outbound/mod.rs`

### Step 3: Create new inbound use case ports for HTTP routes

Create inbound use case ports for the 11 service ports currently exposed via `AppStatePort`:

| Current ServicePort | New UseCasePort | Methods (TBD during implementation) |
|---------------------|-----------------|-------------------------------------|
| `SettingsServicePort` | `SettingsUseCasePort` | get, update, reset, get_for_world, etc. |
| `PromptTemplateServicePort` | `PromptTemplateUseCasePort` | get_all, set, delete, resolve, etc. |
| `AssetServicePort` | `AssetUseCasePort` | get, list, create, update, delete, etc. |
| `GenerationServicePort` | `AssetGenerationUseCasePort` | queue, retry, cancel, get_status, etc. |
| `AssetGenerationQueueServicePort` | (merge into AssetGenerationUseCasePort) | |
| `WorkflowServicePort` | `WorkflowUseCasePort` | get, list, create, update, etc. |
| `GenerationQueueProjectionServicePort` | `QueueProjectionUseCasePort` | get_queue_state, list_pending, etc. |
| `PlayerActionQueueServicePort` | `QueueAdminUseCasePort` | list, cancel, retry, etc. |
| `LlmQueueServicePort` | (merge into QueueAdminUseCasePort) | |
| `DmApprovalQueueServicePort` | (merge into QueueAdminUseCasePort) | |
| `DmActionQueueServicePort` | (merge into QueueAdminUseCasePort) | |

### Step 4: Update `AppStatePort` and HTTP handlers

1. Remove the 11 service getters from `AppStatePort`
2. Add new use case getters
3. Update HTTP route handlers to call use cases instead of services

### Step 5: Delete wrapper-forwarder adapters

1. Update use cases to depend on true outbound ports (repositories) instead of service ports
2. Build required DTOs in use cases
3. Delete the wrapper adapter files listed above
4. Delete the callback ports they implemented (`WorldSnapshotJsonPort`, `PlayerCharacterDtoPort`, etc.)

### Step 6: Clean up remaining outbound service ports

After wrapper adapters are deleted, remove the 6 service ports that were only used by wrappers:

- `PlayerCharacterServicePort` (if not internalized)
- `WorldServicePort` (if not internalized)
- `SceneServicePort` (if not internalized)
- `InteractionServicePort` (if not internalized)
- `StagingServicePort` (if not internalized)
- `ChallengeOutcomeApprovalServicePort` (if not internalized)

### Step 7: Documentation updates

Update:

- `docs/architecture/hexagonal-architecture.md`
- `AGENTS.md`

### Step 8: Verification loop

```bash
cargo xtask arch-check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

---

## Estimated Effort

| Step | Description | Effort |
|------|-------------|--------|
| Step 0 | Pre-flight checks | 15 min |
| Step 1 | Create internal traits directory | 15 min |
| Step 2 | Internalize 20 traits | 2-3 hours |
| Step 3 | Create new inbound use case ports | 2-3 hours |
| Step 4 | Update AppStatePort and HTTP handlers | 2-3 hours |
| Step 5 | Delete wrapper-forwarder adapters | 2-3 hours |
| Step 6 | Clean up remaining outbound ports | 1 hour |
| Step 7 | Documentation updates | 30 min |
| Step 8 | Verification loop | 30 min |
| **Total** | | **11-14 hours** |

---

## Future Work (Out of Scope)

These are follow-up improvements after this migration:

### 1. Apply ISP to Inbound Use Case Ports

If any new use case port becomes too large (10+ methods), split into focused sub-ports:

```rust
// Example split
pub trait AssetQueryUseCasePort: Send + Sync { ... }
pub trait AssetMutationUseCasePort: Send + Sync { ... }
pub trait AssetGenerationUseCasePort: Send + Sync { ... }
```

### 2. Consolidate App-Layer Service Traits

After internalization, review whether internal traits are still needed or if concrete types suffice:

- If a trait has only one implementation and no mocking need → use concrete type
- If a trait is used for testing/mocking → keep as internal trait

---

## Success Criteria

- [ ] All app-implemented `*ServicePort` traits removed from `engine-ports`
- [ ] 20 internal traits moved to `engine-app/src/application/services/internal/`
- [ ] 11 service getters in `AppStatePort` replaced with use case ports
- [ ] Only 2 `*ServicePort` traits remain in `outbound/` (staging adapter ports)
- [ ] All wrapper-forwarder adapters deleted
- [ ] `cargo xtask arch-check` passes
- [ ] `cargo check --workspace` compiles
- [ ] `cargo test --workspace` passes
- [ ] Documentation updated

---

## Rollback Plan

Since this is a big-bang refactor with no backwards compatibility requirement:

1. If issues arise mid-refactor: `git checkout .` to abandon changes
2. If issues arise after commit: `git revert` the commit

No migration scripts or feature flags needed.

---

## Appendix: Full Trait Classification Table

| # | Trait | Classification | Action |
|---|-------|----------------|--------|
| 1 | `ActantialContextServicePort` | NOT A PORT | Internalize |
| 2 | `AssetGenerationQueueServicePort` | INBOUND | Replace with UseCasePort |
| 3 | `AssetServicePort` | INBOUND | Replace with UseCasePort |
| 4 | `ChallengeOutcomeApprovalServicePort` | OUTBOUND (wrapper) | Delete after removing wrapper |
| 5 | `ChallengeResolutionServicePort` | NOT A PORT | Internalize |
| 6 | `ChallengeServicePort` | NOT A PORT | Internalize |
| 7 | `DialogueContextServicePort` | NOT A PORT | Internalize |
| 8 | `DispositionServicePort` | NOT A PORT | Internalize |
| 9 | `DmActionQueueServicePort` | INBOUND | Replace with UseCasePort |
| 10 | `DmApprovalQueueServicePort` | INBOUND | Replace with UseCasePort |
| 11 | `EventChainServicePort` | NOT A PORT | Internalize |
| 12 | `GenerationQueueProjectionServicePort` | INBOUND | Replace with UseCasePort |
| 13 | `GenerationServicePort` | INBOUND | Replace with UseCasePort |
| 14 | `InteractionServicePort` | OUTBOUND (wrapper) | Delete after removing wrapper |
| 15 | `ItemServicePort` | NOT A PORT | Internalize |
| 16 | `LlmQueueServicePort` | INBOUND | Replace with UseCasePort |
| 17 | `LocationServicePort` | NOT A PORT | Internalize |
| 18 | `NarrativeEventApprovalServicePort` | NOT A PORT | Internalize |
| 19 | `NarrativeEventServicePort` | NOT A PORT | Internalize |
| 20 | `OutcomeTriggerServicePort` | NOT A PORT | Internalize |
| 21 | `PlayerActionQueueServicePort` | INBOUND | Replace with UseCasePort |
| 22 | `PlayerCharacterServicePort` | OUTBOUND (wrapper) | Delete after removing wrapper |
| 23 | `PromptContextServicePort` | NOT A PORT | Internalize |
| 24 | `PromptTemplateServicePort` | INBOUND | Replace with UseCasePort |
| 25 | `RegionServicePort` | NOT A PORT | Internalize |
| 26 | `RelationshipServicePort` | NOT A PORT | Internalize |
| 27 | `SceneResolutionServicePort` | NOT A PORT | Internalize |
| 28 | `SceneServicePort` | OUTBOUND (wrapper) | Delete after removing wrapper |
| 29 | `SettingsServicePort` | INBOUND | Replace with UseCasePort |
| 30 | `SheetTemplateServicePort` | NOT A PORT | Internalize |
| 31 | `SkillServicePort` | NOT A PORT | Internalize |
| 32 | `StagingServicePort` | OUTBOUND (wrapper) | Delete after removing wrapper |
| 33 | `StagingUseCaseServicePort` | OUTBOUND | **KEEP** (adapter-implemented) |
| 34 | `StagingUseCaseServiceExtPort` | OUTBOUND | **KEEP** (adapter-implemented) |
| 35 | `StoryEventAdminServicePort` | NOT A PORT | Internalize |
| 36 | `StoryEventQueryServicePort` | NOT A PORT | Internalize |
| 37 | `StoryEventRecordingServicePort` | NOT A PORT | Internalize |
| 38 | `StoryEventServicePort` | NOT A PORT | Internalize |
| 39 | `TriggerEvaluationServicePort` | NOT A PORT | Internalize |
| 40 | `WorkflowServicePort` | INBOUND | Replace with UseCasePort |
| 41 | `WorldServicePort` | OUTBOUND (wrapper) | Delete after removing wrapper |

**Summary**:
- **INBOUND** (replace with UseCasePort): 11 traits
- **OUTBOUND (keep)**: 2 traits
- **OUTBOUND (wrapper - delete)**: 6 traits
- **NOT A PORT (internalize)**: 20 traits

---

## References

- `docs/architecture/hexagonal-architecture.md` - Canonical architecture spec
- `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` - Overall refactor tracking
- `docs/plans/ARCHITECTURE_REMEDIATION_MASTER_PLAN.md` - Architecture remediation tracking
