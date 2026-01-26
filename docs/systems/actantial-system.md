# Actantial Model System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


The Actantial Model System implements a **structured NPC motivation framework** based on Greimas's actantial model from semiotics. It defines what NPCs want, who helps or opposes them, and provides behavioral guidance for the LLM to generate consistent, motivated dialogue.

---

## Game Design

The actantial model enables deep NPC characterization:

1. **Wants**: What NPCs desire (objects of desire)
2. **Targets**: Who or what the want is directed toward (Character, Item, or Goal)
3. **Actantial Views**: How NPCs perceive others relative to their wants (Helper, Opponent, Sender, Receiver)
4. **Visibility**: Whether the player knows about motivations (Known, Suspected, Secret)
5. **Behavioral Tells**: Subtle cues that hint at secret motivations
6. **Deflection Behaviors**: How NPCs redirect conversation when wants are probed

### Greimas's Six Actants

| Actant | Role | Example |
|--------|------|---------|
| Subject | The desiring character | Marcus the Bartender |
| Object | What is desired | Revenge, Wealth, Love |
| Sender | Who/what initiated the desire | A murdered father, prophecy |
| Receiver | Who benefits if desire is achieved | Family, kingdom, self |
| Helper | Who assists the subject | Allies, mentors, tools |
| Opponent | Who blocks the subject | Rivals, obstacles, inner demons |

---

## User Stories

### Implemented

- [x] **US-ACT-001**: As a DM, I can define wants for NPCs
- [x] **US-ACT-002**: As a DM, I can set want targets (Character, Item, or Goal)
- [x] **US-ACT-003**: As a DM, I can define how NPCs view other characters (Helper/Opponent/Sender/Receiver)
- [x] **US-ACT-004**: As a DM, I can set want visibility (Known, Suspected, Secret)
- [x] **US-ACT-005**: As a DM, I can define behavioral tells for secret wants
- [x] **US-ACT-006**: As a DM, I can define deflection behaviors for when wants are probed
- [x] **US-ACT-007**: As a DM, I can get LLM suggestions for want descriptions
- [x] **US-ACT-008**: As a DM, I can get LLM suggestions for actantial view reasons
- [x] **US-ACT-009**: As a DM, I can get LLM suggestions for behavioral tells
- [x] **US-ACT-010**: As a DM, I can get LLM suggestions for deflection behaviors
- [x] **US-ACT-011**: The LLM receives actantial context when generating NPC dialogue

### World Goals

- [x] **US-ACT-012**: As a DM, I can create abstract Goals that NPCs can target
- [x] **US-ACT-013**: As a DM, I can see all goals for a world
- [x] **US-ACT-014**: As a DM, I can delete goals (warns if wants target them)

---

## Data Model

### Neo4j Nodes

```cypher
(:Want {
    id: "uuid",
    description: "Avenge my family's murder",
    intensity: 0.9,                    // 0.0-1.0, how strongly felt
    visibility: "Hidden",              // Known, Suspected, or Hidden
    deflection_behavior: "...",        // How NPC deflects when probed
    tells: "[...]",                    // JSON array of behavioral tells
    created_at: datetime()
})

(:Goal {
    id: "uuid",
    world_id: "uuid",
    name: "Family Honor Restored",
    description: "The stain cleansed from the family name"
})
```

### Neo4j Edges

```cypher
// NPC has a want (with priority for multiple wants)
(character:Character)-[:HAS_WANT {priority: 1}]->(want:Want)

// Want targets something (Character, Item, or Goal)
(want:Want)-[:TARGETS]->(target)

// Actantial views - how the NPC sees others relative to a want
(subject:Character)-[:VIEWS_AS_HELPER {
    want_id: "uuid",
    reason: "Saved my life once"
}]->(helper:Character)

(subject:Character)-[:VIEWS_AS_OPPONENT {
    want_id: "uuid", 
    reason: "Competes for the same goal"
}]->(opponent:Character)

(subject:Character)-[:VIEWS_AS_SENDER {
    want_id: "uuid",
    reason: "Father's dying wish"
}]->(sender:Character)

(subject:Character)-[:VIEWS_AS_RECEIVER {
    want_id: "uuid",
    reason: "My children will benefit"
}]->(receiver:Character)

// World contains goals
(world:World)-[:CONTAINS_GOAL]->(goal:Goal)
```

### Domain Types

```rust
/// Visibility level of a want
pub enum WantVisibility {
    /// Player knows this motivation openly
    Known,
    /// Player senses something but doesn't know details
    Suspected,  
    /// Player has no idea
    Hidden,
}

/// Actantial role in relation to a want
pub enum ActantialRole {
    Helper,
    Opponent,
    Sender,
    Receiver,
}

/// Target of a want
pub enum WantTargetType {
    Character,
    Item,
    Goal,
}

pub enum WantTarget {
    Character { id: Uuid, name: String },
    Item { id: Uuid, name: String },
    Goal { id: Uuid, name: String, description: Option<String> },
}
```

---

## API

### WebSocket Messages

