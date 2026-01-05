# Code Review Fixes - Round 2 Implementation Plan

**Created**: 2026-01-05
**Status**: IN PROGRESS
**Source**: Comprehensive 7-agent code review

---

## Priority 1: Critical Fixes (Blocking/Breaking)

### CR2-1.1 - Protocol GameTimeConfig Missing time_format
**Status**: COMPLETE
**Files**: `crates/protocol/src/types.rs`
**Issue**: Protocol `GameTimeConfig` missing `time_format` field that exists in domain
**Impact**: Deserialization failures between engine and player

**Tasks**:
- [x] Add `TimeFormat` enum to protocol types
- [x] Add `time_format` field to protocol `GameTimeConfig`
- [x] Use `#[serde(default)]` for backward compatibility
- [x] Export `TimeFormat`, `GameTimeConfig`, and related types from lib.rs

---

### CR2-1.2 - Protocol AdHocOutcomes Serde Mismatch
**Status**: COMPLETE
**Files**: `crates/protocol/src/messages.rs`
**Issue**: Protocol `AdHocOutcomes` missing `#[serde(rename_all = "camelCase")]`
**Impact**: JSON field name mismatch with domain type

**Tasks**:
- [x] Add `#[serde(rename_all = "camelCase")]` to protocol `AdHocOutcomes`

---

### CR2-1.3 - WebSocket Unknown Message No Response
**Status**: COMPLETE
**Files**: `crates/engine/src/api/websocket.rs`
**Issue**: `ClientMessage::Unknown` logs but returns `None` - client hangs
**Impact**: Client hangs indefinitely on unknown messages

**Tasks**:
- [x] Return error response for `ClientMessage::Unknown`

---

### CR2-1.4 - Duplicate ConversationError Types
**Status**: COMPLETE
**Files**: `crates/engine/src/use_cases/conversation/continue_conversation.rs`
**Issue**: `ConversationError` defined in both `start.rs` and `continue_conversation.rs`
**Impact**: Import conflicts, type confusion

**Tasks**:
- [x] Add `NpcLeftRegion` variant to shared `ConversationError` in `start.rs`
- [x] Remove duplicate `ConversationError` from `continue_conversation.rs`
- [x] Import shared error from `start.rs`

---

## Priority 2: High Severity Fixes

### CR2-2.1 - UI State clear_scene() Missing Fields
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/state/game_state.rs`
**Issue**: `clear_scene()` doesn't clear `region_items` or `npc_moods`
**Impact**: Stale data persists across sessions

**Tasks**:
- [x] Add `self.region_items.set(Vec::new())` to `clear_scene()`
- [x] Add `self.npc_moods.write().clear()` to `clear_scene()`

---

### CR2-2.2 - ResponseApproved Not Updating State
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
**Issue**: `ResponseApproved` event only logs, doesn't clear `is_llm_processing`
**Impact**: UI may show processing state incorrectly

**Tasks**:
- [x] Add `dialogue_state.is_llm_processing.set(false)` in handler

---

### CR2-2.3 - StagingReady Missing NPC Names
**Status**: PENDING
**Files**: `crates/engine/src/api/websocket.rs`
**Issue**: `StagingReady` broadcast sends empty NPC names
**Impact**: Players see NPCs without names

**Tasks**:
- [ ] Fetch NPC details when building `StagingReady` response
- [ ] Populate name, sprite_asset, portrait_asset from character entity

---

### CR2-2.4 - Neo4j Delete Without Transaction
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/character_repo.rs`
**Issue**: `delete` runs two queries without transaction - orphaned data possible
**Impact**: Database inconsistency on partial failure

**Tasks**:
- [ ] Combine into single atomic query with OPTIONAL MATCH

---

### CR2-2.5 - Neo4j update_position Silent Failure
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/player_character_repo.rs`
**Issue**: `update_position` silently fails if Location doesn't exist
**Impact**: PC ends up with no location relationship

**Tasks**:
- [ ] Combine delete and create into single query
- [ ] Return error if location not found

---

### CR2-2.6 - Forward Compatibility Missing in Domain Enums
**Status**: PENDING
**Files**: `crates/domain/src/entities/want.rs`, `crates/domain/src/entities/lore.rs`
**Issue**: Domain enums missing `#[serde(other)] Unknown` variants
**Impact**: Deserialization fails on new enum values

**Tasks**:
- [ ] Add `Unknown` variant to `WantTargetType`
- [ ] Add `Unknown` variant to `LoreCategory`
- [ ] Add `Unknown` variant to `StagingSource`

---

### CR2-2.7 - Player-App Type Mismatch
**Status**: PENDING
**Files**: `crates/player-app/src/application/services/actantial_service.rs`
**Issue**: `WantResponse.priority` is `i32` but protocol uses `u32`
**Impact**: Type confusion, potential overflow

