# Service Port Inbound Migration Plan

> **Status**: PLANNED
> **Created**: 2026-01-02
> **Type**: Big Bang Refactor (no backwards compatibility required)

## Executive Summary

39 `*ServicePort` traits are incorrectly placed in `engine-ports/src/outbound/` when they should be in `engine-ports/src/inbound/`. These traits are **implemented by `engine-app`** and **called by adapters/handlers**, which is the canonical definition of an **inbound port** in this repo's hexagonal architecture.

Important nuance: today, several of these traits are also used as **engine-app internal dependency boundaries** (e.g., use cases/services store `Arc<dyn *ServicePort>`). After migration, canonical rules require that **application code does not depend on inbound ports**. This plan therefore includes an explicit phase to remove/replace those internal app dependencies.

This plan describes a complete restructure to fix the port taxonomy and establish a clean hexagonal architecture.

---

## Problem Statement

### Current State (Incorrect)

```
engine-ports/src/outbound/
├── character_service_port.rs      # Implemented by engine-app, called by adapters
├── world_service_port.rs          # Implemented by engine-app, called by adapters
├── challenge_service_port.rs      # Implemented by engine-app, called by adapters
└── ... (39 total misplaced traits)
```

### Why This Is Wrong

Per `docs/architecture/hexagonal-architecture.md`:

> **Outbound ports (driven ports)**
>
> - Define what the application _needs_ from the outside world.
> - **Implemented by: adapters.**
> - Depended on by: application (use cases/services).

But these 39 `*ServicePort` traits are:

- **Implemented by**: `engine-app` (not adapters)
- **Called by**: adapters and handlers (not depended on by app)

This is the **opposite** of outbound - it's inbound.

### Target State (Correct)

```
engine-ports/src/inbound/
├── movement_use_case_port.rs       # (existing - correct)
├── challenge_use_case_port.rs      # (existing - correct)
├── ... (other *_use_case_port.rs files)
├── services/
│   ├── character/
│   │   ├── mod.rs
│   │   ├── character_query_port.rs
│   │   ├── character_mutation_port.rs
│   │   └── character_archetype_port.rs
│   ├── world/
│   │   ├── world_query_port.rs
│   │   ├── world_mutation_port.rs
│   │   └── world_time_port.rs
│   └── ...
└── app_state_port.rs               # (existing - correct)

engine-ports/src/outbound/
├── character_repository/            # (existing - correct, keep as-is)
├── challenge_repository/            # (existing - correct, keep as-is)
├── ... (other repository/ submodules)
├── clock_port.rs                    # (existing - correct, keep as-is)
├── random_port.rs                   # (existing - correct, keep as-is)
├── broadcast_port.rs                # (existing - correct, keep as-is)
└── staging_use_case_service_ports.rs # (correct - adapter-implemented)
```

---

## Scope

### Traits to Move (39 total)

All of these are currently in `engine-ports/src/outbound/` and will move to `engine-ports/src/inbound/services/`:

