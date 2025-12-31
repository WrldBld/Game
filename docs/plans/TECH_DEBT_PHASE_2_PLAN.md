# Tech Debt Remediation Plan - Phase 2

## Overview

This plan addresses remaining technical debt identified after completing the initial refactoring (god trait removal, Neo4j helpers, ID parser macros). The issues are categorized by priority and complexity.

## Current State

- **Clippy Warnings**: ~48 total
  - 41 "too many arguments" warnings
  - 4 "large size difference between variants" warnings
  - 3 "very complex type" warnings
- **Architecture Violations**: 5 glob re-export violations in `engine-dto`
- **Remaining God Traits**: 12 (without ISP replacements)

---

## Phase 1: Fix Glob Re-export Violations (Low Effort, High Value)

**Priority**: High  
**Effort**: Small (15-30 minutes)  
**Files**: `crates/engine-dto/src/lib.rs`  
**Status**: Validated - Ready to implement

### Problem

The architecture checker flags 5 glob re-exports that violate the "explicit exports" rule:

```rust
pub use llm::*;
pub use persistence::*;
pub use queue::*;
pub use request_context::*;
pub use staging::*;
```

### Scope (Validated)

| Module | Public Types | Complexity |
|--------|-------------|------------|
| llm.rs | 9 types | Low |
| persistence.rs | 17 types | Medium |
| queue.rs | 32 types + re-exports | High |
| request_context.rs | 1 type | Trivial |
| staging.rs | 2 types | Trivial |

**Total**: 61 public items to enumerate (includes re-exports from `protocol` and `domain` in queue.rs).

### Solution

Replace each glob re-export with explicit type exports. See Appendix A for complete replacement code.

### Downstream Impact

**None** - All 14 downstream usages already use explicit imports or specific module paths.

### Notes

- `queue.rs` re-exports types from `wrldbldr_protocol` and `wrldbldr_domain` - these must be included
- 8 conversion functions in `queue.rs` must also be explicitly exported

### Verification

```bash
cargo xtask arch-check  # Should show 0 glob violations
```

---

## Phase 2: Address Large Variant Size Warnings (Medium Effort)

**Priority**: Medium  
**Effort**: Medium (3.5-5 hours) - **Revised upward**  
**Status**: Validated - Ready with caveats

### Locations and Analysis

| File | Enum | Size Ratio | Large Variant | Update Sites |
|------|------|------------|---------------|--------------|
| `player-app/.../session_service.rs:41` | `SessionEvent` | 576x | `MessageReceived` | 4 |
| `engine-ports/.../use_case_types.rs:36` | `MovementResult` | 6.6x | `SceneChanged` | 13 |
| `engine-ports/.../use_case_types.rs:855` | `ActionResult` | 4.5x | `TravelCompleted` | 10 |
| `player-ports/.../player_events.rs:327` | `PlayerEvent` | 2x | Multiple variants | **~144** |

### Solution

Box the large variants to reduce enum size:

```rust
// SessionEvent - simplest fix
pub enum SessionEvent {
    StateChanged(PortConnectionState),
    MessageReceived(Box<PlayerEvent>),  // Box the whole PlayerEvent
}

// MovementResult
pub enum MovementResult {
    SceneChanged(Box<SceneChangedEvent>),
    StagingPending { ... },
    Blocked { ... },
}
```

### Implementation Order

1. **SessionEvent** (trivial - 4 update sites)
2. **MovementResult** (easy - 13 update sites)
3. **ActionResult** (easy - 10 update sites)
4. **PlayerEvent** (complex - ~144 update sites) - **Consider deferring or simplified approach**

### PlayerEvent Caveat

The `PlayerEvent` enum has 65+ variants and ~144 pattern match sites across `message_translator.rs` and `session_message_handler.rs`. Options:
- **Option A**: Box only the largest variant(s) - minimal changes
- **Option B**: Defer this enum and only fix the other 3 (eliminates 3 of 4 warnings)

### Performance Impact

**Negligible** - These enums are used for event passing over WebSockets and async service boundaries, not in hot loops.

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "large size difference"  # Should be empty (or 1 if PlayerEvent deferred)
```

---

## Phase 3: Address "Too Many Arguments" Warnings (Medium Effort)

**Priority**: Medium  
**Effort**: Medium (6-9 hours total)  
**Count**: 41 warnings  
**Status**: Validated - Ready to implement

### Warning Categories (Validated)

| Category | Count | Solution |
|----------|-------|----------|
| Port trait methods | 22 | `#[allow]` - architecture constraint |
| Service constructors + methods | 9 | Dependency structs |
| Use case constructors | 2 | Dependency structs |
| Domain entity constructors | 2 | `#[allow]` - builders exist |
| DTO constructors | 1 | `#[allow]` |
| Handler/Platform/Other | 5 | Mixed |

### Sub-Phases

