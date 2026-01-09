# Player Actions Protocol Plan (Checklist)

## Goal

Unify player actions under a comprehensive protocol that supports:
1. **Predefined actions** (dialogue, movement, inventory) with deterministic routing
2. **Intent-based actions** (verb + intent + target) with LLM resolution
3. **Travel with intent** (stealth, haste, objectives) that can trigger challenges
4. **DM-generated challenges** from natural language descriptions

All actions flow through DM approval where appropriate, maintaining narrative authority.

---

## Current State (Code Review 2026-01-09)

### What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| **ClientMessage variants** | ✅ Exists | `crates/protocol/src/messages.rs` |
| - StartConversation | ✅ | Line 47 |
| - ContinueConversation | ✅ | Line 49 |
| - PerformInteraction | ✅ | Line 51 |
| - MoveToRegion | ✅ | Line 154 |
| - ExitToLocation | ✅ | Line 157 |
| - EquipItem/UnequipItem/DropItem/PickupItem | ✅ | Lines 226-241 |
| **WebSocket handlers** | ✅ Exists | `crates/engine/src/api/websocket/` |
| - ws_conversation.rs | ✅ | Handles Start/Continue conversation |
| - ws_movement.rs | ✅ | Handles MoveToRegion, ExitToLocation |
| - ws_inventory.rs | ✅ | Handles inventory actions |
| **Use cases** | ✅ Exists | `crates/engine/src/use_cases/` |
| - conversation/ | ✅ | StartConversation, ContinueConversation, EndConversation |
| - movement/ | ✅ | EnterRegion, ExitLocation |
| - inventory/ | ✅ | Inventory operations |
| - player_action/ | ✅ | Generic player action handler |
| **Approval flow** | ✅ Exists | Works for dialogue |
| - ApprovalRequired message | ✅ | Server → DM with proposed_dialogue, reasoning, tools |
| - ApprovalDecision message | ✅ | DM → Server with decision |
| - ws_approval.rs handler | ✅ | Processes approval decisions |
| - ApprovalUseCases | ✅ | Orchestrates approval flow |
| **Queue system** | ✅ Exists | `QueuePort` with type tagging |
| **StoryEvent persistence** | ✅ Exists | DialogueExchange, LocationChange, ChallengeAttempted |
| **Prompt templates** | ✅ Exists | Dialogue, Staging, Outcomes, Suggestions categories |

### What's Missing

| Component | Status | Notes |
|-----------|--------|-------|
| `PlayerRequest` enum | ❌ Missing | No unified player request grouping |
| `RequestPayload::Player` variant | ❌ Missing | Actions use direct ClientMessage variants |
| Intent action types | ❌ Missing | ActionVerb, ActionTarget, IntentAction |
| Action resolution outcomes | ❌ Missing | SuggestChallenge, RevealInformation, etc. |
| Travel intent | ❌ Missing | TravelApproach, TravelIntent |
| Challenge generation from description | ❌ Missing | GenerateChallengeFromDescription message |
| Action history context | ❌ Partial | StoryEvent exists, not integrated into LLM context |

### Architectural Recommendations

1. **Reuse approval flow**: Extend existing `ApprovalRequired` with request type discriminator rather than creating separate `ActionResolutionApprovalRequired`
2. **Reuse queue system**: Tag existing queue entries with action type rather than creating separate `ActionResolutionQueue`
3. **Extend templates**: Add new `PromptTemplateCategory` values for action resolution and challenge generation
4. **Incremental deprecation**: Keep legacy ClientMessage variants during transition, add PlayerRequest as parallel path

---

## Phase 1: Protocol + Engine Routing (Predefined Actions)

**Goal**: Create unified `PlayerRequest` enum while maintaining backward compatibility.

### 1.1 Add PlayerRequest Module
- [ ] Create `crates/protocol/src/requests/player.rs`
- [ ] Add `PlayerRequest` enum with variants matching existing ClientMessage:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  pub enum PlayerRequest {
      StartConversation { npc_id: String, message: String },
      ContinueConversation { npc_id: String, message: String },
      EndConversation { npc_id: String },
      PerformInteraction { interaction_id: String },
      MoveToRegion { region_id: String },
      ExitToLocation { location_id: String, arrival_region_id: Option<String> },
      EquipItem { item_id: String },
      UnequipItem { item_id: String },
      DropItem { item_id: String, quantity: Option<u32> },
      PickupItem { item_id: String },
      UseItem { item_id: String, target_id: Option<String> },
  }
  ```

### 1.2 Register in RequestPayload
- [ ] Add `pub mod player;` to `crates/protocol/src/requests.rs`
- [ ] Add `Player(player::PlayerRequest)` variant to `RequestPayload` enum

### 1.3 Add WebSocket Handler
- [ ] Create `crates/engine/src/api/websocket/ws_player_request.rs`
- [ ] Implement `handle_player_request()` that delegates to existing handlers
- [ ] Route `RequestPayload::Player` in `mod.rs` dispatch

### 1.4 Backward Compatibility
- [ ] Keep existing ClientMessage variants (StartConversation, MoveToRegion, etc.)
- [ ] Mark them with `#[deprecated]` comments for future removal
- [ ] Both paths call same use cases

