# Playtestable State Implementation Plan

## Overview
This plan tracks progress toward a playtestable visual novel state. Created 2026-01-09 based on comprehensive codebase review.

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Complete

---

## Phase 1: Critical Bug Fixes

### 1.1 Approval Queue Polling (CRITICAL)
**Problem:** DMApprovalQueue is never polled in main.rs queue loop. Approval requests sit in queue, DMs never see them, conversations appear broken.

**Location:** `crates/engine/src/main.rs` lines 93-162

**Tasks:**
- [x] Add approval queue dequeue logic to main.rs queue loop
- [x] Send `ApprovalRequired` message to DMs when item dequeued
- [ ] Verify DM sees approval requests in UI
- [ ] Test full conversation flow end-to-end

---

## Phase 2: Legacy Code Removal (CommandBus Cleanup)

### 2.1 Delete Dead Code
- [x] Delete `crates/player/src/runner.rs` (entire file unused)
- [x] Remove unused `user_id` variable from `session_service.rs:126`

### 2.2 Remove Unused WebSocket Code
- [x] Remove `state_to_u8()` and `u8_to_state()` from `websocket/protocol.rs`
- [x] Remove `insert()` method from `websocket/core.rs`
- [x] Remove unused methods from `websocket/desktop/client.rs`:
  - `url()`
  - `state()`
  - `join_world()`
  - `send_action()`
  - `heartbeat()`
  - `request()`
  - `request_with_timeout()`

### 2.3 Update Documentation
- [x] Update comments in `session_types.rs` (line 1)
- [x] Update comments in `application/dto/requests.rs` (line 5)
- [x] Update comments in `application/dto/mod.rs` (line 78)
- [x] Update `crates/player/README.md` to remove GameConnectionPort references

---

## Phase 3: Staging Flow Improvements

### 3.1 Staging Timeout/Fallback
**Problem:** Every region entry blocks for DM approval if no valid staging exists.

- [x] Add configurable staging timeout (default 30 seconds)
- [x] Implement fallback to rule-based staging if DM doesn't respond
- [x] Add "auto-approve rule-based" option in world settings

### 3.2 Pre-staging Support
- [ ] Verify pre-staging workflow works
- [ ] Document how to pre-stage regions for playtesting

---

## Phase 4: Conversation Context Integration

### 4.1 Populate conversation_history
**Status:** ALREADY IMPLEMENTED

The conversation history feature was completed before this plan was created. The implementation:
- `queues/mod.rs:255-269` - Fetches conversation turns from narrative entity
- `entities/narrative.rs:357-375` - Gets turns from repo and converts to ConversationTurn
- `infrastructure/neo4j/narrative_repo.rs:1078-1131` - Neo4j query for DialogueTurn nodes
- `approval/mod.rs:247-264` - Records dialogue when DM approves

- [x] Query DialogueTurn nodes before building LLM prompt
- [x] Populate `conversation_history` field in `GamePromptRequest`
- [x] Wire ContextBudgetEnforcer for token limits (via context_budget field)
- [ ] Test multi-turn conversations maintain context (manual testing needed)

---

## Phase 5: UX Polish

### 5.1 Awaiting DM Feedback ✓
- [x] Add "Awaiting Dungeon Master..." message during approval waits
- [x] Show approval status in player UI (StagingPendingOverlay)
- [x] Add timeout indicator with countdown timer

### 5.2 MiniMap Integration ✓
- [x] Add region bounds to protocol (RegionListItemData, MapBoundsData)
- [x] Create typed ListRegions response with map_bounds
- [x] Connect MiniMap component to engine data via LocationService

### 5.3 Scene Transitions ✓
- [x] Add fade-out/fade-in effect between regions (animate-fade-out)
- [x] Add transition state to GameState (backdrop_transitioning signal)
- [x] Implement backdrop crossfade (backdropCrossfade keyframes)

---

## Progress Log

### 2026-01-10
- Implemented Phase 5.1: Staging timeout indicator with countdown
- Implemented Phase 5.2: MiniMap region bounds from engine
- Implemented Phase 5.3: Scene backdrop fade transitions
- Updated systems documentation (staging-system.md, navigation-system.md, scene-system.md)

### 2026-01-09
- Created plan based on comprehensive codebase review
- Identified critical approval queue polling bug
- Mapped all legacy code requiring removal
- Fixed critical approval queue polling in main.rs
- Deleted runner.rs (dead code)
- Removed unused WebSocket methods (protocol.rs, core.rs, client.rs)
- Updated documentation comments to reference CommandBus
- Updated README.md to remove GameConnectionPort references
- Implemented staging timeout/fallback system:
  - Added `staging_timeout_seconds` and `auto_approve_on_timeout` settings
  - Added `AutoApproved` variant to StagingSource enum
  - Added `created_at` and `world_id` to PendingStagingRequest
  - Created AutoApproveStagingTimeout use case
  - Added staging timeout processor task in main.rs
- Verified Phase 4 (Conversation Context) was already implemented

---

## Files Modified Tracker

This section tracks files modified during implementation for easy review:

### Phase 1
- `crates/engine/src/main.rs` - Approval queue polling

### Phase 2
- `crates/player/src/runner.rs` - DELETED
- `crates/player/src/infrastructure/websocket/protocol.rs` - Remove unused functions
- `crates/player/src/infrastructure/websocket/core.rs` - Remove unused method
- `crates/player/src/infrastructure/websocket/desktop/client.rs` - Remove unused methods
- `crates/player/src/application/services/session_service.rs` - Remove unused variable

### Phase 3
- TBD

### Phase 4
- `crates/engine/src/use_cases/queues/mod.rs` - conversation_history population

### Phase 5
- TBD