| Current File                                  | Trait Name                             | New Location                         |
| --------------------------------------------- | -------------------------------------- | ------------------------------------ |
| `actantial_context_service_port.rs`           | `ActantialContextServicePort`          | `inbound/services/actantial/`        |
| `asset_generation_queue_service_port.rs`      | `AssetGenerationQueueServicePort`      | `inbound/services/asset/`            |
| `asset_service_port.rs`                       | `AssetServicePort`                     | `inbound/services/asset/`            |
| `challenge_outcome_approval_service_port.rs`  | `ChallengeOutcomeApprovalServicePort`  | `inbound/services/challenge/`        |
| `challenge_resolution_service_port.rs`        | `ChallengeResolutionServicePort`       | `inbound/services/challenge/`        |
| `challenge_service_port.rs`                   | `ChallengeServicePort`                 | `inbound/services/challenge/`        |
| `dialogue_context_service_port.rs`            | `DialogueContextServicePort`           | `inbound/services/story_event/`      |
| `disposition_service_port.rs`                 | `DispositionServicePort`               | `inbound/services/character/`        |
| `dm_action_queue_service_port.rs`             | `DmActionQueueServicePort`             | `inbound/services/queue/`            |
| `dm_approval_queue_service_port.rs`           | `DmApprovalQueueServicePort`           | `inbound/services/queue/`            |
| `event_chain_service_port.rs`                 | `EventChainServicePort`                | `inbound/services/narrative/`        |
| `generation_queue_projection_service_port.rs` | `GenerationQueueProjectionServicePort` | `inbound/services/asset/`            |
| `generation_service_port.rs`                  | `GenerationServicePort`                | `inbound/services/asset/`            |
| `interaction_service_port.rs`                 | `InteractionServicePort`               | `inbound/services/interaction/`      |
| `item_service_port.rs`                        | `ItemServicePort`                      | `inbound/services/item/`             |
| `llm_queue_service_port.rs`                   | `LlmQueueServicePort`                  | `inbound/services/queue/`            |
| `location_service_port.rs`                    | `LocationServicePort`                  | `inbound/services/location/`         |
| `narrative_event_approval_service_port.rs`    | `NarrativeEventApprovalServicePort`    | `inbound/services/narrative/`        |
| `narrative_event_service_port.rs`             | `NarrativeEventServicePort`            | `inbound/services/narrative/`        |
| `outcome_trigger_service_port.rs`             | `OutcomeTriggerServicePort`            | `inbound/services/challenge/`        |
| `player_action_queue_service_port.rs`         | `PlayerActionQueueServicePort`         | `inbound/services/queue/`            |
| `player_character_service_port.rs`            | `PlayerCharacterServicePort`           | `inbound/services/player_character/` |
| `prompt_context_service_port.rs`              | `PromptContextServicePort`             | `inbound/services/llm/`              |
| `prompt_template_service_port.rs`             | `PromptTemplateServicePort`            | `inbound/services/llm/`              |
| `region_service_port.rs`                      | `RegionServicePort`                    | `inbound/services/region/`           |
| `relationship_service_port.rs`                | `RelationshipServicePort`              | `inbound/services/character/`        |
| `scene_resolution_service_port.rs`            | `SceneResolutionServicePort`           | `inbound/services/scene/`            |
| `scene_service_port.rs`                       | `SceneServicePort`                     | `inbound/services/scene/`            |
| `settings_service_port.rs`                    | `SettingsServicePort`                  | `inbound/services/settings/`         |
| `sheet_template_service_port.rs`              | `SheetTemplateServicePort`             | `inbound/services/player_character/` |
| `skill_service_port.rs`                       | `SkillServicePort`                     | `inbound/services/skill/`            |
| `staging_service_port.rs`                     | `StagingServicePort`                   | `inbound/services/staging/`          |
| `story_event_admin_service_port.rs`           | `StoryEventAdminServicePort`           | `inbound/services/story_event/`      |
| `story_event_query_service_port.rs`           | `StoryEventQueryServicePort`           | `inbound/services/story_event/`      |
| `story_event_recording_service_port.rs`       | `StoryEventRecordingServicePort`       | `inbound/services/story_event/`      |
| `story_event_service_port.rs`                 | `StoryEventServicePort`                | `inbound/services/story_event/`      |
| `trigger_evaluation_service_port.rs`          | `TriggerEvaluationServicePort`         | `inbound/services/narrative/`        |
| `workflow_service_port.rs`                    | `WorkflowServicePort`                  | `inbound/services/asset/`            |
| `world_service_port.rs`                       | `WorldServicePort`                     | `inbound/services/world/`            |

### Traits to Keep in Outbound (2 total)

These are correctly placed (implemented by adapters):

| File                                | Trait Name                     | Reason                           |
| ----------------------------------- | ------------------------------ | -------------------------------- |
| `staging_use_case_service_ports.rs` | `StagingUseCaseServicePort`    | Implemented by `engine-adapters` |
| `staging_use_case_service_ports.rs` | `StagingUseCaseServiceExtPort` | Implemented by `engine-adapters` |

### Anti-Pattern to Remove

The `connection_port_adapters.rs` file contains adapter wrappers that violate hexagonal architecture:

```rust
// ANTI-PATTERN: Adapter wrapping app service to implement another port
pub struct PlayerCharacterServiceAdapter {
    service: Arc<dyn PlayerCharacterServicePort>,  // depends on "outbound" port
}

impl PlayerCharacterDtoPort for PlayerCharacterServiceAdapter { ... }
```

This will be refactored during the migration.

---

## New Directory Structure

### `engine-ports/src/inbound/` (After Migration)