## Phase 2: Use Case Wiring (Predefined Actions)

**Status**: ✅ Mostly complete - use cases already exist.

### 2.1 Verify Use Case Mapping
- [x] `StartConversation` -> `use_cases::conversation::StartConversation` ✅ EXISTS
- [x] `ContinueConversation` -> `use_cases::conversation::ContinueConversation` ✅ EXISTS
- [x] `EndConversation` -> `use_cases::conversation::EndConversation` ✅ EXISTS
- [x] `MoveToRegion` -> `use_cases::movement::EnterRegion` ✅ EXISTS
- [x] `ExitLocation` -> `use_cases::movement::ExitLocation` ✅ EXISTS
- [x] Inventory actions -> `use_cases::inventory::*` ✅ EXISTS
- [ ] `PerformInteraction` -> verify interaction use case exists or create
- [ ] `UseItem` -> verify item use case exists or create

### 2.2 Handler Pattern Verification
- [x] Handlers are thin (delegate to use cases) ✅ VERIFIED
- [x] Use cases perform orchestration ✅ VERIFIED

## Phase 3: Context + Persistence (Predefined Actions)

**Status**: ✅ Mostly complete.

### 3.1 Verify Context Passing
- [x] Conversation use case receives world_id, pc_id, npc_id ✅
- [x] ApprovalRequestData includes context fields ✅
- [ ] Verify `scene_id`, `location_id`, `game_time` are passed through queue

### 3.2 Verify Narrative Persistence
- [x] Conversation node persisted ✅ EXISTS
- [x] DialogueTurn nodes persisted ✅ EXISTS
- [x] StoryEvent links to conversation ✅ EXISTS
- [ ] Verify GameTime linking

## Phase 4: Player Client Updates (Predefined Actions)

- [ ] Add player service methods that use `RequestPayload::Player`:
  - [ ] `start_conversation(npc_id, message)`
  - [ ] `continue_conversation(npc_id, message)`
  - [ ] `perform_interaction(interaction_id)`
  - [ ] `move_to_region(region_id)`
  - [ ] `exit_location(location_id)`
- [ ] Update UI to use new methods (or keep using existing ClientMessage for now)

## Phase 5: Tests + Validation (Predefined Actions)

- [ ] Add tests for `PlayerRequest` routing in `ws_player_request.rs`
- [ ] Verify both legacy and new paths work
- [ ] Run `cargo check -p wrldbldr-engine -p wrldbldr-protocol`

---

## Phase 6: Intent-Based Actions

