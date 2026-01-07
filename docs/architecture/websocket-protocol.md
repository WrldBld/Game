# WebSocket Protocol

## Overview

WrldBldr uses WebSocket for real-time communication between Player clients and the Engine server. The protocol uses a **world-scoped connection model** where clients join specific worlds rather than sessions. Messages are JSON-encoded with a `type` field for routing.

---

## Connection Flow

```
1. Player connects to ws://engine:3000/ws
2. Player sends JoinWorld with world_id, role, and optional pc_id
3. Server responds with WorldJoined containing world snapshot
4. Bidirectional message exchange begins
5. Player sends Heartbeat periodically; server responds with Pong
6. Player sends LeaveWorld or disconnects to leave
```

---

## Client -> Server Messages

### World Connection

| Message | Fields | Purpose |
|---------|--------|---------|
| `JoinWorld` | `world_id`, `role`, `pc_id?`, `spectate_pc_id?` | Join a world |
| `LeaveWorld` | - | Leave current world |
| `Heartbeat` | - | Connection keepalive |
| `SetSpectateTarget` | `pc_id` | Change spectate target (Spectator role) |

### Generic Request/Response

| Message | Fields | Purpose |
|---------|--------|---------|
| `Request` | `request_id`, `payload: RequestPayload` | CRUD operations and actions |

The `RequestPayload` enum contains 80+ variants for entity CRUD, queries, and actions. Responses come back as `ServerMessage::Response` with the correlated `request_id`.

#### Routing in Engine

Server-side routing lives in `crates/engine/src/api/websocket/mod.rs`. Connection lifecycle and non-request messages are handled directly in `mod.rs`, while `RequestPayload` is dispatched to focused modules:

- `ws_core.rs`: world, character, location, time, npc, items
- `ws_creator.rs`: generation, ai, expression
- `ws_lore.rs`: lore
- `ws_story_events.rs`: story events

### Player Actions

| Message | Fields | Purpose |
|---------|--------|---------|
| `PlayerAction` | `action_type`, `target?`, `dialogue?` | Player speaks/acts |
| `RequestSceneChange` | `scene_id` | Request scene change |
| `SelectPlayerCharacter` | `pc_id` | Select PC to play |
| `MoveToRegion` | `pc_id`, `region_id` | Move within location |
| `ExitToLocation` | `pc_id`, `location_id`, `arrival_region_id?` | Exit to location |

### DM Actions

| Message | Fields | Purpose |
|---------|--------|---------|
| `DirectorialUpdate` | `context: DirectorialContext` | Update directorial context |
| `ApprovalDecision` | `request_id`, `decision` | Approve/reject LLM response |
| `TriggerChallenge` | `challenge_id`, `target_character_id` | Manually trigger challenge |
| `ChallengeSuggestionDecision` | `request_id`, `approved`, `modified_difficulty?` | Approve challenge suggestion |
| `ChallengeOutcomeDecision` | `resolution_id`, `decision` | Approve outcome |
| `NarrativeEventSuggestionDecision` | `request_id`, `event_id`, `approved`, `selected_outcome?` | Approve event trigger |
| `RequestOutcomeSuggestion` | `resolution_id`, `guidance?` | Request LLM outcome suggestions |
| `RequestOutcomeBranches` | `resolution_id`, `guidance?` | Request LLM branches |
| `SelectOutcomeBranch` | `resolution_id`, `branch_id`, `modified_description?` | Select branch |
| `RegenerateOutcome` | `request_id`, `outcome_type?`, `guidance?` | Regenerate outcome |
| `DiscardChallenge` | `request_id`, `feedback?` | Discard suggestion |
| `CreateAdHocChallenge` | `challenge_name`, `skill_name`, `difficulty`, `target_pc_id`, `outcomes` | Create without LLM |
| `ShareNpcLocation` | `pc_id`, `npc_id`, `location_id`, `region_id`, `notes?` | Share NPC whereabouts |
| `TriggerApproachEvent` | `npc_id`, `target_pc_id`, `description`, `reveal` | NPC approaches player |
| `TriggerLocationEvent` | `region_id`, `description` | Location narration |

