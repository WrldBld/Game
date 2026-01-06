# Neo4j Schema

## Overview

WrldBldr uses Neo4j as its primary database, storing all entities as nodes and relationships as edges. This graph-first design maximizes query flexibility and enables rich traversals for LLM context building.

---

## Design Principles

1. **Edges for Relationships**: Any reference to another entity becomes a Neo4j edge
2. **Properties on Edges**: Relationship metadata stored on edges (timestamps, reasons)
3. **JSON Only for Non-Relational**: Configuration blobs, spatial data, deeply nested templates
4. **Query-First Design**: Schema optimized for common graph traversals

---

## Node Types

Node labels and properties in this document are intended to reflect the live persistence layer in `crates/engine/src/infrastructure/neo4j/*`.

### World Structure

```cypher
(:World {
    id: "uuid",
    name: "The Shattered Realms",
    description: "A world torn apart...",
    rule_system: "{...}",  // JSON RuleSystemConfig
    created_at: datetime()
})

(:Act {
    id: "uuid",
    world_id: "uuid",
    name: "The Call",
    stage: "CallToAdventure",  // MonomythStage
    description: "The heroes receive their summons...",
    order_num: 1
})

(:Goal {
    id: "uuid",
    world_id: "uuid",
    name: "Family Honor Restored",
    description: "The stain cleansed"
})
```

### Locations & Regions

```cypher
(:Location {
    id: "uuid",
    world_id: "uuid",
    name: "The Rusty Anchor Tavern",
    description: "A dimly lit tavern...",
    location_type: "Interior",
    backdrop_asset: "/assets/backdrops/tavern.png",
    map_asset: "/assets/maps/tavern.png",
    parent_map_bounds: "{...}",  // JSON map bounds in parent map space
    default_region_id: "uuid",
    atmosphere: "Smoky, raucous",
    presence_cache_ttl_hours: 3,
    use_llm_presence: true
})

(:Region {
    id: "uuid",
    location_id: "uuid",
    name: "The Bar Counter",
    description: "A worn wooden counter...",
    backdrop_asset: "/assets/backdrops/bar.png",
    atmosphere: "Smoky",
    map_bounds: "{...}",  // JSON map bounds in location map space
    is_spawn_point: false,
    order: 1
})
```

### Characters

```cypher
(:Character {
    id: "uuid",
    world_id: "uuid",
    name: "Marcus the Redeemed",
    description: "A former mercenary...",
    sprite_asset: "/assets/sprites/marcus.png",
    portrait_asset: "/assets/portraits/marcus.png",
    base_archetype: "Ally",
    current_archetype: "Mentor",
    archetype_history: "[...]",  // JSON
    stats: "{...}",               // JSON
    is_alive: true,
    is_active: true
})

(:PlayerCharacter {
    id: "uuid",
    session_id: "uuid" | "",  // string; empty when standalone
    user_id: "user-123",
    world_id: "uuid",
    name: "Kira Shadowblade",
    description: "A vengeful warrior...",
    sheet_data: "{...}",  // JSON CharacterSheetData
    current_location_id: "uuid",
    current_region_id: "uuid" | "",
    starting_location_id: "uuid",
    sprite_asset: "/assets/sprites/kira.png",
    portrait_asset: "/assets/portraits/kira.png",
    created_at: datetime(),
    last_active_at: datetime()
})

(:Want {
    id: "uuid",
    description: "Avenge my family's murder",
    intensity: 0.9,
    known_to_player: false,
    created_at: datetime()
})

(:Item {
    id: "uuid",
    world_id: "uuid",
    name: "Sword of the Fallen",
    description: "A blade that once belonged...",
    item_type: "Weapon",
    is_unique: true,
    properties: "{...}",
    can_contain_items: false,     // Is this item a container?
    container_limit: null         // Max items if container (null = unlimited)
})

(:Skill {
    id: "uuid",
    world_id: "uuid",
    name: "Persuasion",
    description: "Influence others...",
    category: "Social",
    base_attribute: "Charisma",
    is_custom: false,
    is_hidden: false,
    skill_order: 1
})
```

### Scenes & Interactions

```cypher
(:Scene {
    id: "uuid",
    act_id: "uuid",
    name: "Meeting the Informant",
    location_id: "uuid",
    time_context: "{...}",          // JSON
    backdrop_override: "" | "...",
    entry_conditions: "[...]",      // JSON
    featured_characters: "[...]",   // JSON array of character ids (legacy, edges are source of truth)
    directorial_notes: "{...}",
    order_num: 1
})

(:Interaction {
    id: "uuid",
    scene_id: "uuid",
    name: "Ask about the Baron",
    interaction_type: "{...}",  // JSON
    target: "{...}",           // JSON (legacy; edges are source of truth)
    prompt_hints: "The informant knows secrets...",
    allowed_tools: "[...]",    // JSON
    conditions: "[...]",       // JSON
    is_available: true,
    order: 1
})
```

