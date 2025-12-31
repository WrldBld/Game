# Port Adapter Tech Debt Remediation Plan

## Executive Summary

The codebase has accumulated structural tech debt in multiple areas that violate hexagonal architecture principles. This document consolidates all identified issues and provides a prioritized remediation plan.

**Total Impact**:
- 8 of 10 use cases with port adapter anti-pattern
- 17 unnecessary adapter structs to delete
- 8 services with concrete type dependencies (validated)
- 5 IoC violations (services creating other services)
- 1 duplicate mock implementation
- 5 glob re-exports (already flagged by arch-check)
- 5 additional anti-patterns (handlers with logic, swallowed errors, etc.)

**Total Effort Estimate**: 5-6 days

**Last Updated**: Validated by automated analysis

---

## Issue Categories

| Priority | Issue | Count | Effort |
|----------|-------|-------|--------|
| **P1** | Use cases depending on inbound ports | 8 use cases, 17 adapters | 2-3 days |
| **P1** | Services depending on concrete types | 8 services (validated) | 0.5 day |
| **P2** | IoC violations (services creating services) | 5 locations | 0.5 day |
| **P2** | Handler with business logic | 1 location | 0.5 day |
| **P2** | Swallowed errors | 2 locations | 0.25 day |
| **P2** | Duplicate MockGameConnectionPort | 2 files | 0.25 day |
| **P3** | Direct rand usage bypassing RandomPort | 1 location | 0.25 day |
| **P3** | Multiple Arc<Mutex> anti-pattern | 1 location | 0.5 day |
| **P3** | Partial batch queue failure | 1 location | 0.5 day |
| **P3** | Glob re-exports in engine-dto | 5 locations | 0.25 day |
| **P3** | `Utc::now()` in test code | ~10 locations | Optional |

---

## Issue 1: Use Cases Depending on Inbound Ports (P1)

### The Problem

In proper hexagonal architecture:
- **Inbound ports**: Traits that use cases *implement* (e.g., `ChallengeUseCasePort`)
- **Outbound ports**: Traits that use cases *depend on* (e.g., `CharacterCrudPort`, `BroadcastPort`)
- **Adapters**: Implement outbound ports (e.g., `Neo4jCharacterRepository`)

```
CORRECT:
  UseCase
    → depends on → OutboundPort
      → implemented by → Adapter/Service

CURRENT (wrong):
  UseCase
    → depends on → InboundPort (misnamed)
      → implemented by → PortAdapter (unnecessary layer)
        → wraps → OutboundPort
          → implemented by → Service
```

### Affected Use Cases

| Use Case | Status | Dependencies with Anti-Pattern |
|----------|--------|-------------------------------|
| `InventoryUseCase` | ✅ Correct | None - uses only outbound ports |
| `ObservationUseCase` | ✅ Correct | None - uses only outbound ports |
| `MovementUseCase` | ❌ Affected | `StagingServicePort`, `StagingStatePort`, `Arc<SceneBuilder>` |
| `ChallengeUseCase` | ❌ Affected | `ChallengeResolutionPort`, `ChallengeOutcomeApprovalPort`, `DmApprovalQueuePort`, `WorldServicePort` |
| `ConnectionUseCase` | ❌ Affected | `ConnectionManagerPort`, `WorldServicePort`, `PlayerCharacterServicePort`, `DirectorialContextPort`, `WorldStatePort` |
| `PlayerActionUseCase` | ❌ Affected | `Arc<MovementUseCase>`, `PlayerActionQueuePort`, `DmNotificationPort` |
| `StagingApprovalUseCase` | ❌ Affected | `StagingServiceExtPort`, `StagingStateExtPort`, `Arc<SceneBuilder>` |
| `SceneUseCase` | ❌ Affected | `SceneServicePort`, `InteractionServicePort`, `WorldStatePort`, `DirectorialContextRepositoryPort`, `DmActionQueuePort` |
| `NarrativeEventUseCase` | ❌ Affected | `Arc<NarrativeEventApprovalService<N>>` (concrete type) |
| `SceneBuilder` | ✅ Correct | Helper class, uses only outbound ports |

### Port Adapters to Delete (17 total)

Located in `crates/engine-adapters/src/infrastructure/ports/`:

