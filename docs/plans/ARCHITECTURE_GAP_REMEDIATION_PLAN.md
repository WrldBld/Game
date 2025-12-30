# Architecture Gap Remediation Plan

**Created**: December 30, 2024  
**Last Updated**: December 30, 2024  
**Status**: IN PROGRESS  
**Estimated Total Effort**: 18-24 hours (revised from 25-30h)  
**Current Architecture Score**: 92/100  
**Target Architecture Score**: 98/100

---

## Executive Summary

This plan addresses all identified gaps from the comprehensive architecture review, organized into 8 phases. Each phase is designed to be independently executable with clear deliverables.

**Revision Note (Dec 30, 2024)**: Plan validated by sub-agents. Several tasks marked complete, time estimates corrected, and two new phases added for discovered tech debt.

---

## Phase 1: Quick Wins (2-3 hours)

**Priority**: HIGH  
**Risk**: LOW  
**Dependencies**: None

### 1.1 Fix Clippy Auto-Fixable Warnings

**Status**: COMPLETE (Dec 30, 2024)

**Result**: Reduced warnings from 424 to 85 (339 fixed)

**Verification**:
```bash
cargo clippy --workspace 2>&1 | grep "warning:" | wc -l
# Result: 85
```

---

### 1.2 Replace `anyhow` with `thiserror` in Domain

**Status**: COMPLETE (Dec 30, 2024)

**Changes Made**:
- `crates/domain/src/value_objects/region.rs` - `RegionShift`, `RegionFrequency` now use `DomainError::parse()`
- `crates/domain/src/entities/observation.rs` - `ObservationType` now uses `DomainError::parse()`
- `crates/domain/Cargo.toml` - `anyhow` dependency removed

**Verification**:
```bash
grep -r "anyhow" crates/domain/src/  # Returns only comment in error.rs
```

---

### 1.3 Fix `derivable_impls` Warnings

**Status**: COMPLETE (Dec 30, 2024)

Fixed by `cargo clippy --fix` in Phase 1.1:
- `monomyth.rs` - Added `#[derive(Default)]` and `#[default]` attribute
- `archetype.rs` - Added `#[derive(Default)]` and `#[default]` attribute
- `rule_system.rs` - Added `#[derive(Default)]` and `#[default]` attribute

---

### 1.4 Add Missing Crates to Arch-Check

**Status**: COMPLETE (Dec 30, 2024)

**Changes Made**: Added to `check_no_glob_reexports()` in `crates/xtask/src/main.rs`:
- `crates/engine-dto/src`
- `crates/domain-types/src`
- `crates/engine-composition/src`

---

## Phase 2: DTO Consolidation (3-4 hours)

**Priority**: HIGH  
**Risk**: MEDIUM  
**Dependencies**: Phase 1.2 (domain changes)

### 2.1 Remove `DmApprovalDecision` Duplication

**Status**: COMPLETE (Dec 30, 2024)

**Resolution**: Domain and engine-dto versions kept separate (different serde attributes).
- Domain uses `#[serde(rename_all = "camelCase")]`
- engine-dto uses `#[serde(tag = "decision")]`

**Changes Made**:
- `engine-ports` now re-exports from `engine-dto` (single DTO source)
- Removed duplicate definition from `engine-ports`

---

### 2.2 Unify `SuggestionContext`

**Status**: COMPLETE (Dec 30, 2024)

**Changes Made**:
- `engine-dto` is now the single source of truth
- `engine-ports` re-exports from `engine-dto`
- `engine-app` re-exports from `engine-dto`
- `player-app` keeps its own simpler version (no `world_id` field - different needs)

---

### 2.3 Document PromptMappingDto/InputDefaultDto Separation

**Status**: PENDING

**Effort**: 30 minutes

Both types intentionally separate due to different serialization needs:
- Protocol version: camelCase for wire format
- engine-dto version: snake_case for Neo4j persistence

Add documentation comments to both files explaining this is intentional.

---

## Phase 3: God Trait Splitting (4-5 hours)

**Priority**: MEDIUM  
**Risk**: MEDIUM  
**Dependencies**: None

**Status**: **MOSTLY COMPLETE** (Dec 30, 2024)

**Revision Note**: Time reduced from 8-10h to 4-5h. Most traits now split; only 2 remain.

### Already Complete (No Action Needed)

The following traits have already been split into ISP-compliant sub-traits:

