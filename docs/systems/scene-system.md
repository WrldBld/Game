# Scene System

## Overview

The Scene System manages the visual novel presentation layer. Scenes combine a backdrop image, character sprites, dialogue, and player interactions. When a player is in a region, the system resolves which scene to display based on location, time, and conditions.

---

## Game Design

The visual novel format provides:

1. **Immersive Presentation**: Full-screen backdrops with positioned character sprites
2. **Character Focus**: One NPC speaks at a time with portrait highlighting
3. **Player Agency**: Dialogue choices and interaction buttons
4. **Cinematic Control**: DM can set atmosphere, featured characters, entry conditions

---

## User Stories

### Implemented

- [x] **US-SCN-001**: As a player, I see scenes with backdrop images
  - *Implementation*: Backdrop component renders scene backdrop_asset
  - *Files*: `crates/player-ui/src/presentation/components/visual_novel/backdrop.rs`

- [x] **US-SCN-002**: As a player, I see character sprites positioned in the scene
  - *Implementation*: CharacterLayer renders sprites at positions from CharacterLayerData
  - *Files*: `crates/player-ui/src/presentation/components/visual_novel/character_sprite.rs`

- [x] **US-SCN-003**: As a player, I see dialogue with typewriter animation
  - *Implementation*: DialogueBox with char-by-char reveal, configurable speed
  - *Files*: `crates/player-ui/src/presentation/components/visual_novel/dialogue_box.rs`

- [x] **US-SCN-004**: As a player, I can select dialogue choices
  - *Implementation*: ChoiceMenu renders choices, sends PlayerAction on selection
  - *Files*: `crates/player-ui/src/presentation/components/visual_novel/choice_menu.rs`

- [x] **US-SCN-005**: As a player, I can interact with scene elements (talk, examine, travel)
  - *Implementation*: ActionPanel with interaction buttons based on scene data
  - *Files*: `crates/player-ui/src/presentation/components/action_panel.rs`

- [x] **US-SCN-006**: As a DM, I can create scenes tied to locations
  - *Implementation*: Scene entity with AT_LOCATION edge
  - *Files*: `crates/domain/src/entities/scene.rs`

- [x] **US-SCN-007**: As a DM, I can feature characters in scenes with roles
  - *Implementation*: FEATURES_CHARACTER edge with role and entrance_cue
  - *Files*: `crates/engine/src/infrastructure/neo4j/scene_repo.rs`

- [x] **US-SCN-008**: As a DM, scenes resolve based on PC location
  - *Implementation*: Scene entity `resolve_scene()` method, integrated in EnterRegion use case
  - *Files*: `crates/engine/src/entities/scene.rs`, `crates/engine/src/use_cases/movement/enter_region.rs`

- [x] **US-SCN-009**: As a DM, I can set entry conditions for scenes
  - *Implementation*: SceneCondition enum fully evaluated in Scene::resolve_scene()
  - *Conditions*: CompletedScene, HasItem, KnowsCharacter, FlagSet, Custom
  - *Files*: `crates/engine/src/entities/scene.rs`, `crates/engine/src/infrastructure/neo4j/scene_repo.rs`

### Pending

- [x] **US-SCN-010**: Flag storage system for FlagSet condition
  - *Status*: Implemented - Flag entity with Neo4j storage in `flag.rs` and `flag_repo.rs`
  - *Files*: `crates/engine/src/entities/flag.rs`, `crates/engine/src/infrastructure/neo4j/flag_repo.rs`

---

## UI Mockups

### Visual Novel Scene

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │                      [BACKDROP IMAGE]                                │   │
│  │                    The Rusty Anchor Tavern                           │   │
│  │                                                                      │   │
│  │      [NPC Sprite]              [NPC Sprite]              [NPC Sprite]│   │
│  │       (dimmed)                 (speaking)                 (dimmed)   │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [Portrait] Marcus                                                    │   │
│  │                                                                      │   │
│  │ "Welcome to the Rusty Anchor. What brings you here tonight?"        │   │
│  │ [▌ typewriter cursor]                                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│  │ Talk       │ │ Examine    │ │ Travel     │ │ Character  │               │
│  │ [Marcus]   │ │ [Room]     │ │ [Exit]     │ │ [Sheet]    │               │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Scene with Choices

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                          [Scene backdrop + sprites]                         │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Marcus                                                               │   │
│  │                                                                      │   │
│  │ "I could tell you more about the Baron, but information has a       │   │
│  │  price. What are you willing to offer?"                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  ┌───────────────────────────────────────────────────────────────┐  │   │
│  │  │ "I have gold. Name your price."                                │  │   │
│  │  └───────────────────────────────────────────────────────────────┘  │   │
│  │                                                                      │   │
│  │  ┌───────────────────────────────────────────────────────────────┐  │   │
│  │  │ "Perhaps I can help you with something in return?"             │  │   │
│  │  └───────────────────────────────────────────────────────────────┘  │   │
│  │                                                                      │   │
│  │  ┌───────────────────────────────────────────────────────────────┐  │   │
│  │  │ "I'll find another way to get the information."                │  │   │
│  │  └───────────────────────────────────────────────────────────────┘  │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Scene Entry Conditions Editor (DM)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Scene: Meeting the Informant                                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Entry Conditions ──────────────────────────────────────────────────── │
│  Scene only appears when ALL conditions are met:                            │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ ✓ Completed Scene: "Tavern Introduction"                        [X]   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ ✓ Has Item: "Baron's Letter"                                    [X]   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ ✓ Flag Set: "talked_to_bartender"                               [X]   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  [+ Add Condition ▼]                                                        │
│  ┌────────────────────────┐                                                 │
│  │ Completed Scene        │                                                 │
│  │ Has Item               │                                                 │
│  │ Knows Character        │                                                 │
│  │ Flag Set               │                                                 │
│  │ Custom                 │                                                 │
│  └────────────────────────┘                                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending (Editor UI. Engine evaluates built-in conditions; `Custom` is treated as unmet.)

