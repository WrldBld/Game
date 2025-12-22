# WebSocket Protocol

## Overview

WrldBldr uses WebSocket for real-time communication between Player clients and the Engine server. Messages are JSON-encoded with a `type` field for routing.

---

## Connection Flow

```
1. Player connects to ws://engine:3000/ws
2. Player sends JoinSession with user_id, role, world_id
3. Server responds with SessionJoined containing world snapshot
4. Bidirectional message exchange begins
5. Player sends Heartbeat periodically; server responds with Pong
```

---

## Client → Server Messages

### Session Management

| Message | Fields | Purpose |
|---------|--------|---------|
| `JoinSession` | `user_id`, `role`, `world_id` | Join/create session |
| `Heartbeat` | - | Connection keepalive |

### Player Actions

| Message | Fields | Purpose |
|---------|--------|---------|
| `PlayerAction` | `action_type`, `target_id?`, `content` | Player speaks/acts |
| `RequestSceneChange` | `scene_id` | Request scene change |

### DM Actions

| Message | Fields | Purpose |
|---------|--------|---------|
| `DirectorialUpdate` | `notes` | Update directorial context |
| `ApprovalDecision` | `action_id`, `decision`, `modified_dialogue?`, `approved_tools?`, `feedback?` | Approve/reject LLM response |
| `TriggerChallenge` | `challenge_id`, `target_pc_id` | Manually trigger challenge |
| `ChallengeSuggestionDecision` | `suggestion_id`, `approved`, `modified_dc?` | Approve challenge suggestion |
| `ChallengeOutcomeDecision` | `outcome_id`, `approved`, `modified_text?` | Approve outcome |
| `NarrativeEventSuggestionDecision` | `event_id`, `approved`, `selected_outcome?` | Approve event trigger |
| `RequestOutcomeSuggestion` | `challenge_id` | Request LLM outcome suggestions |
| `RequestOutcomeBranches` | `challenge_id` | Request LLM branches |
| `SelectOutcomeBranch` | `branch_id` | Select branch |
| `RegenerateOutcome` | `challenge_id` | Regenerate outcome |
| `DiscardChallenge` | `challenge_id` | Discard suggestion |
| `CreateAdHocChallenge` | `name`, `skill_id`, `difficulty`, `target_pc_id` | Create without LLM |
| `ShareNpcLocation` | `npc_id`, `pc_id`, `location_id`, `region_id?`, `notes?` | Share NPC whereabouts |
| `TriggerApproachEvent` | `npc_id`, `target_pc_id`, `description`, `reveal` | NPC approaches player (optionally unrevealed) |
| `TriggerLocationEvent` | `region_id`, `description` | Location narration |
| `AdvanceGameTime` | `hours` | Advance in-game time |

### Navigation

| Message | Fields | Purpose |
|---------|--------|---------|
| `SelectPlayerCharacter` | `pc_id` | Select PC to play |
| `MoveToRegion` | `region_id` | Move within location |
| `ExitToLocation` | `location_id`, `arrival_region_id?` | Exit to location |

### Challenges

| Message | Fields | Purpose |
|---------|--------|---------|
| `ChallengeRoll` | `challenge_id`, `roll_result`, `modifier` | Submit roll |
| `ChallengeRollInput` | `challenge_id`, `dice_input` | Submit dice input |

---

## Server → Client Messages

### Session

| Message | Fields | Purpose |
|---------|--------|---------|
| `SessionJoined` | `session_id`, `world_snapshot`, `participants` | Session joined |
| `PlayerJoined` | `user_id`, `role`, `character_name?` | Participant joined |
| `PlayerLeft` | `user_id` | Participant left |
| `Error` | `message` | Error occurred |
| `Pong` | - | Heartbeat response |

### Actions

| Message | Fields | Purpose |
|---------|--------|---------|
| `ActionReceived` | `action_id` | Action acknowledged |
| `ActionQueued` | `action_id`, `position` | Action queued |
| `QueueStatus` | `player_queue`, `dm_queue`, `llm_queue` | Queue depths |

### Dialogue