| File | Adapters | Count |
|------|----------|-------|
| `challenge_adapters.rs` | `ChallengeResolutionAdapter`, `ChallengeOutcomeApprovalAdapter`, `ChallengeDmApprovalQueueAdapter` | 3 |
| `player_action_adapters.rs` | `PlayerActionQueueAdapter`, `DmNotificationAdapter` | 2 |
| `staging_state_adapter.rs` | `StagingStateAdapter` | 1 |
| `staging_service_adapter.rs` | `StagingServiceAdapter` | 1 |
| `scene_adapters.rs` | `SceneServiceAdapter`, `InteractionServiceAdapter`, `SceneWorldStateAdapter`, `DirectorialContextAdapter`, `DmActionQueuePlaceholder` | 5 |
| `connection_adapters.rs` | `WorldServiceAdapter`, `PlayerCharacterServiceAdapter`, `ConnectionDirectorialContextAdapter`, `ConnectionWorldStateAdapter` | 4 |
| `connection_manager_adapter.rs` | `ConnectionManagerAdapter` | 1 |

### Duplicate Port Traits to Consolidate

These traits exist in both `inbound/use_case_ports.rs` and `outbound/`:

| Port Name | Inbound Version | Outbound Version | Resolution |
|-----------|-----------------|------------------|------------|
| `SceneServicePort` | Thin subset | Full interface | Keep outbound, delete inbound |
| `InteractionServicePort` | Thin subset | Full interface | Keep outbound, delete inbound |
| `WorldServicePort` | Only `export_world_snapshot` | Full CRUD + snapshot | Keep outbound, delete inbound |
| `PlayerCharacterServicePort` | Only `get_pc` | Full interface | Keep outbound, delete inbound |
| `StagingServicePort` | Subset | Full interface | Keep outbound, delete inbound |
| `DirectorialContextRepositoryPort` | Different interface | Full interface | Keep outbound, delete inbound |

### Non-Duplicate "Inbound" Ports to Move to Outbound

These exist only in `inbound/use_case_ports.rs` but are used as dependencies:

- `ChallengeResolutionPort` → Merge with `ChallengeResolutionServicePort`
- `ChallengeOutcomeApprovalPort` → Merge with `ChallengeOutcomeApprovalServicePort`
- `ChallengeDmApprovalQueuePort` → Merge with `DmApprovalQueueServicePort`
- `ConnectionManagerPort` → Move to outbound
- `WorldStatePort` → Move to outbound
- `StagingStatePort` → Move to outbound
- `StagingStateExtPort` → Move to outbound
- `StagingServiceExtPort` → Move to outbound
- `PlayerActionQueuePort` → Move to outbound
- `DmNotificationPort` → Move to outbound
- `SceneDmActionQueuePort` → Move to outbound
- `DirectorialContextPort` → Move to outbound
- `NarrativeRollContext` → Keep (this is a DTO, not a port)

### Import Path Impact (96 files affected)

| Area | Files Affected | Import Changes Required |
|------|----------------|------------------------|
| Use Cases | 8 files | Change `inbound::*Port` to `outbound::*Port` |
| WebSocket Handlers | 15+ files | Change `AppStatePort`, `UseCaseContext` imports |
| HTTP Routes | 6 files | Change `AppStatePort` imports |
| Port Adapters | 7 files | Will be deleted, but need to update callers first |
| Engine App Handlers | 10 files | Change `RequestContext` imports |
| Player UI | 2 files | Change `PlayerEvent`, `CharacterPosition` imports |
| Composition Layer | 7+ files | Update use case port imports |

---

## Issue 2: Services Depending on Concrete Types (P1)

### The Problem

Application services should depend on port traits (`Arc<dyn SomePort>`), not concrete service types (`Arc<SomeService>`). This violates Dependency Inversion Principle and makes unit testing difficult.

### Validated Affected Services

| Service | File | Concrete Dependencies | Status |
|---------|------|----------------------|--------|
| `ChallengeOutcomeApprovalService` | `challenge_outcome_approval_service.rs` | `Arc<OutcomeTriggerService>`, `Arc<SettingsService>`, `Arc<PromptTemplateService>` | ✅ Verified |
| `SuggestionService` | `suggestion_service.rs` | `Arc<PromptTemplateService>` | ✅ Verified |
| `PromptBuilder` | `llm/prompt_builder.rs` | `Arc<PromptTemplateService>` | ✅ Verified |
| `StagingService` | `staging_service.rs` | `Arc<PromptTemplateService>` | ✅ Verified |
| `LLMQueueService` | `llm_queue_service.rs` | `Arc<PromptTemplateService>` | ✅ Verified |
| `OutcomeSuggestionService` | `outcome_suggestion_service.rs` | `Arc<PromptTemplateService>` | ✅ Verified |
| `AppRequestHandler` | `handlers/request_handler.rs` | `Arc<SheetTemplateService>`, `Arc<GenerationQueueProjectionService>` | ✅ Verified |
| `GenerationQueueProjectionService` | `generation_queue_projection_service.rs` | `AssetServiceImpl` (concrete) | ✅ Verified |
| `LLMService` | `llm/mod.rs` | `Arc<PromptTemplateService>` | ✅ **Added** (was missed) |

