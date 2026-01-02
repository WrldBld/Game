# ServicePort Purist Refactor Plan (Option B / Big Bang)

> **Status**: IN PROGRESS
> **Created**: 2026-01-02
> **Updated**: 2026-01-02
> **Type**: Big Bang Refactor (no backwards compatibility required)

## Progress

- [x] Step 0: Pre-flight checks
- [x] Step 1: Create internal traits directory
- [x] Step 2: Internalize 24 NOT-A-PORT traits (commit `8267811`)
- [ ] Step 3: Handle 11 INBOUND service ports (REVISED - see below)
- [ ] Step 4: Update AppStatePort and HTTP handlers
- [ ] Step 5: Delete wrapper-forwarder adapters
- [ ] Step 6: Clean up remaining outbound service ports
- [ ] Step 7: Documentation updates
- [ ] Step 8: Final verification

## Executive Summary

The engine currently defines **41** `*ServicePort` traits under `crates/engine-ports/src/outbound/`, plus the extension trait `StagingUseCaseServiceExtPort` (**42 service-related traits total**). Most of these traits are **implemented by `engine-app`**, and several are **called by adapters** (primarily via `AppStatePort` getters or via wrapper-forwarder adapters).

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
└── ... (41 total `*ServicePort` traits)
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

| Trait                                  | AppStatePort Method                     | Called by Adapter           |
| -------------------------------------- | --------------------------------------- | --------------------------- |
| `SettingsServicePort`                  | `settings_service()`                    | `settings_routes.rs`        |
| `PromptTemplateServicePort`            | `prompt_template_service()`             | `prompt_template_routes.rs` |
| `AssetServicePort`                     | `asset_service()`                       | `asset_routes.rs`           |
| `GenerationServicePort`                | `generation_service()`                  | `asset_routes.rs`           |
| `AssetGenerationQueueServicePort`      | `asset_generation_queue_service()`      | `queue_routes.rs`           |
| `WorkflowServicePort`                  | `workflow_service()`                    | `workflow_routes.rs`        |
| `GenerationQueueProjectionServicePort` | `generation_queue_projection_service()` | `queue_routes.rs`           |
| `PlayerActionQueueServicePort`         | `player_action_queue_service()`         | `queue_routes.rs`           |
| `LlmQueueServicePort`                  | `llm_queue_service()`                   | `queue_routes.rs`           |
| `DmApprovalQueueServicePort`           | `dm_approval_queue_service()`           | `queue_routes.rs`           |
| `WorldServicePort`                     | `world_service()`                       | `export_routes.rs`          |

> **Note**: `DmActionQueueServicePort` was previously listed here but is NOT exposed via `AppStatePort`. It is only used internally by composition code, making it NOT A PORT (see below).

**Action**: Create new inbound `*UseCasePort` traits and update HTTP handlers to call them.

### OUTBOUND (2 traits) - Keep in `engine-ports/src/outbound/`

These are adapter-implemented and remain valid outbound dependencies for application code:

| Trait                          | Reason                                 | Keep/Delete |
| ------------------------------ | -------------------------------------- | ----------- |
| `StagingUseCaseServicePort`    | Implemented by `StagingServiceAdapter` | **KEEP**    |
| `StagingUseCaseServiceExtPort` | Implemented by `StagingServiceAdapter` | **KEEP**    |

### ADAPTER-CALLED VIA WRAPPERS (5 traits) - Delete (anti-pattern)

These traits are implemented by `engine-app` but are currently called from adapters through wrapper-forwarder modules. Under the rubric, they are not “outbound”; they’re an adapter-calls-app coupling that Option B eliminates.

| Trait                                 | Reason                                     | Keep/Delete           |
| ------------------------------------- | ------------------------------------------ | --------------------- |
| `PlayerCharacterServicePort`          | Wrapped by `PlayerCharacterServiceAdapter` | DELETE (anti-pattern) |
| `SceneServicePort`                    | Wrapped by `SceneServiceAdapter`           | DELETE (anti-pattern) |
| `InteractionServicePort`              | Wrapped by `InteractionServiceAdapter`     | DELETE (anti-pattern) |
| `StagingServicePort`                  | Wrapped by `StagingServiceAdapter`         | DELETE (anti-pattern) |
| `ChallengeOutcomeApprovalServicePort` | Wrapped for approval flow                  | DELETE (anti-pattern) |