| Message | Fields | Purpose |
|---------|--------|---------|
| `LLMProcessing` | `npc_name` | Thinking indicator |
| `ApprovalRequired` | `action_id`, `dialogue`, `reasoning`, `tools?`, `challenge_suggestion?`, `event_suggestion?` | DM approval needed |
| `ResponseApproved` | `action_id` | Approval confirmed |
| `DialogueResponse` | `npc_name`, `dialogue`, `choices?` | NPC response |

### Scene

| Message | Fields | Purpose |
|---------|--------|---------|
| `SceneUpdate` | `scene`, `characters`, `interactions` | Scene changed |
| `SceneChanged` | `region`, `npcs_present`, `navigation_options` | PC moved |
| `MovementBlocked` | `reason` | Movement failed |
| `PcSelected` | `pc_id`, `pc_name`, `location`, `region` | PC selection confirmed |

### Challenges

| Message | Fields | Purpose |
|---------|--------|---------|
| `ChallengePrompt` | `challenge`, `target_pc`, `skill`, `modifier` | Challenge started |
| `ChallengeRollSubmitted` | `challenge_id`, `roll`, `modifier`, `total` | Roll received |
| `ChallengeOutcomePending` | `challenge_id`, `outcome_type`, `details` | Awaiting DM approval |
| `ChallengeResolved` | `challenge_id`, `result`, `outcome` | Challenge completed |
| `OutcomeSuggestionReady` | `challenge_id`, `suggestions` | LLM suggestions ready |
| `OutcomeBranchesReady` | `challenge_id`, `branches` | LLM branches ready |
| `OutcomeRegenerated` | `challenge_id`, `outcome` | New outcome |
| `ChallengeDiscarded` | `challenge_id` | Challenge discarded |
| `AdHocChallengeCreated` | `challenge` | Ad-hoc created |

### Events

| Message | Fields | Purpose |
|---------|--------|---------|
| `NarrativeEventTriggered` | `event_id`, `name`, `description`, `effects?` | Event fired |
| `ApproachEvent` | `npc_id`, `npc_name`, `npc_sprite?`, `description`, `reveal` | NPC approached (can be "Unknown Figure") |
| `LocationEvent` | `region_id`, `description` | Location narration |
| `NpcLocationShared` | `npc_id`, `npc_name`, `location`, `region?`, `notes?` | DM shared info |
| `GameTimeUpdated` | `display`, `time_of_day`, `is_paused` | Time advanced |
| `SplitPartyNotification` | `pc_ids`, `locations` | Party split warning |

### Generation

| Message | Fields | Purpose |
|---------|--------|---------|
| `GenerationQueued` | `batch_id`, `position` | Batch queued |
| `GenerationProgress` | `batch_id`, `completed`, `total` | Progress update |
| `GenerationComplete` | `batch_id`, `assets` | Batch finished |
| `GenerationFailed` | `batch_id`, `error` | Batch failed |
| `ComfyUIStateChanged` | `state`, `message`, `retry_in?` | ComfyUI status |

### Suggestions

| Message | Fields | Purpose |
|---------|--------|---------|
| `SuggestionQueued` | `task_id`, `entity_type`, `field` | Suggestion queued |
| `SuggestionProgress` | `task_id` | Processing |
| `SuggestionComplete` | `task_id`, `suggestion` | Suggestion ready |
| `SuggestionFailed` | `task_id`, `error` | Suggestion failed |

---

## Message Format

### Request

```json
{
  "type": "PlayerAction",
  "action_type": "talk",
  "target_id": "character-uuid",
  "content": "Hello, Marcus!"
}
```

### Response

```json
{
  "type": "DialogueResponse",
  "npc_name": "Marcus",
  "dialogue": "Well met, traveler. What brings you to the Rusty Anchor?",
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

## Error Handling

```json
{
  "type": "Error",
  "message": "Session not found",
  "code": "SESSION_NOT_FOUND"
}
```

---

## Related Documents

- [Hexagonal Architecture](./hexagonal-architecture.md) - Layer structure
- [Queue System](./queue-system.md) - Message processing