### Services Incorrectly Listed (Now Removed)

| Service | Reason for Removal |
|---------|-------------------|
| `NarrativeEventApprovalService` | Uses generic `N: NarrativeEventService` and `Arc<dyn StoryEventService>` - correct abstractions |
| `StagingContextProvider` | Uses `Arc<dyn StoryEventService>` - correct abstraction |
| `DMApprovalQueueService.ItemService` | Uses generic `I: ItemService` - correct abstraction |

### Port Traits Status

**Already Exist (use these instead of creating new):**
| Service | Existing Port Trait |
|---------|-------------------|
| `PromptTemplateService` | `PromptTemplateServicePort` |
| `SettingsService` | `SettingsServicePort` |
| `SheetTemplateService` | `SheetTemplateServicePort` |
| `GenerationQueueProjectionService` | `GenerationQueueProjectionServicePort` |
| `AssetService` | `AssetServicePort` |

**Need to Create:**
| Service | New Port Trait |
|---------|---------------|
| `OutcomeTriggerService` | `OutcomeTriggerServicePort` |
| `ToolExecutionService` | `ToolExecutionServicePort` |
| `NarrativeEventApprovalService` | `NarrativeEventApprovalServicePort` |

---

## Issue 3: IoC Violations - Services Creating Other Services (P2)

### The Problem

Some services create other services inside their constructors or methods, violating Inversion of Control. Dependencies should be injected, not instantiated internally.

### All Violations (5 total)

| # | File | Line | Code | Context |
|---|------|------|------|---------|
| 1 | `dm_approval_queue_service.rs` | 70 | `ToolExecutionService::new()` | Constructor |
| 2 | `llm_queue_service.rs` | 93-96 | `LLMService::new(...)` | Constructor |
| 3 | `llm_queue_service.rs` | 594 | `SuggestionService::new(...)` | Spawned task in worker |
| 4 | `challenge_outcome_approval_service.rs` | 325 | `OutcomeSuggestionService::new(...)` | Spawned task |
| 5 | `challenge_outcome_approval_service.rs` | 720 | `OutcomeSuggestionService::new(...)` | Spawned task |

### Notes on Spawned Task Violations (#3-5)

Violations 3-5 are inside `tokio::spawn()` blocks, making them harder to refactor. Options:
1. Inject as `Arc<dyn SomeTrait>` and clone into the task
2. Use factory pattern - inject a factory that creates the service
3. Convert to stateless functions if no state needed

---

## Issue 4: Handler with Business Logic (P2)

### Location
`crates/engine-adapters/src/infrastructure/http/asset_routes.rs` - `queue_generation()` (lines 325-420)

### Problem
HTTP handler contains orchestration logic that belongs in a use case:
- Creates `GenerationBatch` entity directly in handler
- Loops to enqueue multiple generation items
- Contains batch-to-queue orchestration logic

### Fix
Extract to `AssetGenerationService.queue_batch()` in application layer.

---

## Issue 5: Swallowed Errors (P2)

### Location
`crates/engine-app/src/application/handlers/challenge_handler.rs` - lines 154-156 and 214-216

### Problem
```rust
let _ = challenge_service
    .set_required_skill(created.id, skill_id)
    .await;
```

After creating/updating a challenge, skill relationship errors are silently ignored. Users get success even if skill wasn't set.

### Fix
Propagate errors or return partial success response with warnings.

---

## Issue 6: Duplicate MockGameConnectionPort (P2)

### Problem
Two nearly identical mock implementations:
- `player-ports/src/outbound/testing/mock_game_connection.rs` (414 lines)
- `player-adapters/src/infrastructure/testing/mock_game_connection.rs` (366 lines)

### Resolution
Delete from `player-ports/outbound/testing/`, keep only in `player-adapters`.

**Files to modify:**
1. DELETE: `crates/player-ports/src/outbound/testing/mock_game_connection.rs`
2. MODIFY: `crates/player-ports/src/outbound/testing/mod.rs` - remove module
3. MODIFY: `crates/player-ports/src/outbound/mod.rs` - remove re-exports (lines 32-44)