### Challenges

| Message | Fields | Purpose |
|---------|--------|---------|
| `ChallengeRoll` | `challenge_id`, `roll` | Submit roll (legacy) |
| `ChallengeRollInput` | `challenge_id`, `input_type: DiceInputType` | Submit dice input (formula or manual) |

### Staging System (NPC Presence)

| Message | Fields | Purpose |
|---------|--------|---------|
| `StagingApprovalResponse` | `request_id`, `approved_npcs`, `ttl_hours`, `source` | DM approves NPC staging |
| `StagingRegenerateRequest` | `request_id`, `guidance` | Request new LLM staging suggestions |
| `PreStageRegion` | `region_id`, `npcs`, `ttl_hours` | Pre-stage before player arrives |

### Inventory

| Message | Fields | Purpose |
|---------|--------|---------|
| `EquipItem` | `pc_id`, `item_id` | Equip an item |
| `UnequipItem` | `pc_id`, `item_id` | Unequip an item |
| `DropItem` | `pc_id`, `item_id`, `quantity` | Drop/destroy item |
| `PickupItem` | `pc_id`, `item_id` | Pick up from region |

### Utility

| Message | Fields | Purpose |
|---------|--------|---------|
| `CheckComfyUIHealth` | - | Request ComfyUI health check |

---

## Server -> Client Messages

### World Connection

| Message | Fields | Purpose |
|---------|--------|---------|
| `WorldJoined` | `world_id`, `snapshot`, `connected_users`, `your_role`, `your_pc?` | Successfully joined world |
| `WorldJoinFailed` | `world_id`, `error: JoinError` | Join failed |
| `UserJoined` | `user_id`, `username?`, `role`, `pc?` | Another user joined |
| `UserLeft` | `user_id` | User left world |
| `Response` | `request_id`, `result: ResponseResult` | Response to Request message |
| `EntityChanged` | `EntityChangedData` | Entity change broadcast for cache invalidation |
| `SpectateTargetChanged` | `pc_id`, `pc_name` | Spectate target changed |
| `Error` | `code`, `message` | Error occurred |
| `Pong` | - | Heartbeat response |

### Actions

| Message | Fields | Purpose |
|---------|--------|---------|
| `ActionReceived` | `action_id`, `player_id`, `action_type` | Action acknowledged |
| `ActionQueued` | `action_id`, `player_name`, `action_type`, `queue_depth` | Action queued |
| `QueueStatus` | `player_actions_pending`, `llm_requests_pending`, `llm_requests_processing`, `approvals_pending` | Queue depths |

### Dialogue

| Message | Fields | Purpose |
|---------|--------|---------|
| `LLMProcessing` | `action_id` | Thinking indicator |
| `ApprovalRequired` | `request_id`, `npc_name`, `proposed_dialogue`, `internal_reasoning`, `proposed_tools`, `challenge_suggestion?`, `narrative_event_suggestion?` | DM approval needed |
| `ResponseApproved` | `npc_dialogue`, `executed_tools` | Approval confirmed |
| `DialogueResponse` | `speaker_id`, `speaker_name`, `text`, `choices` | NPC response |

### Scene & Navigation

| Message | Fields | Purpose |
|---------|--------|---------|
| `SceneUpdate` | `scene`, `characters`, `interactions` | Scene changed |
| `SceneChanged` | `pc_id`, `region`, `npcs_present`, `navigation`, `region_items` | PC moved |
| `MovementBlocked` | `pc_id`, `reason` | Movement failed |
| `PcSelected` | `pc_id`, `pc_name`, `location_id`, `region_id?` | PC selection confirmed |
| `GameTimeUpdated` | `game_time: GameTime` | Time advanced |
| `SplitPartyNotification` | `location_count`, `locations` | Party split warning |

### Challenges