#### Phase 3a: Quick Wins (1 hour)

Add `#[allow(clippy::too_many_arguments)]` to:
- `story_event_recording_service_port.rs` (8 warnings)
- `dialogue_context_service_port.rs` (1 warning)
- `staging_service_port.rs` (2 warnings)
- `use_case_ports.rs` (3 warnings)
- `story_event_service.rs` (8 warnings - mirrors port)
- Domain constructors: `Staging::new`, `GenerationBatch::new` (2 warnings)
- DTO constructor: `ChallengeResolvedNotification::new` (1 warning)
- `Platform::new` (1 warning)

**Result**: 26 warnings eliminated

#### Phase 3b: Service Dependency Structs (2-3 hours)

Create `*Dependencies` structs for:
- `StagingService` (constructor + 3 methods)
- `LlmQueueService`
- `AssetGenerationQueueService`
- `ChallengeOutcomeApprovalService`
- `event_effect_executor.rs` method
- `challenge_resolution_service.rs` method

**Result**: 9 warnings eliminated

#### Phase 3c: Use Case Dependency Structs (1-2 hours)

Create `*Dependencies` structs for:
- `StagingApprovalUseCase`
- `MovementUseCase`

**Result**: 2 warnings eliminated

#### Phase 3d: Remaining Cleanup (1-2 hours)

- `core_services.rs` - CoreServices::new
- `connection.rs` - `initiate_connection` → `ConnectionConfig` struct
- `world_state_manager.rs` - constructor
- `misc_handler.rs` - `add_actantial_view`

**Result**: 4 warnings eliminated

### Template Pattern

Follow existing pattern in `crates/engine-runner/src/composition/factories/`:
- `CoreServiceDependencies`
- `AssetServiceDependencies`
- `UseCaseDependencies`

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "too many arguments" | wc -l  # Target: 0
```

---

## Phase 4: Address Complex Type Warnings (Low Effort)

**Priority**: Low  
**Effort**: Small (15-30 minutes)  
**Count**: 3 warnings  
**Status**: Validated - Ready to implement

### Locations and Types (Verified)

| File | Line | Complex Type | Proposed Alias |
|------|------|--------------|----------------|
| `player-adapters/.../desktop/client.rs` | 30 | `Arc<Mutex<Option<Box<dyn Fn(ServerMessage) + Send + Sync>>>>` | `MessageCallback` |
| `player-adapters/.../desktop/client.rs` | 31 | `Arc<Mutex<Option<Box<dyn Fn(ConnectionState) + Send + Sync>>>>` | `StateChangeCallback` |
| `engine-adapters/.../comfyui.rs` | 143 | `Arc<Mutex<Option<(DateTime<Utc>, bool)>>>` | `CachedHealthCheck` |

### Solution

Create type aliases in each file:

```rust
// desktop/client.rs
type MessageCallback = Arc<Mutex<Option<Box<dyn Fn(ServerMessage) + Send + Sync>>>>;
type StateChangeCallback = Arc<Mutex<Option<Box<dyn Fn(ConnectionState) + Send + Sync>>>>;

// comfyui.rs
type CachedHealthCheck = Arc<Mutex<Option<(DateTime<Utc>, bool)>>>;
```

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "very complex type"  # Should be empty
```

---

## Phase 5: Migrate Remaining God Traits

**Status**: Validated - Split into 5a (Medium Priority) and 5b (Optional)

### Phase 5a: Migrate Large God Traits (Medium Priority)

**Priority**: Medium  
**Effort**: Large (per trait: 2-4 hours)

These traits have 10+ methods and clear ISP benefit:

| Trait | Actual Methods | Proposed Split |
|-------|----------------|----------------|
| InteractionRepositoryPort | **14** | CrudPort (5), TargetPort (5), RequirementPort (4) |
| AssetRepositoryPort | **13** | AssetCrudPort (6), BatchPort (7) |
| ItemRepositoryPort | **10** | CrudPort (6), ContainerPort (4) |

### Phase 5b: Remaining Traits (Optional/Low Priority)

**Priority**: Low (no immediate consumers requesting ISP)  
**Effort**: Medium (per trait: 1-2 hours)

| Trait | Actual Methods | Complexity |
|-------|----------------|------------|
| SheetTemplateRepositoryPort | 8 | Low |
| WorldRepositoryPort | 7 | Low |
| FlagRepositoryPort | 6 | Low |
| RelationshipRepositoryPort | 6 | Low |
| SkillRepositoryPort | 5 | Low |
| GoalRepositoryPort | 5 | Low |
| ObservationRepositoryPort | 5 | Low |
| WorkflowRepositoryPort | 4 | Low |
| WantRepositoryPort | 2 | Trivial |

### Template Reference

Follow existing ISP pattern in `crates/engine-ports/src/outbound/player_character_repository/`

### Approach (per trait)

