# NPC System

## Overview

The NPC System determines **where NPCs are** at any given time without simulating their movement. When the player enters a region, the system calculates which NPCs should be present based on their location relationships (works at, lives at, frequents, avoids) and the current game time. The DM can also trigger events that bring NPCs to players or narrate location-wide occurrences.

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
  - *Implementation*: `PresenceService` queries NPC-Region relationships, checks time of day
  - *Files*: `Engine/src/application/services/presence_service.rs`

- [x] **US-NPC-002**: As a DM, I can define where NPCs work (region + shift)
  - *Implementation*: `WORKS_AT_REGION` edge with `shift` property (day/night/always)
  - *Files*: `Engine/src/infrastructure/persistence/character_repository.rs`

- [x] **US-NPC-003**: As a DM, I can define where NPCs live
  - *Implementation*: `HOME_REGION` edge, NPCs more likely present at night
  - *Files*: `Engine/src/infrastructure/persistence/character_repository.rs`

- [x] **US-NPC-004**: As a DM, I can define where NPCs frequently visit
  - *Implementation*: `FREQUENTS_REGION` edge with `frequency` (always/often/sometimes/rarely) and `time_of_day`
  - *Files*: `Engine/src/infrastructure/persistence/character_repository.rs`

- [x] **US-NPC-005**: As a DM, I can define regions NPCs avoid
  - *Implementation*: `AVOIDS_REGION` edge overrides other presence rules
  - *Files*: `Engine/src/infrastructure/persistence/character_repository.rs`

- [x] **US-NPC-006**: As a DM, I can make an NPC approach a specific player
  - *Implementation*: `TriggerApproachEvent` WebSocket message, NPC appears in PC's region
  - *Files*: `Engine/src/infrastructure/websocket.rs`

- [x] **US-NPC-007**: As a DM, I can trigger a location-wide event (narration)
  - *Implementation*: `TriggerLocationEvent` WebSocket message, all PCs in region see it
  - *Files*: `Engine/src/infrastructure/websocket.rs`

### Pending

- [ ] **US-NPC-008**: As a player, I see an NPC approach me with a description
  - *Notes*: Engine sends `ApproachEvent`, Player UI needs approach animation/modal

- [ ] **US-NPC-009**: As a player, I see location events as narrative text
  - *Notes*: Engine sends `LocationEvent`, Player UI needs event display

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

**Status**: ⏳ Pending

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

**Status**: ⏳ Pending

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

Result: List of NPCs present with reasoning
```

### Presence Cache

```rust
pub struct PresenceCache {
    entries: HashMap<(RegionId, CharacterId), PresenceCacheEntry>,
}

pub struct PresenceCacheEntry {
    pub is_present: bool,
    pub reasoning: String,       // For DM review
    pub cached_at_game_time: DateTime<Utc>,
    pub ttl_game_hours: u32,     // Invalidate after N in-game hours
}
```

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
| PresenceService | ✅ | - | Rule-based + probability |
| Presence Cache | ✅ | - | TTL-based invalidation |
| Approach Events | ✅ | ⏳ | Engine sends, Player pending |
| Location Events | ✅ | ⏳ | Engine sends, Player pending |
| Share NPC Location | ✅ | ⏳ | Engine sends, Player pending |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `src/domain/value_objects/region.rs` | RegionRelationship types |
| Application | `src/application/services/presence_service.rs` | Presence calculation |
| Infrastructure | `src/infrastructure/persistence/character_repository.rs` | NPC-Region queries |
| Infrastructure | `src/infrastructure/persistence/region_repository.rs` | Region queries |
| Infrastructure | `src/infrastructure/websocket.rs` | DM event handlers |
| Infrastructure | `src/infrastructure/websocket/messages.rs` | Event message types |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `src/application/dto/websocket_messages.rs` | Event message types |
| Presentation | `src/presentation/handlers/session_message_handler.rs` | Handle events |

---

## Related Systems

- **Depends on**: [Navigation System](./navigation-system.md) (regions, game time)
- **Used by**: [Scene System](./scene-system.md) (NPCs in scene), [Dialogue System](./dialogue-system.md) (NPC context), [Observation System](./observation-system.md) (track NPC sightings)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-18 | Initial version extracted from MVP.md |