> **Note**: `WorldServicePort` is exposed via `AppStatePort.world_service()` and called by `export_routes.rs`, making it INBOUND (see above).

**Action**: Delete these service ports from `engine-ports` after removing wrapper-forwarder adapters. If any are still needed inside `engine-app`, internalize them into `engine-app/services/internal/` instead.

### NOT A PORT (24 traits) - Internalize to `engine-app`

These are purely app-internal: implemented by app, called by app, never touched by adapters:

| Trait                               | Current Caller                    | Target Location                 |
| ----------------------------------- | --------------------------------- | ------------------------------- |
| `ActantialContextServicePort`       | Composition only                  | `engine-app/services/internal/` |
| `ChallengeResolutionServicePort`    | Composition only                  | `engine-app/services/internal/` |
| `ChallengeServicePort`              | `ChallengeResolutionService`      | `engine-app/services/internal/` |
| `CharacterServicePort`              | Composition only                  | `engine-app/services/internal/` |
| `DialogueContextServicePort`        | `DmApprovalQueueService`          | `engine-app/services/internal/` |
| `DispositionServicePort`            | Composition only                  | `engine-app/services/internal/` |
| `DmActionQueueServicePort`          | Composition only                  | `engine-app/services/internal/` |
| `EventChainServicePort`             | Composition only                  | `engine-app/services/internal/` |
| `ItemServicePort`                   | Composition only                  | `engine-app/services/internal/` |
| `LocationServicePort`               | Composition only                  | `engine-app/services/internal/` |
| `NarrativeEventApprovalServicePort` | `NarrativeEventUseCase`           | `engine-app/services/internal/` |
| `NarrativeEventServicePort`         | `NarrativeEventApprovalService`   | `engine-app/services/internal/` |
| `OutcomeTriggerServicePort`         | `ChallengeOutcomeApprovalService` | `engine-app/services/internal/` |
| `PromptContextServicePort`          | Composition only                  | `engine-app/services/internal/` |
| `RegionServicePort`                 | Composition only                  | `engine-app/services/internal/` |
| `RelationshipServicePort`           | Composition only                  | `engine-app/services/internal/` |
| `SceneResolutionServicePort`        | Composition only                  | `engine-app/services/internal/` |
| `SheetTemplateServicePort`          | Composition only                  | `engine-app/services/internal/` |
| `SkillServicePort`                  | `ChallengeResolutionService`      | `engine-app/services/internal/` |
| `StoryEventAdminServicePort`        | Composition only                  | `engine-app/services/internal/` |
| `StoryEventQueryServicePort`        | Composition only                  | `engine-app/services/internal/` |
| `StoryEventRecordingServicePort`    | `NarrativeEventApprovalService`   | `engine-app/services/internal/` |
| `StoryEventServicePort`             | Composition only                  | `engine-app/services/internal/` |
| `TriggerEvaluationServicePort`      | Composition only                  | `engine-app/services/internal/` |

**Action**: Move these traits to `crates/engine-app/src/application/services/internal/` and remove from `engine-ports`.

---

## Scope

### 1) Replace adapter access to "service ports" with inbound use cases (REQUIRED)

`AppStatePort` currently exposes 11 `Arc<dyn *ServicePort>` getters called directly by HTTP routes.

Option B replaces these with inbound use case ports and updates adapters to call use cases.

### 2) Internalize app-only `*ServicePort` traits (REQUIRED)

The 24 traits classified as "NOT A PORT" must be:

1. Moved to `crates/engine-app/src/application/services/internal/`
2. Removed from `engine-ports`
3. Imports updated across `engine-app`

### 3) Keep only adapter-implemented outbound ports (REQUIRED)

After this refactor, the only remaining outbound “service” traits are:

- `StagingUseCaseServicePort` (the only remaining `*ServicePort`)
- `StagingUseCaseServiceExtPort` (extension trait)

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

### Step 2: Internalize the 24 "NOT A PORT" traits

For each of the 24 traits:

