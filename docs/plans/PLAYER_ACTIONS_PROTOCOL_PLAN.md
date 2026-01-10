# Player Actions Protocol Plan (Checklist)

## Goal

Unify player actions under a comprehensive protocol that supports:
1. **Predefined actions** (dialogue, movement, inventory) with deterministic routing
2. **Intent-based actions** (verb + intent + target) with LLM resolution
3. **Travel with intent** (stealth, haste, objectives) that can trigger challenges
4. **DM-generated challenges** from natural language descriptions

All actions flow through DM approval where appropriate, maintaining narrative authority.

---

## Conversation System Architecture (Deep Dive)

### Conversations ARE First-Class Neo4j Objects

The system already has rich conversation infrastructure that must be properly integrated:

**Neo4j Nodes:**
```cypher
(:Conversation {
    id: UUID,
    world_id: UUID,
    started_at: DateTime,
    ended_at: DateTime,        // Empty if active
    topic_hint: String,        // First topic discussed
    is_active: Boolean,
    last_updated_at: DateTime
})

(:DialogueTurn {
    id: UUID,
    conversation_id: UUID,
    speaker_id: UUID,          // PC or NPC ID
    speaker_type: "pc"|"npc",
    text: String,
    order: Integer,            // Sequence in conversation
    is_dm_override: Boolean,
    is_llm_generated: Boolean,
    game_time: DateTime
})
```

**Graph Relationships:**
```
World -[HAS_CONVERSATION]-> Conversation
PlayerCharacter -[PARTICIPATED_IN]-> Conversation
Character(NPC) -[PARTICIPATED_IN]-> Conversation
Conversation -[HAS_TURN {order}]-> DialogueTurn
Conversation -[IN_SCENE]-> Scene (optional)
Conversation -[AT_LOCATION]-> Location (optional)
Conversation -[AT_REGION]-> Region (optional)
Conversation -[OCCURRED_AT]-> GameTime
DialogueTurn -[OCCURRED_AT]-> GameTime
DialogueTurn -[OCCURRED_IN_SCENE]-> Scene

// Relationship metadata (quick lookup without scanning events)
PlayerCharacter -[SPOKE_TO {
    first_dialogue_at: DateTime,
    last_dialogue_at: DateTime,
    last_topic: String,
    conversation_count: Integer
}]-> Character
```

### Use Case Returns Conversation Context

The `StartConversation` use case already returns:
```rust
pub struct ConversationStarted {
    pub conversation_id: Uuid,      // ← This should be exposed in protocol
    pub action_queue_id: Uuid,
    pub npc_name: String,
    pub npc_disposition: Option<String>,
}
```

### Three-Tier Emotional Model

| Tier | Name | Storage | Scope | Example |
|------|------|---------|-------|---------|
| 1 | Disposition | CHARACTER_HAS_DISPOSITION | Per PC relationship | Friendly, Suspicious, Hostile |
| 2 | Mood | INCLUDES_NPC edge (Staging) | Per region, temporary | Anxious, Calm, Excited |
| 3 | Expression | Inline dialogue markers | Per turn, transient | `*happy*`, `*sighs\|sad*` |

**Critical**: NPC mood (Tier 2) is stored on the **staging relationship**, not the character.

### Current Gaps

| Gap | Issue | Impact |
|-----|-------|--------|
| `conversation_history` empty | Field exists in `GamePromptRequest` but never populated | LLM has no memory of prior turns |
| `conversation_id` not in protocol | Use case returns it, but StartConversation response doesn't include it | Client can't track conversations |
| No Conversation CRUD in protocol | Can't query, list, or get conversation details | Client can't display conversation history |
| Intent actions bypass conversation | If player "examines NPC" instead of "talks to", no conversation context | Lost narrative continuity |

---

## Current State (Code Review 2026-01-09)

