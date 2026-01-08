# Character System

## Overview

The Character System manages all characters in the game world: NPCs (Non-Player Characters) controlled by the AI/DM, and PCs (Player Characters) controlled by players. It implements Campbell's Hero's Journey archetypes for character roles and Greimas' Actantial Model for character motivations. The system also handles social relationships with sentiment tracking and item inventory.

---

## Game Design

Characters are the heart of storytelling. This system provides:

1. **Campbell Archetypes**: Each character plays a narrative role (Hero, Mentor, Shadow, etc.)
2. **Actantial Model**: Each character has their own view of who helps them, who opposes them, and what they desire
3. **Dynamic Relationships**: Sentiment changes based on player actions
4. **Inventory**: Characters can possess, give, and receive items

The key insight is that the same person can be a HELPER in one character's model and an OPPONENT in another's - these are one-way, per-character relationships.

---

## User Stories

### Implemented

- [x] **US-CHAR-001**: As a DM, I can create NPCs with Campbell archetypes
  - *Implementation*: Character entity with `base_archetype` and `current_archetype` fields
  - *Files*: `crates/domain/src/entities/character.rs`

- [x] **US-CHAR-002**: As a DM, I can define what a character wants (their desire/goal)
  - *Implementation*: Want entity with `HAS_WANT` edge, intensity 0.0-1.0
  - *Files*: `crates/domain/src/entities/want.rs`, `crates/engine/src/infrastructure/neo4j/character_repo.rs`

- [x] **US-CHAR-003**: As a DM, I can set who a character views as helper/opponent/sender/receiver
  - *Implementation*: `VIEWS_AS_*` edges with want_id and reason
  - *Files*: `crates/engine/src/infrastructure/neo4j/character_repo.rs`

- [x] **US-CHAR-004**: As a DM, I can define relationships between characters with sentiment
  - *Implementation*: `RELATES_TO` edge with sentiment (-1.0 to 1.0) and relationship_type
  - *Files*: `crates/domain/src/value_objects/relationship.rs`

- [x] **US-CHAR-005**: As a DM, I can change a character's archetype and track the history
  - *Implementation*: `ARCHETYPE_CHANGED` self-referential edge with timestamp
  - *Files*: `crates/engine/src/entities/character.rs`

- [x] **US-CHAR-006**: As a player, I can create a PC and bind it to a session
  - *Implementation*: PlayerCharacter entity with session binding, character sheet data
  - *Files*: `crates/domain/src/entities/player_character.rs`

- [x] **US-CHAR-007**: As a player, I can give/receive items from NPCs
  - *Implementation*: `POSSESSES` edge with quantity, equipped, acquisition_method
  - *Files*: `crates/domain/src/entities/item.rs`, `crates/engine/src/use_cases/conversation/tool_execution.rs`

- [x] **US-CHAR-008**: As a DM, I can view and edit character sheets based on rule system
  - *Implementation*: CharacterSheetTemplate with dynamic field types
  - *Files*: `crates/domain/src/entities/sheet_template.rs`, `crates/player-ui/src/presentation/components/character_sheet_viewer.rs`

- [x] **US-CHAR-009**: As a player, I can view my character's inventory
  - *Implementation*: Full inventory panel with item categories (All/Equipped/Consumables/Key) and actions
  - *Files*: `crates/player-ui/src/presentation/components/inventory_panel.rs`, `crates/engine/src/entities/inventory.rs`

### Pending

*No pending stories - all character system stories implemented.*

---

## UI Mockups

### Character Form (Creator Mode)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Create Character                                                    [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Name: [Marcus the Redeemed___________]                                     │
│                                                                             │
│  Description:                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ A former mercenary seeking redemption for past sins...              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Archetype: [Mentor          ▼]                                             │
│                                                                             │
│  ─── Wants ───────────────────────────────────────────────────────────────  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [+] Add Want                                                         │   │
│  │                                                                      │   │
│  │ 1. "Atone for the village massacre"   Intensity: [████████░░] 0.8   │   │
│  │    Target: [Goal: Redemption]                                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Character Sheet ─────────────────────────────────────────────────────  │
│  [▼ Expand]                                                                 │
│                                                                             │
│                                        [Cancel]  [Save Character]           │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Motivations Tab (Character Editor)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [Basic Info]  [Appearance]  [Backstory]  [▶ Motivations]  [Sheet]          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  WANTS                                                        [+ Add Want]  │
│  ───────────────────────────────────────────────────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ★ Priority 1                                    🔒 Hidden [Edit] [X]│   │
│  │ "Atone for the village massacre"                                    │   │
│  │ Target: Redemption (Goal)      Intensity: ████████░░ Strong (0.8)  │   │
│  │ ▼ Actantial Roles (Helpers, Opponents, Sender, Receiver)           │   │
│  │ ▼ Secret Behavior (deflection, behavioral tells)                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  GOALS (World Library)                                       [+ New Goal]  │
│  ───────────────────────────────────────────────────────────────────────── │
│  │ Redemption (2) │ Power (3) │ Peace (1) │ [+ Common Goals...]         │   │
│                                                                             │
│  SOCIAL STANCE (Aggregated)                                                │
│  ───────────────────────────────────────────────────────────────────────── │
│  │ Allies: Elena, Aldric │ Enemies: Lord Vorn                           │   │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (basic structure, refinement pending)
- Component: `crates/player-ui/src/presentation/components/creator/motivations_tab.rs`
- Integrated into character_form.rs for existing characters