| Original Trait | Split Module | Sub-traits Created | Status |
|----------------|--------------|-------------------|--------|
| `LocationRepositoryPort` | `location_repository/` | 4 sub-traits | PRIOR |
| `RegionRepositoryPort` | `region_repository/` | 4 sub-traits | PRIOR |
| `CharacterRepositoryPort` | `character_repository/` | 6 sub-traits | PRIOR |
| `StoryEventRepositoryPort` | `story_event_repository/` | 4 sub-traits | PRIOR |
| `NarrativeEventRepositoryPort` | `narrative_event_repository/` | 4 sub-traits | PRIOR |
| `ChallengeRepositoryPort` | `challenge_repository/` | 5 sub-traits | PRIOR |
| `PlayerCharacterRepositoryPort` | `player_character_repository/` | 4 sub-traits | **COMPLETE** (Dec 30) |
| `SceneRepositoryPort` | `scene_repository/` | 5 sub-traits | **COMPLETE** (Dec 30) |
| `EventChainRepositoryPort` | `event_chain_repository/` | 4 sub-traits | **COMPLETE** (Dec 30) |
| `GameConnectionPort` (player-ports) | `game_connection/` | 6 sub-traits | PRIOR |

### 3.1 Split `WorldConnectionManagerPort` (20 methods → 4 traits)

**Status**: **COMPLETE** (Dec 30, 2024)

| New Trait | Methods |
|-----------|---------|
| `ConnectionQueryPort` | 8 methods (has_dm, get_dm_info, get_connected_users, stats, etc.) |
| `ConnectionContextPort` | 7 methods (get_user_id_by_client_id, get_connection_context, etc.) |
| `ConnectionBroadcastPort` | 4 methods (broadcast_to_world, broadcast_to_dms, etc.) |
| `ConnectionLifecyclePort` | 1 method (unregister_connection) |

**Changes Made**:
- Created `world_connection_manager/` module with 4 sub-traits
- Deleted monolithic `world_connection_manager_port.rs`
- Updated `AppStatePort` with 4 separate getter methods
- Updated all WebSocket handlers to use specific ports
- Updated `AppState` constructor to accept 4 port parameters

---

### 3.2 Split `WorldStatePort` (18 methods → 6 traits)

**Status**: PENDING  
**Effort**: 1.5 hours

| New Trait | Methods |
|-----------|---------|
| `WorldTimeStatePort` | get_game_time, set_game_time, advance_game_time |
| `WorldConversationStatePort` | add_conversation, get_conversation_history, clear_conversation_history |
| `WorldApprovalStatePort` | add_pending_approval, remove_pending_approval, get_pending_approvals |
| `WorldSceneStatePort` | get_current_scene, set_current_scene |
| `WorldDirectorialStatePort` | get_directorial_context, set_directorial_context, clear_directorial_context |
| `WorldLifecyclePort` | initialize_world, cleanup_world, is_world_initialized |

---

### 3.3 Split `PlayerCharacterRepositoryPort` (16 methods → 4 traits)

**Status**: **COMPLETE** (Dec 30, 2024)

| New Trait | Methods |
|-----------|---------|
| `PlayerCharacterCrudPort` | create, get, update, delete, unbind_from_session |
| `PlayerCharacterQueryPort` | get_by_location, get_by_user_and_world, get_all_by_world, get_unbound_by_user |
| `PlayerCharacterPositionPort` | update_location, update_region, update_position |
| `PlayerCharacterInventoryPort` | add_inventory_item, get_inventory, get_inventory_item, update_inventory_item, remove_inventory_item |

---

### 3.4 Split `SceneRepositoryPort` (16 methods → 5 traits)

**Status**: **COMPLETE** (Dec 30, 2024)

| New Trait | Methods |
|-----------|---------|
| `SceneCrudPort` | create, get, update, delete, update_directorial_notes |
| `SceneQueryPort` | list_by_act, list_by_location |
| `SceneLocationPort` | set_location, get_location |
| `SceneFeaturedCharacterPort` | add_featured_character, get_featured_characters, update_featured_character, remove_featured_character, get_scenes_for_character |
| `SceneCompletionPort` | mark_scene_completed, is_scene_completed, get_completed_scenes |

---

### 3.5 Split `EventChainRepositoryPort` (17 methods → 4 traits)

**Status**: **COMPLETE** (Dec 30, 2024)

