# Navigation System

## Overview

The Navigation System handles how players move through the game world. It implements a JRPG-style exploration model where **Locations** contain **Regions** (sub-locations), and players navigate between regions within a location or exit to connected locations. The system also manages **Game Time**, which is DM-controlled and affects NPC presence and event triggers.

---

## Game Design

Navigation creates the physical framework for storytelling. Players explore the world screen-by-screen, discovering NPCs and triggering events based on where they go. Key design principles:

1. **Regions as Screens**: Each region is a distinct "screen" with its own backdrop and NPCs
2. **No Simulation**: NPCs don't walk between locations - presence is calculated when queried
3. **DM-Controlled Time**: Time only advances when the DM says so, preventing unintended NPC schedule changes
4. **Spawn Points**: PCs choose where to start when entering a location

---

## User Stories

### Implemented

- [x] **US-NAV-001**: As a player, I can move between regions within a location so that I can explore different areas
  - *Implementation*: `MoveToRegion` WebSocket message validates connection and updates PC position
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`, `crates/domain/src/entities/region.rs`

- [x] **US-NAV-002**: As a player, I can exit a location to travel to a connected location so that I can explore the world
  - *Implementation*: `ExitToLocation` WebSocket message uses `EXITS_TO_LOCATION` edge, sets arrival region
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`, `crates/engine-adapters/src/infrastructure/persistence/region_repository.rs`

- [x] **US-NAV-003**: As a DM, I can create locations with a hierarchy (town contains tavern contains back room)
  - *Implementation*: `CONTAINS_LOCATION` edges in Neo4j, LocationService methods
  - *Files*: `crates/engine-app/src/application/services/location_service.rs`

- [x] **US-NAV-004**: As a DM, I can create regions within a location with spawn points
  - *Implementation*: Region entity with `is_spawn_point` flag, `HAS_REGION` edge
  - *Files*: `crates/domain/src/entities/region.rs`, `crates/engine-adapters/src/infrastructure/persistence/region_repository.rs`

- [x] **US-NAV-005**: As a DM, I can advance game time to affect NPC schedules
  - *Implementation*: `AdvanceGameTime` WebSocket message updates `GameTime`, invalidates presence cache
  - *Files*: `crates/domain/src/value_objects/game_time.rs`, `crates/engine-adapters/src/infrastructure/websocket.rs`

- [x] **US-NAV-006**: As a DM, I can connect regions within a location
  - *Implementation*: `CONNECTED_TO_REGION` edge with bidirectional flag
  - *Files*: `crates/engine-adapters/src/infrastructure/persistence/region_repository.rs`

- [x] **US-NAV-007**: As a DM, I can create exits from regions to other locations
  - *Implementation*: `EXITS_TO_LOCATION` edge with arrival_region_id
  - *Files*: `crates/engine-adapters/src/infrastructure/persistence/region_repository.rs`

### Pending

- [ ] **US-NAV-008**: As a player, I can see navigation options in the scene UI
  - *Notes*: Engine sends `NavigationOptions` in `SceneChanged`, Player UI needs to render buttons

- [ ] **US-NAV-009**: As a player, I can see the current game time displayed
  - *Notes*: Engine sends `GameTimeUpdated`, Player UI needs time display component

- [ ] **US-NAV-010**: As a player, I can see a mini-map of the current location with clickable regions
  - *Notes*: Region `map_bounds` fields exist for clickable areas, UI not implemented

### Future Improvements

- [ ] **US-NAV-011**: As a DM, I can set travel time between regions/locations
  - *Design*: Add `travel_time_minutes` to `CONNECTED_TO_REGION` and `EXITS_TO_LOCATION` edges
  - *Effect*: Triggers game time advancement, opportunity for random encounters
  - *Priority*: Medium - adds realism and gameplay opportunity

- [ ] **US-NAV-012**: As a DM, I can create party formations for coordinated exploration
  - *Design*: New `Party` entity linking PCs, leader designation, formation rules
  - *Effect*: Coordinated movement, split party handled automatically
  - *Priority*: Low - complex feature, current independent PC model works

- [ ] **US-NAV-013**: As a player, I can see where my party members are on a map
  - *Design*: DM-controlled party visibility, mini-map with PC icons
  - *Effect*: Better coordination without revealing all information
  - *Priority*: Low - depends on party formation feature

---

## UI Mockups