1. Move trait file from `engine-ports/src/outbound/` to `engine-app/src/application/services/internal/`
2. Update imports in `engine-app`
3. Remove export from `engine-ports/src/outbound/mod.rs`

### Step 3: Handle 11 INBOUND service ports (REVISED)

> **IMPORTANT**: This step was revised after implementation attempt revealed architectural constraints.

#### The Problem

The original plan was to simply move the 11 service port traits from `outbound/` to `inbound/`. This failed because:

1. **arch-check violation**: The rule "application code must not depend on inbound ports" is correct. Application services (in `engine-app`) should not have dependencies on inbound ports as trait objects.

2. **Dual usage pattern**: These 11 service ports are used in two ways:
   - As interfaces that HTTP handlers call (INBOUND - correct)
   - As dependencies for other application services (violates inbound port rules)

For example, `PromptTemplateServicePort` is:
- Called by `prompt_template_routes.rs` (handler → service, correct for INBOUND)
- Depended on by `PromptBuilder`, `StagingService`, `SuggestionService`, etc. (service → service, NOT correct for INBOUND)

#### The Correct Approach

The 11 "INBOUND" service ports should be handled differently based on their usage:

**Option A: Internalize + Create Use Case Wrappers (Recommended)**

1. **Internalize the service traits** to `engine-app/services/internal/` (like the 24 NOT-A-PORT traits)
2. **Create thin inbound use case ports** in `engine-ports/src/inbound/` that:
   - Define only the methods HTTP handlers actually need
   - Are implemented by thin wrapper use cases in `engine-app/use_cases/`
   - Delegate to the internal service implementations

```
engine-ports/src/inbound/
├── settings_use_case_port.rs        # HTTP handler interface (INBOUND)
└── ...

engine-app/src/application/
├── services/
│   └── internal/
│       └── settings_service_port.rs  # Internal trait (NOT A PORT)
├── use_cases/
│   └── settings_use_case.rs          # Implements inbound port, delegates to service
└── services/
    └── settings_service.rs           # Implements internal trait
```

**Option B: Keep in Outbound (Pragmatic Short-Term)**

For now, keep the 11 service ports in `outbound/` with clear documentation that:
- They are called by HTTP handlers (technically incorrect for outbound)
- They are depended on by services (correct for outbound)
- This is a known architectural compromise

**Option C: Accept Mixed Usage (Alternative)**

Update the arch-check rule to allow services that **implement** inbound ports to import them (but not depend on them as trait objects). This is complex to enforce.

#### Current Decision: Option B (Pragmatic)

For this big-bang refactor, we will:

1. **Keep the 11 service ports in `outbound/`** for now
2. **Document the compromise** in the architecture docs
3. **Create a follow-up task** to properly refactor to Option A

This allows completing Steps 4-8 without blocking on a complex refactor.

#### Future Work (Option A Implementation)

When ready to implement Option A properly:

| Current ServicePort                    | Internal Trait                            | Inbound UseCase Port          |
| -------------------------------------- | ----------------------------------------- | ----------------------------- |
| `SettingsServicePort`                  | `internal::SettingsServicePort`           | `SettingsUseCasePort`         |
| `PromptTemplateServicePort`            | `internal::PromptTemplateServicePort`     | `PromptTemplateUseCasePort`   |
| `AssetServicePort`                     | `internal::AssetServicePort`              | `AssetUseCasePort`            |
| `GenerationServicePort`                | `internal::GenerationServicePort`         | `GenerationUseCasePort`       |
| `AssetGenerationQueueServicePort`      | `internal::AssetGenerationQueueServicePort` | (merge into GenerationUseCasePort) |
| `WorkflowServicePort`                  | `internal::WorkflowServicePort`           | `WorkflowUseCasePort`         |
| `GenerationQueueProjectionServicePort` | `internal::GenerationQueueProjectionServicePort` | `QueueProjectionUseCasePort` |
| `PlayerActionQueueServicePort`         | `internal::PlayerActionQueueServicePort`  | `QueueAdminUseCasePort`       |
| `LlmQueueServicePort`                  | `internal::LlmQueueServicePort`           | (merge into QueueAdminUseCasePort) |
| `DmApprovalQueueServicePort`           | `internal::DmApprovalQueueServicePort`    | (merge into QueueAdminUseCasePort) |
| `WorldServicePort`                     | `internal::WorldServicePort`              | `ExportUseCasePort`           |

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