### What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| **Conversation System** | ✅ Rich | Neo4j nodes, relationships, use cases |
| - Conversation entity | ✅ | First-class Neo4j node |
| - DialogueTurn entity | ✅ | Linked to Conversation, ordered |
| - SPOKE_TO relationship | ✅ | Metadata tracking per PC-NPC pair |
| - record_dialogue_exchange | ✅ | Persists StoryEvent + updates relationships |
| **ClientMessage variants** | ✅ Exists | `crates/protocol/src/messages.rs` |
| - StartConversation | ✅ | Returns action_queue_id (not conversation_id) |
| - ContinueConversation | ✅ | No conversation_id parameter |
| - ConversationEnded | ✅ | Server → Player notification |
| **WebSocket handlers** | ✅ Exists | `crates/engine/src/api/websocket/` |
| - ws_conversation.rs | ✅ | Handles Start/Continue/End |
| - ws_movement.rs | ✅ | MoveToRegion, ExitToLocation |
| - ws_inventory.rs | ✅ | Inventory actions |
| **Use cases** | ✅ Exists | Full conversation lifecycle |
| - StartConversation | ✅ | Returns ConversationStarted with conversation_id |
| - ContinueConversation | ✅ | Validates NPC still staged |
| - EndConversation | ✅ | Marks conversation inactive |
| **LLM Context** | ⚠️ Partial | Infrastructure ready, not wired |
| - GamePromptRequest.conversation_history | ⚠️ | Empty (never populated) |
| - ContextBudgetConfig | ✅ | Token limits defined |
| - TokenCounter | ✅ | Multiple counting methods |
| - Budget enforcement | ❌ | Not wired into prompt building |
| **Approval flow** | ✅ Works | DM gates all LLM responses |
| **StoryEvent persistence** | ✅ Works | DialogueExchange, SPOKE_TO updates |

### What's Missing

| Component | Status | Notes |
|-----------|--------|-------|
| `PlayerRequest` enum | ❌ Missing | No unified player request grouping |
| `conversation_id` in responses | ❌ Missing | Not exposed to client |
| `conversation_id` in ContinueConversation | ❌ Missing | Client can't reference specific conversation |
| Conversation CRUD requests | ❌ Missing | GetConversation, ListConversations |
| Intent action types | ❌ Missing | ActionVerb, ActionTarget, IntentAction |
| Action resolution outcomes | ❌ Missing | SuggestChallenge, RevealInformation |
| Travel intent | ❌ Missing | TravelApproach, TravelIntent |
| Challenge generation from description | ❌ Missing | GenerateChallengeFromDescription |
| Conversation history in LLM context | ❌ Missing | conversation_history always empty |
| Action history in LLM context | ❌ Missing | Recent player actions not fed to LLM |

### Architectural Recommendations

1. **Expose conversation_id**: Return in StartConversation response, accept in ContinueConversation
2. **Populate conversation_history**: Query DialogueTurn nodes before LLM calls
3. **Reuse approval flow**: Extend existing `ApprovalRequired` for intent action resolutions
4. **Reuse queue system**: Tag existing queue entries with action type
5. **Extend templates**: Add `ActionResolution` and `ChallengeGeneration` template categories
6. **Intent actions with NPC target**: Route to conversation system with context

---

## Phase 1: Protocol + Engine Routing (Predefined Actions)

**Goal**: Create unified `PlayerRequest` enum with proper conversation support.