### Navigation Options Panel

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  The Rusty Anchor Tavern - Bar Counter                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         [Scene Backdrop]                             │   │
│  │                                                                      │   │
│  │                        [Character Sprites]                           │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌───────────────┬───────────────┬───────────────┬───────────────────────┐ │
│  │ → Tables      │ → Back Room   │ ← Entrance    │ ⇐ Exit to Market     │ │
│  │   (region)    │   (locked)    │   (region)    │   (location)          │ │
│  └───────────────┴───────────────┴───────────────┴───────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending (Engine provides data, Player UI not implemented)

### Game Time Display

```
┌──────────────────────────┐
│  Day 3, Evening          │
│  ☾ 7:30 PM               │
└──────────────────────────┘
```

**Status**: ⏳ Pending (US-NAV-009)

### Navigation Panel (Player View)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Current Region: Bar Counter                                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Move Within Location ──────────────────────────────────────────────── │
│                                                                             │
│  ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐            │
│  │ → Tables         │ │ 🔒 Back Room     │ │ → Entrance       │            │
│  │   "Open seating" │ │   "Door locked"  │ │   "Main door"    │            │
│  └──────────────────┘ └──────────────────┘ └──────────────────┘            │
│                                                                             │
│  ─── Exit to Other Locations ───────────────────────────────────────────── │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ ⇐ Exit to Market Square                                              │  │
│  │   "Step outside into the bustling market"                            │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending (US-NAV-008 - Engine complete, Player UI not implemented)