| New Trait | Methods |
|-----------|---------|
| `EventChainCrudPort` | create, get, update, delete |
| `EventChainQueryPort` | list_by_world, list_active, list_favorites, get_chains_for_event |
| `EventChainMembershipPort` | add_event_to_chain, remove_event_from_chain, complete_event |
| `EventChainStatePort` | toggle_favorite, set_active, reset, get_status, list_statuses |

**Note**: `EventChainServicePort` (16 methods) was NOT split. It's a facade service that exposes the same operations to higher layers. ISP splitting is less critical for facade services since they are typically injected as a single dependency.

---

### 3.6 Add Deprecation Notices to Legacy Monolithic Traits

**Status**: PENDING  
**Effort**: 30 minutes

Add `#[deprecated]` attributes to:
- `LocationRepositoryPort` (points to `location_repository/` module)
- `RegionRepositoryPort` (points to `region_repository/` module)

---

## Phase 4: app_state.rs Decomposition (2-3 hours)

**Priority**: MEDIUM  
**Risk**: MEDIUM  
**Dependencies**: None

**Revision Note**: Time reduced from 4-5h to 2-3h. Service grouping already done in `engine-composition/`. Focus is now on extracting factory functions from the wiring logic, with a realistic target of ~600 lines (not 150).

### Already Complete (No Action Needed)

Service container types already exist in `engine-composition/`:
- `CoreServices` (core_services.rs)
- `GameServices` (game_services.rs)
- `UseCases` (use_cases.rs)
- `QueueServices` (queue_services.rs)
- `EventInfra` (event_infra.rs)
- `AssetServices` (asset_services.rs)
- `PlayerServices` (player_services.rs)

### 4.1 Extract Repository Factory Function

**Status**: PENDING  
**Effort**: 1 hour

Extract lines ~317-400 from `new_app_state()` into:
```rust
pub fn create_repositories(neo4j: &Neo4jRepository) -> RepositoryPorts {
    // ~90 lines of Arc creation and trait coercion
}
```

### 4.2 Extract Queue Infrastructure Factory Function

**Status**: PENDING  
**Effort**: 1 hour

Extract lines ~613-660 from `new_app_state()` into:
```rust
pub async fn create_queue_infrastructure(
    config: &AppConfig,
    core: &CoreServiceBundle,
) -> Result<QueueInfrastructure> {
    // Queue backends, event bus setup
}
```

### 4.3 Extract WorkerServices Creation

**Status**: PENDING  
**Effort**: 30 minutes

Extract lines ~1275-1291 into separate function.

**Target**: Reduce `new_app_state()` from 1313 lines to ~600-700 lines.

---

## Phase 5: Manual Clippy Fixes (4-6 hours)

**Priority**: LOW  
**Risk**: LOW  
**Dependencies**: Phase 1.1 (auto-fix first)

**Revision Note**: Time increased from 2-3h to 4-6h. `too_many_arguments` scope is 31 functions (not 3).

### Current Warning Summary (85 total)

| Warning Type | Count | Effort |
|--------------|-------|--------|
| `too_many_arguments` | 31 | HIGH - struct wrapping needed |
| `result_large_err` | 13 | MEDIUM - Box error types |
| `doc list item overindented` | 11 | LOW - auto-fixable |
| `large_enum_variant` | 4 | MEDIUM - Box large variants |
| `type_complexity` | 3 | LOW - type alias |
| Other (auto-fixable) | 23 | TRIVIAL |

### 5.1 Fix `result_large_err` Warnings

**Status**: PENDING  
**Effort**: 45 minutes  
**File**: `crates/engine-adapters/src/infrastructure/websocket/context.rs`  
**Count**: 13 warnings

**Option A** (Recommended for boundary code): Add `#[allow(clippy::result_large_err)]`  
**Option B**: Box the error type in return signatures

---

### 5.2 Fix `large_enum_variant` Warnings

**Status**: PENDING  
**Effort**: 45 minutes  
**Count**: 4 locations

| File | Line | Fix |
|------|------|-----|
| `player-ports/src/inbound/player_events.rs` | 327 | Box large variant |
| `engine-ports/src/outbound/use_case_types.rs` | 36 | Box large variant |
| `engine-ports/src/outbound/use_case_types.rs` | 855 | Box large variant |
| `player-app/src/application/services/session_service.rs` | 41 | Box large variant |

---

### 5.3 Address `too_many_arguments` (31 functions)

**Status**: PENDING  
**Effort**: 3-4 hours

