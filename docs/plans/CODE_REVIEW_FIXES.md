# Code Review Fixes Implementation Plan

**Created**: 2026-01-05
**Status**: IN PROGRESS
**Source**: Comprehensive code review session

---

## Priority 1: Critical Fixes (Blocking Functionality)

### P1.1 - Approval Popup Button Handlers
**Status**: COMPLETE
**Files**: `crates/player-ui/src/presentation/components/dm_panel/approval_popup.rs`
**Issue**: "Approve Challenge", "Skip Challenge", "Trigger Event", "Skip Event" buttons have no onclick handlers
**Impact**: DM cannot approve challenges or trigger narrative events from the approval popup

**Tasks**:
- [x] Add `on_approve_challenge: Option<EventHandler<ChallengeSuggestionInfo>>` prop
- [x] Add `on_skip_challenge: Option<EventHandler<ChallengeSuggestionInfo>>` prop
- [x] Add `on_trigger_event: Option<EventHandler<NarrativeEventSuggestionInfo>>` prop
- [x] Add `on_skip_event: Option<EventHandler<NarrativeEventSuggestionInfo>>` prop
- [x] Wire onclick handlers to buttons
- [ ] Update all call sites to provide handlers (deferred - call sites will add as needed)

---

### P1.2 - Cypher Injection Fix
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/character_repo.rs:1045-1068`
**Issue**: `remove_region_relationship` uses string formatting for relationship type in Cypher query
**Impact**: Potential security vulnerability

**Tasks**:
- [ ] Replace `format!()` with match statement using static queries
- [ ] Each relationship type gets its own pre-defined query
- [ ] Return error for unknown relationship types

---

### P1.3 - Game Time in Scene Resolution
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/enter_region.rs:269-272`
**Issue**: Uses `self.clock.now()` (wall clock) instead of world's game time for scene resolution
**Impact**: Scenes resolve based on real time, not in-game time

**Tasks**:
- [ ] Pass `current_game_time` to scene resolution
- [ ] Use world's game time for TimeOfDay checks
- [ ] Update method signatures as needed

---

### P1.4 - Wire FlagRepo to Effect Executor
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/narrative/execute_effects.rs:265-276`
**Issue**: SetFlag effect returns "not implemented" but FlagRepo exists
**Impact**: Narrative events with flag effects don't work

**Tasks**:
- [ ] Add FlagRepo to ExecuteEffects struct
- [ ] Update constructor to inject FlagRepo
- [ ] Implement SetFlag case using FlagRepo
- [ ] Update App wiring

---

## Priority 2: High Severity Bugs

### P2.1 - UTF-8 Safe String Truncation
**Status**: PENDING
**Files**: `crates/engine/src/entities/narrative.rs:313-318`
**Issue**: `truncate_dialogue` can panic on multi-byte UTF-8 characters
**Impact**: Server crash on certain dialogue text

**Tasks**:
- [ ] Replace byte slicing with `chars().take()`
- [ ] Add unit test for multi-byte characters

---

### P2.2 - Staging World ID Fix
**Status**: PENDING
**Files**: `crates/engine/src/infrastructure/neo4j/staging_repo.rs:76-103`
**Issue**: `r.world_id` may not exist on Region nodes
**Impact**: Null world_id in staging data

**Tasks**:
- [ ] Update Cypher to traverse Region->Location->World
- [ ] Or ensure Region has world_id property

---

### P2.3 - Expression Sheet Image Slicing
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/assets/expression_sheet.rs:208-262`
**Issue**: Image slicing returns placeholder results
**Impact**: Expression sheets don't actually get sliced

**Tasks**:
- [ ] Add `image` crate dependency
- [ ] Implement actual image loading and slicing
- [ ] Save individual sprites with expression names

---

## Priority 3: Medium Severity Gaps

### P3.1 - Add Timestamps to Entities
**Status**: PENDING
**Files**: Multiple domain entities
**Issue**: 9 entities missing created_at/updated_at timestamps
**Impact**: Audit trail incomplete

**Entities to update**:
- [ ] Character
- [ ] Location
- [ ] Region
- [ ] Scene
- [ ] Challenge
- [ ] Item
- [ ] Goal
- [ ] Skill
- [ ] GridMap

---

### P3.2 - NPC Mood Changed Handler
**Status**: PENDING
**Files**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs:1147-1149`
**Issue**: NpcMoodChanged event logged but doesn't update UI state
**Impact**: Mood changes not reflected in UI

**Tasks**:
- [ ] Add mood tracking to game_state or scene_characters
- [ ] Update NpcMoodChanged handler to modify state
- [ ] Ensure character sprites reflect mood

---

### P3.3 - Duplicate Method Cleanup
**Status**: PENDING
**Files**: `crates/engine/src/entities/location.rs`
**Issue**: `get()` vs `get_location()`, `list_in_world()` vs `list_locations_in_world()`
**Impact**: Code confusion, maintenance burden

**Tasks**:
- [ ] Deprecate longer method names
- [ ] Update all call sites
- [ ] Remove deprecated methods

---

## Priority 4: Code Quality Improvements

### P4.1 - Extract Staging Resolution Helper
**Status**: PENDING
**Files**: `crates/engine/src/use_cases/movement/enter_region.rs`, `exit_location.rs`
**Issue**: Staging resolution logic duplicated
**Impact**: Maintenance burden

**Tasks**:
- [ ] Create shared function in `movement/mod.rs`
- [ ] Refactor both use cases to use it

---

### P4.2 - Extract Item Action Helpers in PC View
**Status**: PENDING
**Files**: `crates/player-ui/src/presentation/views/pc_view.rs:1062-1306`
**Issue**: 10+ nearly identical `send_*_item` functions
**Impact**: Maintenance burden

**Tasks**:
- [ ] Create `with_client` helper function
- [ ] Refactor all item action senders

---

## Implementation Order

1. P1.1 - Approval buttons (unblocks DM workflow)
2. P1.4 - FlagRepo wiring (unblocks narrative events)
3. P1.2 - Cypher injection (security)
4. P1.3 - Game time fix (correctness)
5. P2.1 - UTF-8 truncation (stability)
6. P2.2 - Staging world_id (data integrity)
7. P3.2 - Mood changed handler (UI completeness)
8. P3.3 - Duplicate methods (cleanup)
9. P4.1 - Staging helper (code quality)
10. P4.2 - Item action helpers (code quality)

---

## Progress Tracking

| ID | Description | Status | Commit |
|----|-------------|--------|--------|
| P1.1 | Approval popup buttons | COMPLETE | (pending) |
| P1.2 | Cypher injection fix | PENDING | - |
| P1.3 | Game time in scenes | PENDING | - |
| P1.4 | FlagRepo wiring | PENDING | - |
| P2.1 | UTF-8 truncation | PENDING | - |
| P2.2 | Staging world_id | PENDING | - |
| P2.3 | Image slicing | PENDING | - |
| P3.1 | Entity timestamps | PENDING | - |
| P3.2 | Mood changed handler | PENDING | - |
| P3.3 | Duplicate methods | PENDING | - |
| P4.1 | Staging helper | PENDING | - |
| P4.2 | Item action helpers | PENDING | - |
