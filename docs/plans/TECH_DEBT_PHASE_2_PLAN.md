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
**Status**: Validated

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
| llm.rs | 10 types | Low |
| persistence.rs | 18 types | Medium |
| queue.rs | 25+ types | High (includes re-exports) |
| request_context.rs | 1 type | Trivial |
| staging.rs | 2 types | Trivial |

**Total**: ~56 public items to enumerate.

### Solution

Replace each glob re-export with explicit type exports:

```rust
pub use llm::{LlmRequest, LlmResponse, ChatMessage, /* ... */};
pub use persistence::{SheetTemplateStorageDto, /* ... */};
// etc.
```

### Downstream Impact

**None** - All 14 downstream usages already use explicit imports or specific module paths.

### Verification

```bash
cargo xtask arch-check  # Should show 0 glob violations
```

---

## Phase 2: Address Large Variant Size Warnings (Medium Effort)

**Priority**: Medium  
**Effort**: Medium (2.5-3 hours)  
**Status**: Validated

### Locations and Analysis

| File | Enum | Size Ratio | Large Variant |
|------|------|------------|---------------|
| `player-ports/.../player_events.rs:327` | `PlayerEvent` | 2x (576 vs 288 bytes) | `ApprovalRequired` |
| `engine-ports/.../use_case_types.rs:36` | `MovementResult` | 6.6x (264 vs 40 bytes) | `SceneChanged` |
| `engine-ports/.../use_case_types.rs:855` | `ActionResult` | 4.5x (288 vs 64 bytes) | `TravelCompleted` |
| `player-app/.../session_service.rs:41` | `SessionEvent` | 576x (576 vs 1 byte) | `MessageReceived` |

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

1. **SessionEvent** (easiest - 2 update sites)
2. **MovementResult** (~13 update sites)
3. **ActionResult** (~10 update sites)
4. **PlayerEvent** (~5-10 update sites)

### Performance Impact

**Negligible** - These enums are used for event passing over WebSockets and async service boundaries, not in hot loops.

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "large size difference"  # Should be empty
```

---

## Phase 3: Address "Too Many Arguments" Warnings (Medium Effort)

**Priority**: Medium  
**Effort**: Medium (6-9 hours total)  
**Count**: 41 warnings  
**Status**: Validated - **Revised from original "Large" effort estimate**

### Warning Categories (Validated)

| Category | Count | Solution |
|----------|-------|----------|
| Port trait methods | 16 | `#[allow]` - architecture constraint |
| Service constructors | 10 | Dependency structs |
| Use case constructors | 4 | Dependency structs |
| Domain entity constructors | 2 | `#[allow]` - builders exist |
| DTO constructors | 1 | `#[allow]` |
| Handler/Platform functions | 8 | Mixed - some refactor, some `#[allow]` |

### Sub-Phases

#### Phase 3a: Quick Wins (1 hour)

Add `#[allow(clippy::too_many_arguments)]` to:
- All port trait methods in `story_event_recording_service_port.rs`
- All port trait methods in `dialogue_context_service_port.rs`
- Domain constructors (`Staging::new`, `GenerationBatch::new`)
- DTO constructor (`ChallengeResolvedNotification::new`)
- `Platform::new`

**Result**: ~20 warnings eliminated

#### Phase 3b: Service Dependency Structs (2-3 hours)

Create `*Dependencies` structs for:
- `StagingService`
- `LlmQueueService`
- `AssetGenerationQueueService`
- `ChallengeOutcomeApprovalService`

**Result**: ~8 warnings eliminated

#### Phase 3c: Use Case Dependency Structs (1-2 hours)

Create `*Dependencies` structs for:
- `StagingApprovalUseCase`
- `MovementUseCase`

**Result**: ~4 warnings eliminated

#### Phase 3d: Remaining Cleanup (2-3 hours)

- Create parameter structs for staging methods (`ApprovalParams`, etc.)
- Refactor `initiate_connection` → `ConnectionConfig` struct
- Apply `#[allow]` to remaining edge cases

**Result**: ~9 warnings eliminated

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "too many arguments" | wc -l  # Target: 0
```

---

## Phase 4: Address Complex Type Warnings (Low Effort)

**Priority**: Low  
**Effort**: Small (15-30 minutes)  
**Count**: 3 warnings  
**Status**: Validated

### Locations and Types

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

**Status**: Validated - **Split into 5a (Medium Priority) and 5b (Optional)**

### Phase 5a: Migrate Large God Traits (Medium Priority)

**Priority**: Medium  
**Effort**: Large (per trait: 2-4 hours)

These traits have 10+ methods and clear ISP benefit:

| Trait | Actual Methods | Proposed Split |
|-------|----------------|----------------|
| InteractionRepositoryPort | **15** | CrudPort (5), TargetPort (5), RequirementPort (5) |
| AssetRepositoryPort | **14** | AssetCrudPort (6), BatchPort (8) |
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
| 2 | Large variant boxing | Medium | Medium | - |
| 3a | Too many args - quick wins | Small | Medium | - |
| 3b | Too many args - services | Medium | Medium | 3a |
| 3c | Too many args - use cases | Medium | Medium | 3a |
| 3d | Too many args - cleanup | Medium | Low | 3b, 3c |
| 5a | Large god traits (3) | Large | Medium | - |
| 5b | Remaining god traits (9) | Large | Low | - |

**Recommended order**: Phase 1 → Phase 4 → Phase 3a → Phase 2 → Phase 3b/3c → Phase 5a → Phase 3d → Phase 5b

---

## Success Criteria

- [ ] `cargo xtask arch-check` passes with 0 violations
- [ ] `cargo clippy --workspace` has significantly fewer warnings
- [ ] All tests pass
- [ ] No new tech debt introduced

---

## Notes

- Phase 3a (quick wins with `#[allow]`) provides immediate progress with minimal risk
- Phase 5a focuses on the three genuine god traits (10+ methods) - others are small enough to defer
- Some `#[allow]` annotations are acceptable for cases where refactoring adds complexity without benefit
- The dependency struct pattern already exists in the codebase (`CoreServiceDependencies`, `AssetServiceDependencies`) - Phase 3b/3c extends this pattern for consistency

---

## Revision History

| Date | Changes |
|------|---------|
| Initial | Original plan created |
| Post-Review | Phase 3 effort reduced from Large to Medium, split into sub-phases. Phase 5 split into 5a (medium priority, 3 traits) and 5b (optional, 9 traits). Method counts corrected based on actual analysis. |