1. Analyze trait methods and group by responsibility
2. Create ISP traits in `engine-ports/src/outbound/<name>_repository/`
3. Update adapter implementations
4. Migrate consumers to use ISP traits
5. Remove god trait from `repository_port.rs`

### Decision

- **Phase 5a**: Address when working on related features or as dedicated tech debt sprint
- **Phase 5b**: Defer unless a specific need arises

---

## Execution Order

| Phase | Description | Effort | Priority | Depends On |
|-------|-------------|--------|----------|------------|
| 1 | Glob re-export fixes | Small | High | - |
| 4 | Complex type aliases | Small | Low | - |
| 3a | Too many args - quick wins | Small | Medium | - |
| 2 | Large variant boxing (3 enums) | Medium | Medium | - |
| 3b | Too many args - services | Medium | Medium | 3a |
| 3c | Too many args - use cases | Medium | Medium | 3a |
| 3d | Too many args - cleanup | Small | Low | 3b, 3c |
| 5a | Large god traits (3) | Large | Medium | - |
| 5b | Remaining god traits (9) | Large | Low | - |

**Recommended order**: Phase 1 → Phase 4 → Phase 3a → Phase 2 (without PlayerEvent) → Phase 3b/3c → Phase 5a → Phase 3d → Phase 5b

---

## Success Criteria

- [ ] `cargo xtask arch-check` passes with 0 violations
- [ ] `cargo clippy --workspace` has significantly fewer warnings
- [ ] All tests pass
- [ ] No new tech debt introduced

---

## Notes

- Phase 3a (quick wins with `#[allow]`) provides immediate progress with minimal risk
- Phase 2: Consider deferring `PlayerEvent` boxing due to ~144 update sites - other 3 enums provide good value
- Phase 5a focuses on the three genuine god traits (10+ methods) - others are small enough to defer
- Some `#[allow]` annotations are acceptable for cases where refactoring adds complexity without benefit
- The dependency struct pattern already exists in the codebase - Phase 3b/3c extends this pattern

---

## Appendix A: Phase 1 Replacement Code

Complete replacement for `crates/engine-dto/src/lib.rs`:

```rust
//! WrldBldr Engine DTOs - Shared data types for engine internals

pub mod llm;
pub mod persistence;
pub mod queue;
pub mod request_context;
pub mod staging;

// llm.rs exports (9 types)
pub use llm::{
    ChatMessage, FinishReason, ImageData, LlmRequest, LlmResponse, 
    MessageRole, TokenUsage, ToolCall, ToolDefinition,
};

// persistence.rs exports (17 types)
pub use persistence::{
    DifficultyRequestDto, FieldTypeDto, InputDefaultDto, ItemListTypeDto, 
    OutcomeRequestDto, OutcomeTriggerRequestDto, OutcomesRequestDto, 
    PromptMappingDto, PromptMappingTypeDto, SectionLayoutDto, SelectOptionDto, 
    SheetFieldDto, SheetSectionDto, SheetTemplateStorageDto,
    TriggerConditionRequestDto, TriggerTypeRequestDto,
};

// queue.rs exports - structs and enums (18 types)
pub use queue::{
    ApprovalItem, AssetGenerationItem, ChallengeOutcomeApprovalItem, 
    DMAction, DMActionItem, DecisionType, DecisionUrgency, DmApprovalDecision,
    EnhancedChallengeSuggestion, EnhancedOutcomes, LLMRequestItem, LLMRequestType,
    OutcomeDetail, PlayerActionItem, QueueItem, QueueItemId, QueueItemStatus, 
    SuggestionContext,
};

// queue.rs exports - re-exports from protocol/domain (5 types)
pub use queue::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, GamePromptRequest,
    NarrativeEventSuggestionInfo, ProposedToolInfo,
};

// queue.rs exports - conversion functions (8 functions)
pub use queue::{
    challenge_suggestion_to_info, info_to_challenge_suggestion, 
    info_to_narrative_event_suggestion, info_to_outcomes, info_to_proposed_tool, 
    narrative_event_suggestion_to_info, outcomes_to_info, proposed_tool_to_info,
};

// request_context.rs exports (1 type)
pub use request_context::RequestContext;

// staging.rs exports (2 types)
pub use staging::{StagedNpcProposal, StagingProposal};
```

---

## Revision History

| Date | Changes |
|------|---------|
| Initial | Original plan created |
| Post-Review #1 | Phase 3 effort reduced from Large to Medium, split into sub-phases. Phase 5 split into 5a/5b. Method counts corrected. |
| Post-Review #2 | Phase 1: Updated to 61 items, added Appendix A with complete code. Phase 2: Updated effort to 3.5-5 hours, noted PlayerEvent has ~144 update sites. Phase 3: Corrected sub-phase counts (3a:26, 3b:9, 3c:2, 3d:4). Phase 5: Corrected method counts (Interaction:14, Asset:13). |
