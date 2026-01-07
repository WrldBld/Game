# Code Review Fixes - Round 4

## Overview

This plan addresses issues found in the comprehensive code review of January 2026.
Organized by priority with estimated complexity.

**Excluded**: Authentication/authorization issues (per user request)

---

## Phase 1: Critical - Delete Operations (Complexity: Medium)

Missing delete operations prevent proper data cleanup and world management.

### CR4-1.1 - Location/Region Delete
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/entities/location.rs`
- `crates/engine/src/infrastructure/neo4j/location_repo.rs`
- `crates/engine/src/infrastructure/ports.rs`

**Tasks**:
- [x] Add `delete_location` to LocationRepo port trait
- [x] Add `delete_region` to LocationRepo port trait
- [x] Implement in location_repo.rs (DETACH DELETE for relationships)
- [x] Add wrapper methods in location.rs entity

---

### CR4-1.2 - Scene Delete
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/entities/scene.rs`
- `crates/engine/src/infrastructure/neo4j/scene_repo.rs`
- `crates/engine/src/infrastructure/ports.rs`

**Tasks**:
- [x] Add `delete` to SceneRepo port trait
- [x] Implement in scene_repo.rs
- [x] Add wrapper method in scene.rs entity

---

### CR4-1.3 - Narrative Delete Operations
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/entities/narrative.rs`
- `crates/engine/src/infrastructure/neo4j/narrative_repo.rs`
- `crates/engine/src/infrastructure/ports.rs`

**Tasks**:
- [x] Add `delete_event` to NarrativeRepo port trait
- [x] Add `delete_chain` to NarrativeRepo port trait
- [x] Add `delete_story_event` to NarrativeRepo port trait
- [x] Implement all three in narrative_repo.rs
- [x] Add wrapper methods in narrative.rs entity

---

### CR4-1.4 - Item Delete
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/entities/inventory.rs`
- `crates/engine/src/infrastructure/neo4j/item_repo.rs`
- `crates/engine/src/infrastructure/ports.rs`

**Tasks**:
- [x] Add `delete` to ItemRepo port trait
- [x] Implement in item_repo.rs
- [x] Add wrapper method or use existing inventory entity

---

### CR4-1.5 - Want/Relationship Delete
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/entities/character.rs`
- `crates/engine/src/infrastructure/neo4j/character_repo.rs`
- `crates/engine/src/infrastructure/ports.rs`

**Tasks**:
- [x] Add `delete_want` to CharacterRepo port trait
- [x] Add `delete_relationship` to CharacterRepo port trait
- [x] Implement both in character_repo.rs
- [x] Add wrapper methods in character.rs entity

---

## Phase 2: Critical - Time Period Detection Bug (Complexity: Low)

### CR4-2.1 - Fix Time Period Detection
**Status**: COMPLETE
**Files**: `crates/engine/src/use_cases/time/mod.rs`
**Lines**: 133, 149

**Issue**: Unused variables `previous_period` and `new_period` in Auto mode (warnings).
**Analysis**: Period detection is actually handled correctly by `build_time_advance_data()`.
The Suggested mode correctly uses these variables for its period_change field.

**Tasks**:
- [x] Remove unused variables in Auto mode
- [x] Add comment explaining period detection is handled by build_time_advance_data()
- [x] Suggested mode was already correct (uses period_change field)

---

## Phase 3: Critical - UI State Clear on Disconnect (Complexity: Low)

### CR4-3.1 - Clear selected_pc_id on Disconnect
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/game_state.rs`
**Lines**: 574-595 (clear_scene method)

**Tasks**:
- [x] Add `self.selected_pc_id.set(None)` to `clear_scene()`

---

### CR4-3.2 - Clear ComfyUI State on Disconnect
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/connection_state.rs`
**Lines**: 195-205 (clear method)

**Tasks**:
- [x] Reset comfyui_state to "connected"
- [x] Reset comfyui_message to None
- [x] Reset comfyui_retry_in_seconds to None

---

## Phase 4: High - Code Duplication (Complexity: Medium)

### CR4-4.1 - Extract Scene Resolution Helper
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/use_cases/movement/mod.rs`
- `crates/engine/src/use_cases/movement/enter_region.rs`
- `crates/engine/src/use_cases/movement/exit_location.rs`

**Issue**: `resolve_scene_for_region` is copy-pasted between both use cases.

**Tasks**:
- [x] Create shared `resolve_scene_for_region` function in mod.rs
- [x] Update EnterRegion to use shared helper
- [x] Update ExitLocation to use shared helper

---

## Phase 5: High - UI State Bugs (Complexity: Low)

### CR4-5.1 - Clear NPC Dispositions on PC Switch
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
**Lines**: 547-570 (PcSelected handler)

**Tasks**:
- [x] Add `game_state.clear_npc_dispositions()` after PC selection

---

### CR4-5.2 - Clear Dialogue on Scene Change
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
**Lines**: 572-606 (SceneChanged handler)

