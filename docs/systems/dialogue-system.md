# Dialogue System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


The Dialogue System powers NPC conversations using an LLM (Ollama). When a player speaks to an NPC, the Engine builds rich context from the graph database (character motivations, relationships, location, narrative events) and sends it to the LLM. The generated response goes to the DM for approval before the player sees it. The LLM can also suggest tool calls (give items, change relationships) and challenge/event triggers.

Conversations are persisted as first-class graph nodes tied to scenes and game time so dialogue history can drive narrative triggers, time-based availability, and scene continuity.

---

## Game Design

This is the heart of the AI game master experience:

1. **Rich Context**: LLM receives character wants, relationships, location atmosphere, active events
2. **Token Budgets**: Each context category has a max token limit; excess is summarized
3. **DM Authority**: Every LLM response requires approval (accept/modify/reject/takeover)
4. **Tool Calls**: LLM can suggest game actions (give items, reveal info, etc.)
5. **Conversation History**: Recent dialogue is included in context

### Dialogue Rules

- **Response length** defaults to 1–3 sentences unless the DM overrides.
- **Tool calls have no side effects** until the DM approves them; rejected tools are ignored.
- **Context edges** (`IN_SCENE`, `AT_LOCATION`, `AT_REGION`, `OCCURRED_AT`) should be set when known, but conversations can exist without them.

---

## User Stories

### Implemented

- [x] **US-DLG-001**: As a player, I can speak to NPCs and receive contextual responses
  - *Implementation*: PlayerAction → queue processing → LLM client → DM approval → DialogueResponse
  - *Files*: `crates/engine/src/use_cases/queues/mod.rs`, `crates/engine/src/use_cases/conversation/start.rs`

- [x] **US-DLG-002**: As a DM, I can approve LLM responses before players see them
  - *Implementation*: ApprovalRequired WebSocket message, ApprovalDecision handling
  - *Files*: `crates/engine/src/api/websocket/mod.rs`, `crates/player/src/ui/presentation/components/dm_panel/approval_popup.rs`

- [x] **US-DLG-003**: As a DM, I can modify LLM responses before approving
  - *Implementation*: AcceptWithModification decision type, modified dialogue text
  - *Files*: `crates/engine/src/api/websocket/mod.rs`

- [x] **US-DLG-004**: As a DM, I can reject and request regeneration with feedback
  - *Implementation*: Reject decision type, feedback included in retry, max 3 retries
  - *Files*: `crates/engine/src/api/websocket/mod.rs`

- [x] **US-DLG-005**: As a DM, I can take over and write my own response
  - *Implementation*: TakeOver decision type, DM-written dialogue
  - *Files*: `crates/engine/src/api/websocket/mod.rs`

- [x] **US-DLG-006**: As a DM, I can approve/reject LLM tool call suggestions
  - *Implementation*: ProposedToolInfo in approval, approved_tools filtering
  - *Files*: `crates/engine/src/use_cases/queues/tool_extractor.rs`

- [x] **US-DLG-007**: As a DM, I can set directorial notes that guide the LLM
  - *Implementation*: DirectorialNotes value object, included in LLM system prompt
  - *Files*: `crates/domain/src/value_objects/directorial.rs`

- [x] **US-DLG-008**: As a player, I see a "thinking" indicator while LLM processes
  - *Implementation*: LLMProcessing WebSocket message, UI shows animated indicator
  - *Files*: `crates/player/src/ui/presentation/views/pc_view.rs`

- [x] **US-DLG-009**: As a DM, I can configure token budgets per context category
  - *Implementation*: Settings API at `/api/settings` and `/api/worlds/{world_id}/settings` exposes all 10 ContextBudgetConfig fields; metadata endpoint provides field descriptions for UI rendering
  - *Files*: `crates/domain/src/value_objects/context_budget.rs`, `crates/engine/src/api/http.rs`

- [x] **US-DLG-017**: As a player, I can end a conversation so that I can return to exploration without waiting for more dialogue
  - *Implementation*: Client sends `EndConversation`, WS handler ends active conversation via use case and broadcasts `ConversationEnded`; UI shows end button + confirmation modal and clears dialogue state on receipt
  - *Files*: `crates/engine/src/api/websocket/ws_conversation.rs`, `crates/engine/src/use_cases/conversation/end.rs`, `crates/player/src/ui/presentation/views/pc_view.rs`, `crates/player/src/ui/presentation/components/action_panel.rs`

### Pending

- [ ] **US-DLG-010**: As a DM, I can customize the LLM response format through configurable templates
  - *Design*: Template metadata + overrides resolved at request time
  - *Reference*: `crates/domain/src/value_objects/prompt_templates.rs`