After wrapper adapters are deleted, remove the 5 service ports that were only used by wrappers:

- `PlayerCharacterServicePort` (if not internalized)
- `SceneServicePort` (if not internalized)
- `InteractionServicePort` (if not internalized)
- `StagingServicePort` (if not internalized)
- `ChallengeOutcomeApprovalServicePort` (if not internalized)

> **Note**: `WorldServicePort` is INBOUND (exposed via AppStatePort), not a wrapper port.

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

### Phase 1 (Current Scope)

| Step      | Description                           | Effort     | Status      |
| --------- | ------------------------------------- | ---------- | ----------- |
| Step 0    | Pre-flight checks                     | 15 min     | **Done**    |
| Step 1    | Create internal traits directory      | 15 min     | **Done**    |
| Step 2    | Internalize 24 traits                 | 3-4 hours  | **Done**    |
| Step 3    | Handle 11 INBOUND ports (pragmatic)   | 30 min     | Skip (keep in outbound) |
| Step 4    | Update AppStatePort and HTTP handlers | N/A        | Skip (no changes needed) |
| Step 5    | Delete wrapper-forwarder adapters     | 2-3 hours  | Pending     |
| Step 6    | Clean up remaining outbound ports     | 1 hour     | Pending     |
| Step 7    | Documentation updates                 | 30 min     | Pending     |
| Step 8    | Verification loop                     | 30 min     | Pending     |
| **Total** |                                       | **~8 hours** |           |

### Phase 2 (Future - Option A)

| Step      | Description                           | Effort          |
| --------- | ------------------------------------- | --------------- |
| P2-1      | Internalize 11 service ports          | 2-3 hours       |
| P2-2      | Create 7 inbound use case ports       | 2-3 hours       |
| P2-3      | Create use case wrappers              | 2-3 hours       |
| P2-4      | Update AppStatePort                   | 1-2 hours       |
| P2-5      | Update HTTP handlers                  | 1-2 hours       |
| P2-6      | Verification                          | 1 hour          |
| **Total** |                                       | **10-14 hours** |

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

### Phase 1 (Current Scope)

- [x] 24 internal traits moved to `engine-app/src/application/services/internal/`
- [ ] All wrapper-forwarder adapters deleted (5 wrapper ports removed)
- [ ] 5 wrapper service ports deleted or internalized
- [ ] `cargo xtask arch-check` passes
- [ ] `cargo check --workspace` compiles
- [ ] `cargo test --workspace` passes
- [ ] Documentation updated

### Phase 2 (Future - Option A Implementation)

- [ ] 11 INBOUND service ports internalized to `engine-app/services/internal/`
- [ ] 7 new inbound use case ports created in `engine-ports/inbound/`
- [ ] Use case wrappers created in `engine-app/use_cases/`
- [ ] AppStatePort returns use case ports instead of service ports
- [ ] HTTP handlers call use case ports
- [ ] Only `StagingUseCaseServicePort` + `StagingUseCaseServiceExtPort` remain in `outbound/`

---

## Rollback Plan

Since this is a big-bang refactor with no backwards compatibility requirement:

1. If issues arise mid-refactor: `git checkout .` to abandon changes
2. If issues arise after commit: `git revert` the commit

No migration scripts or feature flags needed.

---

## Appendix: Full Trait Classification Table