### Challenges

```cypher
(:Challenge {
    id: "uuid",
    world_id: "uuid",
    name: "Convince the Guard",
    description: "Persuade the guard...",
    challenge_type: "SkillCheck",
    difficulty_json: "{...}",            // JSON
    outcomes_json: "{...}",              // JSON
    triggers_json: "[...]",              // JSON
    active: true,
    challenge_order: 1,
    is_favorite: false,
    tags_json: "[...]"                   // JSON
})
```

### Events

```cypher
(:NarrativeEvent {
    id: "uuid",
    world_id: "uuid",
    name: "The Baron's Arrival",
    description: "The Baron arrives...",
    tags_json: "[...]",          // JSON
    triggers_json: "[...]",      // JSON
    trigger_logic: "All" | "Any" | "AtLeast(2)",
    scene_direction: "The door swings open...",
    suggested_opening: "" | "Well, well...",
    outcomes_json: "[...]",      // JSON
    default_outcome: "" | "...",
    is_active: true,
    is_triggered: false,
    triggered_at: "" | datetime(),
    selected_outcome: "" | "...",
    is_repeatable: false,
    trigger_count: 0,
    delay_turns: 0,
    expires_after_turns: -1,
    priority: 10,
    is_favorite: false,
    created_at: datetime(),
    updated_at: datetime()
})

(:EventChain {
    id: "uuid",
    world_id: "uuid",
    name: "The Baron's Downfall",
    description: "Events leading to...",
    events: ["uuid", "uuid"],
    is_active: true,
    current_position: 0,
    completed_events: ["uuid"],
    act_id: "" | "uuid",
    tags_json: "[...]",       // JSON
    color: "#FF5733",
    is_favorite: false,
    created_at: datetime(),
    updated_at: datetime()
})

(:StoryEvent {
    id: "uuid",
    world_id: "uuid",
    event_type_json: "{...}",  // JSON
    timestamp: datetime(),
    game_time: "" | "Day 3, Evening",
    summary: "Kira spoke with Marcus...",
    is_hidden: false,
    tags_json: "[...]"         // JSON
})
```

### Assets

```cypher
(:GalleryAsset {
    id: "uuid",
    entity_type: "Character" | "Location" | "Item",
    entity_id: "uuid",
    asset_type: "Portrait" | "Sprite" | "Backdrop" | "Map",
    file_path: "/assets/generated/abc.png",
    is_active: true,
    label: "" | "...",
    generation_metadata: "{...}",  // JSON
    created_at: datetime()
})

(:GenerationBatch {
    id: "uuid",
    world_id: "uuid",
    entity_type: "Character" | "Location" | "Item",
    entity_id: "uuid",
    asset_type: "Portrait" | "Sprite" | "Backdrop" | "Map",
    workflow: "...",
    prompt: "...",
    negative_prompt: "" | "...",
    count: 4,
    status: "{...}",            // JSON
    assets: "[...]",            // JSON array of asset ids
    style_reference_id: "" | "uuid",
    requested_at: datetime(),
    completed_at: "" | datetime()
})

(:WorkflowConfiguration {
    id: "uuid",
    slot: "portrait",            // unique key (composition root)
    name: "Portrait v1",
    workflow_json: "{...}",      // JSON
    prompt_mappings: "[...]",    // JSON
    input_defaults: "[...]",     // JSON
    locked_inputs: "[...]",      // JSON
    created_at: datetime(),
    updated_at: datetime()
})
```

### Character Sheets

```cypher
(:SheetTemplate {
    id: "uuid",
    world_id: "uuid",
    name: "D&D 5e Character Sheet",
    sections: "{...}",           // JSON - deeply nested template sections
    created_at: datetime(),
    updated_at: datetime()
})
```

### Tactical Maps

```cypher
(:GridMap {
    id: "uuid",
    location_id: "uuid",
    tiles: "{...}",              // JSON - 2D spatial tile data
    width: 20,
    height: 15,
    created_at: datetime()
})
```

### Staging (NPC Presence)