### Actantial Model Viewer

```
         SENDER ──────────────▶ OBJECT ◀────────────── RECEIVER
      His father's            Redemption              The village
       dying wish                                     survivors
            │                      ▲                        │
            │                      │                        │
            ▼                      │                        ▼
         HELPER ◀──────────── MARCUS ─────────────▶ OPPONENT
      Kira (the PC)           (Subject)             Baron Valdris
```

**Status**: ⏳ Pending (visual diagram not implemented, data accessible via Motivations Tab)

### Inventory Panel (Player View)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Inventory                                                           [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Equipped ───────────────────────────────────────────────────────────── │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ ⚔️ Sword of the Fallen                                     [Unequip]  │ │
│  │    "A blade that once belonged to a fallen hero"                      │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ─── Items (3) ─────────────────────────────────────────────────────────── │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ 🔑 Rusty Key                                               [Use] [Drop]│ │
│  │    "An old key, might fit something in the tavern"                    │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ 📜 Baron's Letter                                     [Read] [Use] [Drop]│
│  │    "A sealed letter addressed to someone important"                   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ 💰 Gold Coins (15)                                                    │ │
│  │    "Standard currency"                                                │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (US-CHAR-009)

---

## Data Model

### Neo4j Nodes

```cypher
// NPC Character
(:Character {
    id: "uuid",
    name: "Marcus the Redeemed",
    description: "A former mercenary seeking redemption...",
    sprite_asset: "/assets/sprites/marcus.png",
    portrait_asset: "/assets/portraits/marcus.png",
    base_archetype: "Ally",
    current_archetype: "Mentor",
    is_alive: true,
    is_active: true
})

// Player Character
(:PlayerCharacter {
    id: "uuid",
    user_id: "user-123",
    name: "Kira Shadowblade",
    description: "A vengeful warrior...",
    sprite_asset: "/assets/sprites/kira.png",
    portrait_asset: "/assets/portraits/kira.png",
    sheet_data: "{...}",  // JSON CharacterSheetData
    created_at: datetime(),
    last_active_at: datetime()
})

// Want - Character desire
(:Want {
    id: "uuid",
    description: "Avenge my family's murder",
    intensity: 0.9,
    known_to_player: false,
    created_at: datetime()
})

// Goal - Abstract desire target
(:Goal {
    id: "uuid",
    name: "Family Honor Restored",
    description: "The stain on the family name is cleansed"
})

// Item - Possessable object
(:Item {
    id: "uuid",
    name: "Sword of the Fallen",
    description: "A blade that once belonged to a fallen hero",
    item_type: "Weapon",
    is_unique: true,
    properties: "{...}"
})
```

### Neo4j Edges

```cypher
// Character has a want
(character:Character)-[:HAS_WANT {
    priority: 1
}]->(want:Want)

// Want targets something
(want:Want)-[:TARGETS]->(target)
// target can be: Character, Item, or Goal

// Actantial relationships (per want, directional)
(subject:Character)-[:VIEWS_AS_HELPER {
    want_id: "uuid",
    reason: "Saved my life",
    assigned_at: datetime()
}]->(helper:Character)

(subject:Character)-[:VIEWS_AS_OPPONENT {
    want_id: "uuid",
    reason: "Killed my family"
}]->(opponent:Character)

(subject:Character)-[:VIEWS_AS_SENDER {
    want_id: "uuid",
    reason: "My father's dying wish"
}]->(sender:Character)

(subject:Character)-[:VIEWS_AS_RECEIVER {
    want_id: "uuid",
    reason: "My village will be safe"
}]->(receiver:Character)

// Social relationships
(from:Character)-[:RELATES_TO {
    relationship_type: "Mentorship",
    sentiment: 0.7,          // -1.0 to 1.0
    known_to_player: true,
    established_at: datetime()
}]->(to:Character)

// Inventory
(character:Character)-[:POSSESSES {
    quantity: 1,
    equipped: true,
    acquired_at: datetime(),
    acquisition_method: "Inherited"
}]->(item:Item)

// Archetype history
(character:Character)-[:ARCHETYPE_CHANGED {
    from_archetype: "Hero",
    to_archetype: "Shadow",
    reason: "Consumed by vengeance",
    changed_at: datetime(),
    order: 1
}]->(character:Character)
```