| #   | Trait                                  | Classification           | Action                         |
| --- | -------------------------------------- | ------------------------ | ------------------------------ |
| 1   | `ActantialContextServicePort`          | NOT A PORT               | Internalize                    |
| 2   | `AssetGenerationQueueServicePort`      | INBOUND                  | Replace with UseCasePort       |
| 3   | `AssetServicePort`                     | INBOUND                  | Replace with UseCasePort       |
| 4   | `ChallengeOutcomeApprovalServicePort`  | ADAPTER-CALLED (wrapper) | Delete after removing wrapper  |
| 5   | `ChallengeResolutionServicePort`       | NOT A PORT               | Internalize                    |
| 6   | `ChallengeServicePort`                 | NOT A PORT               | Internalize                    |
| 7   | `CharacterServicePort`                 | NOT A PORT               | Internalize                    |
| 8   | `DialogueContextServicePort`           | NOT A PORT               | Internalize                    |
| 9   | `DispositionServicePort`               | NOT A PORT               | Internalize                    |
| 10  | `DmActionQueueServicePort`             | NOT A PORT               | Internalize                    |
| 11  | `DmApprovalQueueServicePort`           | INBOUND                  | Replace with UseCasePort       |
| 12  | `EventChainServicePort`                | NOT A PORT               | Internalize                    |
| 13  | `GenerationQueueProjectionServicePort` | INBOUND                  | Replace with UseCasePort       |
| 14  | `GenerationServicePort`                | INBOUND                  | Replace with UseCasePort       |
| 15  | `InteractionServicePort`               | ADAPTER-CALLED (wrapper) | Delete after removing wrapper  |
| 16  | `ItemServicePort`                      | NOT A PORT               | Internalize                    |
| 17  | `LlmQueueServicePort`                  | INBOUND                  | Replace with UseCasePort       |
| 18  | `LocationServicePort`                  | NOT A PORT               | Internalize                    |
| 19  | `NarrativeEventApprovalServicePort`    | NOT A PORT               | Internalize                    |
| 20  | `NarrativeEventServicePort`            | NOT A PORT               | Internalize                    |
| 21  | `OutcomeTriggerServicePort`            | NOT A PORT               | Internalize                    |
| 22  | `PlayerActionQueueServicePort`         | INBOUND                  | Replace with UseCasePort       |
| 23  | `PlayerCharacterServicePort`           | ADAPTER-CALLED (wrapper) | Delete after removing wrapper  |
| 24  | `PromptContextServicePort`             | NOT A PORT               | Internalize                    |
| 25  | `PromptTemplateServicePort`            | INBOUND                  | Replace with UseCasePort       |
| 26  | `RegionServicePort`                    | NOT A PORT               | Internalize                    |
| 27  | `RelationshipServicePort`              | NOT A PORT               | Internalize                    |
| 28  | `SceneResolutionServicePort`           | NOT A PORT               | Internalize                    |
| 29  | `SceneServicePort`                     | ADAPTER-CALLED (wrapper) | Delete after removing wrapper  |
| 30  | `SettingsServicePort`                  | INBOUND                  | Replace with UseCasePort       |
| 31  | `SheetTemplateServicePort`             | NOT A PORT               | Internalize                    |
| 32  | `SkillServicePort`                     | NOT A PORT               | Internalize                    |
| 33  | `StagingServicePort`                   | ADAPTER-CALLED (wrapper) | Delete after removing wrapper  |
| 34  | `StagingUseCaseServicePort`            | OUTBOUND                 | **KEEP** (adapter-implemented) |
| 35  | `StagingUseCaseServiceExtPort`         | OUTBOUND                 | **KEEP** (adapter-implemented) |
| 36  | `StoryEventAdminServicePort`           | NOT A PORT               | Internalize                    |
| 37  | `StoryEventQueryServicePort`           | NOT A PORT               | Internalize                    |
| 38  | `StoryEventRecordingServicePort`       | NOT A PORT               | Internalize                    |
| 39  | `StoryEventServicePort`                | NOT A PORT               | Internalize                    |
| 40  | `TriggerEvaluationServicePort`         | NOT A PORT               | Internalize                    |
| 41  | `WorkflowServicePort`                  | INBOUND                  | Replace with UseCasePort       |
| 42  | `WorldServicePort`                     | INBOUND                  | Replace with UseCasePort       |

**Summary**:

- **INBOUND** (replace with UseCasePort): 11 traits
- **OUTBOUND (keep)**: 2 traits
- **ADAPTER-CALLED via wrappers (delete)**: 5 traits
- **NOT A PORT (internalize)**: 24 traits
- **Total**: 41 `*ServicePort` traits + 1 extension trait = 42 traits (40 `*ServicePort` traits to refactor)

---

## References

- `docs/architecture/hexagonal-architecture.md` - Canonical architecture spec
- `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` - Overall refactor tracking
- `docs/plans/ARCHITECTURE_REMEDIATION_MASTER_PLAN.md` - Architecture remediation tracking