#### Server -> Client

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

#### LLM Suggestions

| Message | Fields | Purpose |
|---------|--------|---------|
| `DeflectionSuggestions` | `npc_id`, `want_id`, `suggestions` | Deflection behavior suggestions |
| `TellsSuggestions` | `npc_id`, `want_id`, `suggestions` | Behavioral tells suggestions |
| `WantDescriptionSuggestions` | `npc_id`, `suggestions` | Want description suggestions |
| `ActantialReasonSuggestions` | `npc_id`, `want_id`, `target_id`, `role`, `suggestions` | Actantial view reason suggestions |

### Request/Response (via `Request` message)

| RequestPayload | Purpose |
|----------------|---------|
| `GetActantialContext` | Get full actantial context for NPC |
| `ListWants` | List wants for a character |
| `GetWant` | Fetch a single want |
| `CreateWant` | Create a want for an NPC |
| `UpdateWant` | Update want properties |
| `DeleteWant` | Delete a want |
| `SetWantTarget` | Set want target (Character/Item/Goal) |
| `RemoveWantTarget` | Remove want target |
| `AddActantialView` | Add Helper/Opponent/Sender/Receiver view |
| `RemoveActantialView` | Remove actantial view |
| `ListGoals` | Get all goals for a world |
| `GetGoal` | Fetch a goal |
| `CreateGoal` | Create a goal |
| `UpdateGoal` | Update goal |
| `DeleteGoal` | Delete goal |
| `SuggestWantDescription` | Get LLM suggestions for want |
| `SuggestDeflection` | Get LLM suggestions for deflection |
| `SuggestTells` | Get LLM suggestions for tells |
| `SuggestActantialReason` | Get LLM suggestions for view reason |

---

## LLM Context Integration

The actantial model is wired into the LLM prompt context:

```rust
pub struct MotivationsContext {
    /// Known motivations (player knows about these)
    pub known: Vec<MotivationEntry>,
    /// Suspected motivations (player senses something)
    pub suspected: Vec<MotivationEntry>,
    /// Secret motivations (player has no idea)
    pub secret: Vec<SecretMotivationEntry>,
}

pub struct SecretMotivationEntry {
    /// Description of the secret motivation
    pub description: String,
    /// Who/what initiated this motivation
    pub sender: Option<String>,
    /// Subtle behavioral tells that hint at this motivation
    pub tells: Vec<String>,
}
```

### Example LLM Prompt Context

```
## Marcus's Motivations

Known:
- Wants to pay off debts to the thieves' guild

Suspected:
- Seems to have romantic interest in someone...

Secret:
- Secretly wants to avenge his brother's death
  - Sender: His brother's dying words
  - Behavioral tells: "glances at the door whenever guards pass", "changes subject when mercenaries mentioned"
  - Deflection: "When pressed about his past, Marcus busies himself with glasses and deflects with jokes"

## Marcus's Social Views (for this want)

Helpers:
- Kira Shadowblade: "Saved my life once - I owe her"

Opponents:
- Baron Valdris: "He ordered my brother's death"
```

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Want Entity | ✅ | - | `entities/want.rs` |
| Goal Entity | ✅ | - | `entities/goal.rs` |
| ActantialContext VO | ✅ | - | `value_objects/actantial_context.rs` |
| WantRepository | ✅ | - | Neo4j persistence |
| GoalRepository | ✅ | - | Neo4j persistence |
| CharacterWantPort | ✅ | - | Port traits |
| CharacterActantialPort | ✅ | - | Port traits |
| ActantialContextService | ✅ | - | Builds full context |
| Protocol Messages | ✅ | ✅ | WebSocket messages |
| WebSocket Request Handlers | ⏳ | - | Requests not wired in engine yet |
| LLM Context Integration | ✅ | - | `MotivationsContext` in prompts |
| LLM Suggestions | ⏳ | ✅ | Want/actantial reason wired; deflection/tells pending |
| NPC Motivations Panel | - | ✅ | DM UI for editing |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/want.rs` | Want entity |
| Domain | `crates/domain/src/entities/goal.rs` | Goal entity |
| Domain | `crates/domain/src/value_objects/actantial_context.rs` | Actantial context types |
| Engine | `crates/engine/src/llm_context.rs` | MotivationsContext for LLM |
| Infrastructure | `crates/engine/src/infrastructure/ports.rs` | Want, actantial, goal port traits |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/character_repo.rs` | Want/actantial persistence |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/goal_repo.rs` | Goal persistence |
| Use Case | `crates/engine/src/use_cases/actantial/mod.rs` | Actantial context operations |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Protocol | `crates/protocol/src/messages.rs` | Actantial messages |
| Presentation | `crates/player/src/ui/presentation/components/dm_panel/npc_motivation.rs` | Motivations panel |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (NPC entities), [Dialogue System](./dialogue-system.md) (LLM prompts)
- **Used by**: [Dialogue System](./dialogue-system.md) (NPC context in prompts), [NPC System](./npc-system.md) (character context)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-31 | Initial documentation |