---

## Issue 7: Direct rand Usage (P3)

### Location
`crates/engine-adapters/src/infrastructure/http/workflow_helpers.rs` - lines 8 and 103

### Problem
```rust
use rand::Rng;
let mut rng = rand::thread_rng();
```

Uses `rand` directly instead of `RandomPort`, making code non-deterministic and untestable.

### Fix
Accept `Arc<dyn RandomPort>` as parameter or move to application layer.

---

## Issue 8: Multiple Arc<Mutex> Anti-Pattern (P3)

### Location
`crates/engine-adapters/src/infrastructure/comfyui.rs` - lines 53-55 and 141-143

### Problem
```rust
struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_count: Arc<Mutex<u8>>,
    last_failure: Arc<Mutex<Option<DateTime<Utc>>>>,
}
```

Multiple individual locks for related state that should be atomic.

### Fix
Consolidate into single `Arc<RwLock<CircuitBreakerInner>>`.

---

## Issue 9: Partial Batch Queue Failure (P3)

### Location
`crates/engine-adapters/src/infrastructure/http/asset_routes.rs` - lines 370-406

### Problem
If enqueueing fails partway through a batch, some items are queued, some aren't. Error logged but batch shows success.

### Fix
Use transaction pattern (all-or-nothing) or track failed enqueues in batch status.

---

## Issue 10: Glob Re-Exports (P3)

### Location
`engine-dto/src/lib.rs` - lines 20-24

```rust
pub use llm::*;
pub use persistence::*;
pub use queue::*;
pub use request_context::*;
pub use staging::*;
```

### Fix
Replace with explicit exports. Already flagged by `cargo xtask arch-check`.

---

## Remediation Plan

### Phase 1: Template Fix (Day 1)

Fix `ChallengeUseCase` as a template:

1. Change dependencies from inbound ports to outbound ports
2. Update composition root to wire outbound ports directly
3. Delete `ChallengeResolutionAdapter`, `ChallengeOutcomeApprovalAdapter`, `ChallengeDmApprovalQueueAdapter`
4. Run tests, fix breakage

### Phase 2: Port Trait Migration (Day 1-2)

Move dependency port traits from inbound to outbound:

1. Move 12 port traits from `inbound/use_case_ports.rs` to appropriate files in `outbound/`
2. Update all import paths (96 files affected - use sed/codemod)
3. Remove duplicate traits from `inbound/use_case_ports.rs`
4. Keep actual use case ports (`*UseCasePort`) in `inbound/`

### Phase 3: Apply to Remaining Use Cases (Day 2)

Apply same transformation to:
- `ConnectionUseCase`
- `SceneUseCase`
- `MovementUseCase`
- `PlayerActionUseCase`
- `StagingApprovalUseCase`
- `NarrativeEventUseCase`

### Phase 4: Create Missing Port Traits (Day 2)

Create only the 3 port traits that don't exist:
- `OutcomeTriggerServicePort`
- `ToolExecutionServicePort`
- `NarrativeEventApprovalServicePort`

### Phase 5: Fix Service Dependencies (Day 3)

Update services to use existing port traits instead of concrete types:

| Service | Change To |
|---------|-----------|
| `ChallengeOutcomeApprovalService` | `Arc<dyn PromptTemplateServicePort>`, `Arc<dyn SettingsServicePort>`, `Arc<dyn OutcomeTriggerServicePort>` |
| `SuggestionService` | `Arc<dyn PromptTemplateServicePort>` |
| `PromptBuilder` | `Arc<dyn PromptTemplateServicePort>` |
| `StagingService` | `Arc<dyn PromptTemplateServicePort>` |
| `LLMQueueService` | `Arc<dyn PromptTemplateServicePort>` |
| `OutcomeSuggestionService` | `Arc<dyn PromptTemplateServicePort>` |
| `AppRequestHandler` | `Arc<dyn SheetTemplateServicePort>`, `Arc<dyn GenerationQueueProjectionServicePort>` |
| `GenerationQueueProjectionService` | `Arc<dyn AssetServicePort>` |
| `LLMService` | `Arc<dyn PromptTemplateServicePort>` |

### Phase 6: Fix IoC Violations (Day 3)

1. `DMApprovalQueueService` - inject `ToolExecutionService` via `Arc<dyn ToolExecutionServicePort>`
2. `LLMQueueService` - inject `LLMService` via port trait
3. Spawned task violations - inject service factories or use `Arc::clone()`

