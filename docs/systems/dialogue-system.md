# Dialogue System

## Overview

The Dialogue System powers NPC conversations using an LLM (Ollama). When a player speaks to an NPC, the Engine builds rich context from the graph database (character motivations, relationships, location, narrative events) and sends it to the LLM. The generated response goes to the DM for approval before the player sees it. The LLM can also suggest tool calls (give items, change relationships) and challenge/event triggers.

---

## Game Design

This is the heart of the AI game master experience:

1. **Rich Context**: LLM receives character wants, relationships, location atmosphere, active events
2. **Token Budgets**: Each context category has a max token limit; excess is summarized
3. **DM Authority**: Every LLM response requires approval (accept/modify/reject/takeover)
4. **Tool Calls**: LLM can suggest game actions (give items, reveal info, etc.)
5. **Conversation History**: Recent dialogue is included in context

---

## User Stories

### Implemented

- [x] **US-DLG-001**: As a player, I can speak to NPCs and receive contextual responses
  - *Implementation*: PlayerAction → LLMQueueService → Ollama → DM approval → DialogueResponse
  - *Files*: `Engine/src/application/services/llm_queue_service.rs`

- [x] **US-DLG-002**: As a DM, I can approve LLM responses before players see them
  - *Implementation*: ApprovalRequired WebSocket message, ApprovalDecision handling
  - *Files*: `Engine/src/infrastructure/websocket.rs`, `Player/src/presentation/components/dm_panel/approval_popup.rs`

- [x] **US-DLG-003**: As a DM, I can modify LLM responses before approving
  - *Implementation*: AcceptWithModification decision type, modified dialogue text
  - *Files*: `Engine/src/infrastructure/websocket.rs`

- [x] **US-DLG-004**: As a DM, I can reject and request regeneration with feedback
  - *Implementation*: Reject decision type, feedback included in retry, max 3 retries
  - *Files*: `Engine/src/infrastructure/websocket.rs`

- [x] **US-DLG-005**: As a DM, I can take over and write my own response
  - *Implementation*: TakeOver decision type, DM-written dialogue
  - *Files*: `Engine/src/infrastructure/websocket.rs`

- [x] **US-DLG-006**: As a DM, I can approve/reject LLM tool call suggestions
  - *Implementation*: ProposedToolInfo in approval, approved_tools filtering
  - *Files*: `Engine/src/application/services/tool_execution_service.rs`

- [x] **US-DLG-007**: As a DM, I can set directorial notes that guide the LLM
  - *Implementation*: DirectorialNotes value object, included in LLM system prompt
  - *Files*: `Engine/src/domain/value_objects/directorial.rs`

- [x] **US-DLG-008**: As a player, I see a "thinking" indicator while LLM processes
  - *Implementation*: LLMProcessing WebSocket message, UI shows animated indicator
  - *Files*: `Player/src/presentation/views/pc_view.rs`

- [x] **US-DLG-009**: As a DM, I can configure token budgets per context category
  - *Implementation*: Settings API at `/api/settings` and `/api/worlds/{world_id}/settings` exposes all 10 ContextBudgetConfig fields; metadata endpoint provides field descriptions for UI rendering
  - *Files*: `Engine/src/domain/value_objects/context_budget.rs`, `Engine/src/infrastructure/http/settings_routes.rs`

### Pending (Dialogue Tracking Enhancement)

- [ ] **US-DLG-010**: As a system, I persist dialogue exchanges as StoryEvents for later querying
  - *Implementation*: Call `record_dialogue_exchange` in DMApprovalQueueService after approval
  - *Files*: `Engine/src/application/services/dm_approval_queue_service.rs`, `Engine/src/application/services/story_event_service.rs`

- [ ] **US-DLG-011**: As a system, I can query the last dialogues with a specific NPC
  - *Implementation*: Add `get_dialogues_with_npc` method to StoryEventRepository
  - *Files*: `Engine/src/application/ports/outbound/story_event_repository_port.rs`, `Engine/src/infrastructure/persistence/story_event_repository.rs`

