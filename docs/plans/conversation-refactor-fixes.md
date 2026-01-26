**Created:** January 25, 2026
**Status:** Complete - All fixes implemented and reviewed
**Owner:** OpenCode
**Scope:** Conversation management refactor hardening

---

## Goal

Resolve all verified bugs/tech debt from the conversation_id refactor, align with ADR-011, and ensure DM conversation management is safe, correct, and fully wired.

## Summary

✅ **Backend fixes complete** - All engine changes implemented (see additional fixes below)
✅ **Frontend fixes complete** - All player crate changes implemented
✅ **Review round complete** - Code + architecture review issues resolved
✅ **Tests/checks complete** - cargo check clean across engine/player/shared

---

## Findings (validated)

1) **World scoping missing for DM details/end** (BUG)
   - Details/end allow cross-world access; list already checks conn world

2) **conversation_id not validated against participants** (BUG)
   - ContinueConversation can attach to wrong conversation

3) **ConversationEnded clears unrelated dialogue state** (BUG)
   - Player UI resets even when different conversation ends

4) **Conversation details query correctness** (BUG)
   - Turn lookup not scoped to conversation
   - Participant relationships can exclude entries

5) **N+1 query for turn counts** (TECH DEBT)
   - List active conversations scales poorly

6) **Loading flags not cleared** (BUG)
   - DM panel stays "Loading…" when list/details returns same length or replaces details

 7) **include_ended ignored** (FIXED)
     - Protocol flag now honored - repo includes ended conversations when include_ended=true

8) **Legacy non-UUID conversation IDs** (BUG)
   - list/details may hard-fail on legacy data

9) **ADR-011 violation** (N/A)
   - Conversion helpers in use cases are allowed if called in API boundary

10) **Duplicate conversion logic** (TECH DEBT)
   - list_active + conversation_protocol both map protocol

11) **Unused deps in conversation use cases** (TECH DEBT)
   - character/player_character repos unused in list_active/get_details/end_by_id

---

## Progress Tracking

- [x] Validate findings vs codebase (bugs vs tech debt)
  - [x] Backend fixes (engine/shared)
    - [x] World scoping for DM conversation details/end (repo-level World scoping added)
    - [x] conversation_id validation in ContinueConversation
    - [x] Fix conversation details query (SPOKE_TO optional, turn scoped)
    - [x] Remove N+1 turn-count query in list_active
    - [x] include_ended flag implementation
    - [x] Legacy non-UUID ID handling (removed - now fail-fast with RepoError)
    - [x] Consolidate protocol conversion (API layer only)
    - [x] Remove unused repo dependencies
    - [x] Fail-fast error mapping (NotFound/Conflict/BadRequest)
  - [x] Frontend fixes (player)
    - [x] Only clear dialogue state when ConversationEnded conversation_id matches active conversation_id
    - [x] Clear LLM processing flag on PlayerEvent::Error
    - [x] Clear conversation_details and details_loading when DM ends a conversation
    - [x] Loading flags cleared on ActiveConversationsList/ConversationDetails arrival (via use_effect)
- [x] Review round (code + architecture)
- [x] Tests/checks and final verification

---

## Planned Fixes (post-validation)

- [x] **World scoping:** enforce conn_info.world_id in handlers; add world check in repo queries
  - Added world_id parameter to GetConversationDetailsInput and EndConversationById
  - Handlers now pass conn_info.world_id for scoping
  - Added World scoping to repo queries (MATCH from World for get_conversation_details and end_conversation_by_id)
  - Removed TODO comments about world validation - now handled at repo level

- [x] **Conversation validation:** ensure conversation_id belongs to (pc_id, npc_id, world_id)
  - Added participant validation in ContinueConversation when conversation_id is provided
  - Validates conversation details match expected pc_id and npc_id
  - Returns BadRequest error on mismatch

- [x] **UI clearing:** only clear dialogue if conversation_id matches active (FRONTEND - COMPLETE)
  - Modified ConversationEnded handler to compare ended_conversation_id with active_conversation_id
  - Only clears dialogue_state when conversation being ended is the active one
  - Preserves dialogue state for unrelated conversations

- [x] **Query correctness:** scope turns to conversation; make relationship optional
  - get_conversation_details: Made SPOKE_TO relationship OPTIONAL
  - get_conversation_details: Scoped turn lookup to specific conversation (was returning wrong turns)

- [x] **Turn counts:** fold into main query to avoid N+1
  - list_active_conversations: Aggregated turn_count in main query instead of separate query per conversation

- [x] **Loading flags:** clear on ActiveConversationsList/ConversationDetails regardless of size/replace; clear on error/timeout (FRONTEND - COMPLETE)
  - conversations_loading cleared when ActiveConversationsList arrives via use_effect in content.rs
  - details_loading cleared when ConversationDetails arrives via use_effect in content.rs
  - is_llm_processing flag cleared on PlayerEvent::Error in session_message_handler.rs

  - [x] **include_ended:** implement flag to include ended conversations
   - Added include_ended parameter to list_active_conversations in NarrativeRepo trait
   - Updated Neo4j query to remove is_active filter when include_ended is true
   - Updated use case to accept and pass through include_ended parameter
   - Updated handler to pass include_ended to use case (previously unused parameter)
   - is_active field already returned from query, so DM list correctly shows status

