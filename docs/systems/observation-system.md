# Observation System

## Overview

The Observation System tracks what players know about NPC whereabouts. When a player sees an NPC, learns about them from dialogue, or deduces their location through investigation, that information is recorded. This creates a "fog of war" where player knowledge differs from reality, enabling mystery and investigation gameplay.

---

## Game Design

Players don't have omniscient knowledge of where NPCs are. Instead:

1. **Direct Observations**: Auto-recorded when NPCs appear in a scene
2. **Heard Information**: DM shares intel ("The bartender mentioned seeing Marcus at the docks")
3. **Deduced Information**: Challenge results reveal NPC patterns
4. **Unrevealed Interactions (Hidden NPCs)**: Observations can be recorded without revealing identity (shown as "Unknown Figure")

This supports mystery scenarios where players must investigate to find people.

---

## User Stories

### Implemented

- [x] **US-OBS-001**: As a player, my observations are recorded when NPCs appear in scenes
  - *Implementation*: `record_observation()` called when scene displays NPCs
  - *Files*: `crates/domain/src/entities/observation.rs`, `crates/engine-adapters/src/infrastructure/persistence/observation_repository.rs`

- [x] **US-OBS-002**: As a DM, I can share NPC location information with a player
  - *Implementation*: `ShareNpcLocation` WebSocket message creates `HeardAbout` observation
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`

- [x] **US-OBS-003**: As a player, challenge successes can reveal NPC information
  - *Implementation*: Challenge outcome effects can create `Deduced` observations
  - *Files*: `crates/engine-app/src/application/services/event_effect_executor.rs`

- [x] **US-OBS-004**: As a player, I can see a panel showing NPCs I know about
  - *Implementation*: `KnownNpcsPanel` component with observation cards and type icons
  - *Files*: `crates/player-ui/src/presentation/components/known_npcs_panel.rs`

- [x] **US-OBS-005**: As a player, I can see where/when I last saw each NPC
  - *Implementation*: Observation cards display last seen location and game time
  - *Files*: `crates/player-ui/src/presentation/components/known_npcs_panel.rs`, `crates/player-app/src/application/services/observation_service.rs`

### Implemented (Unrevealed Interactions)

- [x] **US-OBS-006**: As a DM, I can record an interaction without revealing the NPC
  - *Implementation*: Unrevealed observations render as `npc_name = "Unknown Figure"` and have no portrait/sprite
  - *Completed 2025-12-25*:
    - Added `is_revealed_to_player` to observation entity + persistence
    - Approach events can set `reveal=false` to create an unrevealed direct observation
    - Observation list API scrubs identity when unrevealed
    - Player Known NPCs UI respects the reveal flag
  - *Files*: `crates/domain/src/entities/observation.rs`, `crates/engine-adapters/src/infrastructure/persistence/observation_repository.rs`

---

## UI Mockups

### Known NPCs Panel

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Known NPCs                                                          [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ 👁️ Marcus the Bartender                                               │ │
│  │    Last seen: Bar Counter • Just now                                  │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ 👂 Suspicious Stranger                                                │ │
│  │    Last heard: Docks • 2 days ago (game time)                        │ │
│  │    "The bartender mentioned seeing him at the docks"                  │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ 🧠 Baron Valdris                                                      │ │
│  │    Deduced: Castle • 1 day ago                                       │ │
│  │    "Investigation revealed his evening routine"                       │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  Legend: 👁️ Saw directly  👂 Heard about  🧠 Deduced                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (US-OBS-004/005)

---

## Data Model

### Neo4j Edge

```cypher
// PC observed an NPC
(pc:PlayerCharacter)-[:OBSERVED_NPC {
    location_id: "uuid",
    region_id: "uuid",
    game_time: datetime(),
    observation_type: "direct",  // direct, heard_about, deduced
    is_revealed_to_player: true,  // false => show "Unknown Figure"
    notes: "Saw them arguing with the bartender"
}]->(npc:Character)
```

### Observation Types

| Type | Source | Example |
|------|--------|---------|
| `Direct` | PC saw NPC in region | "You see Marcus at the bar" |
| `HeardAbout` | DM shared information | "The bartender mentions Marcus was here earlier" |
| `Deduced` | Challenge result | "Investigation success: Marcus frequents the docks at night" |

### Domain Entity

```rust
pub struct NpcObservation {
    pub pc_id: PlayerCharacterId,
    pub npc_id: CharacterId,
    pub location_id: LocationId,
    pub region_id: RegionId,
    pub game_time: GameTime,
    pub observation_type: ObservationType,
    pub notes: Option<String>,
}

pub enum ObservationType {
    Direct,
    HeardAbout,
    Deduced,
}

pub struct ObservationSummary {
    pub npc_id: CharacterId,
    pub npc_name: String,
    pub last_location: String,
    pub last_region: String,
    pub game_time_ago: String,
    pub observation_type: ObservationType,
    pub notes: Option<String>,
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/player-characters/{id}/observations` | List PC's NPC observations | ✅ |
| POST | `/api/player-characters/{id}/observations` | Create observation | ✅ |
| DELETE | `/api/observations/{id}` | Remove observation | ✅ |

### WebSocket Messages

#### Client → Server (DM only)

| Message | Fields | Purpose |
|---------|--------|---------|
| `ShareNpcLocation` | `npc_id`, `pc_id`, `location_id`, `region_id`, `notes` | Share NPC whereabouts |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `NpcLocationShared` | `npc_id`, `npc_name`, `location`, `region`, `notes` | DM shared info |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Observation Entity | ✅ | - | Three observation types + reveal flag |
| Observation Repository | ✅ | - | Neo4j OBSERVED_NPC edge with is_revealed |
| Auto-record on Scene | ✅ | - | Direct observations |
| DM Share Location | ✅ | ✅ | WebSocket handler complete |
| Known NPCs Panel | - | ✅ | Full UI with observation types |
| Unrevealed Observations | ✅ | ✅ | "Unknown Figure" for hidden NPCs |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `src/domain/entities/observation.rs` | Observation entity |
| Infrastructure | `src/infrastructure/persistence/observation_repository.rs` | Neo4j impl |
| Infrastructure | `src/infrastructure/websocket/handlers/misc.rs` | ShareNpcLocation, TriggerApproachEvent handlers |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `src/application/dto/websocket_messages.rs` | Message types |
| Presentation | *pending* | Known NPCs panel |

---

## Related Systems

- **Depends on**: [Navigation System](./navigation-system.md) (location/region references), [NPC System](./npc-system.md) (NPC presence), [Character System](./character-system.md) (NPC data)
- **Used by**: [Dialogue System](./dialogue-system.md) (context about known NPCs)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-26 | Marked US-OBS-006 (unrevealed interactions) as complete |
| 2025-12-24 | Marked US-OBS-004/005 complete |
| 2025-12-18 | Initial version extracted from MVP.md |
