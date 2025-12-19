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
    name: "The Call",
    stage: "CallToAdventure",  // MonomythStage
    description: "The heroes receive their summons...",
    order: 1
})

(:Goal {
    id: "uuid",
    name: "Family Honor Restored",
    description: "The stain cleansed"
})
```

### Locations & Regions

```cypher
(:Location {
    id: "uuid",
    name: "The Rusty Anchor Tavern",
    description: "A dimly lit tavern...",
    location_type: "Interior",
    backdrop_asset: "/assets/backdrops/tavern.png",
    atmosphere: "Smoky, raucous"
})

(:Region {
    id: "uuid",
    name: "The Bar Counter",
    description: "A worn wooden counter...",
    backdrop_asset: "/assets/backdrops/bar.png",
    atmosphere: "Smoky",
    map_bounds_x: 100,
    map_bounds_y: 200,
    map_bounds_width: 300,
    map_bounds_height: 150,
    is_spawn_point: false,
    order: 1
})
```

### Characters

```cypher
(:Character {
    id: "uuid",
    name: "Marcus the Redeemed",
    description: "A former mercenary...",
    sprite_asset: "/assets/sprites/marcus.png",
    portrait_asset: "/assets/portraits/marcus.png",
    base_archetype: "Ally",
    current_archetype: "Mentor",
    is_alive: true,
    is_active: true
})

(:PlayerCharacter {
    id: "uuid",
    user_id: "user-123",
    name: "Kira Shadowblade",
    description: "A vengeful warrior...",
    sheet_data: "{...}",  // JSON CharacterSheetData
    created_at: datetime()
})

(:Want {
    id: "uuid",
    description: "Avenge my family's murder",
    intensity: 0.9,
    known_to_player: false
})

(:Item {
    id: "uuid",
    name: "Sword of the Fallen",
    description: "A blade that once belonged...",
    item_type: "Weapon",
    is_unique: true,
    properties: "{...}"
})

(:Skill {
    id: "uuid",
    name: "Persuasion",
    description: "Influence others...",
    category: "Social",
    base_attribute: "Charisma",
    is_custom: false
})
```

### Scenes & Interactions

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
    prompt_hints: "The informant knows secrets...",
    is_available: true,
    order: 1
})
```

### Challenges

```cypher
(:Challenge {
    id: "uuid",
    name: "Convince the Guard",
    description: "Persuade the guard...",
    challenge_type: "SkillCheck",
    difficulty: "Medium",
    difficulty_class: 15,
    active: true,
    is_favorite: false
})
```

### Events

```cypher
(:NarrativeEvent {
    id: "uuid",
    name: "The Baron's Arrival",
    description: "The Baron arrives...",
    scene_direction: "The door swings open...",
    suggested_opening: "Well, well...",
    is_active: true,
    is_triggered: false,
    is_repeatable: false,
    trigger_count: 0,
    priority: 10
})

(:EventChain {
    id: "uuid",
    name: "The Baron's Downfall",
    description: "Events leading to...",
    is_active: true,
    current_position: 0,
    color: "#FF5733"
})

(:StoryEvent {
    id: "uuid",
    event_type: "DialogueExchange",
    timestamp: datetime(),
    game_time: "Day 3, Evening",
    summary: "Kira spoke with Marcus...",
    is_hidden: false,
    tags: ["plot", "marcus"]
})
```

### Assets

```cypher
(:GalleryAsset {
    id: "uuid",
    file_path: "/assets/generated/abc.png",
    thumbnail_path: "/assets/generated/abc_thumb.png",
    asset_type: "Portrait",
    entity_type: "Character",
    entity_id: "character-uuid",
    prompt: "A grizzled bartender...",
    workflow_slot: "portrait",
    is_active: true
})

(:GenerationBatch {
    id: "uuid",
    entity_type: "Character",
    entity_id: "character-uuid",
    asset_type: "Portrait",
    prompt: "...",
    count: 4,
    status: "processing",
    completed_count: 2
})

(:WorkflowConfiguration {
    id: "uuid",
    slot: "portrait",
    workflow_json: "{...}",
    prompt_mappings: "[...]",
    style_reference_mapping: "AutoDetect"
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
    is_locked: false
}]->(to)
(location)-[:HAS_REGION]->(region)
(region)-[:CONNECTED_TO_REGION {
    description: "Door to back room",
    bidirectional: true,
    is_locked: false
}]->(other)
(region)-[:EXITS_TO_LOCATION {
    description: "Exit to market",
    arrival_region_id: "uuid"
}]->(location)
```