- [x] **Legacy IDs:** fail-fast on non-UUID (no longer supported)
   - Removed warn+skip handling from list_active_conversations, get_active_conversation_id, end_active_conversation
   - All conversation_id parsing now returns RepoError on invalid UUID format

- [x] **Dup conversions:** single conversion helper in API layer
   - Removed duplicate conversion types from list_active.rs
   - Kept only domain types in use case
   - Protocol conversion now only in conversation_protocol.rs (API layer per ADR-011)

- [x] **Unused deps:** remove or use repos
  - Removed unused character, player_character repos from list_active, get_details, end_by_id
  - Updated constructor signatures to match

- [x] **Code review fixes (Jan 26)**
  - continue_conversation now propagates repo errors for is_conversation_active
  - Participant last_spoke and speaker_name scoped by conversation + speaker_id
  - Start conversation validates empty/length like continue
  - VisualStateChanged uses Option visual_state (clears overrides)
  - include_ended implemented in list_active query

- [x] **Frontend review fixes (Jan 26)**
  - ConversationEnded clears dialogue only when appropriate
  - VisualStateChanged handles optional region_id
  - LLM processing cleared on error

---

## Additional Backend Fixes (Code Review Round, January 26)

### 1. ContinueConversation - Error Propagation Fix
- **Issue:** Used `.unwrap_or(false)` on `is_conversation_active()`, silently swallowing repository errors
- **Fix:** Replaced with `?` operator to properly propagate `RepoError` via `ConversationError::Repo`
- **File:** `engine/src/use_cases/conversation/continue_conversation.rs`
- **Impact:** Now fail-fast on repo errors instead of treating them as "conversation not active"

### 2. NarrativeRepo - Query Scoping Fixes
- **Issue:** `last_spoke` data not scoped to conversation; turn speaker resolution used non-existent SPEAKER edge
- **Fixes:**
  - Scoped `last_spoke` lookup to conversation with `(c)-[:HAS_TURN]->(last_t:DialogueTurn {speaker_id: p.id, order: last_order})`
  - Resolved speaker names via `t.speaker_id` lookup to pc/npc nodes instead of SPEAKER edge
  - Used `coalesce()` for speaker_name fallback
- **Files:** `engine/src/infrastructure/neo4j/narrative_repo.rs`
- **Impact:** Fixed incorrect last_spoke_at/last_spoke data; eliminated query errors from missing SPEAKER edges

### 3. WebSocket Staging - Visual State Option Handling
- **Issue:** Defaulted `VisualStateChanged` to empty struct, always sending override even when None
- **Fix:** Send option directly; `None` clears override, `Some` sets override
- **Files:** `engine/src/api/websocket/ws_staging.rs`
- **Impact:** Allows DM to clear visual overrides via None value instead of always overwriting with empty state

### 4. WebSocket Conversation - Message Validation
- **Issue:** `handle_start_conversation` lacked message length/empty validation (present in `handle_continue_conversation`)
- **Fixes:**
  - Added `MAX_MESSAGE_LENGTH` (2000 chars) check
  - Added empty message check
  - Removed unused `normalize_conversation_message()` function
- **Files:** `engine/src/api/websocket/ws_conversation.rs`
- **Impact:** Consistent input validation across start/continue conversation paths

### 5. ListActiveConversations - WorldNotFound Removal
- **Issue:** Returned `WorldNotFound` error when world missing, but empty list is semantically correct
- **Fix:** Removed `WorldNotFound` variant; repo returns empty list when world doesn't exist
- **Files:**
  - `engine/src/use_cases/conversation/list_active.rs` (removed variant and error handling)
  - `engine/src/api/websocket/ws_conversation.rs` (removed WorldNotFound handler match)
- **Impact:** Cleaner API; "no conversations" is the correct response for world with no conversations

## Frontend Fixes (Code Review Round, January 26)

### 1. ConversationEnded - Dialogue State Clearing Fix
- **Issue:** When `conversation_id` is None, cleared dialogue state unconditionally (even for unrelated NPCs)
- **Fix:** Match on `ended_conversation_id`:
  - If `Some(id)`: Clear only if it matches `active_conversation_id`
  - If `None`: Clear if `speaker_id` matches `npc_id` OR if there's no active conversation
- **File:** `crates/player/src/ui/presentation/handlers/session_message_handler.rs`
- **Impact:** Prevents clearing unrelated dialogue state when different conversations end

### 2. VisualStateChanged - Override Clearing Fix
- **Issue:** When `visual_state` is `None` for current region, override not cleared
- **Fix:** Apply visual state even when `None` (handler now calls `set_visual_state_override` unconditionally when region matches)
- **Note:** `set_visual_state_override()` already handles `Option<T>` correctly - no changes needed to GameState
- **Files:**
  - `crates/player/src/ui/presentation/handlers/session_message_handler.rs` (handler fix)
  - `crates/player/src/ui/presentation/state/game_state.rs` (no changes needed)
- **Impact:** Allows DM to clear visual state overrides by sending `None` instead of always overwriting