---

## Data Model

### Neo4j Nodes

```cypher
(:Scene {
    id: "uuid",
    name: "Meeting the Informant",
    time_context: "Evening",
    backdrop_override: null,
    directorial_notes: "{...}",
    order: 1
})

(:InteractionTemplate {
    id: "uuid",
    name: "Ask about the Baron",
    interaction_type: "Dialogue",
    prompt_hints: "The informant knows secrets about the Baron's past",
    is_available: true,
    order: 1
})
```

### Neo4j Edges

```cypher
// Scene is at a location
(scene:Scene)-[:AT_LOCATION]->(location:Location)

// Scene belongs to act
(scene:Scene)-[:BELONGS_TO_ACT]->(act:Act)

// Scene features characters
(scene:Scene)-[:FEATURES_CHARACTER {
    role: "Primary",
    entrance_cue: "Already present when scene begins"
}]->(character:Character)

// Interaction belongs to scene
(interaction:InteractionTemplate)-[:BELONGS_TO_SCENE]->(scene:Scene)

// Interaction targets
(interaction:InteractionTemplate)-[:TARGETS_CHARACTER]->(character:Character)
(interaction:InteractionTemplate)-[:TARGETS_ITEM]->(item:Item)
(interaction:InteractionTemplate)-[:TARGETS_REGION]->(region:Region)

// Interaction requirements
(interaction:InteractionTemplate)-[:REQUIRES_ITEM]->(item:Item)
(interaction:InteractionTemplate)-[:REQUIRES_CHARACTER_PRESENT]->(character:Character)
```

### Scene Resolution

```rust
pub struct SceneResolutionService {
    // ...
}

impl SceneResolutionService {
    /// Resolve which scene to display for a PC at a given location/region
    pub async fn resolve_scene(
        &self,
        pc_id: &PlayerCharacterId,
        location_id: &LocationId,
        region_id: Option<&RegionId>,
        time_of_day: TimeOfDay,
    ) -> Result<Option<Scene>> {
        // 1. Find scenes at this location
        // 2. Filter by time_context if set
        // 3. Evaluate entry conditions
        // 4. Return highest-order matching scene
    }
}
```

### Character Presentation

```rust
pub struct CharacterLayerData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
    pub is_highlighted: bool,
}

pub enum CharacterPosition {
    Left,
    Center,
    Right,
    Custom { x: f32, y: f32 },
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/scenes` | List scenes | ✅ |
| POST | `/api/worlds/{id}/scenes` | Create scene | ✅ |
| GET | `/api/scenes/{id}` | Get scene | ✅ |
| PUT | `/api/scenes/{id}` | Update scene | ✅ |
| DELETE | `/api/scenes/{id}` | Delete scene | ✅ |
| GET | `/api/scenes/{id}/characters` | Get featured characters | ✅ |
| POST | `/api/scenes/{id}/characters` | Feature character | ✅ |
| GET | `/api/scenes/{id}/interactions` | List interactions | ✅ |

### WebSocket Messages

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `SceneUpdate` | `scene`, `characters`, `interactions` | Scene changed |
| `SceneChanged` | `region`, `npcs_present`, `navigation_options` | PC moved |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Scene Entity | ✅ | ✅ | Full property support |
| InteractionTemplate | ✅ | ✅ | Targets, requirements |
| Scene Repository | ✅ | - | Neo4j with all edges |
| SceneResolutionService | ✅ | - | Scene.resolve_scene() with condition evaluation |
| SceneService | ✅ | ✅ | CRUD operations |
| Backdrop Component | - | ✅ | Image rendering |
| CharacterLayer | - | ✅ | Sprite positioning |
| DialogueBox | - | ✅ | Typewriter animation |
| ChoiceMenu | - | ✅ | Choice selection |
| ActionPanel | - | ✅ | Interaction buttons |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/scene.rs` | Scene entity, SceneCondition enum |
| Entity | `crates/engine/src/entities/scene.rs` | Scene operations, resolve_scene() |
| Use Case | `crates/engine/src/use_cases/movement/enter_region.rs` | Scene resolution integration |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/scene_repo.rs` | Neo4j scene repo with COMPLETED_SCENE tracking |
| Infrastructure | `crates/engine/src/infrastructure/ports.rs` | SceneRepo trait |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `src/domain/entities/scene.rs` | Scene types |
| Domain | `src/domain/entities/character.rs` | Character display |
| Presentation | `src/presentation/components/visual_novel/backdrop.rs` | Backdrop |
| Presentation | `src/presentation/components/visual_novel/character_sprite.rs` | Sprites |
| Presentation | `src/presentation/components/visual_novel/dialogue_box.rs` | Dialogue |
| Presentation | `src/presentation/components/visual_novel/choice_menu.rs` | Choices |
| Presentation | `src/presentation/components/action_panel.rs` | Interactions |
| Presentation | `src/presentation/views/pc_view.rs` | Main PC view |

---

## Related Systems

- **Depends on**: [Navigation System](./navigation-system.md) (location resolution), [Character System](./character-system.md) (featured characters), [Dialogue System](./dialogue-system.md) (NPC responses)
- **Used by**: [Narrative System](./narrative-system.md) (scene triggers)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-18 | Initial version extracted from MVP.md |