Intent-based actions allow players to express what they want to do in natural language, with the LLM reasoning about how to resolve it. Examples:
- "Examine the wall to find a weakness"
- "Search the room for hidden compartments"
- "Attempt to pick the lock quietly"

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
      Npc { npc_id: CharacterId },
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
      SuggestChallenge {
          challenge_name: String,
          skill_name: String,
          difficulty: String,
          description: String,
          reasoning: String,
      },
      RevealInformation {
          information: String,
          observation_type: Option<String>,
      },
      TriggerSceneChange {
          description: String,
          suggested_event_id: Option<String>,
      },
      RequiresTime {
          description: String,
          time_cost_minutes: u32,
          outcome_on_completion: String,
      },
      NotPossible {
          reason: String,
          alternatives: Vec<String>,
      },
      NeedsClarification {
          question: String,
          options: Vec<String>,
      },
      MultipleApproaches {
          description: String,
          approaches: Vec<ActionApproachOption>,
      },
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ActionApproachOption {
      pub id: String,
      pub name: String,
      pub description: String,
      pub skill_hint: Option<String>,
      pub difficulty_hint: Option<String>,
  }
  ```

### 6.2 Protocol Types

Add to `crates/protocol/src/requests/player.rs`:

- [ ] Add `PerformIntentAction` variant to `PlayerRequest`:
  ```rust
  PerformIntentAction {
      action: IntentActionData,
  }
  ```

- [ ] Add `IntentActionData` wire type (mirrors domain but with String IDs)

Add to `crates/protocol/src/messages.rs`:

- [ ] Add `ActionResolutionPending` server message:
  ```rust
  ActionResolutionPending {
      action_id: String,
      action_description: String,
  }
  ```

- [ ] Extend `ApprovalRequired` OR add `ActionResolutionApprovalRequired`:
  ```rust
  // Option A: Extend ApprovalRequired with optional action field
  ApprovalRequired {
      // ... existing fields ...
      #[serde(default)]
      action_resolution: Option<ActionResolutionData>,
  }

  // Option B: Separate message (more explicit)
  ActionResolutionApprovalRequired {
      request_id: String,
      pc_id: String,
      pc_name: String,
      action: IntentActionData,
      suggested_outcomes: Vec<ActionResolutionOutcomeData>,
      llm_reasoning: String,
      context_summary: String,
  }
  ```

- [ ] Add `ActionResolutionApproved` server message:
  ```rust
  ActionResolutionApproved {
      action_id: String,
      outcome_type: String,
      description: String,
      challenge: Option<ChallengePromptData>,
      revelation: Option<String>,
  }
  ```

Add to `crates/protocol/src/messages.rs` (Client):

- [ ] Add `ActionResolutionDecision` client message:
  ```rust
  ActionResolutionDecision {
      request_id: String,
      decision: ActionResolutionDecisionType,
  }
  ```

### 6.3 Engine Implementation

- [ ] Create `crates/engine/src/use_cases/intent_action/`:
  - [ ] `mod.rs` - IntentActionUseCases struct
  - [ ] `resolve.rs` - ResolveIntentAction use case
  - [ ] `prompt_builder.rs` - Build LLM context for action resolution

- [ ] Add action resolution prompt template:
  - [ ] Add `ActionResolution` variant to `PromptTemplateCategory`
  - [ ] Create default action resolution prompt

- [ ] Wire through existing queue system:
  - [ ] Add `QueueItemType::ActionResolution` (or reuse existing with tag)
  - [ ] Queue intent action for LLM processing
  - [ ] Route LLM response to approval flow

- [ ] Create/extend approval handler:
  - [ ] Handle `ActionResolutionDecision` in `ws_approval.rs`
  - [ ] Execute approved outcome (create challenge, reveal info, etc.)

### 6.4 Integration Points

- [ ] Add to `ws_player_request.rs`:
  ```rust
  PlayerRequest::PerformIntentAction { action } => {
      self.use_cases.intent_action.resolve.execute(pc_id, action).await
  }
  ```

---

## Phase 7: Travel with Intent

Travel can have player intent that affects how the journey unfolds.

### 7.1 Domain & Protocol Types

- [ ] Add `TravelIntent` to domain:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct TravelIntent {
      pub approach: TravelApproach,
      pub objective: Option<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum TravelApproach {
      Normal,
      Stealth,
      Haste,
      Caution,
      Custom(String),
  }
  ```

- [ ] Extend `PlayerRequest` movement variants:
  ```rust
  MoveToRegion {
      region_id: String,
      #[serde(default)]
      intent: Option<TravelIntent>,
  },
  ExitToLocation {
      location_id: String,
      arrival_region_id: Option<String>,
      #[serde(default)]
      intent: Option<TravelIntent>,
  },
  ```

### 7.2 Engine Implementation

- [ ] Modify `EnterRegion` use case:
  - [ ] Check for travel intent
  - [ ] If non-Normal intent, route through action resolution
  - [ ] Generate appropriate challenges for Stealth/Haste/Caution

- [ ] Add server message for travel resolution (if complex):
  ```rust
  TravelResolutionRequired {
      travel_id: String,
      destination_name: String,
      intent: TravelIntent,
      suggested_challenges: Vec<GeneratedChallengeData>,
  }
  ```

---

## Phase 8: DM Challenge Generation

DMs should be able to describe a challenge concept and have the LLM generate full challenge details.

### 8.1 Protocol Types

- [ ] Add `GenerateChallengeFromDescription` client message:
  ```rust
  GenerateChallengeFromDescription {
      description: String,
      target_pc_id: String,
      context: ChallengeGenerationContext,
  }

  pub struct ChallengeGenerationContext {
      pub suggested_skills: Vec<String>,
      pub difficulty_hint: Option<String>,
      pub branching_options: Vec<String>,
      pub location_context: Option<String>,
  }
  ```

- [ ] Add `GeneratedChallengesReady` server message:
  ```rust
  GeneratedChallengesReady {
      request_id: String,
      shared_context: String,
      challenges: Vec<GeneratedChallengeData>,
  }

  pub struct GeneratedChallengeData {
      pub id: String,
      pub name: String,
      pub description: String,
      pub skill_name: String,
      pub difficulty: String,
      pub success_outcome: String,
      pub failure_outcome: String,
      pub branch_label: Option<String>,
  }
  ```