### Phase 7: Cleanup Port Adapters (Day 4)

1. Delete `engine-adapters/src/infrastructure/ports/` directory
2. Update `engine-runner` composition to wire directly

### Phase 8: Fix Other Anti-Patterns (Day 4)

1. Extract `queue_generation` business logic to `AssetGenerationService`
2. Fix swallowed errors in `challenge_handler.rs`
3. Delete duplicate `MockGameConnectionPort` from player-ports
4. Replace glob re-exports in engine-dto

### Phase 9: Update Composition Layer (Day 5)

Update all composition files:
- `engine-runner/src/composition/factories/use_cases.rs`
- `engine-runner/src/composition/app_state.rs`
- `engine-composition/src/use_cases.rs`
- `engine-composition/src/app_state.rs`
- `engine-runner/src/composition/factories/core_services.rs`
- `engine-runner/src/composition/factories/queue_services.rs`
- `engine-runner/src/composition/factories/asset_services.rs`

### Phase 10: Test Migration (Day 5)

1. Update test imports from inbound to outbound mock types
2. Verify all mock types properly re-exported
3. Run full test suite

### Phase 11: Add Arch-Check Rules (Day 5)

Add rules to `cargo xtask arch-check`:
- Use cases should not import from `engine-ports/inbound/` except for `UseCaseContext`
- Services should not import concrete service types from same layer
- No glob re-exports

---

## Files to Delete

```
crates/engine-adapters/src/infrastructure/ports/
├── challenge_adapters.rs
├── connection_adapters.rs
├── connection_manager_adapter.rs
├── player_action_adapters.rs
├── scene_adapters.rs
├── staging_service_adapter.rs
├── staging_state_adapter.rs
└── mod.rs

crates/player-ports/src/outbound/testing/mock_game_connection.rs
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking changes in signatures | High | Medium | Incremental migration, one component at a time |
| Import path breakage | High | Medium | Use sed/codemod for batch updates, grep to verify |
| Test breakage | High | Low | Run tests after each phase |
| Merge conflicts | Medium | Medium | Complete in one focused effort |
| Missing port trait methods | Low | Medium | Most port traits already exist and are complete |

---

## Success Criteria

1. All use cases depend only on outbound ports (except `UseCaseContext`)
2. All services depend on port traits, not concrete types
3. No services create other services internally
4. `engine-adapters/infrastructure/ports/` directory deleted
5. No duplicate port trait names between inbound/outbound
6. No duplicate mock implementations
7. No glob re-exports
8. No swallowed errors in handlers
9. `cargo xtask arch-check` passes with new rules
10. All existing tests pass

---

## Appendix A: Validation Notes

This plan was validated by automated analysis on the codebase. Key findings:

1. **Port traits**: 8 of 9 originally proposed traits already exist - only 3 need creating
2. **Service dependencies**: 2 services incorrectly listed were actually using proper abstractions
3. **IoC violations**: 3 additional violations found in spawned tasks
4. **Import scope**: 96 files need import path updates (not just use cases)
5. **Composition layer**: 7+ files need updates (originally only 1 mentioned)

---

## Appendix B: Correct Patterns

### Use Case Dependencies (Correct)

```rust
// CORRECT: Use case depends on outbound ports
pub struct InventoryUseCase {
    pc_crud: Arc<dyn PlayerCharacterCrudPort>,      // outbound
    pc_inventory: Arc<dyn PlayerCharacterInventoryPort>, // outbound
    broadcast: Arc<dyn BroadcastPort>,              // outbound
}
```

### Service Dependencies (Correct)

```rust
// CORRECT: Service depends on port traits
pub struct StagingService {
    template_service: Arc<dyn PromptTemplateServicePort>,  // port trait
    llm: Arc<dyn LlmPort>,                                  // port trait
    clock: Arc<dyn ClockPort>,                              // port trait
}
```

### Dependency Injection (Correct)

```rust
// CORRECT: All dependencies injected via constructor
impl DMApprovalQueueService {
    pub fn new(
        queue_port: Arc<dyn ApprovalQueuePort>,
        tool_execution: Arc<dyn ToolExecutionServicePort>,  // injected, not created
    ) -> Self {
        Self { queue_port, tool_execution }
    }
}
```

---

## References

- `docs/architecture/hexagonal-architecture.md` - Authoritative architecture spec
- `crates/engine-app/src/application/use_cases/inventory.rs` - Example of correct use case pattern
- `crates/engine-app/src/application/use_cases/observation.rs` - Example of correct use case pattern