### Mini-map with Clickable Regions

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  The Rusty Anchor Tavern                                            [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │    ┌───────────┐     ┌───────────┐                                  │   │
│  │    │           │     │  Back     │                                  │   │
│  │    │  Tables   │     │  Room 🔒  │                                  │   │
│  │    │           │     │           │                                  │   │
│  │    └───────────┘     └───────────┘                                  │   │
│  │                                                                      │   │
│  │    ┌───────────────────────────────┐                                │   │
│  │    │                               │                                │   │
│  │    │      ★ Bar Counter            │  ← You are here                │   │
│  │    │        (current)              │                                │   │
│  │    │                               │                                │   │
│  │    └───────────────────────────────┘                                │   │
│  │                                                                      │   │
│  │    ┌───────────┐                                                    │   │
│  │    │ Entrance  │ ⇒ Exit to Market Square                            │   │
│  │    └───────────┘                                                    │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Click a region to move there                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending (US-NAV-010 - Region bounds data exists, UI not implemented)

---

## Data Model

### Neo4j Nodes

```cypher
// Location - Physical or conceptual place
(:Location {
    id: "uuid",
    name: "The Rusty Anchor Tavern",
    description: "A dimly lit tavern frequented by sailors...",
    location_type: "Interior",  // Interior, Exterior, Abstract
    backdrop_asset: "/assets/backdrops/tavern.png",
    atmosphere: "Smoky, raucous, smells of ale and salt"
})

// Region - Sub-location within a location
(:Region {
    id: "uuid",
    name: "The Bar Counter",
    description: "A worn wooden counter with brass fittings",
    backdrop_asset: "/assets/backdrops/bar_counter.png",
    atmosphere: "Smoky, the barkeep polishes glasses",
    map_bounds_x: 100,
    map_bounds_y: 200,
    map_bounds_width: 300,
    map_bounds_height: 150,
    is_spawn_point: false,
    order: 1
})
```

### Neo4j Edges

```cypher
// Location hierarchy
(parent:Location)-[:CONTAINS_LOCATION]->(child:Location)

// Location connections
(from:Location)-[:CONNECTED_TO {
    connection_type: "Door",
    description: "A heavy oak door",
    bidirectional: true,
    travel_time: 0,
    is_locked: false
}]->(to:Location)

// Location has regions
(location:Location)-[:HAS_REGION]->(region:Region)

// Region connections within location
(region:Region)-[:CONNECTED_TO_REGION {
    description: "A door leads to the back room",
    bidirectional: true,
    is_locked: false,
    lock_description: null
}]->(other:Region)

// Region exits to another location
(region:Region)-[:EXITS_TO_LOCATION {
    description: "Step outside into the market",
    arrival_region_id: "uuid",
    bidirectional: true
}]->(location:Location)

// PC current position
(pc:PlayerCharacter)-[:CURRENTLY_AT]->(location:Location)
(pc:PlayerCharacter)-[:CURRENTLY_IN_REGION]->(region:Region)

// PC starting position
(pc:PlayerCharacter)-[:STARTED_AT]->(location:Location)
(pc:PlayerCharacter)-[:STARTED_IN_REGION]->(region:Region)
```

### Game Time Value Object

```rust
pub struct GameTime {
    pub current: DateTime<Utc>,      // Current in-game date/time
    pub time_scale: f32,             // 0.0 = paused (default)
    pub last_updated: DateTime<Utc>, // Real-world timestamp
}

pub enum TimeOfDay {
    Morning,    // 6:00 - 11:59
    Afternoon,  // 12:00 - 17:59
    Evening,    // 18:00 - 21:59
    Night,      // 22:00 - 5:59
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/locations` | List locations | ✅ |
| POST | `/api/worlds/{id}/locations` | Create location | ✅ |
| GET | `/api/locations/{id}` | Get location | ✅ |
| PUT | `/api/locations/{id}` | Update location | ✅ |
| DELETE | `/api/locations/{id}` | Delete location | ✅ |
| GET | `/api/locations/{id}/regions` | List regions | ✅ |
| POST | `/api/locations/{id}/regions` | Create region | ✅ |
| GET | `/api/regions/{id}` | Get region | ✅ |
| PUT | `/api/regions/{id}` | Update region | ✅ |
| DELETE | `/api/regions/{id}` | Delete region | ✅ |
| POST | `/api/regions/{id}/connections` | Connect regions | ✅ |
| POST | `/api/regions/{id}/exits` | Create exit to location | ✅ |

### WebSocket Messages

#### Client → Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `MoveToRegion` | `region_id` | Move PC within location |
| `ExitToLocation` | `location_id`, `arrival_region_id?` | Move PC to different location |
| `AdvanceGameTime` | `hours` | DM advances in-game time |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `SceneChanged` | `region`, `npcs_present`, `navigation_options` | PC arrived at new region |
| `MovementBlocked` | `reason` | Movement failed (locked, etc.) |
| `GameTimeUpdated` | `display`, `time_of_day`, `is_paused` | Time advanced |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Location Entity | ✅ | ✅ | Full CRUD |
| Region Entity | ✅ | ✅ | Spawn points, connections |
| GameTime Value Object | ✅ | ✅ | DM-controlled |
| Location Repository | ✅ | - | Neo4j with hierarchy |
| Region Repository | ✅ | - | Neo4j with connections |
| Location Service | ✅ | ✅ | Business logic |
| HTTP Routes | ✅ | - | REST API |
| WebSocket Navigation | ✅ | ⏳ | Engine sends, Player pending |
| Navigation UI | - | ⏳ | Buttons for region/exit |
| Time Display UI | - | ⏳ | Show current game time |
| Mini-map UI | - | ⏳ | Clickable regions |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `src/domain/entities/location.rs` | Location entity |
| Domain | `src/domain/entities/region.rs` | Region entity |
| Domain | `src/domain/value_objects/game_time.rs` | GameTime, TimeOfDay |
| Application | `src/application/services/location_service.rs` | Location business logic |
| Infrastructure | `src/infrastructure/persistence/location_repository.rs` | Neo4j Location CRUD |
| Infrastructure | `src/infrastructure/persistence/region_repository.rs` | Neo4j Region CRUD |
| Infrastructure | `src/infrastructure/http/location_routes.rs` | REST routes |
| Infrastructure | `src/infrastructure/http/region_routes.rs` | REST routes |
| Infrastructure | `src/infrastructure/websocket/messages.rs` | WS messages |
| Infrastructure | `src/infrastructure/websocket.rs` | Navigation handlers |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `src/application/dto/world_snapshot.rs` | Location/Region types |
| Application | `src/application/dto/websocket_messages.rs` | WS message types |
| Presentation | `src/presentation/state/game_state.rs` | Current location state |
| Presentation | `src/presentation/handlers/session_message_handler.rs` | Handle SceneChanged |

---

## Related Systems

- **Depends on**: None (foundational system)
- **Used by**: [NPC System](./npc-system.md) (presence at regions), [Scene System](./scene-system.md) (scene resolution), [Narrative System](./narrative-system.md) (location triggers)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-18 | Initial version extracted from MVP.md |
