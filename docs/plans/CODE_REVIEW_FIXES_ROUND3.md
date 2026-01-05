# Code Review Fixes - Round 3 Implementation Plan

**Created**: 2026-01-05
**Status**: IN PROGRESS
**Source**: Comprehensive 9-agent code review

---

## Overview

This plan addresses findings from the comprehensive code review, organized by priority.
Authentication-related items are explicitly excluded per requirements.

---

## Phase 1: Critical UI State Fixes

### CR3-1.1 - Clear All State on Disconnect
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/handlers/session_event_handler.rs`
**Issue**: When connection state transitions to Disconnected/Failed, only engine_client is cleared. GameState, DialogueState, GenerationState persist with stale data.
**Impact**: Users reconnecting to different world see previous world's data.

**Tasks**:
- [x] Add `game_state.clear()` call on disconnect
- [x] Add `dialogue_state.clear()` call on disconnect  
- [x] Add `generation_state.clear()` call on disconnect
- [x] Add `session_state.clear()` call on disconnect
- [x] Verify all state signals are properly reset

---

### CR3-1.2 - Bound Conversation Log Growth
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/approval_state.rs`
**Issue**: `conversation_log` grows unbounded - memory leak in long sessions.

**Tasks**:
- [x] Add MAX_LOG_ENTRIES constant (500)
- [x] Trim old entries when limit exceeded in `add_log_entry()`

---

### CR3-1.3 - Bound Challenge Results Growth
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/challenge_state.rs`
**Issue**: `challenge_results` grows unbounded.

**Tasks**:
- [x] Add MAX_CHALLENGE_RESULTS constant (100)
- [x] Trim old entries when limit exceeded

---

### CR3-1.4 - Bound Decision History Growth
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/approval_state.rs`
**Issue**: `decision_history` grows unbounded.

**Tasks**:
- [x] Add MAX_HISTORY_ENTRIES constant (200)
- [x] Trim old entries when limit exceeded

---

## Phase 2: Critical Entity/Use Case Fixes

### CR3-2.1 - Fix Location.get_exits() Silent Skip
**Status**: COMPLETE
**Files**: `crates/engine/src/entities/location.rs`
**Issue**: Invalid exits are silently skipped with `continue`. Should log warning.

**Tasks**:
- [x] Add tracing::warn! when skipping invalid exits
- [x] Include location and target info in warning

---

### CR3-2.2 - Fix Narrative Trigger Error Swallowing
**Status**: PENDING
**Files**: `crates/engine/src/entities/narrative.rs`
**Issue**: `check_triggers()` uses `unwrap_or_default()` on observation fetch, hiding DB errors.

**Tasks**:
- [ ] Propagate observation fetch errors properly
- [ ] Or log warning when observations can't be fetched

---

### CR3-2.3 - Add Scene Resolution to ExitLocation
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/exit_location.rs`
**Issue**: ExitLocation always returns `resolved_scene: None`, missing scene content.

**Tasks**:
- [ ] Extract scene resolution logic from EnterRegion to shared helper
- [ ] Call scene resolution in ExitLocation
- [ ] Update ExitLocationResult to include resolved scene

---

### CR3-2.4 - Fix Scene Custom Conditions
**Status**: PENDING
**Files**: `crates/engine/src/entities/scene.rs`
**Issue**: `SceneCondition::Custom` always passes with a warning log.

**Tasks**:
- [ ] Either remove Custom variant
- [ ] Or implement basic expression evaluation
- [ ] Or return false with explicit "not supported" behavior

---

## Phase 3: Neo4j Transaction Safety

### CR3-3.1 - Make Scene Save Atomic
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/scene_repo.rs`
**Issue**: `save()` deletes then loops to add characters - non-atomic.

**Tasks**:
- [ ] Use UNWIND for batch character edge creation
- [ ] Combine delete and create into single query

---

### CR3-3.2 - Make Scene set_current Atomic
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/scene_repo.rs`
**Issue**: `set_current()` has separate DELETE then CREATE queries.

**Tasks**:
- [ ] Combine into single atomic query with OPTIONAL MATCH for old current

---

### CR3-3.3 - Make Staging save_pending Atomic
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/staging_repo.rs`
**Issue**: Creates Staging node, then loops for NPC edges.

**Tasks**:
- [ ] Use UNWIND for batch NPC edge creation

---

### CR3-3.4 - Add Silent Failure Checks
**Status**: PENDING
**Files**: Multiple Neo4j repos
**Issue**: Many operations don't verify success.

**Tasks**:
- [ ] Add RETURN clause and check result for `character_repo.save`
- [ ] Add RETURN clause and check result for `scene_repo.set_current`
- [ ] Add RETURN clause and check result for `character_repo.update_position`