- [ ] **US-DLG-018**: As a player, I see dialogue that is scoped to a conversation_id so I can follow the correct thread
  - *Acceptance*: Dialogue responses and choices include `conversation_id`; UI renders only turns matching the active `conversation_id`; switching scenes clears or swaps conversation state
  - *Implementation*: Add `conversation_id` to dialogue-related wire messages and client dialogue state keying
  - *Files*: `crates/shared/src/messages/dialogue.rs`, `crates/engine/src/api/websocket/ws_conversation.rs`, `crates/player/src/ui/presentation/state/dialogue_state.rs`

- [ ] **US-DLG-019**: As a DM, I can end a specific conversation by id so I can resolve stuck dialogues
  - *Acceptance*: DM can send an end request with `conversation_id`; server ends only that conversation; all participants receive `ConversationEnded` with the same id
  - *Implementation*: Add DM WebSocket command `EndConversationById` routed to conversation end use case
  - *Files*: `crates/shared/src/messages/conversation.rs`, `crates/engine/src/api/websocket/ws_conversation.rs`, `crates/engine/src/use_cases/conversation/end.rs`
  - *UI Design*: See "DM End Conversation Confirmation" mockup below

- [ ] **US-DLG-020**: As a DM, I can list active conversations so I can monitor which players are in dialogue
  - *Acceptance*: Server returns active conversations with id, participants, last_updated_at, and location/scene metadata; DM UI shows list and refreshes on updates
  - *Implementation*: Add `ListActiveConversations` query and store lookup for active conversations
  - *Files*: `crates/shared/src/messages/conversation.rs`, `crates/engine/src/use_cases/conversation/list_active.rs`, `crates/player/src/ui/presentation/components/dm_panel/conversations_list.rs`
  - *UI Design*: See "DM Active Conversations Panel" mockup below

- [ ] **US-DLG-021**: As a DM, I can view participants for a conversation so I can see who is involved
  - *Acceptance*: Conversation details include PC/NPC participants with ids and display names; participants render in the DM list and detail view
  - *Implementation*: Return participants from conversation query and map to UI view models
  - *Files*: `crates/engine/src/infrastructure/ports.rs`, `crates/engine/src/infrastructure/neo4j/conversation_repo.rs`, `crates/player/src/ui/presentation/components/dm_panel/conversation_details.rs`
  - *UI Design*: See "DM Conversation Details Panel" mockup below

- [ ] **US-DLG-022**: As a system, I track conversation `is_active` semantics consistently so state is not ambiguous
  - *Acceptance*: `is_active` is true only when `ended_at` is null; ending a conversation sets `ended_at` and flips `is_active` false; re-open creates a new conversation_id
  - *Implementation*: Update conversation persistence and end flow to enforce `ended_at` and `is_active` invariants
  - *Files*: `crates/engine/src/infrastructure/neo4j/conversation_repo.rs`, `crates/engine/src/use_cases/conversation/end.rs`
  - *UI Design*: See "DM Conversation Status Badges" mockup below

- [ ] **US-DLG-023**: As a system, I send updated message payloads so clients can rely on conversation-aware fields
  - *Acceptance*: `ApprovalRequired`, `DialogueResponse`, `ConversationEnded`, and `LLMProcessing` include `conversation_id`; payloads include `npc_id` and `pc_id` where applicable
  - *Implementation*: Extend shared message structs and update API mapping in WebSocket handlers
  - *Files*: `crates/shared/src/messages/dialogue.rs`, `crates/shared/src/messages/approvals.rs`, `crates/engine/src/api/websocket/ws_conversation.rs`, `crates/engine/src/api/websocket/mod.rs`


### Implemented (Dialogue Tracking Enhancement)

- [x] **US-DLG-011**: As a system, I persist dialogue exchanges as StoryEvents for later querying
  - *Implementation*: Narrative repositories persist dialogue exchanges from approval handler
  - *Files*: `crates/engine/src/repositories/narrative.rs`, `crates/engine/src/api/websocket/mod.rs`
  - *Completed*: 2026-01-03 implemented in simplified architecture

- [x] **US-DLG-012**: As a system, I can query the last dialogues with a specific NPC
  - *Implementation*: `get_dialogues_with_npc()` in NarrativeRepo trait
  - *Files*: `crates/engine/src/infrastructure/ports.rs`, `crates/engine/src/infrastructure/neo4j/narrative_repo.rs`
  - *Completed*: 2026-01-03 implemented in simplified architecture