### Character Position

```cypher
(pc)-[:CURRENTLY_AT]->(location)
(pc)-[:CURRENTLY_IN_REGION]->(region)
(pc)-[:STARTED_AT]->(location)
(pc)-[:STARTED_IN_REGION]->(region)
```

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
(character)-[:POSSESSES {
    quantity: 1,
    equipped: true,
    acquired_at: datetime(),
    acquisition_method: "Inherited"
}]->(item)
```

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
(scene)-[:AT_LOCATION]->(location)
(scene)-[:BELONGS_TO_ACT]->(act)
(scene)-[:FEATURES_CHARACTER {role: "Primary", entrance_cue: "..."}]->(character)
(interaction)-[:BELONGS_TO_SCENE]->(scene)
(interaction)-[:TARGETS_CHARACTER]->(character)
(interaction)-[:REQUIRES_ITEM]->(item)
(interaction)-[:REQUIRES_CHARACTER_PRESENT]->(character)
```

### Challenge Relationships

```cypher
(challenge)-[:REQUIRES_SKILL]->(skill)
(challenge)-[:AVAILABLE_AT_LOCATION {always_available: false, time_restriction: "Evening"}]->(location)
(challenge)-[:AVAILABLE_AT_REGION]->(region)
(challenge)-[:REQUIRES_COMPLETION_OF]->(prerequisite)
(challenge)-[:ON_SUCCESS_UNLOCKS]->(location)
```

### Event Relationships

```cypher
(event)-[:TIED_TO_LOCATION]->(location)
(event)-[:TIED_TO_REGION]->(region)
(event)-[:TIED_TO_SCENE]->(scene)
(event)-[:BELONGS_TO_ACT]->(act)
(event)-[:FEATURES_NPC {role: "Primary"}]->(character)
(chain)-[:CONTAINS_EVENT {position: 1, is_completed: false}]->(event)
(event)-[:CHAINS_TO {delay_turns: 2, chain_reason: "..."}]->(next)
(event)-[:TRIGGERED_BY_ENTERING_LOCATION]->(location)
(event)-[:TRIGGERED_BY_ENTERING_REGION]->(region)
(event)-[:TRIGGERED_BY_TALKING_TO]->(character)
(event)-[:TRIGGERED_BY_CHALLENGE_COMPLETE {success_required: true}]->(challenge)
(event)-[:EFFECT_GIVES_ITEM {outcome: "success", quantity: 1}]->(item)
(event)-[:EFFECT_MODIFIES_RELATIONSHIP {sentiment_change: 0.3}]->(character)
(event)-[:ENABLES_CHALLENGE]->(challenge)
(event)-[:DISABLES_CHALLENGE]->(challenge)
```

### Story Event Relationships

```cypher
(story_event)-[:OCCURRED_AT]->(location)
(story_event)-[:OCCURRED_IN_SCENE]->(scene)
(story_event)-[:OCCURRED_IN_SESSION]->(session)
(story_event)-[:INVOLVES {role: "Speaker"}]->(character)
(story_event)-[:TRIGGERED_BY_NARRATIVE]->(narrative_event)
(story_event)-[:RECORDS_CHALLENGE]->(challenge)
```

### Observation

```cypher
(pc)-[:OBSERVED_NPC {
    location_id: "uuid",
    region_id: "uuid",
    game_time: datetime(),
    observation_type: "direct",
    notes: "..."
}]->(npc)
```

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

## Acceptable JSON Blobs

| Entity | Field | Reason |
|--------|-------|--------|
| GridMap | `tiles` | 2D spatial data |
| CharacterSheetTemplate | `sections` | Deeply nested template |
| CharacterSheetData | Full sheet | Per ADR-001, form data |
| WorkflowConfiguration | `workflow_json` | ComfyUI format |
| RuleSystemConfig | Full config | System configuration |
| DirectorialNotes | Full notes | Scene metadata |
| NarrativeEvent | `triggers`, `outcomes` | Complex nested structures |
| StoryEvent | `event_type` | Discriminated union |

---

## Related Documents

- [Hexagonal Architecture](./hexagonal-architecture.md) - Repository pattern
- [System Documents](../systems/) - Entity details per system