| Message | Fields | Purpose |
|---------|--------|---------|
| `ChallengePrompt` | `challenge_id`, `challenge_name`, `skill_name`, `difficulty_display`, `description`, `character_modifier`, `suggested_dice?`, `rule_system_hint?` | Challenge started |
| `ChallengeRollSubmitted` | `challenge_id`, `challenge_name`, `roll`, `modifier`, `total`, `outcome_type`, `status` | Roll received |
| `ChallengeOutcomePending` | `resolution_id`, `challenge_id`, `challenge_name`, `character_id`, `character_name`, `roll`, `modifier`, `total`, `outcome_type`, `outcome_description`, `outcome_triggers`, `roll_breakdown?` | Awaiting DM approval |
| `ChallengeResolved` | `challenge_id`, `challenge_name`, `character_name`, `roll`, `modifier`, `total`, `outcome`, `outcome_description`, `roll_breakdown?`, `individual_rolls?` | Challenge completed |
| `OutcomeSuggestionReady` | `resolution_id`, `suggestions` | LLM suggestions ready |
| `OutcomeBranchesReady` | `resolution_id`, `outcome_type`, `branches` | LLM branches ready |
| `OutcomeRegenerated` | `request_id`, `outcome_type`, `new_outcome` | New outcome |
| `ChallengeDiscarded` | `request_id` | Challenge discarded |
| `AdHocChallengeCreated` | `challenge_id`, `challenge_name`, `target_pc_id` | Ad-hoc created |

### Events

| Message | Fields | Purpose |
|---------|--------|---------|
| `NarrativeEventTriggered` | `event_id`, `event_name`, `outcome_description`, `scene_direction` | Event fired |
| `ApproachEvent` | `npc_id`, `npc_name`, `npc_sprite?`, `description`, `reveal` | NPC approached |
| `LocationEvent` | `region_id`, `description` | Location narration |
| `NpcLocationShared` | `npc_id`, `npc_name`, `region_name`, `notes?` | DM shared info |

### Staging System

| Message | Fields | Purpose |
|---------|--------|---------|
| `StagingApprovalRequired` | `request_id`, `region_id`, `region_name`, `location_id`, `location_name`, `game_time`, `previous_staging?`, `rule_based_npcs`, `llm_based_npcs`, `default_ttl_hours`, `waiting_pcs` | DM needs to approve staging |
| `StagingPending` | `region_id`, `region_name` | Player waiting for staging |
| `StagingReady` | `region_id`, `npcs_present` | Staging approved, NPCs visible |
| `StagingRegenerated` | `request_id`, `llm_based_npcs` | LLM regenerated suggestions |

### Inventory

| Message | Fields | Purpose |
|---------|--------|---------|
| `ItemEquipped` | `pc_id`, `item_id`, `item_name` | Item was equipped |
| `ItemUnequipped` | `pc_id`, `item_id`, `item_name` | Item was unequipped |
| `ItemDropped` | `pc_id`, `item_id`, `item_name`, `quantity` | Item was dropped |
| `ItemPickedUp` | `pc_id`, `item_id`, `item_name` | Item was picked up |
| `InventoryUpdated` | `pc_id` | Inventory changed signal |

### Character Stats

| Message | Fields | Purpose |
|---------|--------|---------|
| `CharacterStatUpdated` | `character_id`, `character_name`, `stat_name`, `old_value`, `new_value`, `delta`, `source` | Stat changed |

### NPC Disposition

| Message | Fields | Purpose |
|---------|--------|---------|
| `NpcDispositionChanged` | `npc_id`, `npc_name`, `pc_id`, `disposition`, `relationship`, `reason?` | Disposition changed |
| `NpcDispositionsResponse` | `pc_id`, `dispositions` | All dispositions for PC |

### Actantial Model / NPC Motivations