- [x] **US-DLG-013**: As a system, I track (PC)-[:SPOKE_TO]->(NPC) relationships with last dialogue metadata
  - *Implementation*: `update_spoke_to()` creates/updates SPOKE_TO edge with `first_dialogue_at`, `last_dialogue_at`, `last_topic`, `conversation_count`
  - *Files*: `crates/engine/src/infrastructure/ports.rs`, `crates/engine/src/infrastructure/neo4j/narrative_repo.rs`
  - *Completed*: 2026-01-03 implemented in simplified architecture

- [x] **US-DLG-014**: Capture player dialogue text in exchange records
  - *Implementation*: `ApprovalRequestData.player_dialogue` passed through approval flow
  - *Completed*: 2026-01-03 data now captured from original player action

- [x] **US-DLG-015**: Extract topics from dialogue content
  - *Implementation*: `ApprovalRequestData.topics` passed through approval flow
  - *Completed*: 2026-01-03 topics now captured from LLM response

- [x] **US-DLG-016**: Include scene/location/game_time context in records
  - *Implementation*: `ApprovalRequestData` includes `scene_id`, `location_id`, `game_time` fields
  - *Completed*: 2026-01-03 context now available (graph edges for scene/location TBD)

> **Note**: Dialogue history is stored in Conversation + DialogueTurn nodes, with StoryEvents linked for narrative history queries. Scene and GameTime edges are part of the model.

---

## UI Mockups