### Campbell Archetypes

| Archetype | Role | Typical Behaviors |
|-----------|------|-------------------|
| **Hero** | Protagonist | Faces challenges, grows, transforms |
| **Mentor** | Guide | Provides wisdom, training, gifts |
| **Threshold Guardian** | Tester | Challenges hero's commitment |
| **Herald** | Messenger | Announces change, call to adventure |
| **Shapeshifter** | Deceiver | Changes loyalties, creates doubt |
| **Shadow** | Antagonist | Represents dark side, opposes hero |
| **Trickster** | Chaos Agent | Brings humor, challenges thinking |
| **Ally** | Supporter | Provides help, companionship |

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/characters` | List characters | ✅ |
| POST | `/api/worlds/{id}/characters` | Create character | ✅ |
| GET | `/api/characters/{id}` | Get character | ✅ |
| PUT | `/api/characters/{id}` | Update character | ✅ |
| DELETE | `/api/characters/{id}` | Delete character | ✅ |
| GET | `/api/characters/{id}/wants` | List wants | ✅ |
| POST | `/api/characters/{id}/wants` | Create want | ✅ |
| PUT | `/api/wants/{id}` | Update want | ✅ |
| DELETE | `/api/wants/{id}` | Delete want | ✅ |
| PUT | `/api/wants/{id}/target` | Set want target | ✅ |
| DELETE | `/api/wants/{id}/target` | Remove want target | ✅ |
| GET | `/api/characters/{id}/actantial-context` | Get full context | ✅ |
| POST | `/api/characters/{id}/actantial-views` | Add actantial view | ✅ |
| POST | `/api/characters/{id}/actantial-views/remove` | Remove actantial view | ✅ |
| GET | `/api/worlds/{id}/goals` | List goals | ✅ |
| POST | `/api/worlds/{id}/goals` | Create goal | ✅ |
| GET | `/api/goals/{id}` | Get goal | ✅ |
| PUT | `/api/goals/{id}` | Update goal | ✅ |
| DELETE | `/api/goals/{id}` | Delete goal | ✅ |
| GET | `/api/characters/{id}/relationships` | Get relationships | ✅ |
| POST | `/api/characters/{id}/relationships` | Create relationship | ✅ |
| PUT | `/api/relationships/{id}` | Update relationship | ✅ |
| GET | `/api/worlds/{id}/player-characters` | List PCs | ✅ |
| POST | `/api/worlds/{id}/player-characters` | Create PC | ✅ |
| GET | `/api/player-characters/{id}` | Get PC | ✅ |

### WebSocket Messages

#### Client → Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `CreateNpcWant` | `npc_id`, `want` | Create a want for NPC |
| `UpdateNpcWant` | `npc_id`, `want_id`, `updates` | Update want properties |
| `DeleteNpcWant` | `npc_id`, `want_id` | Delete a want |
| `SetWantTarget` | `want_id`, `target_type`, `target_id` | Set want target |
| `RemoveWantTarget` | `want_id` | Remove want target |
| `AddActantialView` | `npc_id`, `want_id`, `role`, `target_id`, `target_type`, `reason` | Add helper/opponent/etc |
| `RemoveActantialView` | `npc_id`, `want_id`, `role`, `target_id`, `target_type` | Remove view |
| `GetNpcActantialContext` | `npc_id` | Request full context |
| `GetWorldGoals` | `world_id` | Request world goals |
| `CreateGoal` | `world_id`, `name`, `description` | Create goal |
| `UpdateGoal` | `goal_id`, `name`, `description` | Update goal |
| `DeleteGoal` | `goal_id` | Delete goal |
| `SuggestDeflectionBehavior` | `npc_id`, `want_id` | Request LLM suggestions |
| `SuggestBehavioralTells` | `npc_id`, `want_id` | Request LLM suggestions |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `CharacterUpdated` | `character_id`, `changes` | Character data changed |
| `RelationshipChanged` | `from_id`, `to_id`, `sentiment` | Relationship modified |
| `ItemTransferred` | `item_id`, `from_id`, `to_id` | Item given/taken |
| `NpcWantCreated` | `npc_id`, `want` | Want created (broadcast) |
| `NpcWantUpdated` | `npc_id`, `want` | Want updated (broadcast) |
| `NpcWantDeleted` | `npc_id`, `want_id` | Want deleted (broadcast) |
| `WantTargetSet` | `want_id`, `target` | Target set (broadcast) |
| `WantTargetRemoved` | `want_id` | Target removed (broadcast) |
| `ActantialViewAdded` | `npc_id`, `want_id`, `role`, `actor` | View added (broadcast) |
| `ActantialViewRemoved` | `npc_id`, `want_id`, `role`, `target_id` | View removed (broadcast) |
| `NpcActantialContextResponse` | `npc_id`, `context` | Full context response |
| `WorldGoalsResponse` | `world_id`, `goals` | Goals list response |
| `GoalCreated` | `world_id`, `goal` | Goal created (broadcast) |
| `GoalUpdated` | `goal` | Goal updated (broadcast) |
| `GoalDeleted` | `goal_id` | Goal deleted (broadcast) |
| `DeflectionSuggestions` | `npc_id`, `want_id`, `suggestions` | LLM suggestions |
| `TellsSuggestions` | `npc_id`, `want_id`, `suggestions` | LLM suggestions |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Character Entity | ✅ | ✅ | Full archetype support |
| PlayerCharacter Entity | ✅ | ✅ | Session binding |
| Want Entity | ✅ | ✅ | Visibility, deflection, tells |
| Goal Entity | ✅ | ✅ | Abstract targets with common defaults |
| Item Entity | ✅ | ✅ | Inventory support |
| Actantial Edges | ✅ | ✅ | All 4 role types, NPC+PC targets |
| Actantial Context Service | ✅ | - | Full aggregation logic |
| Relationship Edges | ✅ | ✅ | Sentiment tracking |
| Archetype History | ✅ | - | Change tracking |
| WebSocket Request Handlers | ⏳ | - | Character/PC/Goal/Want/Actantial requests not wired |
| Character Form | - | ✅ | UI exists; engine handlers missing |
| Motivations Tab | - | ✅ | UI exists; engine handlers missing |
| Character Sheet Viewer | - | ✅ | Read-only display |
| Inventory UI | - | ✅ | Full panel with categories |
| LLM Motivations Context | ✅ | - | Full context in prompts |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/character.rs` | Character entity |
| Domain | `crates/domain/src/entities/player_character.rs` | PC entity |
| Domain | `crates/domain/src/entities/want.rs` | Want entity with visibility, deflection, tells |
| Domain | `crates/domain/src/entities/goal.rs` | Goal entity with common goals |
| Domain | `crates/domain/src/entities/item.rs` | Item entity |
| Domain | `crates/domain/src/value_objects/archetype.rs` | CampbellArchetype |
| Domain | `crates/domain/src/value_objects/actantial_context.rs` | ActantialContext, WantContext |
| Domain | `crates/domain/src/value_objects/llm_context.rs` | MotivationsContext for LLM |
| Domain | `crates/domain/src/value_objects/relationship.rs` | Relationship types |
| Entity | `crates/engine/src/entities/character.rs` | Character operations |
| Entity | `crates/engine/src/entities/player_character.rs` | PC operations |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/character_repo.rs` | Neo4j character persistence |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/goal_repo.rs` | Neo4j goal persistence |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/player_character_repo.rs` | Neo4j PC persistence |
| API | `crates/engine/src/api/websocket/mod.rs` | RequestPayload handlers (not wired yet) |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `crates/player/src/application/services/character_service.rs` | Character API |
| Application | `crates/player/src/application/services/player_character_service.rs` | PC API |
| Application | `crates/player/src/application/services/actantial_service.rs` | Actantial API |
| Presentation | `crates/player/src/ui/presentation/components/creator/character_form.rs` | Character editor |
| Presentation | `crates/player/src/ui/presentation/components/creator/motivations_tab.rs` | Motivations tab |
| Presentation | `crates/player/src/ui/presentation/components/character_sheet_viewer.rs` | Sheet viewer |
| Presentation | `crates/player/src/ui/presentation/components/dm_panel/npc_motivation.rs` | NPC panel |

---

## Related Systems

- **Depends on**: None (foundational system)
- **Used by**: [Dialogue System](./dialogue-system.md) (NPC context), [Challenge System](./challenge-system.md) (skill modifiers), [Scene System](./scene-system.md) (featured characters), [NPC System](./npc-system.md) (presence rules)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-25 | Added Motivations Tab, actantial API routes, WebSocket messages |
| 2025-12-24 | Marked US-CHAR-009 complete |
| 2025-12-18 | Initial version extracted from MVP.md |
