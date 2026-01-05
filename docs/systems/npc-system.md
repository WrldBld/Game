# NPC System

## Overview

The NPC System determines **where NPCs are** at any given time without simulating their movement. It defines NPC-Region relationships (works at, lives at, frequents, avoids) that describe an NPC's connection to locations. The DM can also trigger events that bring NPCs to players or narrate location-wide occurrences.

> **Note**: The actual determination of which NPCs appear when a player enters a region is handled by the [Staging System](./staging-system.md), which uses these relationships as input for both rule-based and LLM-enhanced presence decisions, with DM approval.

---

## Game Design

This system creates a living world without the complexity of AI pathfinding or schedules. Key design principles:

1. **Rule-Based Presence**: NPCs don't move - we calculate if they "should" be present based on their relationships
2. **Time Awareness**: A bartender is present during evening shifts, not at 3 AM
3. **Frequency Probabilities**: "Often" visits ≠ "always" - adds unpredictability
4. **DM Override**: The DM can make any NPC appear anywhere via Approach Events
5. **No Simulation**: Computationally simple, narratively flexible

---

## User Stories

### Implemented

- [x] **US-NPC-001**: As a player, I see relevant NPCs when I enter a region based on their schedules
  - *Implementation*: [Staging System](./staging-system.md) queries NPC-Region relationships, generates suggestions, and requires DM approval
  - *Files*: `crates/engine/src/entities/staging.rs` (replaces `presence_service.rs`)

- [x] **US-NPC-002**: As a DM, I can define where NPCs work (region + shift)
  - *Implementation*: `WORKS_AT_REGION` edge with `shift` property (day/night/always)
  - *Files*: `crates/engine/src/infrastructure/neo4j/character_repo.rs`

- [x] **US-NPC-003**: As a DM, I can define where NPCs live
  - *Implementation*: `HOME_REGION` edge, NPCs more likely present at night
  - *Files*: `crates/engine/src/infrastructure/neo4j/character_repo.rs`

- [x] **US-NPC-004**: As a DM, I can define where NPCs frequently visit
  - *Implementation*: `FREQUENTS_REGION` edge with `frequency` (always/often/sometimes/rarely) and `time_of_day`
  - *Files*: `crates/engine/src/infrastructure/neo4j/character_repo.rs`

- [x] **US-NPC-005**: As a DM, I can define regions NPCs avoid
  - *Implementation*: `AVOIDS_REGION` edge overrides other presence rules
  - *Files*: `crates/engine/src/infrastructure/neo4j/character_repo.rs`

- [x] **US-NPC-006**: As a DM, I can make an NPC approach a specific player
  - *Implementation*: `TriggerApproachEvent` WebSocket message, NPC appears in PC's region
  - *Files*: `crates/engine/src/api/websocket.rs`

- [x] **US-NPC-007**: As a DM, I can trigger a location-wide event (narration)
  - *Implementation*: `TriggerLocationEvent` WebSocket message, all PCs in region see it
  - *Files*: `crates/engine/src/api/websocket.rs`

- [x] **US-NPC-008**: As a player, I see an NPC approach me with a description
  - *Implementation*: `ApproachEventOverlay` modal with NPC sprite and "Continue" button
  - *Files*: `crates/player-ui/src/presentation/components/event_overlays.rs`, `crates/player-ui/src/presentation/state/game_state.rs`

- [x] **US-NPC-009**: As a player, I see location events as narrative text
  - *Implementation*: `LocationEventBanner` component at top of screen, click to dismiss
  - *Files*: `crates/player-ui/src/presentation/components/event_overlays.rs`, `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

### Future Improvements

- [ ] **US-NPC-010**: As a DM, I can define multi-slot schedules for NPCs
  - *Design*: Replace single `time_of_day` with `schedule: Vec<ScheduleSlot>` where each slot has `start_hour`, `end_hour`, `days_of_week`
  - *Effect*: NPCs can change locations throughout the day (e.g., merchant opens shop at 9am, goes to tavern at 6pm, home at 10pm)
  - *Current Limitation*: Single `time_of_day` only supports one slot per relationship
  - *Priority*: Medium - adds realism for recurring NPCs

- [ ] **US-NPC-011**: As a DM, I can preview NPC schedules as a daily timeline
  - *Design*: Visual timeline showing where each NPC is during each time slot
  - *Effect*: Easier schedule planning, conflict detection
  - *Priority*: Low - depends on multi-slot schedule feature

---

## UI Mockups

### Approach Event Display

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                         [Scene with NPC appearing]                          │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ A hooded figure slides into the seat next to you. Their face is     │   │
│  │ obscured, but you catch a glint of steel beneath their cloak.       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│                          [Continue]                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (US-NPC-008)

### Location Event Display

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    ✦ The lights flicker and go out ✦                │   │
│  │                                                                      │   │
│  │   A cold draft sweeps through the tavern. Conversations stop.       │   │
│  │   Everyone looks toward the door...                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (US-NPC-009)

---

## Data Model

### Neo4j Edges (NPC-Region Relationships)

```cypher
// NPC works at region (schedule-based)
(npc:Character)-[:WORKS_AT_REGION {
    shift: "day",              // day, night, always
    role: "Bartender"
}]->(region:Region)