- [ ] Add `AcceptGeneratedChallenge` client message:
  ```rust
  AcceptGeneratedChallenge {
      request_id: String,
      challenge_id: String,
      modifications: Option<ChallengeModifications>,
  }
  ```

### 8.2 Engine Implementation

- [ ] Create `use_cases/challenge/generate_from_description.rs`
- [ ] Add challenge generation prompt template
- [ ] On accept, create Challenge entity and trigger for target PC

---

## Phase 9: Action History for Context

Track player actions to provide richer LLM context.

### 9.1 Domain Types

- [ ] Add `PlayerActionRecord` to domain (or extend StoryEvent):
  ```rust
  pub struct PlayerActionRecord {
      pub id: Uuid,
      pub pc_id: PlayerCharacterId,
      pub action_type: String,
      pub target_description: String,
      pub intent: Option<String>,
      pub outcome_summary: String,
      pub game_time: GameTime,
      pub region_id: RegionId,
  }
  ```

### 9.2 Persistence

- [ ] Extend `StoryEvent` with `PlayerAction` variant or create separate node type
- [ ] Record action outcomes when resolved

### 9.3 Context Integration

- [ ] Add `recent_player_actions` to staging LLM context
- [ ] Add `recent_player_actions` to dialogue LLM context
- [ ] Query last N actions or actions within time window

---

## Phase 10: UI Updates for Intent Actions

### 10.1 Action Input UI

- [ ] Create `IntentActionInput` component:
  - Verb selector dropdown
  - Target selector (environment, NPC, item)
  - Intent text input
  - Submit button

- [ ] Add quick action buttons to scene view

### 10.2 DM Approval UI

- [ ] Create `ActionResolutionApproval` component (or extend existing approval popup)

### 10.3 Travel Intent UI

- [ ] Add travel mode selector to navigation panel

---

## Protocol Summary

### New Server Messages

| Message | Audience | Purpose |
|---------|----------|---------|
| `ActionResolutionPending` | Player | Action is being resolved |
| `ActionResolutionApprovalRequired` | DM | DM must approve action resolution |
| `ActionResolutionApproved` | Player | Action resolved with outcome |
| `TravelResolutionRequired` | DM | Travel with intent needs approval |
| `GeneratedChallengesReady` | DM | LLM-generated challenges ready |

### New Client Messages

| Message | Sender | Purpose |
|---------|--------|---------|
| `PerformIntentAction` | Player | Player performs intent-based action |
| `ActionResolutionDecision` | DM | DM approves/modifies action resolution |
| `GenerateChallengeFromDescription` | DM | DM requests challenge generation |
| `AcceptGeneratedChallenge` | DM | DM accepts generated challenge |

---

## Acceptance Criteria

### Phase 1-5 (Predefined Actions)
- [ ] `PlayerRequest` enum exists with all action variants
- [ ] `RequestPayload::Player` routes to handlers
- [ ] Both legacy ClientMessage and new PlayerRequest paths work
- [ ] Handlers are thin, use cases perform orchestration

### Phase 6 (Intent Actions)
- [ ] Players can submit intent-based actions (verb + target + intent)
- [ ] LLM suggests resolution outcomes
- [ ] DM approves/modifies before player sees result
- [ ] Challenges from actions trigger challenge flow

### Phase 7 (Travel Intent)
- [ ] Players can travel with stealth/haste/caution
- [ ] Travel intent can trigger challenges
- [ ] DM can approve/modify travel resolutions

### Phase 8 (DM Challenge Generation)
- [ ] DM describes challenge, LLM generates full details
- [ ] Branching options generate separate challenges
- [ ] DM can accept/modify before triggering

### Phase 9 (Action History)
- [ ] Player actions recorded in Neo4j
- [ ] Recent actions in staging LLM context
- [ ] Recent actions in dialogue LLM context

---

## Implementation Priority

1. **Phase 1**: Add `PlayerRequest` enum (foundation for everything)
2. **Phase 6.1-6.2**: Domain + protocol types for intent actions
3. **Phase 6.3**: Engine use cases and queue integration
4. **Phase 8**: DM challenge generation (high DM value)
5. **Phase 7**: Travel with intent (builds on intent actions)
6. **Phase 9**: Action history context (improves LLM quality)
7. **Phase 10**: UI updates (last, after backend is solid)

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-09 | Code review: documented existing state, marked completed items, added recommendations |
| 2026-01-09 | Major expansion: Added phases 6-10 for intent actions, travel intent, DM challenge generation, action history |
| 2026-01-XX | Initial version: Basic predefined action unification |