```cypher
(:Staging {
    id: "uuid",
    region_id: "uuid",
    location_id: "uuid",
    world_id: "uuid",
    game_time: datetime,         // Game time when approved
    approved_at: datetime,       // Real time when approved
    ttl_hours: 3,                // How long valid in game hours
    approved_by: "client_id",    // Who approved
    source: "llm",               // "rule" | "llm" | "custom" | "prestaged"
    dm_guidance: "" | "...",     // Optional guidance for regeneration
    is_active: true              // Current active staging for region
})
```

---

## Edge Types

### World Structure

```cypher
(world)-[:CONTAINS_ACT {order: 1}]->(act)
(world)-[:CONTAINS_LOCATION]->(location)
(world)-[:CONTAINS_CHARACTER]->(character)
(world)-[:CONTAINS_SKILL]->(skill)
(world)-[:CONTAINS_CHALLENGE]->(challenge)
(world)-[:CONTAINS_GOAL]->(goal)
(act)-[:CONTAINS_SCENE {order: 1}]->(scene)
```

### Location Hierarchy

```cypher
(parent)-[:CONTAINS_LOCATION]->(child)
(from)-[:CONNECTED_TO {
    connection_type: "Door",
    description: "A heavy oak door",
    bidirectional: true,
    is_locked: false,
    lock_description: ""
}]->(to)
(location)-[:HAS_REGION]->(region)
(region)-[:CONNECTED_TO_REGION {
    description: "Door to back room",
    bidirectional: true,
    is_locked: false,
    lock_description: ""
}]->(other)
(region)-[:EXITS_TO_LOCATION {
    description: "Exit to market",
    arrival_region_id: "uuid",
    is_locked: false,
    lock_description: ""
}]->(location)
```

### Character Position

```cypher
(pc)-[:AT_LOCATION]->(location)
(pc)-[:STARTED_AT]->(location)

(pc)-[:IN_WORLD]->(world)
(session)-[:HAS_PC]->(pc)
```

> Note: the current persistence layer tracks location via `AT_LOCATION`/`STARTED_AT` edges and also stores `current_location_id`/`current_region_id` as properties on `PlayerCharacter` for convenience. There are no `CURRENTLY_AT` / `CURRENTLY_IN_REGION` / `STARTED_IN_REGION` edges today.

### NPC-Location Relationships

```cypher
(npc)-[:HOME_LOCATION {description: "..."}]->(location)
(npc)-[:WORKS_AT {role: "Bartender", schedule: "Evenings"}]->(location)
(npc)-[:FREQUENTS {frequency: "Often", time_of_day: "Evening", reason: "..."}]->(location)
(npc)-[:AVOIDS {reason: "Bad memories"}]->(location)
(npc)-[:WORKS_AT_REGION {shift: "day", role: "..."}]->(region)
(npc)-[:FREQUENTS_REGION {frequency: "often", time_of_day: "Evening"}]->(region)
(npc)-[:HOME_REGION]->(region)
(npc)-[:AVOIDS_REGION {reason: "..."}]->(region)
```

### Social Relationships

```cypher
(from)-[:RELATES_TO {
    relationship_type: "Rivalry",
    sentiment: -0.7,
    known_to_player: true,
    established_at: datetime()
}]->(to)
```

### Actantial Model

```cypher
(character)-[:HAS_WANT {priority: 1}]->(want)
(want)-[:TARGETS]->(target)  // Character, Item, or Goal
(subject)-[:VIEWS_AS_HELPER {want_id: "...", reason: "..."}]->(helper)
(subject)-[:VIEWS_AS_OPPONENT {want_id: "...", reason: "..."}]->(opponent)
(subject)-[:VIEWS_AS_SENDER {want_id: "...", reason: "..."}]->(sender)
(subject)-[:VIEWS_AS_RECEIVER {want_id: "...", reason: "..."}]->(receiver)
```

### Inventory

```cypher
// NPC inventory (legacy - may not be fully implemented)
(character)-[:POSSESSES {
    quantity: 1,
    equipped: true,
    acquired_at: datetime(),
    acquisition_method: "Inherited"
}]->(item)

// PC inventory (US-INV-001)
(playerCharacter)-[:POSSESSES {
    quantity: 1,
    equipped: false,
    acquired_at: datetime(),
    acquisition_method: "Gifted" | "Purchased" | "Found" | "Crafted" | "Inherited" | "Stolen" | "Rewarded"
}]->(item)

// Container system - items can contain other items (US-INV-001)
(containerItem)-[:CONTAINS {
    quantity: 1,
    added_at: datetime()
}]->(item)
```

> Note: Region item placement uses `(Region)-[:CONTAINS_ITEM]->(Item)` - see US-REGION-ITEMS (not yet implemented).

### Archetype History