### DM Approval Popup

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  LLM Response - Awaiting Approval                                    [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Player: "Can you help me find the Baron?"                                  │
│                                                                             │
│  ─── NPC Response ──────────────────────────────────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Marcus leans in, lowering his voice. "The Baron? A dangerous man    │   │
│  │ to seek. But I've heard rumors... he frequents the docks at night,  │   │
│  │ meeting with smugglers. If you're serious, I might know someone     │   │
│  │ who can help."                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Internal Reasoning (hidden from player) ───────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Marcus has information about the Baron due to his past as a         │   │
│  │ mercenary. He's willing to help because the player previously       │   │
│  │ helped him. Suggesting a contact creates a hook for the next event. │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Suggested Tools ───────────────────────────────────────────────────── │
│  ☑ RevealInfo: "Baron frequents docks at night"                            │
│  ☐ ChangeRelationship: Marcus → Player (+0.1)                              │
│                                                                             │
│  [Accept] [Modify] [Reject] [Take Over]                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Player Dialogue View

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                         [Scene Backdrop]                                    │
│                                                                             │
│                    [Marcus sprite - speaking]                               │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Marcus                                                               │   │
│  │                                                                      │   │
│  │ "The Baron? A dangerous man to seek. But I've heard rumors..."      │   │
│  │ [typewriter effect ▌]                                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐            │
│  │ "Tell me more"   │ │ "Who's the       │ │ "Never mind"     │            │
│  │                  │ │  contact?"       │ │                  │            │
│  └──────────────────┘ └──────────────────┘ └──────────────────┘            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Player End Conversation Button

```
┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────┐
│ Continue   │ │ Talk       │ │ Examine    │ │ End Conversation   │
│ [Dialogue] │ │ [Other]    │ │ [Marcus]   │ │ [× Marcus]         │
└────────────┘ └────────────┘ └────────────┘ └────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  End Conversation?                                                          │
│  End conversation with Marcus?                                             │
│  [Yes, End It]  [Keep Talking]                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### DM Active Conversations Panel

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Active Conversations (2)                                           [↻] [X]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Active ───────────────────────────────────────────────────────────   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🗣️  Aldara ↔ Eldrin                      [● Active]        [View ▼] │   │
│  │    "Discussing the ancient seal" • Turns: 7 • Last: 2 min ago        │   │
│  │    📍 The Elder's Study  • 🕐 Day 3, 14:32                            │   │
│  │                                                                      │   │
│  │    👤 Aldara (PC)     🧙 Eldrin (NPC)                               │   │
│  │    [End Conversation]                                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🗣️  Thorne ↔ Marcus                       [● Active]        [View ▼] │   │
│  │    "Asking about the smugglers" • Turns: 3 • Last: 30 sec ago        │   │
│  │    📍 The Rusty Anchor • 🕐 Day 3, 14:28                              │   │
│  │                                                                      │   │
│  │    👤 Thorne (PC)     🍺 Marcus (NPC)                               │   │
│  │    [End Conversation]                                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Pending Approval ──────────────────────────────────────────────────   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ⏳ Aldara ↔ Eldrin                         [🔍 View Approval]        │   │
│  │    Waiting for DM approval... (2 min)                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Context**: Displays in the DM panel as a monitoring component
**Interactions**:
- `[↻]` → Refresh conversation list
- `[View ▼]` → Expand to show conversation details panel
- `[End Conversation]` → Show confirmation modal to end specific conversation
- Status badges: `● Active` = ongoing, `⏳ Pending` = awaiting approval

**Status**: ⏳ Planned (US-DLG-020)

### DM Conversation Details Panel

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Conversation Details: Aldara ↔ Eldrin                              [Close] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Overview ───────────────────────────────────────────────────────────   │
│                                                                             │
│  Conversation ID: 550e8400-e29b-41d4-a716-446655440000                        │
│  Started: Day 3, 14:18 • Last Updated: Day 3, 14:32                        │
│  Topic: "Discussing the ancient seal"                                       │
│  Status: [● Active]                                                         │
│                                                                             │
│  ─── Location Context ───────────────────────────────────────────────────   │
│                                                                             │
│  📍 Location: The Elder's Study                                             │
│  🌍 Region: The Academy                                                     │
│  🎬 Scene: Eldrin's Workshop                                                 │
│                                                                             │
│  ─── Participants (2) ───────────────────────────────────────────────────   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 👤 Aldara                                                           │   │
│  │    Player Character • ID: 123e4567-e89b-12d3-a456-426614174000        │   │
│  │    Speaking: 4 turns • Last spoke: "What about the second seal?"     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🧙 Eldrin                                                           │   │
│  │    NPC • Archetype: The Sage (Campbell)                             │   │
│  │    Speaking: 3 turns • Last spoke: "Careful with those words..."     │   │
│  │    Want: Protect ancient knowledge                                  │   │
│  │    Relationship to Aldara: Wary (+0.2)                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Recent Dialogue (Last 3 turns) ─────────────────────────────────────   │
│                                                                             │
│  [14:28] Aldara: "What about the second seal?"                             │
│  [14:29] Eldrin: "Careful with those words. Not all should be heard..."   │
│  [14:32] Aldara: "I need to understand before the Baron arrives."          │
│                                                                             │
│  ─── Actions ─────────────────────────────────────────────────────────────   │
│                                                                             │
│  [End This Conversation]  [View Full History]  [Jump to Approval Queue]   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Context**: Expanded view when user clicks "View" on a conversation card
**Interactions**:
- `[Close]` → Collapse back to list view
- `[End This Conversation]` → Show confirmation modal (US-DLG-019)
- `[View Full History]` → Open conversation history modal with all turns
- `[Jump to Approval Queue]` → If this conversation has pending approval, jump there

**Status**: ⏳ Planned (US-DLG-021)

### DM End Conversation Confirmation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  End Conversation                                                [X] Cancel │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Conversation to End ─────────────────────────────────────────────────   │
│                                                                             │
│  🗣️  Aldara ↔ Eldrin                                                       │
│                                                                             │
│  ─── Details ─────────────────────────────────────────────────────────────   │
│                                                                             │
│  Started: Day 3, 14:18                                                      │
│  Duration: 14 minutes                                                       │
│  Total turns: 7                                                             │
│  Location: The Elder's Study                                                │
│                                                                             │
│  ─── Participants ─────────────────────────────────────────────────────────   │
│                                                                             │
│  👤 Aldara (Player Character)                                                │
│  🧙 Eldrin (NPC - The Sage)                                                 │
│                                                                             │
│  ─── Warning ─────────────────────────────────────────────────────────────   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ⚠️  This will end the conversation and clear any pending dialogue     │   │
│  │    approvals. Players will receive a "Conversation Ended" message.    │   │
│  │    This action cannot be undone.                                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  [Cancel]  [End Conversation]                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Context**: Modal shown when DM clicks "End Conversation" on any conversation
**Interactions**:
- `[Cancel]` → Dismiss modal without ending
- `[End Conversation]` → Send `EndConversationById` request to server

**Status**: ⏳ Planned (US-DLG-019)

### DM Conversation Status Badges

| Badge | State | Meaning | Color |
|-------|-------|---------|-------|
| `● Active` | Active | Conversation is ongoing | Green |
| `⏳ Pending` | Pending | Awaiting DM approval | Yellow/Orange |
| `✓ Ended` | Ended | Conversation completed normally | Gray |
| `✗ Forced` | Forced | Ended by DM (not by player) | Red |

**Context**: Used throughout the DM panel to indicate conversation state
**Usage**:
- Appears in the Active Conversations panel
- Appears in the Details panel header
- Appears in the approval queue when a conversation is pending

**Status**: ⏳ Planned (US-DLG-022)

## UI Design: DM Conversation Management

### User Story Mapping

| User Story | UI Component | Status |
|------------|--------------|--------|
| US-DLG-019: DM ends conversation by id | End Conversation Confirmation Modal | ⏳ Planned |
| US-DLG-020: DM lists active conversations | Active Conversations Panel | ⏳ Planned |
| US-DLG-021: DM views conversation participants | Conversation Details Panel | ⏳ Planned |

### User Flows

#### Flow 1: Monitor Active Conversations

```
[DM Panel View] → [Click "Active Conversations" tab] → [Active Conversations Panel displays]
     ↓
[Panel refreshes automatically] → [New conversation starts] → [New card appears with animation]
     ↓
[Click "View ▼" on conversation] → [Conversation Details Panel expands]
```

#### Flow 2: End Conversation by ID

```
[Active Conversations Panel] → [Click "End Conversation" on card] → [Confirmation Modal appears]
     ↓
[Review details in modal] → [Click "End Conversation"] → [Request sent to server]
     ↓
[Server processes] → [ConversationEnded broadcast] → [Card removed with fade animation]
     ↓
[All participants see notification] → [Conversation marked as ended in database]
```

#### Flow 3: View Conversation Participants

```
[Active Conversations Panel] → [Click "View ▼" on conversation] → [Details Panel expands]
     ↓
[Scroll to Participants section] → [View participant cards with metadata]
     ↓
[Click NPC participant] → [Open NPC details modal] (optional enhancement)
     ↓
[Click PC participant] → [Open character sheet] (optional enhancement)
```

### Component Breakdown

| Component | Purpose | Existing? | Location |
|-----------|---------|-----------|----------|
| `ActiveConversationsPanel` | Lists all active conversations with status | New | `dm_panel/active_conversations.rs` |
| `ConversationCard` | Individual conversation summary row | New | Within panel |
| `ConversationDetailsPanel` | Expanded view with participants and history | New | `dm_panel/conversation_details.rs` |
| `ParticipantCard` | Shows PC or NPC with relationship data | New | Within details |
| `EndConversationModal` | Confirmation dialog for ending conversations | New | `dm_panel/end_conversation_modal.rs` |
| `ConversationStatusBadge` | Visual status indicator | New | Shared component |

### Data Flow

```
WebSocket: ListActiveConversations (request)
         → UseCase: ListActiveConversations
         → Repo: Query active conversations
         → Response: List<ConversationInfo>
         → UI: Update ActiveConversationsPanel state

WebSocket: EndConversationById (request)
         → UseCase: EndConversation
         → Repo: Update conversation (set ended_at, is_active=false)
         → Broadcast: ConversationEnded to all participants
         → UI: Remove card from panel, show toast confirmation
```

### Message Protocol Extensions

#### Server → Client

```rust
// Active conversations list
pub struct ConversationInfo {
    pub conversation_id: Uuid,
    pub topic_hint: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub participants: Vec<ConversationParticipant>,
    pub location: Option<LocationContext>,
    pub scene: Option<SceneContext>,
    pub game_time: GameTime,
    pub turn_count: u32,
    pub pending_approval: bool,  // true if awaiting DM approval
}

pub struct ConversationParticipant {
    pub id: Uuid,
    pub name: String,
    pub participant_type: ParticipantType,  // PC or NPC
    pub turn_count: u32,
    pub last_spoke_at: Option<DateTime<Utc>>,
    pub relationship_to_others: Option<RelationshipSummary>,  // for NPCs
}

pub struct LocationContext {
    pub location_id: Uuid,
    pub location_name: String,
    pub region_name: String,
}

pub struct SceneContext {
    pub scene_id: Uuid,
    pub scene_name: String,
}
```

#### Client → Server

```rust
// List active conversations
pub struct ListActiveConversations {
    pub world_id: Uuid,
    pub include_ended: bool,  // optional: show recent ended convos too
}

// End conversation by ID (DM only)
pub struct EndConversationById {
    pub conversation_id: Uuid,
    pub reason: Option<String>,  // optional DM note
}
```

### Animation Notes

- **New conversation card**: Slide in from right with fade (300ms)
- **Conversation ended**: Fade out + slide up (300ms) then remove
- **Panel expand**: Accordion animation with height transition (200ms)
- **Status badge pulse**: Yellow badge pulses while pending approval
- **Toast notification**: Slide up from bottom right, auto-dismiss after 3s

### Accessibility Considerations

- **Keyboard navigation**:
  - Tab through conversation cards
  - Enter/Space to expand details
  - Escape to close modals
- **Screen reader support**:
  - Cards announced with conversation ID, participant count, status
  - Status badges use `aria-label` (e.g., "Active conversation")
  - Modal traps focus when open
- **Color contrast**:
  - Active: Green (passes WCAG AA)
  - Pending: Yellow/Orange (passes WCAG AA)
  - Ended: Gray (neutral)
  - Forced: Red (passes WCAG AA)

### Error States

#### End Conversation Failed
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Error                                                              [X]      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Failed to end conversation: "Aldara ↔ Eldrin"                               │
│                                                                             │
│  Reason: Conversation not found or already ended                           │
│                                                                             │
│  [Close]  [Retry]                                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### List Active Conversations Failed
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Active Conversations                                              [↻] [X] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ⚠️  Failed to load conversations. Click ↻ to retry.                       │
│                                                                             │
│  Error: Network timeout when fetching data                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Empty States

#### No Active Conversations

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Active Conversations (0)                                           [↻] [X]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                             │
│  │     💬                                                                      │
│  │                                                                             │
│  │     No active conversations                                                │
│  │                                                                             │
│  │     Players will start conversations when they interact with NPCs.       │
│  │                                                                             │
│  │     All conversations are logged and searchable in the game history.      │
│  │                                                                             │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Conversation Details - Empty History

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Conversation Details: Aldara ↔ Eldrin                              [Close] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Overview ───────────────────────────────────────────────────────────   │
│  ...(same as before)...                                                    │
│                                                                             │
│  ─── Recent Dialogue ─────────────────────────────────────────────────────   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 💬 No dialogue turns recorded yet                                   │   │
│  │    This conversation was just started. The first dialogue will      │   │
│  │    appear here after the first player-NPC exchange.                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Edge Case: Multiple Conversations with Same Participants

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Active Conversations (3)                                           [↻] [X]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🗣️  Aldara ↔ Eldrin                      [● Active]        [View ▼] │   │
│  │    "Discussing the ancient seal" • Turns: 7 • Session #2          │   │
│  │    📍 The Elder's Study  • 🕐 Day 3, 14:32                            │   │
│  │                                                                      │   │
│  │    👤 Aldara (PC)     🧙 Eldrin (NPC)                               │   │
│  │    [End Conversation]                                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🗣️  Aldara ↔ Eldrin                      [✓ Ended]        [View ▼] │   │
│  │    "Asking about the seal's history" • Turns: 5 • Session #1       │   │
│  │    📍 The Elder's Study  • 🕐 Day 3, 13:15                            │   │
│  │    Duration: 12 minutes • Ended naturally                            │   │
│  │                                                                      │   │
│  │    👤 Aldara (PC)     🧙 Eldrin (NPC)                               │   │
│  │    [View History]                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Note**: When the same participants have multiple conversations (due to scene breaks, time gaps, or restarts), each is shown as a separate card with session numbering for clarity.

### Toast Notifications

#### Conversation Ended Successfully

```
┌────────────────────────────────────────────┐
│  ✓  Conversation ended                      │
│     "Aldara ↔ Eldrin" was ended            │
│                                             │
│     [Dismiss]                               │
└────────────────────────────────────────────┘
```
*Appears: Top-right, auto-dismiss after 3s*

#### New Conversation Started

```
┌────────────────────────────────────────────┐
│  💬  New conversation started               │
│     "Thorne ↔ Marcus"                       │
│                                             │
│     [View] [Dismiss]                        │
└────────────────────────────────────────────┘
```
*Appears: Top-right, auto-dismiss after 5s, [View] jumps to conversations panel*

#### End Conversation Failed

```
┌────────────────────────────────────────────┐
│  ✗  Failed to end conversation             │
│     Conversation not found                 │
│                                             │
│     [Retry] [Dismiss]                       │
└────────────────────────────────────────────┘
```
*Appears: Top-right, persists until user dismisses*

### Compact View for DM Dashboard

When space is constrained (e.g., embedded in main DM panel), a compact list view is available:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Conversations (2)                                            [+ End All]   │
├─────────────────────────────────────────────────────────────────────────────┤
│  🗣️ Aldara↔Eldrin • ● Active • 7 turns • The Elder's Study  [End] [×]   │
│  🗣️ Thorne↔Marcus • ⏳ Pending • 3 turns • The Rusty Anchor  [End] [×]   │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Interactions**:
- `[+ End All]` → Shows modal to end all active conversations at once
- `[End]` → Quick end button on each row
- `[×]` → Click row to expand details
- Click participant names → Filter to show only that user's conversations

### Responsive Considerations

| Breakpoint | Layout |
|------------|--------|
| Desktop (1280px+) | Full panel with cards showing all details side-by-side |
| Tablet (768-1279px) | Stacked cards, details expand in modal overlay |
| Mobile (<768px) | Compact list view, tap to show details modal |

---

## Data Model

### Conversation + DialogueTurn Nodes

```cypher
(:Conversation {
    id: "uuid",
    started_at: datetime(),
    ended_at: datetime(),
    topic_hint: "Baron's whereabouts",
    is_active: true,
    last_updated_at: datetime()
})

(:DialogueTurn {
    id: "uuid",
    speaker_id: "uuid",
    speaker_type: "pc|npc",
    text: "The Baron? A dangerous man...",
    order: 3,
    is_dm_override: false,
    is_llm_generated: true,
    game_time: datetime()
})
```

### Conversation Context Edges

```cypher
(pc:PlayerCharacter)-[:PARTICIPATED_IN]->(conversation:Conversation)
(npc:Character)-[:PARTICIPATED_IN]->(conversation:Conversation)
(conversation)-[:IN_SCENE]->(scene:Scene)
(conversation)-[:AT_LOCATION]->(location:Location)
(conversation)-[:AT_REGION]->(region:Region)
(conversation)-[:HAS_TURN {order: 3}]->(turn:DialogueTurn)
(turn)-[:OCCURRED_AT]->(time:GameTime)
```

### Context Categories

| Category | Description | Default Max Tokens |
|----------|-------------|-------------------|
| `scene` | Location, time, atmosphere, present characters | 500 |
| `npc_identity` | Name, description, archetype, behaviors | 400 |
| `npc_actantial` | Wants, helpers, opponents, senders, receivers | 800 |
| `npc_locations` | Where NPC lives, works, frequents | 300 |
| `npc_relationships` | Sentiment toward PC and others in scene | 500 |
| `narrative_events` | Active events and their trigger conditions | 600 |
| `challenges` | Active challenges at this location | 400 |
| `conversation` | Recent dialogue turns | 1500 |
| `directorial` | DM's guidance, tone, forbidden topics | 400 |

### Context Budget Configuration

```rust
pub struct ContextBudgetConfig {
    pub scene_max_tokens: u32,
    pub npc_identity_max_tokens: u32,
    pub npc_actantial_max_tokens: u32,
    pub npc_locations_max_tokens: u32,
    pub npc_relationships_max_tokens: u32,
    pub narrative_events_max_tokens: u32,
    pub challenges_max_tokens: u32,
    pub conversation_max_tokens: u32,
    pub directorial_max_tokens: u32,
    pub total_max_tokens: u32,
}
```

### Tool Types

```rust
pub enum GameTool {
    GiveItem { item_id: String, quantity: u32 },
    TakeItem { item_id: String, quantity: u32 },
    RevealInfo { info: String, importance: InfoImportance },
    ChangeRelationship { target_id: String, change: RelationshipChange },
    TriggerEvent { event_id: String },
    SetFlag { flag_name: String, value: bool },
    ModifyStat { stat_name: String, amount: i32 },
    UnlockLocation { location_id: String },
    SpawnNpc { npc_id: String },
    EndConversation,
    Custom { action: String },
}
```

---

## API

### WebSocket Messages

#### Client → Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `PlayerAction` | `action_type`, `target`, `content` | Player speaks/acts |
| `ApprovalDecision` | `decision`, `modified_dialogue`, `approved_tools`, `feedback` | DM approves |
| `DirectorialUpdate` | `notes` | DM sets guidance |
| `ListActiveConversations` | `world_id`, `include_ended` | DM requests conversation list (US-DLG-020) |
| `EndConversationById` | `conversation_id`, `reason` | DM ends specific conversation (US-DLG-019) |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `LLMProcessing` | `npc_name` | "Thinking" indicator |
| `ApprovalRequired` | `dialogue`, `reasoning`, `tools`, `challenge_suggestion`, `event_suggestion` | DM approval needed |
| `DialogueResponse` | `npc_name`, `dialogue`, `choices` | Approved response |
| `ResponseApproved` | `action_id` | Confirmation |
| `ActiveConversationsList` | `conversations: Vec<ConversationInfo>` | Returns active conversations list (US-DLG-020) |
| `ConversationEnded` | `conversation_id`, `ended_by`, `reason` | Broadcast when conversation ends (US-DLG-019) |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| PromptContextService | ✅ | - | Graph-based context building |
| ContextBudgetConfig | ✅ | - | Token limits per category |
| Token Counter | ✅ | - | Multiple counting methods |
| Summarization | ✅ | - | Auto-summarize when over budget |
| Prompt Builder | ✅ | - | System prompt with all categories |
| NPC Mood in Prompts | ✅ | - | Wired in build_prompt_from_action (2025-12-26) |
| Actantial Context | ✅ | - | Wired in build_prompt_from_action (2025-12-26) |
| Featured NPC Names | ✅ | - | Included in narrative event context (2025-12-26) |
| Tool Parsing | ✅ | - | Parse LLM tool suggestions |
| Tool Execution | ✅ | - | Execute approved tools |
| DM Approval Flow | ✅ | ✅ | Full approval UI |
| Conversation History | ✅ | ✅ | 30-turn limit (in-memory) |
| Dialogue Persistence | ✅ | - | `record_dialogue_exchange()` creates StoryEvent::DialogueExchange |
| NPC Dialogue Queries | ✅ | - | `get_dialogues_with_npc()`, `get_dialogue_summary_for_npc()` |
| SPOKE_TO Edges | ✅ | - | `update_spoke_to_edge()` tracks PC-NPC metadata |
| Directorial Notes | ✅ | ✅ | DM guidance |
| Dialogue Display | - | ✅ | Typewriter effect |
| Choice Selection | - | ✅ | Player choices |
| Region Items in Context | ⏳ | - | Hardcoded to empty; needs WorldStateManager |
| Active Conversations List | - | ⏳ | DM conversation monitoring (US-DLG-020) |
| Conversation Details Panel | - | ⏳ | Participant view with metadata (US-DLG-021) |
| End Conversation by ID | ⏳ | ⏳ | DM force-end conversations (US-DLG-019) |
| Conversation Status Badges | - | ⏳ | Visual state indicators (US-DLG-022) |
| Conversation ID Scoping | ⏳ | ⏳ | Dialogue linked to conversation_id (US-DLG-018) |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Engine | `crates/engine/src/llm_context.rs` | Context structures |
| Domain | `crates/domain/src/value_objects/context_budget.rs` | Budget config |
| Domain | `crates/domain/src/value_objects/game_tools.rs` | Tool definitions |
| Domain | `crates/domain/src/value_objects/directorial.rs` | Directorial notes |
| Infrastructure | `crates/engine/src/infrastructure/ollama.rs` | LLM operations |
| Use Case | `crates/engine/src/use_cases/queues/mod.rs` | LLM request assembly + processing |
| Use Case | `crates/engine/src/use_cases/queues/tool_builder.rs` | Tool definition assembly |
| Use Case | `crates/engine/src/use_cases/queues/tool_extractor.rs` | Tool extraction + routing |
| Infrastructure | `crates/engine/src/infrastructure/ollama.rs` | LLM client |
| API | `crates/engine/src/api/websocket/mod.rs` | Approval handling |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Presentation | `src/presentation/components/dm_panel/approval_popup.rs` | Approval UI |
| Presentation | `src/presentation/components/dm_panel/active_conversations.rs` | Active conversations list (US-DLG-020) |
| Presentation | `src/presentation/components/dm_panel/conversation_details.rs` | Conversation details with participants (US-DLG-021) |
| Presentation | `src/presentation/components/dm_panel/end_conversation_modal.rs` | End conversation confirmation (US-DLG-019) |
| Presentation | `src/presentation/components/visual_novel/dialogue_box.rs` | Dialogue display |
| Presentation | `src/presentation/components/visual_novel/choice_menu.rs` | Player choices |
| Presentation | `src/presentation/components/dm_panel/directorial_notes.rs` | DM notes input |
| Presentation | `src/presentation/state/dialogue_state.rs` | Dialogue state |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (NPC context), [Navigation System](./navigation-system.md) (location context), [Challenge System](./challenge-system.md) (challenge suggestions), [Narrative System](./narrative-system.md) (event suggestions), [Prompt Template System](./prompt-template-system.md) (configurable response format)
- **Provides data to**: [Staging System](./staging-system.md) (dialogue history for LLM context in presence decisions)
- **Used by**: [Scene System](./scene-system.md) (dialogue in scenes)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-18 | Initial version extracted from MVP.md |
| 2025-12-19 | Added dialogue tracking enhancements for Staging System |
| 2025-12-26 | Marked NPC mood, actantial context, and featured NPC names as implemented |
| 2025-12-26 | Code review: US-DLG-011/012/013 confirmed as IMPLEMENTED |
| 2025-12-26 | Added US-DLG-014/015/016 for remaining data quality gaps |
| 2026-01-23 | Added DM Conversation Management UI design (US-DLG-019/020/021/022) |