### 1.1 Add PlayerRequest Module
- [ ] Create `crates/protocol/src/requests/player.rs`
- [ ] Add `PlayerRequest` enum with conversation-aware variants:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  pub enum PlayerRequest {
      // Conversation actions - now with conversation_id support
      StartConversation {
          npc_id: String,
          message: String,
      },
      ContinueConversation {
          npc_id: String,
          message: String,
          /// Optional - if provided, validates against active conversation
          #[serde(default)]
          conversation_id: Option<String>,
      },
      EndConversation {
          npc_id: String,
          #[serde(default)]
          conversation_id: Option<String>,
      },

      // Interaction actions
      PerformInteraction { interaction_id: String },

      // Movement actions
      MoveToRegion { region_id: String },
      ExitToLocation {
          location_id: String,
          arrival_region_id: Option<String>,
      },

      // Inventory actions
      EquipItem { item_id: String },
      UnequipItem { item_id: String },
      DropItem { item_id: String, quantity: Option<u32> },
      PickupItem { item_id: String },
      UseItem { item_id: String, target_id: Option<String> },

      // Conversation queries
      GetConversation { conversation_id: String },
      GetConversationHistory {
          npc_id: String,
          #[serde(default)]
          limit: Option<u32>,
      },
  }
  ```

### 1.2 Add Conversation Response Types
- [ ] Add `ConversationStartedResponse`:
  ```rust
  pub struct ConversationStartedResponse {
      pub conversation_id: String,
      pub npc_name: String,
      pub npc_disposition: Option<String>,
      pub npc_mood: Option<String>,
  }
  ```

- [ ] Add `ConversationData` for queries:
  ```rust
  pub struct ConversationData {
      pub id: String,
      pub npc_id: String,
      pub npc_name: String,
      pub started_at: String,
      pub is_active: bool,
      pub topic_hint: Option<String>,
      pub turn_count: u32,
  }

  pub struct DialogueTurnData {
      pub speaker_type: String,  // "pc" or "npc"
      pub speaker_name: String,
      pub text: String,
      pub game_time: Option<String>,
  }
  ```

### 1.3 Register in RequestPayload
- [ ] Add `pub mod player;` to `crates/protocol/src/requests.rs`
- [ ] Add `Player(player::PlayerRequest)` variant to `RequestPayload` enum

### 1.4 Add WebSocket Handler
- [ ] Create `crates/engine/src/api/websocket/ws_player_request.rs`
- [ ] Implement `handle_player_request()` that delegates to existing handlers
- [ ] Route `RequestPayload::Player` in `mod.rs` dispatch

### 1.5 Update ServerMessage for Conversation
- [ ] Update `DialogueResponse` to include conversation context:
  ```rust
  DialogueResponse {
      conversation_id: String,  // NEW
      speaker_id: String,
      speaker_name: String,
      text: String,
      mood: Option<String>,     // NEW - Tier 2 mood
      expression: Option<String>, // NEW - Tier 3 from markers
      choices: Vec<DialogueChoice>,
  }
  ```

### 1.6 Backward Compatibility
- [ ] Keep existing ClientMessage variants
- [ ] Add `#[deprecated]` comments
- [ ] Both paths call same use cases

## Phase 2: Conversation Context Integration

**Goal**: Populate `conversation_history` in LLM prompts.

### 2.1 Query Dialogue History Before LLM Call
- [ ] In queue processing (`use_cases/queues/mod.rs`), before building prompt:
  ```rust
  // Get recent dialogue turns for this conversation
  let conversation_history = if let Some(conversation_id) = &action.conversation_id {
      self.narrative.get_conversation_turns(conversation_id, 30).await?
  } else if let (Some(pc_id), Some(npc_id)) = (&action.pc_id, &action.npc_id) {
      // Fallback: get recent dialogues with this NPC
      self.narrative.get_recent_dialogue_turns(pc_id, npc_id, 30).await?
  } else {
      vec![]
  };
  ```

### 2.2 Add Repository Methods
- [ ] Add to `NarrativeRepo` trait:
  ```rust
  async fn get_conversation_turns(
      &self,
      conversation_id: &Uuid,
      limit: usize,
  ) -> Result<Vec<ConversationTurn>, RepoError>;

  async fn get_recent_dialogue_turns(
      &self,
      pc_id: &PlayerCharacterId,
      npc_id: &CharacterId,
      limit: usize,
  ) -> Result<Vec<ConversationTurn>, RepoError>;
  ```

- [ ] Implement in `Neo4jNarrativeRepo`:
  ```cypher
  // Get turns for specific conversation
  MATCH (c:Conversation {id: $conversation_id})-[r:HAS_TURN]->(t:DialogueTurn)
  WITH t, r.order as order
  ORDER BY order DESC
  LIMIT $limit
  RETURN t.speaker_type as speaker_type, t.text as text
  ORDER BY order ASC
  ```

### 2.3 Wire into GamePromptRequest
- [ ] Update `build_prompt()` to populate `conversation_history`:
  ```rust
  GamePromptRequest {
      // ... other fields ...
      conversation_history: conversation_turns.into_iter()
          .map(|t| ConversationTurn {
              speaker: if t.speaker_type == "pc" { pc_name.clone() } else { npc_name.clone() },
              text: t.text,
          })
          .collect(),
  }
  ```

### 2.4 Verify Token Budget Enforcement
- [ ] Wire `ContextBudgetEnforcer` into prompt building
- [ ] Truncate conversation_history if over budget (keep most recent turns)

## Phase 3: Use Case Wiring (Predefined Actions)

**Status**: ✅ Mostly complete - use cases already exist.