---

## Phase 4: Missing Entity Operations

### CR3-4.1 - Add Challenge Delete
**Status**: PENDING
**Files**: 
- `crates/engine/src/infrastructure/ports.rs`
- `crates/engine/src/infrastructure/neo4j/challenge_repo.rs`
- `crates/engine/src/entities/challenge.rs`

**Tasks**:
- [ ] Add `delete` to ChallengeRepo trait
- [ ] Implement in challenge_repo.rs
- [ ] Add wrapper in Challenge entity

---

### CR3-4.2 - Add Observation Delete
**Status**: PENDING
**Files**: 
- `crates/engine/src/infrastructure/ports.rs`
- `crates/engine/src/infrastructure/neo4j/observation_repo.rs`
- `crates/engine/src/entities/observation.rs`

**Tasks**:
- [ ] Add `delete_observation` to ObservationRepo trait
- [ ] Implement in observation_repo.rs
- [ ] Add wrapper in Observation entity

---

### CR3-4.3 - Add Asset Delete
**Status**: PENDING
**Files**: 
- `crates/engine/src/infrastructure/ports.rs`
- `crates/engine/src/infrastructure/neo4j/asset_repo.rs`
- `crates/engine/src/entities/assets.rs`

**Tasks**:
- [ ] Add `delete` to AssetRepo trait
- [ ] Implement in asset_repo.rs
- [ ] Add wrapper in Assets entity

---

## Phase 5: Code Duplication Extraction

### CR3-5.1 - Extract row_to_item Helper
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/helpers.rs`
**Issue**: `row_to_item` duplicated in character_repo, player_character_repo, item_repo.

**Tasks**:
- [ ] Move row_to_item to helpers.rs
- [ ] Update character_repo to use shared helper
- [ ] Update player_character_repo to use shared helper
- [ ] Update item_repo to use shared helper

---

### CR3-5.2 - Extract Movement Staging Resolution
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/mod.rs`
**Issue**: Staging resolution duplicated in enter_region and exit_location.

**Tasks**:
- [ ] Create `resolve_staging_for_region` helper function
- [ ] Refactor EnterRegion to use helper
- [ ] Refactor ExitLocation to use helper

---

### CR3-5.3 - Extract Time Suggestion Helper
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/mod.rs`
**Issue**: Time suggestion logic duplicated.

**Tasks**:
- [ ] Create `suggest_time_for_movement` helper function
- [ ] Refactor both use cases to use helper

---

## Phase 6: Protocol/Domain Alignment

### CR3-6.1 - Fix Serde Case Mismatches
**Status**: PENDING
**Files**: `crates/protocol/src/messages.rs`
**Issue**: WantVisibility, ActantialRole, WantTargetType use `lowercase` in protocol but `camelCase` in domain.

**Tasks**:
- [ ] Change WantVisibilityData to `#[serde(rename_all = "camelCase")]`
- [ ] Change ActantialRoleData to `#[serde(rename_all = "camelCase")]`
- [ ] Change WantTargetTypeData to `#[serde(rename_all = "camelCase")]`

---

### CR3-6.2 - Add Missing Unknown Variants
**Status**: PENDING
**Files**: `crates/domain/src/entities/*.rs`
**Issue**: Multiple enums lack forward-compatible Unknown variants.

**Tasks**:
- [ ] Add Unknown to ChallengeType
- [ ] Add Unknown to Difficulty
- [ ] Add Unknown to LocationType
- [ ] Add Unknown to OutcomeTrigger
- [ ] Add Unknown to AcquisitionMethod
- [ ] Add Unknown to FrequencyLevel

---

## Phase 7: UI Handler Fixes

### CR3-7.1 - Store Lore Events in State
**Status**: PENDING
**Files**: 
- `crates/player-ui/src/presentation/state/mod.rs`
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

**Tasks**:
- [ ] Create LoreState struct with known_lore signal
- [ ] Handle LoreDiscovered event to add lore
- [ ] Handle LoreRevoked event to remove lore
- [ ] Handle LoreUpdated event to update lore

---

### CR3-7.2 - Fix ViewMode PC Selection
**Status**: PENDING
**Files**: `crates/player-ui/src/presentation/state/game_state.rs`
**Issue**: `start_viewing_as()` doesn't update `selected_pc_id`.

**Tasks**:
- [ ] Update selected_pc_id when entering ViewingAsCharacter mode
- [ ] Clear/restore selected_pc_id when exiting view mode

---

