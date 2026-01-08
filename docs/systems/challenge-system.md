# Challenge System

## Overview

The Challenge System handles skill checks, ability tests, and other dice-based resolution mechanics. It supports multiple TTRPG rule systems (D20 like D&D, D100 like Call of Cthulhu, Narrative like Fate) and integrates with the LLM to suggest contextually appropriate challenges during gameplay.

## WebSocket Coverage

- The Engine now routes `wrldbldr_protocol::ChallengeRequest` through `crates/engine/src/api/websocket/ws_challenge.rs`, returning the same shape expected by `ChallengeData`.
- Supported operations: list/get/create/update/delete challenges plus `SetChallengeActive` and `SetChallengeFavorite`.
- Responses include serialized outcomes, trigger conditions, and metadata so the Player app can keep the DM library and favorites dashboard in sync.

---

## Game Design

Challenges create dramatic tension and player agency. Key design principles:

1. **LLM Suggestions**: The AI notices when a challenge might be appropriate and suggests it
2. **DM Authority**: All challenges require DM approval before triggering
3. **Multiple Rule Systems**: Same challenge entity works with D&D, CoC, Fate, etc.
4. **Branching Outcomes**: Success, failure, partial success, and criticals each have different effects
5. **Character Modifiers**: Player stats and skills affect rolls

---

## User Stories

### Implemented

- [x] **US-CHAL-001**: As a DM, I can create challenges with skills and difficulty
  - *Implementation*: Challenge entity with ChallengeType, Difficulty, required skill
  - *Files*: `crates/domain/src/entities/challenge.rs`

- [x] **US-CHAL-002**: As a DM, I can define outcomes for success/failure/partial/critical
  - *Implementation*: ChallengeOutcomes with multiple Outcome variants and triggers
  - *Files*: `crates/domain/src/entities/challenge.rs`

- [x] **US-CHAL-003**: As a player, the LLM suggests challenges during dialogue
  - *Implementation*: LLM outputs `<challenge_suggestion>` XML tags, parsed by LlmService
  - *Files*: `crates/engine/src/use_cases/conversation/llm_queue.rs`

- [x] **US-CHAL-004**: As a DM, I can approve or reject challenge suggestions
  - *Implementation*: ChallengeSuggestionDecision WebSocket message, DM approval popup
  - *Files*: `crates/engine/src/api/websocket/mod.rs`, `crates/player-ui/src/presentation/components/dm_panel/approval_popup.rs`

- [x] **US-CHAL-005**: As a player, I can roll dice for challenges
  - *Implementation*: ChallengeRollModal with d20 rolls, platform-specific randomness
  - *Files*: `crates/player-ui/src/presentation/components/tactical/challenge_roll.rs`

- [x] **US-CHAL-006**: As a DM, I can manually trigger challenges
  - *Implementation*: TriggerChallengeModal, TriggerChallenge WebSocket message
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/trigger_challenge_modal.rs`

- [x] **US-CHAL-007**: As a DM, I can approve/edit challenge outcomes before they execute
  - *Implementation*: ChallengeOutcomeApproval component, DM can edit narrative text
  - *Files*: `crates/engine/src/use_cases/challenge/resolve.rs`

- [x] **US-CHAL-008**: As a DM, I can browse and manage a challenge library
  - *Implementation*: ChallengeLibrary with search, filtering, favorites
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/challenge_library/`

- [x] **US-CHAL-009**: As a player, I can see my character's skill modifiers during rolls
  - *Implementation*: SkillsDisplay component shows all skills with modifiers; ChallengeRollModal displays modifier in header and result breakdown (dice + modifier + skill = total)
  - *Files*: `crates/player-ui/src/presentation/components/tactical/challenge_roll.rs`, `crates/player-ui/src/presentation/components/tactical/skills_display.rs`

- [x] **US-CHAL-010**: As a DM, I can bind challenges to specific regions (not just locations)
  - *Implementation*: `AVAILABLE_AT_REGION` edge with `ChallengeRegionAvailability` entity
  - *Files*: `crates/domain/src/entities/challenge.rs`, `crates/engine/src/infrastructure/neo4j/challenge_repo.rs`
  - *Completed*: 2025-12-24