### 3.1 Verify Use Case Mapping
- [x] `StartConversation` -> `use_cases::conversation::StartConversation` ✅
- [x] `ContinueConversation` -> `use_cases::conversation::ContinueConversation` ✅
- [x] `EndConversation` -> `use_cases::conversation::EndConversation` ✅
- [x] `MoveToRegion` -> `use_cases::movement::EnterRegion` ✅
- [x] `ExitLocation` -> `use_cases::movement::ExitLocation` ✅
- [x] Inventory actions -> `use_cases::inventory::*` ✅
- [ ] `GetConversation` -> create new query use case
- [ ] `GetConversationHistory` -> create new query use case
- [ ] `PerformInteraction` -> verify/create interaction use case
- [ ] `UseItem` -> verify/create item use case

### 3.2 Handler Pattern Verification
- [x] Handlers are thin (delegate to use cases) ✅
- [x] Use cases perform orchestration ✅

## Phase 4: Player Client Updates

- [ ] Add player service methods using `RequestPayload::Player`
- [ ] Track `conversation_id` in client state when conversation starts
- [ ] Pass `conversation_id` in subsequent ContinueConversation calls
- [ ] Display conversation history from `GetConversationHistory` response

## Phase 5: Tests + Validation

- [ ] Add tests for `PlayerRequest` routing
- [ ] Test conversation_id flow (start → continue → end)
- [ ] Test conversation_history population in LLM context
- [ ] Verify backward compatibility with legacy ClientMessage

---

## Phase 6: Intent-Based Actions

Intent-based actions allow players to express what they want to do in natural language.

### 6.1 Domain Types

Add to `crates/domain/src/value_objects/`:

- [ ] Create `intent_action.rs`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum ActionVerb {
      Examine,      // Observation/investigation
      Attempt,      // Try a physical action
      Search,       // Look for something specific
      Investigate,  // Deep research/inquiry
      Use,          // Use an item/ability on something
      Interact,     // Generic interaction
      Custom(String),
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum ActionTarget {
      Region,                           // Current region/environment
      RegionElement { name: String },   // "the wall", "the door"
      Npc { npc_id: CharacterId },      // An NPC (may start/continue conversation)
      Item { item_id: ItemId },
      Self_,                            // PC themselves
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct IntentAction {
      pub verb: ActionVerb,
      pub target: ActionTarget,
      pub intent: String,              // Free-form player intent
      pub context: Option<String>,
  }
  ```

- [ ] Create `action_resolution.rs`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum ActionResolutionOutcome {
      /// Route to conversation system (target is NPC)
      StartConversation {
          npc_id: CharacterId,
          opening_context: String,  // Intent becomes conversation context
      },
      /// Add context to existing conversation
      ContinueConversationWithContext {
          conversation_id: Uuid,
          additional_context: String,
      },
      /// LLM suggests a challenge
      SuggestChallenge {
          challenge_name: String,
          skill_name: String,
          difficulty: String,
          description: String,
          reasoning: String,
      },
      /// Action reveals information (no roll needed)
      RevealInformation {
          information: String,
          observation_type: Option<String>,
      },
      /// Action triggers scene change
      TriggerSceneChange {
          description: String,
          suggested_event_id: Option<String>,
      },
      /// Action requires time passage
      RequiresTime {
          description: String,
          time_cost_minutes: u32,
          outcome_on_completion: String,
      },
      /// Not possible in current context
      NotPossible {
          reason: String,
          alternatives: Vec<String>,
      },
      /// Need more specificity
      NeedsClarification {
          question: String,
          options: Vec<String>,
      },
      /// Multiple approaches available
      MultipleApproaches {
          description: String,
          approaches: Vec<ActionApproachOption>,
      },
  }
  ```

### 6.2 Protocol Types

- [ ] Add `PerformIntentAction` to `PlayerRequest`:
  ```rust
  PerformIntentAction {
      action: IntentActionData,
  }
  ```

- [ ] Add server messages for intent action flow

### 6.3 Intent Action → Conversation Integration

**Critical**: When intent action targets an NPC, route through conversation system:

```rust
impl ResolveIntentAction {
    async fn execute(&self, pc_id: PcId, action: IntentAction) -> Result<...> {
        match action.target {
            ActionTarget::Npc { npc_id } => {
                // Check if active conversation exists
                let active = self.conversation.get_active(pc_id, npc_id).await?;

                if let Some(conversation) = active {
                    // Add intent as context to existing conversation
                    Ok(ActionResolutionOutcome::ContinueConversationWithContext {
                        conversation_id: conversation.id,
                        additional_context: format!("{:?}: {}", action.verb, action.intent),
                    })
                } else {
                    // Start new conversation with intent as opening context
                    Ok(ActionResolutionOutcome::StartConversation {
                        npc_id,
                        opening_context: format!("{:?}: {}", action.verb, action.intent),
                    })
                }
            }
            ActionTarget::Region | ActionTarget::RegionElement { .. } => {
                // Environment action → LLM resolution
                self.resolve_environment_action(pc_id, action).await
            }
            // ... other targets
        }
    }
}
```

### 6.4 Engine Implementation

- [ ] Create `use_cases/intent_action/` module
- [ ] Add action resolution prompt template
- [ ] Wire through approval flow
- [ ] Track actions as StoryEvents for context

---

## Phase 7: Travel with Intent

### 7.1 Protocol Types

- [ ] Add `TravelIntent`:
  ```rust
  pub struct TravelIntent {
      pub approach: TravelApproach,
      pub objective: Option<String>,
  }

  pub enum TravelApproach {
      Normal,
      Stealth,
      Haste,
      Caution,
      Custom(String),
  }
  ```

- [ ] Extend movement messages with optional intent

### 7.2 Engine Implementation

- [ ] Modify `EnterRegion` to check for travel intent
- [ ] Non-Normal intent triggers action resolution for potential challenges

---

## Phase 8: DM Challenge Generation

### 8.1 Protocol Types

- [ ] Add `GenerateChallengeFromDescription` client message
- [ ] Add `GeneratedChallengesReady` server message
- [ ] Add `AcceptGeneratedChallenge` client message

### 8.2 Engine Implementation

- [ ] Create `use_cases/challenge/generate_from_description.rs`
- [ ] Add challenge generation prompt template
- [ ] Support branching challenge options

---

## Phase 9: Action History for Context

### 9.1 Extend StoryEvent System

- [ ] Ensure all player actions become StoryEvents
- [ ] Add `StoryEventType::PlayerAction` variant (or reuse existing)

### 9.2 Context Integration

- [ ] Query recent player actions before LLM calls
- [ ] Add to staging and dialogue context
- [ ] Limit by time window or count

---

## Phase 10: UI Updates

### 10.1 Conversation UI
- [ ] Display conversation_id in debug/DM mode
- [ ] Show conversation history panel
- [ ] Track mood/expression changes visually

### 10.2 Intent Action UI
- [ ] Verb selector, target selector, intent input
- [ ] Quick action buttons for common verbs

### 10.3 Travel Intent UI
- [ ] Travel mode selector in navigation

---

## Implementation Priority

1. **Phase 2**: Conversation context integration (conversation_history population)
2. **Phase 1**: PlayerRequest with conversation_id support
3. **Phase 6.1-6.3**: Intent action types with NPC→conversation routing
4. **Phase 8**: DM challenge generation
5. **Phase 7**: Travel with intent
6. **Phase 9**: Action history context
7. **Phase 10**: UI updates

---

## Acceptance Criteria

### Phase 1-2 (Conversation Support)
- [ ] `conversation_id` returned from StartConversation
- [ ] `conversation_id` can be passed to ContinueConversation
- [ ] `conversation_history` populated in LLM prompts
- [ ] Client can query conversation history

### Phase 6 (Intent Actions)
- [ ] Intent actions targeting NPCs route to conversation system
- [ ] Intent context becomes conversation context
- [ ] Environment actions go through LLM resolution
- [ ] DM approves action resolutions

### Phase 7-9 (Advanced Features)
- [ ] Travel intent can trigger challenges
- [ ] DM can generate challenges from descriptions
- [ ] Action history appears in LLM context

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-09 | Deep architecture review: documented conversation system, added Phase 2 for context integration |
| 2026-01-09 | Code review: documented existing state, marked completed items |
| 2026-01-09 | Major expansion: Added phases 6-10 for intent actions, travel, challenge generation |
| 2026-01-XX | Initial version |