```
inbound/
├── mod.rs
├── app_state_port.rs              # (existing)
├── challenge_use_case_port.rs      # (existing)
├── connection_use_case_port.rs     # (existing)
├── inventory_use_case_port.rs      # (existing)
├── movement_use_case_port.rs       # (existing)
├── narrative_event_use_case_port.rs # (existing)
├── observation_use_case_port.rs    # (existing)
├── player_action_use_case_port.rs  # (existing)
├── scene_use_case_port.rs          # (existing)
├── staging_use_case_port.rs        # (existing)
├── request_handler.rs             # (existing)
├── use_case_context.rs            # (existing)
├── use_case_ports.rs              # (existing)
├── use_cases.rs                   # (existing; currently unused/planned)

└── services/                      # (NEW)
    ├── mod.rs
    ├── actantial/
    │   ├── mod.rs
    │   └── actantial_context_service_port.rs
    ├── asset/
    │   ├── mod.rs
    │   ├── asset_service_port.rs
    │   ├── asset_generation_queue_service_port.rs
    │   ├── generation_service_port.rs
    │   ├── generation_queue_projection_service_port.rs
    │   └── workflow_service_port.rs
    ├── challenge/
    │   ├── mod.rs
    │   ├── challenge_service_port.rs
    │   ├── challenge_resolution_service_port.rs
    │   ├── challenge_outcome_approval_service_port.rs
    │   └── outcome_trigger_service_port.rs
    ├── character/
    │   ├── mod.rs
    │   ├── disposition_service_port.rs
    │   └── relationship_service_port.rs
    ├── interaction/
    │   ├── mod.rs
    │   └── interaction_service_port.rs
    ├── item/
    │   ├── mod.rs
    │   └── item_service_port.rs
    ├── llm/
    │   ├── mod.rs
    │   ├── prompt_context_service_port.rs
    │   └── prompt_template_service_port.rs
    ├── location/
    │   ├── mod.rs
    │   └── location_service_port.rs
    ├── narrative/
    │   ├── mod.rs
    │   ├── event_chain_service_port.rs
    │   ├── narrative_event_service_port.rs
    │   ├── narrative_event_approval_service_port.rs
    │   └── trigger_evaluation_service_port.rs
    ├── player_character/
    │   ├── mod.rs
    │   ├── player_character_service_port.rs
    │   └── sheet_template_service_port.rs
    ├── queue/
    │   ├── mod.rs
    │   ├── dm_action_queue_service_port.rs
    │   ├── dm_approval_queue_service_port.rs
    │   ├── llm_queue_service_port.rs
    │   └── player_action_queue_service_port.rs
    ├── region/
    │   ├── mod.rs
    │   └── region_service_port.rs
    ├── scene/
    │   ├── mod.rs
    │   ├── scene_service_port.rs
    │   └── scene_resolution_service_port.rs
    ├── settings/
    │   ├── mod.rs
    │   └── settings_service_port.rs
    ├── skill/
    │   ├── mod.rs
    │   └── skill_service_port.rs
    ├── staging/
    │   ├── mod.rs
    │   └── staging_service_port.rs
    ├── story_event/
    │   ├── mod.rs
    │   ├── story_event_service_port.rs
    │   ├── story_event_query_service_port.rs
    │   ├── story_event_recording_service_port.rs
    │   ├── story_event_admin_service_port.rs
    │   └── dialogue_context_service_port.rs
    └── world/
        ├── mod.rs
        └── world_service_port.rs
```

---

## Implementation Phases

## Concrete Execution Checklist (Big Bang)

This is an ordered checklist intended to be executed top-to-bottom. It is more specific than the phase estimates below and is the source of truth for the big-bang sequence.

### 0) Pre-flight

1. Ensure a clean working tree.
2. Record baseline failures (if any):

```bash
cargo xtask arch-check
cargo check --workspace
```

### 1) REQUIRED: Remove `engine-app` → inbound-service dependencies (do this first)

After the move, canonical rules require **application code not to depend on inbound ports**. Today, `engine-app` frequently uses `Arc<dyn *ServicePort>` as internal dependency boundaries.

Do this now so the later file moves don’t leave you with app code importing `wrldbldr_engine_ports::inbound::services::*`.

1. Find all dependencies:

```bash
rg -n "Arc<dyn\s+[A-Za-z0-9_]+ServicePort\b" crates/engine-app/src
rg -n "wrldbldr_engine_ports::outbound::[A-Za-z0-9_]+ServicePort\b" crates/engine-app/src
```

2. For each usage, choose one:

   - **Internal collaboration**: depend on a concrete engine-app type, or move the trait boundary into `engine-app`.
   - **True external dependency**: depend on a focused outbound port (repository/queue/clock/etc.).

3. Confirm the directionality before proceeding:

```bash
cargo xtask arch-check
```

### 2) Create `inbound/services/**` module skeleton

1. Create directories:

```bash
mkdir -p crates/engine-ports/src/inbound/services/{actantial,asset,challenge,character,interaction,item,llm,location,narrative,player_character,queue,region,scene,settings,skill,staging,story_event,world}
```

2. Add `mod.rs` files:
   - `crates/engine-ports/src/inbound/services/mod.rs`
   - One `mod.rs` per subdirectory above.

### 3) Move the 39 service-port files using `git mv`

Run the `git mv` commands listed in the “Traits to Move” mapping above (grouped by domain). Keep `staging_use_case_service_ports.rs` in outbound.

### 4) Update engine-ports module exports