| Message | Fields | Purpose |
|---------|--------|---------|
| `NpcWantCreated` | `npc_id`, `want` | Want created |
| `NpcWantUpdated` | `npc_id`, `want` | Want updated |
| `NpcWantDeleted` | `npc_id`, `want_id` | Want deleted |
| `WantTargetSet` | `want_id`, `target` | Target set |
| `WantTargetRemoved` | `want_id` | Target removed |
| `ActantialViewAdded` | `npc_id`, `view` | Actantial view added |
| `ActantialViewRemoved` | `npc_id`, `want_id`, `target_id`, `role` | View removed |
| `NpcActantialContextResponse` | `npc_id`, `context` | Full actantial context |
| `WorldGoalsResponse` | `world_id`, `goals` | All world goals |
| `GoalCreated` | `world_id`, `goal` | Goal created |
| `GoalUpdated` | `goal` | Goal updated |
| `GoalDeleted` | `goal_id` | Goal deleted |

### LLM Suggestions for Actantial Model

| Message | Fields | Purpose |
|---------|--------|---------|
| `DeflectionSuggestions` | `npc_id`, `want_id`, `suggestions` | Deflection behavior suggestions |
| `TellsSuggestions` | `npc_id`, `want_id`, `suggestions` | Behavioral tells suggestions |
| `WantDescriptionSuggestions` | `npc_id`, `suggestions` | Want description suggestions |
| `ActantialReasonSuggestions` | `npc_id`, `want_id`, `target_id`, `role`, `suggestions` | Actantial view reason suggestions |

### Asset Generation

| Message | Fields | Purpose |
|---------|--------|---------|
| `GenerationQueued` | `batch_id`, `entity_type`, `entity_id`, `asset_type`, `position` | Batch queued |
| `GenerationProgress` | `batch_id`, `progress` | Progress update (0-100) |
| `GenerationComplete` | `batch_id`, `asset_count` | Batch finished |
| `GenerationFailed` | `batch_id`, `error` | Batch failed |
| `ComfyUIStateChanged` | `state`, `message?`, `retry_in_seconds?` | ComfyUI status |

### LLM Suggestions (Field Suggestions)

| Message | Fields | Purpose |
|---------|--------|---------|
| `SuggestionQueued` | `request_id`, `field_type`, `entity_id?` | Suggestion queued |
| `SuggestionProgress` | `request_id`, `status` | Processing |
| `SuggestionComplete` | `request_id`, `suggestions` | Suggestions ready |
| `SuggestionFailed` | `request_id`, `error` | Suggestion failed |

---

## Message Format

### Request

```json
{
  "type": "PlayerAction",
  "action_type": "talk",
  "target": "character-uuid",
  "dialogue": "Hello, Marcus!"
}
```

### Response

```json
{
  "type": "DialogueResponse",
  "speaker_id": "character-uuid",
  "speaker_name": "Marcus",
  "text": "Well met, traveler. What brings you to the Rusty Anchor?",
  "choices": [
    { "id": "c1", "text": "I'm looking for information about the Baron." },
    { "id": "c2", "text": "Just passing through." }
  ]
}
```

---

## Approval Decision Types

| Decision | Description |
|----------|-------------|
| `Accept` | Use response as-is |
| `AcceptWithModification` | Use modified dialogue and filtered tools |
| `Reject` | Regenerate with feedback (max 3 retries) |
| `TakeOver` | Use DM's custom response |

---

## World Roles

| Role | Description |
|------|-------------|
| `DungeonMaster` | Full control, approves content |
| `Player` | Controls a PC, plays the game |
| `Spectator` | Watches another PC's perspective |

---

## Forward Compatibility

All enums include an `Unknown` variant with `#[serde(other)]` to handle new message types gracefully. Older clients will deserialize unknown variants as `Unknown` instead of failing.

---

## Error Handling

```json
{
  "type": "Error",
  "code": "WORLD_NOT_FOUND",
  "message": "World not found"
}
```

---

## Message Statistics

| Category | Count |
|----------|-------|
| ClientMessage variants | 36 |
| ServerMessage variants | 73 |

---

## Related Documents

- [Hexagonal Architecture](./hexagonal-architecture.md) - Layer structure
- [Queue System](./queue-system.md) - Message processing
- [Staging System](../systems/staging-system.md) - NPC presence workflow