- [x] **US-CHAL-011**: Character stats are updated when challenge outcomes modify them
  - *Implementation*: `CharacterStatUpdated` message broadcast after approval; handler resolves "active_pc" placeholder
  - *Files*: `crates/engine/src/use_cases/approval/challenge_outcome.rs`, `crates/protocol/src/messages.rs`
  - *Completed*: 2025-12-26

### Future Improvements

- [ ] **US-CHAL-012**: As a DM, I can see which challenges are available in the current region
  - *Design*: RegionScene includes available challenges based on both location and region edges
  - *Effect*: Context-aware challenge suggestions in Director Mode
  - *Priority*: Low - UI enhancement

---

## UI Mockups

### Challenge Roll Modal

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Persuasion Check                                                    [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Convince the guard to let you pass without papers.                         │
│                                                                             │
│  Difficulty: DC 15 (Medium)                                                 │
│                                                                             │
│  Your Persuasion: +4                                                        │
│                                                                             │
│                      ┌─────────────────┐                                    │
│                      │                 │                                    │
│                      │       🎲        │                                    │
│                      │                 │                                    │
│                      │   [ Roll d20 ]  │                                    │
│                      │                 │                                    │
│                      └─────────────────┘                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

[After rolling]

┌─────────────────────────────────────────────────────────────────────────────┐
│  Persuasion Check - Result                                           [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                           🎲 17 + 4 = 21                                    │
│                                                                             │
│                      ✓ SUCCESS (DC 15)                                      │
│                                                                             │
│  Waiting for DM to approve outcome...                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Challenge Library

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Challenge Library                                               [+ Create] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  [🔍 Search...                    ]  [Type: All ▼]  [★ Favorites]          │
│                                                                             │
│  ─── Social ───────────────────────────────────────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ★ Convince the Guard     Persuasion    DC 15    [Active] [Edit]     │   │
│  │   "Persuade the guard to let you pass without papers"               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │   Intimidate the Merchant  Intimidation  DC 12   [Active] [Edit]    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Investigation ────────────────────────────────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │   Search the Room         Investigation  DC 13   [Active] [Edit]    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

---

## Data Model

### Neo4j Nodes

```cypher
(:Challenge {
    id: "uuid",
    name: "Convince the Guard",
    description: "Persuade the guard to let you pass",
    challenge_type: "SkillCheck",  // SkillCheck, AbilityCheck, SavingThrow, OpposedCheck
    difficulty: "Medium",
    difficulty_class: 15,
    active: true,
    is_favorite: false,
    order: 1
})

(:Skill {
    id: "uuid",
    name: "Persuasion",
    description: "Influence others through words",
    category: "Social",
    base_attribute: "Charisma",
    is_custom: false
})
```

### Neo4j Edges

```cypher
// Challenge requires skill
(challenge:Challenge)-[:REQUIRES_SKILL]->(skill:Skill)

// Challenge available at location/region
(challenge:Challenge)-[:AVAILABLE_AT_LOCATION {
    always_available: false,
    time_restriction: "Evening"
}]->(location:Location)

(challenge:Challenge)-[:AVAILABLE_AT_REGION]->(region:Region)

// Challenge prerequisite
(challenge:Challenge)-[:REQUIRES_COMPLETION_OF]->(prereq:Challenge)

// Challenge can unlock locations
(challenge:Challenge)-[:ON_SUCCESS_UNLOCKS]->(location:Location)
```

### Challenge Outcomes

```rust
pub struct ChallengeOutcomes {
    pub success: Outcome,
    pub failure: Outcome,
    pub partial: Option<Outcome>,
    pub critical_success: Option<Outcome>,
    pub critical_failure: Option<Outcome>,
}

pub struct Outcome {
    pub description: String,
    pub triggers: Vec<OutcomeTrigger>,
}

pub enum OutcomeTrigger {
    RevealInformation { info: String },
    EnableChallenge { challenge_id: String },
    DisableChallenge { challenge_id: String },
    ModifyCharacterStat { stat: String, amount: i32 },
    TriggerScene { scene_id: String },
    GiveItem { item_id: String },
    Custom { action: String },
}
```

### Rule System Support

| System | Difficulty | Resolution | Examples |
|--------|------------|------------|----------|
| D20 | DC number | Roll >= DC | D&D 5e, Pathfinder |
| D100 | Percentage | Roll <= skill% | Call of Cthulhu, RuneQuest |
| Narrative | Descriptor | Interpretation | Fate, PbtA, Kids on Bikes |

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/challenges` | List all challenges | ✅ |
| POST | `/api/worlds/{id}/challenges` | Create challenge | ✅ |
| GET | `/api/challenges/{id}` | Get challenge | ✅ |
| PUT | `/api/challenges/{id}` | Update challenge | ✅ |
| DELETE | `/api/challenges/{id}` | Delete challenge | ✅ |
| PUT | `/api/challenges/{id}/favorite` | Toggle favorite | ✅ |
| PUT | `/api/challenges/{id}/active` | Set active status | ✅ |
| GET | `/api/worlds/{id}/skills` | List skills | ✅ |

### WebSocket Messages

#### Client → Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `TriggerChallenge` | `challenge_id`, `target_pc_id` | DM triggers challenge |
| `ChallengeRoll` | `challenge_id`, `roll_result`, `modifier` | Player submits roll |
| `ChallengeSuggestionDecision` | `suggestion_id`, `approved`, `modified_dc` | DM approves suggestion |
| `ChallengeOutcomeDecision` | `outcome_id`, `approved`, `modified_text` | DM approves outcome |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `ChallengePrompt` | `challenge`, `target_pc`, `skill` | Challenge started |
| `ChallengeResolved` | `challenge_id`, `result`, `outcome` | Challenge completed |
| `ChallengeOutcomePending` | `outcome_id`, `details` | Awaiting DM approval |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Challenge Entity | ✅ | ✅ | Full outcome support |
| Skill Entity | ✅ | ✅ | Categories, attributes |
| Challenge Repository | ✅ | - | Neo4j with skill edges |
| ChallengeService | ✅ | ✅ | Use cases exist; WebSocket CRUD missing |
| ChallengeResolutionService | ✅ | - | Dice, modifiers |
| ChallengeOutcomeApprovalService | ✅ | - | DM approval |
| LLM Challenge Suggestions | ✅ | - | Parse XML tags |
| WebSocket Request Handlers | ⏳ | - | Challenge/Skill requests not wired |
| Challenge Library UI | - | ✅ | Search, filter, favorites |
| Challenge Roll Modal | - | ✅ | Dice rolling |
| Trigger Challenge Modal | - | ✅ | Manual triggering |
| Outcome Approval UI | - | ✅ | DM approval |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/challenge.rs` | Challenge entity |
| Domain | `crates/domain/src/entities/skill.rs` | Skill entity |
| Entity | `crates/engine/src/entities/challenge.rs` | Challenge operations |
| Use Case | `crates/engine/src/use_cases/challenge/resolve.rs` | Resolution |
| Use Case | `crates/engine/src/use_cases/approval/challenge_outcome.rs` | Approval |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/challenge_repo.rs` | Neo4j persistence |
| API | `crates/engine/src/api/websocket/mod.rs` | WebSocket handlers |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `crates/player/src/application/services/challenge_service.rs` | API calls |
| Presentation | `crates/player/src/ui/presentation/components/dm_panel/challenge_library/` | Library UI |
| Presentation | `crates/player/src/ui/presentation/components/tactical/challenge_roll.rs` | Roll modal |
| Presentation | `crates/player/src/ui/presentation/components/dm_panel/trigger_challenge_modal.rs` | Trigger UI |
| Presentation | `crates/player/src/ui/presentation/components/dm_panel/challenge_outcome_approval.rs` | Approval |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (skill modifiers), [Navigation System](./navigation-system.md) (location binding)
- **Used by**: [Dialogue System](./dialogue-system.md) (LLM suggestions), [Narrative System](./narrative-system.md) (trigger conditions)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-18 | Initial version extracted from MVP.md |