```cypher
(character)-[:ARCHETYPE_CHANGED {
    from_archetype: "Hero",
    to_archetype: "Shadow",
    reason: "Consumed by vengeance",
    changed_at: datetime(),
    order: 1
}]->(character)
```

### Scene Relationships

```cypher
(act)-[:CONTAINS_SCENE]->(scene)
(scene)-[:AT_LOCATION]->(location)
(scene)-[:FEATURES_CHARACTER {role: "Primary", entrance_cue: "..."}]->(character)

(scene)-[:HAS_INTERACTION]->(interaction)
(interaction)-[:TARGETS_CHARACTER]->(character)
(interaction)-[:TARGETS_ITEM]->(item)
(interaction)-[:TARGETS_REGION]->(region)
(interaction)-[:REQUIRES_ITEM {consumed: false}]->(item)
(interaction)-[:REQUIRES_CHARACTER_PRESENT]->(character)
```

### Challenge Relationships

```cypher
(world)-[:CONTAINS_CHALLENGE]->(challenge)
(challenge)-[:REQUIRES_SKILL]->(skill)

(challenge)-[:AVAILABLE_AT {always_available: false, time_restriction: "Evening"}]->(location)
(challenge)-[:AVAILABLE_AT_REGION {always_available: true, time_restriction: null}]->(region)
(challenge)-[:TIED_TO_SCENE]->(scene)

(challenge)-[:REQUIRES_COMPLETION_OF {success_required: true}]->(prerequisite)
(challenge)-[:ON_SUCCESS_UNLOCKS]->(location)
```

> Note: Challenges can be bound to both Locations (coarse) and Regions (fine-grained) via `AVAILABLE_AT` and `AVAILABLE_AT_REGION` edges respectively.

### Event Relationships

```cypher
(world)-[:HAS_NARRATIVE_EVENT]->(event)
(world)-[:HAS_EVENT_CHAIN]->(chain)

(event)-[:TIED_TO_LOCATION]->(location)
(event)-[:TIED_TO_SCENE]->(scene)
(event)-[:BELONGS_TO_ACT]->(act)
(event)-[:FEATURES_NPC]->(character)

(chain)-[:CONTAINS_EVENT {position: 1, is_completed: false}]->(event)
```

> Note: narrative triggers/outcomes/effects are currently stored as JSON on the `NarrativeEvent` node (`triggers_json`, `outcomes_json`). Edges like `TRIGGERED_BY_*`, `EFFECT_*`, `ENABLES_CHALLENGE`, etc. are not created by the current persistence layer.

### Story Event Relationships

```cypher
(world)-[:HAS_STORY_EVENT]->(story_event)

(story_event)-[:OCCURRED_AT]->(location)
(story_event)-[:OCCURRED_IN_SCENE]->(scene)
(story_event)-[:OCCURRED_IN_SESSION]->(session)
(story_event)-[:INVOLVES {role: "Speaker"}]->(character)

(story_event)-[:TRIGGERED_BY_NARRATIVE]->(narrative_event)
(story_event)-[:RECORDS_CHALLENGE]->(challenge)
```

> Note: `Session` nodes are referenced by queries/edges (e.g. `OCCURRED_IN_SESSION`, `HAS_PC`), but this repository does not currently create them (`CREATE (s:Session ...)` is absent).

### Observation

Observation is currently persisted as properties on `Character` (see `wrldbldr_domain::entities::NpcObservation`) rather than via an `OBSERVED_NPC` edge.

### Scene Completion

```cypher
(playerCharacter)-[:COMPLETED_SCENE]->(scene)
```

### Region Items

```cypher
(region:Region)-[:CONTAINS_ITEM {
    quantity: 1,
    added_at: datetime()
}]->(item:Item)
```

### NPC Disposition

```cypher
(character:Character)-[:DISPOSITION_TOWARD {
    base_disposition: "neutral",
    current_disposition: "friendly",
    relationship_level: 2,
    last_updated: datetime()
}]->(playerCharacter:PlayerCharacter)
```

### Dialogue History

```cypher
(playerCharacter:PlayerCharacter)-[:SPOKE_TO {
    last_dialogue_at: datetime(),
    last_topic: "...",
    conversation_count: 5
}]->(character:Character)
```

### Staging (NPC Presence)

```cypher
// Current active staging for a region
(region:Region)-[:CURRENT_STAGING]->(staging:Staging)

// All stagings for a region (history)
(region:Region)-[:HAS_STAGING]->(staging:Staging)

// NPCs included in a staging
(staging:Staging)-[:INCLUDES_NPC {
    is_present: true,
    is_hidden_from_players: false,
    reasoning: "Works here during evening shift"
}]->(character:Character)
```