- [ ] **US-DLG-012**: As a system, I track (PC)-[:SPOKE_TO]->(NPC) relationships with last dialogue metadata
  - *Implementation*: SPOKE_TO edge with `last_dialogue_at`, `last_topic`, `conversation_count`
  - *Files*: `Engine/src/infrastructure/persistence/story_event_repository.rs`

> **Note**: These dialogue tracking enhancements are required by the [Staging System](./staging-system.md) to provide LLM context about recent NPC interactions (e.g., if an NPC said they'd be somewhere else).

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

---

## Data Model

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

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `LLMProcessing` | `npc_name` | "Thinking" indicator |
| `ApprovalRequired` | `dialogue`, `reasoning`, `tools`, `challenge_suggestion`, `event_suggestion` | DM approval needed |
| `DialogueResponse` | `npc_name`, `dialogue`, `choices` | Approved response |
| `ResponseApproved` | `action_id` | Confirmation |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| LLMContextService | ✅ | - | Graph-based context building |
| ContextBudgetConfig | ✅ | - | Token limits per category |
| Token Counter | ✅ | - | Multiple counting methods |
| Summarization | ✅ | - | Auto-summarize when over budget |
| Prompt Builder | ✅ | - | System prompt with all categories |
| Tool Parsing | ✅ | - | Parse LLM tool suggestions |
| Tool Execution | ✅ | - | Execute approved tools |
| DM Approval Flow | ✅ | ✅ | Full approval UI |
| Conversation History | ✅ | ✅ | 30-turn limit (in-memory) |
| Dialogue Persistence | ⏳ | - | Store as StoryEvent::DialogueExchange |
| NPC Dialogue Queries | ⏳ | - | get_dialogues_with_npc method |
| SPOKE_TO Edges | ⏳ | - | Track PC-NPC conversation metadata |
| Directorial Notes | ✅ | ✅ | DM guidance |
| Dialogue Display | - | ✅ | Typewriter effect |
| Choice Selection | - | ✅ | Player choices |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `src/domain/value_objects/llm_context.rs` | Context structures |
| Domain | `src/domain/value_objects/context_budget.rs` | Budget config |
| Domain | `src/domain/value_objects/game_tools.rs` | Tool definitions |
| Domain | `src/domain/value_objects/directorial.rs` | Directorial notes |
| Application | `src/application/services/llm_context_service.rs` | Build context |
| Application | `src/application/services/llm/prompt_builder.rs` | Build prompts |
| Application | `src/application/services/llm_queue_service.rs` | LLM processing |
| Application | `src/application/services/tool_execution_service.rs` | Execute tools |
| Infrastructure | `src/infrastructure/ollama.rs` | LLM client |
| Infrastructure | `src/infrastructure/websocket.rs` | Approval handling |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Presentation | `src/presentation/components/dm_panel/approval_popup.rs` | Approval UI |
| Presentation | `src/presentation/components/visual_novel/dialogue_box.rs` | Dialogue display |
| Presentation | `src/presentation/components/visual_novel/choice_menu.rs` | Player choices |
| Presentation | `src/presentation/components/dm_panel/directorial_notes.rs` | DM notes input |
| Presentation | `src/presentation/state/dialogue_state.rs` | Dialogue state |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (NPC context), [Navigation System](./navigation-system.md) (location context), [Challenge System](./challenge-system.md) (challenge suggestions), [Narrative System](./narrative-system.md) (event suggestions)
- **Provides data to**: [Staging System](./staging-system.md) (dialogue history for LLM context in presence decisions)
- **Used by**: [Scene System](./scene-system.md) (dialogue in scenes)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-18 | Initial version extracted from MVP.md |
| 2025-12-19 | Added dialogue tracking enhancements for Staging System |
