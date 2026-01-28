**Created:** January 27, 2026
**Status:** Draft
**Owner:** OpenCode
**Scope:** Next playability priorities after remediation

---

## Ordered Priorities

1) Dialogue conversation scoping + DM recovery controls (US-DLG-018/019/020/021/022/023)
2) Visual state surfaced in staging + player backdrop updates (US-VS-007/008/009)
3) Dialogue-driven time suggestions + player time-advance toast (US-TIME-013/011)
4) Staging settings UI for world/location defaults (US-STG-016/017)
5) Creator flow: character/NPC editing WS wiring

---

## Priority 1 — Dialogue scoping + DM recovery (US-DLG-018..023)

**Status:** ✅ Complete

**Acceptance criteria (summary)**
- Dialogue payloads include `conversation_id` everywhere (responses, approvals, processing, ended).
- Player renders only the active conversation thread.
- DM can list active conversations, view participants, and end by id.
- `is_active` invariant enforced (ended_at implies inactive, no reactivation).

**Backend**
- Ensure `conversation_id` is carried through approval queue → response path.
- `EndConversationById` implemented and broadcasted to participants.
- ListActiveConversations + GetConversationDetails return participants + status fields.
- Enforce world scoping for DM actions.

**Frontend**
- Key dialogue state by `conversation_id`.
- DM panel: list + detail + end by id.
- Clear only matching conversation on `ConversationEnded`.

**Tests**
- WS integration: list/details/end by id + broadcast.
- UI state tests for conversation scoping.

**Files (expected)**
- `crates/shared/src/messages/dialogue.rs`
- `crates/shared/src/messages/approvals.rs`
- `crates/engine/src/api/websocket/ws_conversation.rs`
- `crates/engine/src/use_cases/conversation/*`
- `crates/player/src/ui/presentation/state/dialogue_state.rs`
- `crates/player/src/ui/presentation/components/dm_panel/*`

---

## Priority 2 — Visual state staging + player backdrop (US-VS-007/008/009)

**Status:** 🟡 In progress

**Backend**
- Validate override IDs against available options (store in pending staging or recompute).
- Always broadcast `VisualStateChanged` after approval/auto‑approve.

**Frontend**
- DM staging approval: dropdown + preview + details modal; fetch visual state details on selection.
- Player view: apply `VisualStateChanged` and trigger backdrop transitions.

**Tests**
- Staging approval integration: includes resolved visual state + options; invalid override → ValidationError.
- Player state: visual state precedence in backdrop_url().

**Files (expected)**
- `crates/engine/src/use_cases/staging/*`
- `crates/engine/src/api/websocket/ws_staging.rs`
- `crates/player/src/ui/presentation/components/dm_panel/staging_approval.rs`
- `crates/player/src/ui/presentation/components/dm_panel/visual_state_*`

---

## Priority 3 — Time suggestions + toast (US-TIME-013/011)

**Status:** ✅ Complete

**Backend**
- Ensure dialogue approvals emit `TimeSuggestion` only in Suggested mode.
- Ensure manual `AdvanceGameTime` also emits `GameTimeAdvanced` with a reason.

**Frontend**
- Toast shown on `GameTimeAdvanced` with reason + delta; respects `show_time_to_players`.

**Tests**
- Approval decision emits suggestion; manual advance emits GameTimeAdvanced.

**Files (expected)**
- `crates/engine/src/use_cases/queues/mod.rs`
- `crates/engine/src/use_cases/time/mod.rs`
- `crates/engine/src/api/websocket/ws_time.rs`
- `crates/player/src/ui/presentation/components/common/time_advance_toast.rs`

---

## Priority 4 — Staging settings UI (US-STG-016/017)

**Status:** ✅ Complete

**UI**
- World staging auto‑approve timeout fields in settings.
- Location staging defaults (TTL + LLM presence) in DM location view.

**Tests**
- WS integration for location defaults persistence.

**Files (expected)**
- `crates/player/src/ui/presentation/components/settings/game_settings.rs`
- `crates/player/src/ui/presentation/components/creator/location_form.rs`

---

## Priority 5 — Creator flow wiring (character/NPC)

**Status:** 🔴 Not started

**Backend**
- Ensure WS handlers route character/NPC edits and broadcast `EntityChanged`.

**Frontend**
- Character form: send archetype changes + sheet updates.
- Add NPC service for mood/disposition/region relationships.

**Tests**
- WS integration for character edits + NPC relationship changes.

**Files (expected)**
- `crates/engine/src/api/websocket/ws_character.rs`
- `crates/engine/src/api/websocket/ws_npc.rs`
- `crates/player/src/application/services/npc_service.rs`
- `crates/player/src/ui/presentation/components/creator/character_form.rs`

---

## Priority 1 Expansion: Dialogue Scoping + DM Recovery Controls (US-DLG-018..023)

### Goal

Make dialogue state reliably scoped by `conversation_id` for players and give DMs tooling to monitor and recover from stuck conversations. This removes cross-thread leakage, enables safe resets, and unblocks play when approvals stall.

### User Stories In Scope