**Priority 1 - Most severe (10+ args):**
| Function | Args | Location |
|----------|------|----------|
| `challenge_resolution_service` constructor | 14 | `challenge_resolution_service.rs:369` |
| `story_event_service` constructor | 13 | `story_event_service.rs:31` |
| `llm_queue_service` constructor | 11 | `llm_queue_service.rs:78` |

**Fix Pattern**: Create `*Deps` structs:
```rust
pub struct ChallengeResolutionDeps {
    pub world_service: Arc<dyn WorldService>,
    pub character_service: Arc<dyn CharacterService>,
    // ... other deps
}

pub fn new(deps: ChallengeResolutionDeps) -> Self
```

**Priority 2 - Use cases (8 args)**: Create dependency structs for all use case constructors.

**Priority 3 - Domain entities (8 args)**: Consider `#[allow]` - entity constructors often legitimately need many fields.

---

### 5.4 Fix `type_complexity` Warnings

**Status**: PENDING  
**Effort**: 30 minutes  
**Count**: 3 locations in `player-adapters`

Create type aliases for complex generic types.

---

## Phase 6: Documentation & Cleanup (1-2 hours)

**Priority**: LOW  
**Risk**: LOW  
**Dependencies**: All other phases

### 6.1 Document `LlmPortDyn` Workaround

**Status**: PENDING  
**Effort**: 30 minutes

Add ADR or detailed comment explaining the async trait object workaround.

---

### 6.2 Update Architecture Documentation

**Status**: PARTIALLY COMPLETE

**Remaining**:
- [ ] Update ISP compliance section with new split traits
- [ ] Update crate file counts

---

### 6.3 Archive Completed Plans

**Status**: PARTIALLY COMPLETE

| Document | Current Status | Action Needed |
|----------|----------------|---------------|
| `CODE_QUALITY_REMEDIATION_PLAN.md` | COMPLETE | None |
| `HEXAGONAL_CLEANUP_PLAN.md` | SUPERSEDED | None |
| `HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md` | Complete | None |
| `HEXAGONAL_GAP_REMEDIATION_PLAN.md` | COMPLETE | None |
| `PROTOCOL_AS_OWNER_REFACTOR_PLAN.md` | No status | Add COMPLETE marker |
| `HEXAGONAL_ARCHITECTURE_PHASE2_PLAN.md` | No status | Review and mark |
| `WEBSOCKET_ADAPTER_REFACTORING_PLAN.md` | No status | Review and mark |
| `CHALLENGE_APPROVAL_REFACTORING_PLAN.md` | No status | Review and mark |

---

### 6.4 Update ACTIVE_DEVELOPMENT.md

**Status**: PENDING  
**Effort**: 15 minutes

Fix Phase Q status - currently shows "NOT STARTED" but is actually COMPLETE.

---

## Phase 7: Large Repository Decomposition (NEW)

**Priority**: MEDIUM  
**Risk**: MEDIUM  
**Dependencies**: Phase 3 (trait splitting patterns)  
**Effort**: 4-6 hours

**Rationale**: Three repository files exceed 1800 lines, making them difficult to maintain.

### 7.1 Decompose `character_repository.rs` (2073 lines)

**Status**: PENDING  
**Effort**: 2 hours

Split by character type:
- `character_repository/npc.rs` - NPC-specific queries
- `character_repository/pc.rs` - PlayerCharacter-specific queries
- `character_repository/common.rs` - Shared operations

---

### 7.2 Decompose `narrative_event_repository.rs` (2005 lines)

**Status**: PENDING  
**Effort**: 2 hours

Split by operation category:
- `narrative_event_repository/crud.rs` - Basic CRUD
- `narrative_event_repository/query.rs` - Complex queries
- `narrative_event_repository/trigger.rs` - Trigger evaluation

---

### 7.3 Decompose `story_event_repository.rs` (1814 lines)

**Status**: PENDING  
**Effort**: 2 hours

Split by query type:
- `story_event_repository/crud.rs` - Basic CRUD
- `story_event_repository/edge.rs` - Edge operations
- `story_event_repository/timeline.rs` - Timeline queries

---

## Phase 8: Error Handling Audit (NEW)

**Priority**: HIGH  
**Risk**: LOW  
**Dependencies**: None  
**Effort**: 2-3 hours

**Rationale**: Production `unwrap()` calls and silent error discarding could cause panics or hide bugs.

### 8.1 Fix Production `unwrap()` Calls

**Status**: PENDING  
**Effort**: 1 hour

