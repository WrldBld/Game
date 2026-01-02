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
  - *Files*: `crates/engine-app/src/application/services/llm_queue_service.rs`

- [x] **US-DLG-002**: As a DM, I can approve LLM responses before players see them
  - *Implementation*: ApprovalRequired WebSocket message, ApprovalDecision handling
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`, `crates/player-ui/src/presentation/components/dm_panel/approval_popup.rs`

- [x] **US-DLG-003**: As a DM, I can modify LLM responses before approving
  - *Implementation*: AcceptWithModification decision type, modified dialogue text
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`

- [x] **US-DLG-004**: As a DM, I can reject and request regeneration with feedback
  - *Implementation*: Reject decision type, feedback included in retry, max 3 retries
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`

- [x] **US-DLG-005**: As a DM, I can take over and write my own response
  - *Implementation*: TakeOver decision type, DM-written dialogue
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`

- [x] **US-DLG-006**: As a DM, I can approve/reject LLM tool call suggestions
  - *Implementation*: ProposedToolInfo in approval, approved_tools filtering
  - *Files*: `crates/engine-app/src/application/services/tool_execution_service.rs`

- [x] **US-DLG-007**: As a DM, I can set directorial notes that guide the LLM
  - *Implementation*: DirectorialNotes value object, included in LLM system prompt
  - *Files*: `crates/domain/src/value_objects/directorial.rs`

- [x] **US-DLG-008**: As a player, I see a "thinking" indicator while LLM processes
  - *Implementation*: LLMProcessing WebSocket message, UI shows animated indicator
  - *Files*: `crates/player-ui/src/presentation/views/pc_view.rs`

- [x] **US-DLG-009**: As a DM, I can configure token budgets per context category
  - *Implementation*: Settings API at `/api/settings` and `/api/worlds/{world_id}/settings` exposes all 10 ContextBudgetConfig fields; metadata endpoint provides field descriptions for UI rendering
  - *Files*: `crates/domain/src/value_objects/context_budget.rs`, `crates/engine-adapters/src/infrastructure/http/settings_routes.rs`

- [x] **US-DLG-010**: As a DM, I can customize the LLM response format through configurable templates
  - *Implementation*: `PromptBuilder` resolves `dialogue.response_format`, `dialogue.challenge_suggestion_format`, and `dialogue.narrative_event_format` templates via `PromptTemplateService`
  - *Files*: `crates/engine-app/src/application/services/llm/prompt_builder.rs`, `crates/domain/src/value_objects/prompt_templates.rs`

### Implemented (Dialogue Tracking Enhancement)

- [x] **US-DLG-011**: As a system, I persist dialogue exchanges as StoryEvents for later querying
  - *Implementation*: `record_dialogue_exchange()` called from `DMApprovalQueueService` after approval
  - *Files*: `dm_approval_queue_service.rs:325-340`, `story_event_service.rs:311-359`
  - *Completed*: 2025-12-26 code review confirmed implementation

- [x] **US-DLG-012**: As a system, I can query the last dialogues with a specific NPC
  - *Implementation*: `get_dialogues_with_npc()` and `get_dialogue_summary_for_npc()` methods
  - *Files*: `repository_port.rs:1342-1347`, `story_event_repository.rs:1652-1686`
  - *Completed*: 2025-12-26 code review confirmed implementation

- [x] **US-DLG-013**: As a system, I track (PC)-[:SPOKE_TO]->(NPC) relationships with last dialogue metadata
  - *Implementation*: `update_spoke_to_edge()` creates/updates SPOKE_TO edge with `last_dialogue_at`, `last_topic`, `conversation_count`
  - *Files*: `repository_port.rs:1349-1362`, `story_event_repository.rs:1688-1718`
  - *Completed*: 2025-12-26 code review confirmed implementation

### Remaining Gaps (Dialogue Tracking)

The core functionality is implemented but has data quality gaps:

- [ ] **US-DLG-014**: Capture player dialogue text in exchange records
  - *Issue*: `player_dialogue` passed as empty string (not available in ApprovalItem)
  
- [ ] **US-DLG-015**: Extract topics from dialogue content
  - *Issue*: `topics_discussed` passed as empty vector
  
- [ ] **US-DLG-016**: Include scene/location/game_time context in records
  - *Issue*: Currently passed as None

> **Note**: Core dialogue persistence works for LLM context via `get_dialogue_summary_for_npc()` used by Staging System.

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

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `src/domain/value_objects/llm_context.rs` | Context structures |
| Domain | `src/domain/value_objects/context_budget.rs` | Budget config |
| Domain | `src/domain/value_objects/game_tools.rs` | Tool definitions |
| Domain | `src/domain/value_objects/directorial.rs` | Directorial notes |
| Application | `src/application/services/prompt_context_service.rs` | Build context |
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