### Assets

```cypher
(entity)-[:HAS_ASSET]->(galleryAsset:GalleryAsset)
```

> Note: Entity can be Character, Location, or Item.

---

## Example Queries

### Get NPC Full Context

```cypher
MATCH (npc:Character {id: $npc_id})
OPTIONAL MATCH (npc)-[hw:HAS_WANT]->(want:Want)
OPTIONAL MATCH (want)-[:TARGETS]->(target)
OPTIONAL MATCH (npc)-[vh:VIEWS_AS_HELPER]->(helper)
OPTIONAL MATCH (npc)-[vo:VIEWS_AS_OPPONENT]->(opponent)
OPTIONAL MATCH (npc)-[home:HOME_LOCATION]->(homeLoc)
OPTIONAL MATCH (npc)-[work:WORKS_AT]->(workLoc)
OPTIONAL MATCH (npc)-[freq:FREQUENTS]->(freqLoc)
OPTIONAL MATCH (npc)-[rel:RELATES_TO]->(other)
OPTIONAL MATCH (npc)-[poss:POSSESSES]->(item)
RETURN npc,
       collect(DISTINCT {want: want, target: target}) as wants,
       collect(DISTINCT helper) as helpers,
       collect(DISTINCT opponent) as opponents
```

### Find NPCs at Region

```cypher
MATCH (region:Region {id: $region_id})
OPTIONAL MATCH (npc:Character)-[w:WORKS_AT_REGION]->(region)
WHERE npc.is_active AND (w.shift = "always" OR w.shift = $shift)
OPTIONAL MATCH (npc2:Character)-[h:HOME_REGION]->(region)
WHERE npc2.is_active AND $time_of_day = "Night"
OPTIONAL MATCH (npc3:Character)-[f:FREQUENTS_REGION]->(region)
WHERE npc3.is_active AND (f.time_of_day = "Any" OR f.time_of_day = $time_of_day)
RETURN collect(DISTINCT npc) + collect(DISTINCT npc2) + collect(DISTINCT npc3)
```

---

## Schema Bootstrap (Startup)

Engine startup calls `Neo4jConnection::initialize_schema()` which runs a set of idempotent `CREATE CONSTRAINT ... IF NOT EXISTS` and `CREATE INDEX ... IF NOT EXISTS` statements.

### Constraints

- `World(id)` unique
- `Location(id)` unique
- `Region(id)` unique
- `Character(id)` unique
- `Want(id)` unique
- `PlayerCharacter(id)` unique
- `Scene(id)` unique
- `Act(id)` unique
- `Interaction(id)` unique
- `Skill(id)` unique
- `Challenge(id)` unique
- `StoryEvent(id)` unique
- `NarrativeEvent(id)` unique
- `EventChain(id)` unique
- `SheetTemplate(id)` unique
- `Item(id)` unique
- `GridMap(id)` unique
- `Staging(id)` unique
- `GalleryAsset(id)` unique
- `GenerationBatch(id)` unique
- `WorkflowConfiguration(slot)` unique

### Indexes

- `World(name)`
- `Character(name)`
- `Location(name)`
- `Character(world_id)`
- `Location(world_id)`
- `Region(location_id)`
- `Skill(world_id)`
- `Challenge(world_id)`
- `StoryEvent(world_id)`
- `NarrativeEvent(world_id)`
- `EventChain(world_id)`
- `Scene(act_id)`
- `SheetTemplate(world_id)`
- `PlayerCharacter(world_id)`
- `PlayerCharacter(session_id)`
- `Staging(world_id)`
- `GenerationBatch(world_id)`

---

## Acceptable JSON Blobs

| Entity                 | Field                  | Reason                    |
| ---------------------- | ---------------------- | ------------------------- |
| GridMap                | `tiles`                | 2D spatial data           |
| CharacterSheetTemplate | `sections`             | Deeply nested template    |
| CharacterSheetData     | Full sheet             | Per ADR-001, form data    |
| WorkflowConfiguration  | `workflow_json`        | ComfyUI format            |
| RuleSystemConfig       | Full config            | System configuration      |
| DirectorialNotes       | Full notes             | Scene metadata            |
| NarrativeEvent         | `triggers`, `outcomes` | Complex nested structures |
| StoryEvent             | `event_type`           | Discriminated union       |

---

## Related Documents

- [Hexagonal Architecture](./hexagonal-architecture.md) - Repository pattern
- [System Documents](../systems/) - Entity details per system