### CR3-7.3 - Clear Error Message on Reconnect
**Status**: PENDING
**Files**: `crates/player-ui/src/presentation/state/connection_state.rs`
**Issue**: Error message persists after successful reconnection.

**Tasks**:
- [ ] Clear error_message when transitioning to Connected state

---

## Phase 8: Documentation Updates

### CR3-8.1 - Update Scene System Docs
**Status**: PENDING
**Files**: `docs/systems/scene-system.md`

**Tasks**:
- [ ] Mark US-SCN-010 (Flag storage) as Implemented

---

### CR3-8.2 - Update Narrative System Docs
**Status**: PENDING
**Files**: `docs/systems/narrative-system.md`

**Tasks**:
- [ ] Mark US-NAR-010 (SetFlag) as Implemented

---

### CR3-8.3 - Update Lore System Docs
**Status**: PENDING
**Files**: `docs/systems/lore-system.md`

**Tasks**:
- [ ] Mark US-LORE-001 through US-LORE-008 backend as Implemented
- [ ] Note UI components still pending

---

### CR3-8.4 - Update Visual State System Docs
**Status**: PENDING
**Files**: `docs/systems/visual-state-system.md`

**Tasks**:
- [ ] Update implementation status table to reflect actual state

---

## Implementation Order

1. CR3-1.1 - State clearing (critical memory/UX)
2. CR3-1.2 - Bound conversation log (critical memory)
3. CR3-1.3 - Bound challenge results (critical memory)
4. CR3-1.4 - Bound decision history (critical memory)
5. CR3-2.1 - Location exits warning (data integrity)
6. CR3-2.2 - Narrative trigger errors (data integrity)
7. CR3-2.4 - Scene custom conditions (correctness)
8. CR3-3.1 - Scene save atomic (transaction safety)
9. CR3-3.2 - Scene set_current atomic (transaction safety)
10. CR3-3.4 - Silent failure checks (reliability)
11. CR3-5.1 - row_to_item extraction (code quality)
12. CR3-6.1 - Serde case fixes (protocol correctness)
13. CR3-6.2 - Unknown variants (forward compat)
14. CR3-4.1 - Challenge delete (feature completeness)
15. CR3-4.2 - Observation delete (feature completeness)
16. CR3-4.3 - Asset delete (feature completeness)
17. CR3-7.1 - Lore state (UI feature)
18. CR3-7.2 - ViewMode PC fix (UI correctness)
19. CR3-7.3 - Error message clear (UX)
20. CR3-5.2 - Movement staging extraction (code quality)
21. CR3-5.3 - Time suggestion extraction (code quality)
22. CR3-2.3 - ExitLocation scene resolution (feature)
23. CR3-3.3 - Staging save atomic (transaction safety)
24. CR3-8.* - Documentation updates

---

## Progress Tracking

| ID | Description | Status | Commit |
|----|-------------|--------|--------|
| CR3-1.1 | State clearing on disconnect | COMPLETE | - |
| CR3-1.2 | Bound conversation log | COMPLETE | - |
| CR3-1.3 | Bound challenge results | COMPLETE | - |
| CR3-1.4 | Bound decision history | COMPLETE | - |
| CR3-2.1 | Location exits warning | COMPLETE | - |
| CR3-2.2 | Narrative trigger errors | PENDING | - |
| CR3-2.3 | ExitLocation scene resolution | PENDING | - |
| CR3-2.4 | Scene custom conditions | PENDING | - |
| CR3-3.1 | Scene save atomic | PENDING | - |
| CR3-3.2 | Scene set_current atomic | PENDING | - |
| CR3-3.3 | Staging save atomic | PENDING | - |
| CR3-3.4 | Silent failure checks | PENDING | - |
| CR3-4.1 | Challenge delete | PENDING | - |
| CR3-4.2 | Observation delete | PENDING | - |
| CR3-4.3 | Asset delete | PENDING | - |
| CR3-5.1 | row_to_item extraction | PENDING | - |
| CR3-5.2 | Movement staging extraction | PENDING | - |
| CR3-5.3 | Time suggestion extraction | PENDING | - |
| CR3-6.1 | Serde case fixes | PENDING | - |
| CR3-6.2 | Unknown variants | PENDING | - |
| CR3-7.1 | Lore state | PENDING | - |
| CR3-7.2 | ViewMode PC fix | PENDING | - |
| CR3-7.3 | Error message clear | PENDING | - |
| CR3-8.1 | Scene system docs | PENDING | - |
| CR3-8.2 | Narrative system docs | PENDING | - |
| CR3-8.3 | Lore system docs | PENDING | - |
| CR3-8.4 | Visual state docs | PENDING | - |