- **US-DLG-018**: Player dialogue is scoped to `conversation_id`
- **US-DLG-019**: DM can end a specific conversation by id
- **US-DLG-020**: DM can list active conversations
- **US-DLG-021**: DM can view conversation participants
- **US-DLG-022**: System enforces `is_active` semantics
- **US-DLG-023**: System sends conversation-aware message payloads

### Acceptance Criteria (Per Story)

- **US-DLG-018**
  - Dialogue responses, choices, and processing indicators include `conversation_id`.
  - Player UI renders only dialogue turns that match the active `conversation_id`.
  - Scene or region change clears active conversation state or swaps to new `conversation_id` when provided.

- **US-DLG-019**
  - DM can end a conversation by id via WebSocket command.
  - Server ends only the specified conversation and broadcasts `ConversationEnded` with matching `conversation_id`.
  - Player and DM UIs clear that thread without affecting other active conversations.

- **US-DLG-020**
  - Server returns active conversations list with id, participants, last_updated_at, location/scene metadata, turn_count, and pending_approval.
  - DM UI shows list, supports refresh, and live-updates on new/ended conversations.

- **US-DLG-021**
  - Conversation details include PC/NPC participants (id, name, type), plus per-participant turn_count and last_spoke_at when available.
  - DM UI displays participants in list and details views.

- **US-DLG-022**
  - `is_active` is true only when `ended_at` is null; ending sets `ended_at` and flips `is_active` false.
  - Re-opened conversations always create a new `conversation_id`.

- **US-DLG-023**
  - `ApprovalRequired`, `DialogueResponse`, `ConversationEnded`, and `LLMProcessing` include `conversation_id` and relevant pc/npc ids.
  - Clients rely on `conversation_id` for filtering and state updates; no implicit global dialogue state.

### Backend Changes

- **Protocol + shared messages**
  - Extend dialogue-related wire messages to carry `conversation_id` and ids for PC/NPC where applicable.
  - Update approval payloads to include `conversation_id` so DM decisions map to the correct thread.
  - Files: `crates/shared/src/messages/dialogue.rs`, `crates/shared/src/messages/approvals.rs`, `crates/shared/src/messages/conversation.rs`.

- **WebSocket handlers**
  - Include `conversation_id` in outbound `LLMProcessing`, `DialogueResponse`, `ConversationEnded`, and approval messages.
  - Add DM command `EndConversationById` and route to end use case.
  - Add request/response handler for `ListActiveConversations`.
  - Files: `crates/engine/src/api/websocket/ws_conversation.rs`, `crates/engine/src/api/websocket/mod.rs`.

- **Use cases**
  - `end.rs`: accept `conversation_id`, enforce end semantics, and broadcast end event.
  - `list_active.rs`: query conversations, include participants, pending approval, location/scene metadata.
  - Files: `crates/engine/src/use_cases/conversation/end.rs`, `crates/engine/src/use_cases/conversation/list_active.rs`.

- **Repositories (Neo4j)**
  - Add query for active conversations with participant and location/scene context.
  - Enforce `ended_at`/`is_active` invariants when ending a conversation.
  - Files: `crates/engine/src/infrastructure/ports.rs`, `crates/engine/src/infrastructure/neo4j/conversation_repo.rs`.

### Frontend Changes

- **Player UI (conversation scoping)**
  - Track active `conversation_id` in dialogue state; ignore messages for other threads.
  - Clear or switch state on `ConversationEnded` and scene/region changes.
  - Files: `crates/player/src/ui/presentation/state/dialogue_state.rs`, `crates/player/src/ui/presentation/views/pc_view.rs`, `crates/player/src/ui/presentation/components/action_panel.rs`.

- **DM UI (recovery controls)**
  - Add Active Conversations panel with refresh and per-card actions.
  - Add conversation details view with participants.
  - Add end conversation confirmation modal wired to `EndConversationById`.
  - Files: `crates/player/src/ui/presentation/components/dm_panel/active_conversations.rs`, `crates/player/src/ui/presentation/components/dm_panel/conversation_details.rs`, `crates/player/src/ui/presentation/components/dm_panel/end_conversation_modal.rs`.

### Tests

- **Use case tests**
  - `ListActiveConversations` returns participants, context, and pending_approval.
  - `EndConversation` sets `ended_at` and `is_active=false` and rejects re-ending.
  - Files: `crates/engine/src/use_cases/conversation/list_active_test.rs`, `crates/engine/src/use_cases/conversation/end_test.rs`.

- **WebSocket tests**
  - `EndConversationById` routed correctly and broadcasts `ConversationEnded` with id.
  - `ListActiveConversations` responds with expected data shape.
  - Files: `crates/engine/src/api/websocket/ws_conversation_test.rs`.

- **UI tests (if present)**
  - Dialogue state filters by `conversation_id` and clears on end.
  - DM panel renders active conversations list and participant details.
  - Files: `crates/player/src/ui/presentation/state/dialogue_state_test.rs`, `crates/player/src/ui/presentation/components/dm_panel/active_conversations_test.rs`.

### Implementation Notes

- Prefer `conversation_id` as the single source of truth for filtering, rather than relying on NPC name or active scene.
- DM end action should also clear any pending approval tied to that conversation to prevent stale approvals.
- Do not auto-merge conversations; new scene/time changes create a new id to avoid cross-thread confusion.