**Tasks**:
- [x] Add `dialogue_state.clear()` at start of SceneChanged handler

---

### CR4-5.3 - Add Bounds to Pending Collections
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/approval_state.rs`

**Tasks**:
- [x] Add MAX_PENDING_APPROVALS constant (50)
- [x] Add MAX_PENDING_CHALLENGE_OUTCOMES constant (50)
- [x] Implement bounds checking with oldest removal in add methods

---

## Phase 6: High - WebSocket Broadcast Issues (Complexity: Medium)

### CR4-6.1 - Add UserJoined Broadcast
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/api/websocket/mod.rs`
- `crates/engine/src/api/connections.rs`

**Tasks**:
- [x] Add `broadcast_to_world_except` method to ConnectionManager
- [x] After successful join, broadcast UserJoined to other world members

---

### CR4-6.2 - Add UserLeft Broadcast
**Status**: COMPLETE
**Files**: `crates/engine/src/api/websocket/mod.rs`

**Tasks**:
- [x] Before leaving, broadcast UserLeft to other world members

---

## Phase 7: High - Entity Bugs (Complexity: Low)

### CR4-7.1 - Fix Observation Silent Failure
**Status**: COMPLETE
**Files**: `crates/engine/src/entities/observation.rs`

**Issue**: Returns `Ok(())` on invalid region instead of error or warning.

**Tasks**:
- [x] Add tracing::warn when region not found
- [x] Keep returning Ok() as it's a valid edge case (not an error)

---

## Phase 8: Medium - Protocol/Domain Conversions (Complexity: Medium)

### CR4-8.1 - Add TimeOfDay Conversion
**Status**: COMPLETE
**Files**: `crates/protocol/src/types.rs`

**Tasks**:
- [x] Add `From<domain::TimeOfDay> for TimeOfDayData`
- [x] Add `From<TimeOfDayData> for domain::TimeOfDay`

---

### CR4-8.2 - Add Missing Unknown Variants
**Status**: COMPLETE
**Files**: `crates/protocol/src/types.rs`

**Tasks**:
- [x] Add `#[serde(other)] Unknown` to TriggerCategory
- [x] Add `#[serde(other)] Unknown` to TriggerFieldType
- [x] Update TriggerCategory::label() to handle Unknown variant

---

## Phase 9: Medium - Neo4j Issues (Complexity: Medium)

### CR4-9.1 - Fix N+1 Query in Staging
**Status**: COMPLETE
**Files**: `crates/engine/src/infrastructure/neo4j/staging_repo.rs`

**Tasks**:
- [x] Refactor get_pending_staging to use COLLECT() for NPCs
- [x] Refactor get_staging_history similarly
- [x] Add row_to_staging_with_npcs and parse_collected_npcs helpers

---

### CR4-9.2 - Fix Item UNION ORDER BY
**Status**: COMPLETE
**Files**: `crates/engine/src/infrastructure/neo4j/item_repo.rs`

**Tasks**:
- [x] Move ORDER BY outside the UNION (was incorrectly inside each branch)
- [x] Verified relationship names are correct (IN_REGION, CURRENTLY_IN)

---

## Phase 10: Low - Cleanup (Complexity: Low)

### CR4-10.1 - Remove Unused Clock Fields
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/use_cases/movement/enter_region.rs`
- `crates/engine/src/use_cases/movement/exit_location.rs`
- `crates/engine/src/app.rs`

**Tasks**:
- [x] Remove unused `clock` field from EnterRegion
- [x] Remove unused `clock` field from ExitLocation
- [x] Update App wiring

---

### CR4-10.2 - Fix Flag Deduplication
**Status**: COMPLETE
**Files**: `crates/engine/src/entities/flag.rs`

**Tasks**:
- [x] Use HashSet to deduplicate flags in get_all_flags_for_pc

---

---

## Progress Tracking

| Phase | Items | Completed | Status |
|-------|-------|-----------|--------|
| Phase 1 | 5 | 5 | COMPLETE |
| Phase 2 | 1 | 1 | COMPLETE |
| Phase 3 | 2 | 2 | COMPLETE |
| Phase 4 | 1 | 1 | COMPLETE |
| Phase 5 | 3 | 3 | COMPLETE |
| Phase 6 | 2 | 2 | COMPLETE |
| Phase 7 | 1 | 1 | COMPLETE |
| Phase 8 | 2 | 2 | COMPLETE |
| Phase 9 | 2 | 2 | COMPLETE |
| Phase 10 | 2 | 2 | COMPLETE |
| **Total** | **21** | **21** | **COMPLETE** |

---

## Commit History

| Commit | Task | Description |
|--------|------|-------------|
| 8761791 | Phase 1 | Add delete operations for all major entities |
| 277462d | Phase 2, 3 | Cleanup unused vars and clear UI state on disconnect |
| c52b3cc | Phase 4, 5 | Extract scene helper and fix UI state bugs |
| TBD | Phase 6-10 | WebSocket broadcasts, observation warning, protocol types, Neo4j fixes, cleanup |