**Tasks**:
- [ ] Change `WantResponse.priority` to `u32`

---

## Priority 3: Medium Severity Fixes

### CR2-3.1 - StagingReady Not Clearing pending_staging_approval
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
**Issue**: DM approval popup not cleared when staging becomes ready
**Impact**: Stale approval popup for DM

**Tasks**:
- [x] Clear `pending_staging_approval` in `StagingReady` handler

---

### CR2-3.2 - Extract Duplicate Staging Resolution
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/mod.rs`
**Issue**: Staging resolution duplicated in enter_region and exit_location
**Impact**: Maintenance burden, inconsistency risk

**Tasks**:
- [ ] Create shared `resolve_staging_for_region` helper
- [ ] Refactor both use cases to use it

---

### CR2-3.3 - Extract Duplicate Time Suggestion
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/mod.rs`
**Issue**: Time suggestion logic duplicated
**Impact**: Maintenance burden

**Tasks**:
- [ ] Create shared `suggest_time_for_movement` helper
- [ ] Refactor both use cases to use it

---

### CR2-3.4 - Extract Duplicate row_to_item
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/helpers.rs`
**Issue**: `row_to_item` duplicated in 3 repo files
**Impact**: Maintenance burden

**Tasks**:
- [ ] Move `row_to_item` to helpers.rs
- [ ] Update character_repo, player_character_repo, item_repo to use it

---

### CR2-3.5 - HTTP API Error Logging
**Status**: PENDING
**Files**: `crates/engine/src/api/http.rs`
**Issue**: Internal errors not logged before sanitizing response
**Impact**: Hard to debug production issues

**Tasks**:
- [ ] Add `tracing::error!` for `ApiError::Internal`

---

## Priority 4: Low Severity / Documentation

### CR2-4.1 - Update Lore System Documentation
**Status**: PENDING
**Files**: `docs/systems/lore-system.md`
**Issue**: Docs say "Pending" but backend is fully implemented
**Impact**: Documentation inaccurate

**Tasks**:
- [ ] Update user story statuses to reflect implementation

---

### CR2-4.2 - Update Flag System Documentation
**Status**: PENDING
**Files**: `docs/systems/scene-system.md`
**Issue**: US-SCN-010 says flag storage pending, but it's implemented
**Impact**: Documentation inaccurate

**Tasks**:
- [ ] Mark US-SCN-010 as complete

---

## Implementation Order

1. CR2-1.1 - Protocol GameTimeConfig (breaks serialization)
2. CR2-1.2 - Protocol AdHocOutcomes (breaks serialization)
3. CR2-1.3 - WebSocket Unknown Message (client hangs)
4. CR2-1.4 - Duplicate ConversationError (code quality)
5. CR2-2.1 - UI State clear_scene (stale data)
6. CR2-2.2 - ResponseApproved handler (UI correctness)
7. CR2-2.3 - StagingReady NPC names (user experience)
8. CR2-2.4 - Neo4j delete transaction (data integrity)
9. CR2-2.5 - Neo4j update_position (data integrity)
10. CR2-2.6 - Domain enum forward compatibility
11. CR2-2.7 - Player-App type mismatch
12. CR2-3.1 - StagingReady clear approval
13. CR2-3.2 - Extract staging resolution
14. CR2-3.3 - Extract time suggestion
15. CR2-3.4 - Extract row_to_item
16. CR2-3.5 - HTTP error logging
17. CR2-4.1 - Update lore docs
18. CR2-4.2 - Update flag docs

---

## Progress Tracking

| ID | Description | Status | Commit |
|----|-------------|--------|--------|
| CR2-1.1 | Protocol GameTimeConfig | COMPLETE | - |
| CR2-1.2 | Protocol AdHocOutcomes | COMPLETE | - |
| CR2-1.3 | WebSocket Unknown Message | COMPLETE | - |
| CR2-1.4 | Duplicate ConversationError | COMPLETE | - |
| CR2-2.1 | UI State clear_scene | COMPLETE | - |
| CR2-2.2 | ResponseApproved handler | COMPLETE | - |
| CR2-2.3 | StagingReady NPC names | PENDING | - |
| CR2-2.4 | Neo4j delete transaction | PENDING | - |
| CR2-2.5 | Neo4j update_position | PENDING | - |
| CR2-2.6 | Domain enum forward compat | PENDING | - |
| CR2-2.7 | Player-App type mismatch | PENDING | - |
| CR2-3.1 | StagingReady clear approval | COMPLETE | - |
| CR2-3.2 | Extract staging resolution | PENDING | - |
| CR2-3.3 | Extract time suggestion | PENDING | - |
| CR2-3.4 | Extract row_to_item | PENDING | - |
| CR2-3.5 | HTTP error logging | PENDING | - |
| CR2-4.1 | Update lore docs | PENDING | - |
| CR2-4.2 | Update flag docs | PENDING | - |