// NPC frequents region (probability-based)
(npc:Character)-[:FREQUENTS_REGION {
    frequency: "often",        // always, often, sometimes, rarely
    time_of_day: "Evening"     // Morning, Afternoon, Evening, Night, Any
}]->(region:Region)

// NPC lives at region
(npc:Character)-[:HOME_REGION]->(region:Region)

// NPC avoids region (override)
(npc:Character)-[:AVOIDS_REGION {
    reason: "Was beaten here once"
}]->(region:Region)
```

### Presence Resolution Algorithm

The rule-based presence algorithm uses the following logic:

```
Query: "Which NPCs are present in region R at time T?"

1. Check WORKS_AT_REGION edges
   - If NPC works here AND shift matches T → PRESENT

2. Check HOME_REGION edges
   - If NPC lives here AND T is Night → LIKELY PRESENT

3. Check FREQUENTS_REGION edges
   - If NPC frequents here AND time_of_day matches T:
     - "always" → PRESENT
     - "often" → 70% chance
     - "sometimes" → 40% chance
     - "rarely" → 10% chance

4. Check AVOIDS_REGION edges
   - If NPC avoids here → NOT PRESENT (overrides above)

Result: List of NPCs with presence suggestions and reasoning
```

> **Note**: This algorithm provides the rule-based suggestions for the [Staging System](./staging-system.md). The Staging System also generates LLM-enhanced suggestions that can override rules based on narrative context. The DM reviews both options before approving the final staging.

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/regions/{id}/npcs` | List NPCs present at region | ✅ |
| POST | `/api/characters/{id}/region-relationships` | Add NPC region relationship | ✅ |
| DELETE | `/api/characters/{id}/region-relationships/{type}/{region_id}` | Remove relationship | ✅ |

### WebSocket Messages

#### Client → Server (DM only)

| Message | Fields | Purpose |
|---------|--------|---------|
| `TriggerApproachEvent` | `npc_id`, `target_pc_id`, `description` | NPC approaches player |
| `TriggerLocationEvent` | `region_id`, `description` | Location-wide narration |
| `ShareNpcLocation` | `npc_id`, `pc_id`, `location_id`, `region_id` | Share NPC whereabouts |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `ApproachEvent` | `npc_id`, `npc_name`, `npc_sprite`, `description` | NPC appeared |
| `LocationEvent` | `region_id`, `description` | Location narration |
| `NpcLocationShared` | `npc_id`, `npc_name`, `location`, `region` | DM shared info |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| NPC-Region Edges | ✅ | - | All 4 relationship types |
| Staging System | ✅ | ⏳ | Partial - see [Staging System](./staging-system.md) |
| Approach Events | ✅ | ✅ | Full modal overlay |
| Location Events | ✅ | ✅ | Banner with dismiss |
| Share NPC Location | ✅ | ✅ | Full WebSocket integration |
| NPC Mood Panel | ✅ | ✅ | DM can view/modify moods toward PCs |
| Mood in LLM Context | ✅ | - | Wired to build_prompt_from_action (2025-12-26) |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/value_objects/region.rs` | RegionRelationship types |
| Domain | `crates/domain/src/entities/staging.rs` | Staging entity |
| Entity | `crates/engine/src/entities/staging.rs` | Staging operations |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/character_repo.rs` | NPC-Region queries |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/region_repo.rs` | Region queries |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/staging_repo.rs` | Staging persistence |
| API | `crates/engine/src/api/websocket.rs` | DM event handlers |
| Protocol | `crates/protocol/src/messages.rs` | Staging message types |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `src/application/dto/websocket_messages.rs` | Event message types |
| Presentation | `src/presentation/handlers/session_message_handler.rs` | Handle events |

---

## Related Systems

- **Depends on**: [Navigation System](./navigation-system.md) (regions, game time)
- **Provides data to**: [Staging System](./staging-system.md) (NPC-Region relationships for presence calculation)
- **Used by**: [Scene System](./scene-system.md) (NPCs in scene), [Dialogue System](./dialogue-system.md) (NPC context), [Observation System](./observation-system.md) (track NPC sightings)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-26 | Added NPC Mood Panel and LLM context integration status |
| 2025-12-24 | Marked US-NPC-008/009 complete; updated staging status |
| 2025-12-19 | Updated to reference Staging System for presence determination |
| 2025-12-18 | Initial version extracted from MVP.md |