**Critical locations**:
| File | Line | Issue |
|------|------|-------|
| `staging_service.rs` | 616 | `extract_json_array(response).unwrap()` - panics on malformed LLM response |

**Fix**: Replace with proper error handling using `?` or `map_err`.

---

### 8.2 Review Silent Error Discarding

**Status**: PENDING  
**Effort**: 1-2 hours

**Pattern**: `let _ = potentially_failing_operation();`

**High-priority locations** (88 total, focus on critical paths):
- `broadcast_adapter.rs` - 10 instances of discarding send results
- `story_event_repository.rs` - 4 instances of discarding graph operations

**Fix Options**:
1. Add logging for discarded errors
2. Use `let _ =` with explicit comment explaining why it's OK to discard
3. Propagate errors where appropriate

---

### 8.3 Track Critical TODO Items

**Status**: PENDING  
**Effort**: 30 minutes

Create issues for:
| File | Line | Issue |
|------|------|-------|
| `challenge_outcome_approval_service.rs` | 634 | Missing queue item ID tracking |
| `challenge_outcome_approval_service.rs` | 796 | Missing branch lookup |
| `movement.rs` | 489 | Missing previous staging lookup |
| `trigger_evaluation_service.rs` | 405 | Missing player character inventory |
| `interaction_repository.rs` | 416 | Phase 0.H edge targeting incomplete |

---

## Execution Summary

| Phase | Effort | Priority | Risk | Status |
|-------|--------|----------|------|--------|
| 1. Quick Wins | 2-3h | HIGH | LOW | **COMPLETE** |
| 2. DTO Consolidation | 3-4h | HIGH | MEDIUM | **MOSTLY COMPLETE** |
| 3. God Trait Splitting | 4-5h | MEDIUM | MEDIUM | **MOSTLY COMPLETE** (2 traits remain) |
| 4. app_state.rs Decomposition | 2-3h | MEDIUM | MEDIUM | PENDING |
| 5. Manual Clippy Fixes | 4-6h | LOW | LOW | PENDING |
| 6. Documentation | 1-2h | LOW | LOW | PARTIALLY COMPLETE |
| 7. Large Repository Decomposition | 4-6h | MEDIUM | MEDIUM | PENDING |
| 8. Error Handling Audit | 2-3h | HIGH | LOW | PENDING |
| **Total** | **18-24h** | | | |

---

## Verification Checklist

After each phase, run:
```bash
cargo check --workspace
cargo test --workspace
cargo xtask arch-check
cargo clippy --workspace
```

Final verification:
```bash
# All must pass
cargo check --workspace          # Compilation
cargo test --workspace           # Tests
cargo xtask arch-check           # Architecture rules
cargo clippy --workspace         # Lint warnings reduced

# Metrics to track
wc -l crates/engine-runner/src/composition/app_state.rs  # Target: <700 lines
cargo clippy --workspace 2>&1 | grep "warning:" | wc -l   # Target: <50 warnings
```

---

## Change Log

| Date | Changes |
|------|---------|
| Dec 30, 2024 | Initial plan created |
| Dec 30, 2024 | Phase 1 completed (clippy auto-fix, anyhow removal, derivable_impls, arch-check) |
| Dec 30, 2024 | Phase 2.1, 2.2 completed (DmApprovalDecision, SuggestionContext consolidation) |
| Dec 30, 2024 | Plan validated by sub-agents; Phase 3 scope reduced (6 traits already split) |
| Dec 30, 2024 | Phase 4 target revised (600 lines realistic, not 150) |
| Dec 30, 2024 | Phase 5 scope expanded (31 functions, not 3) |
| Dec 30, 2024 | Added Phase 7 (Large Repository Decomposition) |
| Dec 30, 2024 | Added Phase 8 (Error Handling Audit) |
| Dec 30, 2024 | Removed Phase 2.3 LoadGenerationQueueItemsResult (type never existed) |
| Dec 30, 2024 | Phase A & B: Protocol import fixes, unused dep removal, DTO consolidation |
| Dec 30, 2024 | Phase C: Split PlayerCharacterRepositoryPort, SceneRepositoryPort, EventChainRepositoryPort |

---

## Notes

- Phases 3 and 4 can run in parallel if multiple developers are available
- Phase 5 can use strategic `#[allow]` for lower-priority warnings (domain entities)
- All trait splits maintain backward compatibility via blanket implementations
- Phase 7 (repository decomposition) should follow Phase 3 patterns
- Phase 8 is high priority due to potential production panics