1. `crates/engine-ports/src/inbound/mod.rs`

   - Add `pub mod services;`
   - Prefer re-exporting the moved traits at `inbound::*` to minimize import churn.

2. `crates/engine-ports/src/outbound/mod.rs`
   - Remove `pub mod` and `pub use` entries for moved service ports.

### 5) Update import paths across the workspace

Perform workspace-wide replacements (repeat per trait name as needed):

```text
wrldbldr_engine_ports::outbound::XServicePort  ->  wrldbldr_engine_ports::inbound::XServicePort
```

Then fix remaining compilation errors by updating grouped import lists and composition wiring.

### 6) Update `AppStatePort` to return inbound service ports

`AppStatePort` currently returns many `Arc<dyn ...ServicePort>` types from `outbound`. After the move:

1. Update imports in `crates/engine-ports/src/inbound/app_state_port.rs`.
2. Update all return types of getters (e.g., `asset_service()`, `llm_queue_service()`, `generation_service()`).
3. Update `engine-composition`’s concrete `AppStatePort` implementation.

### 7) DECIDED anti-pattern fix: delete the connection callback ports + wrapper adapters

Goal: remove the `engine-adapters` wrappers that implement outbound DTO ports by calling back into application services.

Concrete approach:

1. In `ConnectionUseCase`, replace:

   - `WorldSnapshotJsonPort` → `WorldExporterPort` (convert `PlayerWorldSnapshot` to `serde_json::Value` in the use case).
   - `PlayerCharacterDtoPort` → outbound player-character repository/query port(s) (build `PcData` in the use case).

2. Delete:

   - `crates/engine-adapters/src/infrastructure/connection_port_adapters.rs`
   - `crates/engine-ports/src/outbound/world_snapshot_json_port.rs`
   - `crates/engine-ports/src/outbound/player_character_dto_port.rs`

3. Remove the exports from `crates/engine-ports/src/outbound/mod.rs` and update any references.

Helper search:

```bash
rg -n "WorldSnapshotJsonPort|PlayerCharacterDtoPort" crates
```

### 8) Documentation updates

Update `docs/architecture/hexagonal-architecture.md` and `AGENTS.md` to reflect the new location of service ports and remove/adjust any “taxonomy drift” notes.

### 9) Verification loop

Run in this order and fix errors iteratively:

```bash
cargo xtask arch-check
cargo check --workspace
cargo test --workspace
```

Only after the above, run:

```bash
cargo clippy --workspace --all-targets
```

---

## Estimated Effort

| Phase     | Description                  | Effort          |
| --------- | ---------------------------- | --------------- |
| Phase 1   | Create directory structure   | 1 hour          |
| Phase 2   | Move trait files             | 2-3 hours       |
| Phase 3   | Update module exports        | 1-2 hours       |
| Phase 4   | Update import paths          | 3-4 hours       |
| Phase 5   | Remove adapter anti-patterns | 1-2 hours       |
| Phase 6   | Update documentation         | 1 hour          |
| Phase 7   | Verification                 | 1 hour          |
| **Total** |                              | **10-14 hours** |

---

## Future Work (Out of Scope)

These are follow-up improvements after this migration:

### 1. Eliminate Dual Trait Pattern

Currently, `engine-app` defines both:

- `CharacterService` (app-layer trait with 17 methods)
- `CharacterServicePort` (port with 3 methods, now in inbound)

Future work: Consolidate into one trait in `inbound/` with ISP splits.

### 2. Apply ISP to Service Ports

Split large service ports into focused sub-ports:

```rust
// Instead of one CharacterServicePort with 17 methods
pub trait CharacterQueryPort: Send + Sync { ... }      // 3 methods
pub trait CharacterMutationPort: Send + Sync { ... }   // 4 methods
pub trait CharacterArchetypePort: Send + Sync { ... }  // 3 methods
```

### 3. Strict Hexagonal for Adapters

Currently, some adapters call inbound ports to query data. Strict hexagonal would have:

- Use cases pass all needed data to outbound port calls
- Adapters never call back into the application

This is a larger architectural change that can be done incrementally.

---

## Success Criteria

- [ ] All 39 `*ServicePort` traits moved to `engine-ports/src/inbound/services/`
- [ ] Only 2 `*ServicePort` traits remain in `outbound/` (staging adapter ports)
- [ ] `engine-app` does not depend on any inbound `services/*` traits
- [ ] All import paths updated across the codebase
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

## References

- `docs/architecture/hexagonal-architecture.md` - Canonical architecture spec
- `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` - Overall refactor tracking
- `docs/plans/ARCHITECTURE_REMEDIATION_MASTER_PLAN.md` - Architecture remediation tracking
