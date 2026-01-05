# Visual State System

## Overview

The Visual State System manages **dynamic visual configurations** for locations and regions based on activation rules. LocationStates define city-wide visual changes (holidays, sieges, festivals), while RegionStates define region-specific changes (time-of-day backdrops, post-event modifications). States are resolved during staging and approved by the DM alongside NPC presence.

---

## Game Design

The Visual State System creates a dynamic, responsive world:

1. **Layered States**: Location and region states layer together - a city holiday affects the whole location while individual regions can have their own time-of-day variations
2. **Rule-Based Activation**: States activate based on hard rules (date, time, events, flags) or soft rules (LLM evaluation)
3. **DM Approval**: State resolution is part of staging - DM approves visual state alongside NPCs
4. **Priority System**: When multiple states match, highest priority wins
5. **Default States**: Fallback states ensure there's always a valid visual configuration
6. **Asset Overrides**: States can override backdrops, atmosphere text, and ambient sounds

### State Hierarchy

```
Location (City)
  +-- LocationState: "Festival Day" (overrides city backdrop, adds festive atmosphere)
  |
  +-- Region (Tavern)
  |     +-- RegionState: "Morning" (bright backdrop, morning sounds)
  |     +-- RegionState: "Evening" (warm backdrop, tavern music)
  |     +-- RegionState: "After Explosion" (damaged backdrop, debris atmosphere)
  |
  +-- Region (Market)
        +-- RegionState: "Morning" (busy, vendors setting up)
        +-- RegionState: "Afternoon" (peak activity)
        +-- RegionState: "Night" (empty, quiet)
```

### Activation Rules

| Rule Type | Evaluation | Examples |
|-----------|------------|----------|
| **Hard Rules** (Engine) | | |
| `Always` | Always true | Default states |
| `DateExact` | Specific month/day | Holidays, anniversaries |
| `DateRange` | Date range | Festival week, winter season |
| `TimeOfDay` | Morning/Afternoon/Evening/Night | Daily cycles |
| `EventTriggered` | After narrative event | Post-battle, after revelation |
| `FlagSet` | When game flag is true | Quest completion states |
| `CharacterPresent` | When NPC is staged | VIP arrival changes |
| **Soft Rules** (LLM) | | |
| `Custom` | LLM evaluates description | "When tension is high", "If party is being stealthy" |

---

## User Stories

### Backend Implemented (UI Pending)

- [x] **US-VS-002**: As a DM, I can create region states with visual overrides
  - *Backend*: `RegionState` entity with CRUD operations in `region_state.rs`, `region_state_repo.rs`
  - *UI*: Creator mode editor not yet implemented

- [x] **US-VS-003**: As a DM, I can define hard activation rules (date, time, event, flag)
  - *Backend*: `ActivationRule` enum in domain with all rule types
  - *UI*: Rule editor not yet implemented

- [x] **US-VS-005**: As a DM, I can set priorities for states when multiple might match
  - *Backend*: `priority` field on RegionState, resolution logic implemented
  - *UI*: Priority control not yet implemented

- [x] **US-VS-006**: As a DM, I can mark a state as the default fallback
  - *Backend*: `is_default` field, `get_default()` method
  - *UI*: Default toggle not yet implemented

### UI Pending

- [ ] **US-VS-001**: As a DM, I can create location states with visual overrides (backdrop, atmosphere, sound)
  - *Note*: LocationState entity not yet implemented (only RegionState exists)

- [ ] **US-VS-004**: As a DM, I can define soft activation rules (custom LLM conditions)
  - *Implementation*: Free-text condition with optional LLM prompt

- [ ] **US-VS-007**: As a DM, I see resolved visual states in the staging approval popup
  - *Implementation*: Extend StagingApproval component

- [ ] **US-VS-008**: As a DM, I can override the auto-resolved visual state during staging
  - *Implementation*: State selector in staging approval

- [ ] **US-VS-009**: As a player, I see the appropriate backdrop based on current state
  - *Implementation*: State-aware scene rendering

- [ ] **US-VS-010**: As a player, I hear ambient sounds based on current state
  - *Implementation*: Audio system integration

- [ ] **US-VS-011**: As a DM, I can preview what a region looks like in different states
  - *Implementation*: State preview in editor

---

## UI Mockups

### Location State Editor

```
+-----------------------------------------------------------------------------+
|  Location States: Riverview City                                    [X]      |
+-----------------------------------------------------------------------------+
|                                                                              |
|  States determine the city-wide visual configuration based on conditions.    |
|                                                                              |
|  [+ Add State]                                                               |
|                                                                              |
|  --- States (sorted by priority) -----------------------------------------  |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Festival of Lights                                    Priority: 100   | |
|  | Active: DateRange (Month 6, Day 20 - Month 6, Day 25)                  | |
|  |                                                                        | |
|  | Backdrop: /assets/riverview_festival.png                              | |
|  | Atmosphere: The city is alive with colored lanterns and music...      | |
|  | Sound: /audio/festival_ambience.ogg                                   | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Under Siege                                            Priority: 90    | |
|  | Active: FlagSet("city_siege_active")                                   | |
|  |                                                                        | |
|  | Backdrop: /assets/riverview_siege.png                                 | |
|  | Atmosphere: Smoke rises from burning buildings. Soldiers rush past... | |
|  | Sound: /audio/siege_ambience.ogg                                      | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Normal Day                                             Priority: 0     | |
|  | Active: Always                                         [DEFAULT]       | |
|  |                                                                        | |
|  | Backdrop: (uses location default)                                     | |
|  | Atmosphere: (uses location default)                                   | |
|  | Sound: /audio/city_ambience.ogg                                       | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

### Region State Editor

```
+-----------------------------------------------------------------------------+
|  Region States: The Rusty Anchor - Bar Counter                      [X]      |
+-----------------------------------------------------------------------------+
|                                                                              |
|  States determine the region's visual configuration based on conditions.     |
|                                                                              |
|  [+ Add State]                                                               |
|                                                                              |
|  --- States (sorted by priority) -----------------------------------------  |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Post-Explosion                                         Priority: 100   | |
|  | Active: EventTriggered("tavern_explosion")                             | |
|  |                                                                        | |
|  | Backdrop: /assets/bar_counter_damaged.png                             | |
|  | Atmosphere: Broken glass and splintered wood litter the floor...      | |
|  | Sound: /audio/debris_settling.ogg                                     | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Evening                                                Priority: 10    | |
|  | Active: TimeOfDay(Evening)                                             | |
|  |                                                                        | |
|  | Backdrop: /assets/bar_counter_evening.png                             | |
|  | Atmosphere: Warm candlelight flickers across polished brass...        | |
|  | Sound: /audio/tavern_evening.ogg                                      | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Morning                                                Priority: 10    | |
|  | Active: TimeOfDay(Morning)                                             | |
|  |                                                                        | |
|  | Backdrop: /assets/bar_counter_morning.png                             | |
|  | Atmosphere: Sunlight streams through dusty windows...                 | |
|  | Sound: /audio/tavern_morning.ogg                                      | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Default                                                Priority: 0     | |
|  | Active: Always                                         [DEFAULT]       | |
|  |                                                                        | |
|  | Backdrop: (uses region default)                                       | |
|  | Atmosphere: (uses region default)                                     | |
|  |                                                        [Edit] [Delete] | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

### State Rule Editor

```
+-----------------------------------------------------------------------------+
|  Edit Activation Rules                                              [X]      |
+-----------------------------------------------------------------------------+
|                                                                              |
|  State: "Evening"                                                            |
|                                                                              |
|  Logic: ( ) All rules must match   (x) Any rule can match   ( ) At least [2] |
|                                                                              |
|  --- Rules -----------------------------------------------------------      |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Rule 1                                                           [x]   | |
|  | Type: [v Time of Day        ]                                          | |
|  | Period: [v Evening          ]                                          | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Rule 2 (Custom - LLM Evaluated)                                  [x]   | |
|  | Type: [v Custom             ]                                          | |
|  | Condition: [The tavern is hosting a special evening event        ]     | |
|  | LLM Prompt (optional):                                                  | |
|  | [Check if any active events mention evening gatherings at this   ]     | |
|  | [location...                                                     ]     | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  [+ Add Rule]                                                                |
|                                                                              |
|  +--------------------+                                                      |
|  |   Save Rules      |                                                      |
|  +--------------------+                                                      |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

### Extended Staging Approval (with Visual State)

```
+-----------------------------------------------------------------------------+
|  Stage the Scene                                                    [X]      |
+-----------------------------------------------------------------------------+
|                                                                              |
|  Location: Riverview City                  Region: The Bar Counter           |
|  Time: Day 3, Evening (7:30 PM)                                              |
|                                                                              |
|  --- Visual State --------------------------------------------------        |
|                                                                              |
|  +----------------------------------------------------------------------+   |
|  | Location State: [v Festival of Lights      ]  (auto-resolved)        |   |
|  | Reason: DateRange matches (Festival week, Day 22)                    |   |
|  |                                                                      |   |
|  | Region State:   [v Evening                 ]  (auto-resolved)        |   |
|  | Reason: TimeOfDay matches (Evening)                                  |   |
|  |                                                                      |   |
|  | Preview:                                                             |   |
|  | +------------------------------------------------------------------+ |   |
|  | |  [Thumbnail of combined backdrop]                                | |   |
|  | |  "Warm candlelight flickers... The festival music drifts in..." | |   |
|  | +------------------------------------------------------------------+ |   |
|  +----------------------------------------------------------------------+   |
|                                                                              |
|  --- NPC Presence --------------------------------------------------        |
|                                                                              |
|  [x] Marcus the Bartender    [x] Old Sal    [ ] Mysterious Stranger         |
|                                                                              |
|  --- Cache Duration ------------------------------------------------        |
|                                                                              |
|  Valid for: [v 3 hours ] (until 10:30 PM game time)                          |
|                                                                              |
|  +-----------------------------+                                             |
|  |     Approve Staging        |                                             |
|  +-----------------------------+                                             |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

### Timeline/History View (Static Preview)

```
+-----------------------------------------------------------------------------+
|  Location Timeline: Riverview City                                           |
+-----------------------------------------------------------------------------+
|                                                                              |
|  Preview how locations looked at different points in history.                |
|                                                                              |
|  Date: [v Month 6, Day 22   ]    Time: [v Evening        ]                  |
|                                                                              |
|  --- Active States at This Time ----------------------------------------    |
|                                                                              |
|  Location State: Festival of Lights                                          |
|    Backdrop: /assets/riverview_festival.png                                 |
|    "The city is alive with colored lanterns and music..."                   |
|                                                                              |
|  --- Regions --------------------------------------------------------       |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | The Bar Counter                                                        | |
|  | State: Evening                                                         | |
|  | [Preview Image]                                                        | |
|  | "Warm candlelight flickers across polished brass..."                  | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | The Market Square                                                      | |
|  | State: Festival Evening                                                | |
|  | [Preview Image]                                                        | |
|  | "Festival stalls line the square, lanterns swaying..."                | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  --- Historical Events at This Time ------------------------------------    |
|                                                                              |
|  - Day 22: Festival of Lights begins                                         |
|  - Day 20: Party arrived in Riverview                                        |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

---

## Data Model

### Neo4j Nodes

```cypher
// LocationState - visual configuration for a location
(:LocationState {
    id: "uuid",
    location_id: "uuid",
    world_id: "uuid",
    name: "Festival of Lights",
    description: "City-wide festival celebration",
    backdrop_override: "/assets/riverview_festival.png",
    atmosphere_override: "The city is alive with colored lanterns...",
    ambient_sound: "/audio/festival_ambience.ogg",
    map_overlay: "/assets/riverview_festival_map_overlay.png",
    activation_rules: [  // JSON array
        { "type": "DateRange", "start_month": 6, "start_day": 20, "end_month": 6, "end_day": 25 }
    ],
    activation_logic: "all",  // all, any, at_least
    priority: 100,
    is_default: false,
    created_at: datetime,
    updated_at: datetime
})

// RegionState - visual configuration for a region
(:RegionState {
    id: "uuid",
    region_id: "uuid",
    location_id: "uuid",
    world_id: "uuid",
    name: "Evening",
    description: "Evening atmosphere",
    backdrop_override: "/assets/bar_counter_evening.png",
    atmosphere_override: "Warm candlelight flickers...",
    ambient_sound: "/audio/tavern_evening.ogg",
    activation_rules: [
        { "type": "TimeOfDay", "period": "evening" }
    ],
    activation_logic: "all",
    priority: 10,
    is_default: false,
    created_at: datetime,
    updated_at: datetime
})
```

### Neo4j Edges

```cypher
// Location has state options
(location:Location)-[:HAS_STATE]->(state:LocationState)

// Region has state options
(region:Region)-[:HAS_STATE]->(state:RegionState)

// Currently active state (set during staging)
(location:Location)-[:ACTIVE_STATE]->(state:LocationState)
(region:Region)-[:ACTIVE_STATE]->(state:RegionState)
```

### Extended Staging Entity

The Staging entity is extended to track resolved visual state:

```rust
pub struct Staging {
    // ... existing fields ...
    
    /// Resolved location state (if any)
    pub location_state_id: Option<LocationStateId>,
    /// Resolved region state (if any)
    pub region_state_id: Option<RegionStateId>,
    /// How visual state was resolved
    pub visual_state_source: VisualStateSource,  // HardRulesOnly, WithLlmEvaluation, DmOverride, Default
    /// LLM reasoning for soft rules (if evaluated)
    pub visual_state_reasoning: Option<String>,
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/locations/{id}/states` | List location states | Pending |
| POST | `/api/locations/{id}/states` | Create location state | Pending |
| GET | `/api/location-states/{id}` | Get state by ID | Pending |
| PUT | `/api/location-states/{id}` | Update state | Pending |
| DELETE | `/api/location-states/{id}` | Delete state | Pending |
| GET | `/api/regions/{id}/states` | List region states | Pending |
| POST | `/api/regions/{id}/states` | Create region state | Pending |
| GET | `/api/region-states/{id}` | Get state by ID | Pending |
| PUT | `/api/region-states/{id}` | Update state | Pending |
| DELETE | `/api/region-states/{id}` | Delete state | Pending |

### WebSocket Messages

#### Server -> Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `VisualStateChanged` | `location_id`, `region_id`, `location_state`, `region_state` | Visual state updated |

The staging approval messages are extended to include visual state:

```rust
// Extended StagingApprovalRequired
pub struct StagingApprovalRequired {
    // ... existing fields ...
    pub resolved_location_state: Option<ResolvedStateInfo>,
    pub resolved_region_state: Option<ResolvedStateInfo>,
    pub available_location_states: Vec<StateOption>,
    pub available_region_states: Vec<StateOption>,
}

// Extended StagingApprovalResponse
pub struct StagingApprovalResponse {
    // ... existing fields ...
    pub location_state_id: Option<LocationStateId>,
    pub region_state_id: Option<RegionStateId>,
}
```

---

## State Resolution Algorithm

When staging is requested, states are resolved as follows:

```
1. Gather all LocationStates for the location
2. Gather all RegionStates for the region
3. For each state:
   a. Evaluate hard rules against current context (date, time, flags, events, NPCs)
   b. Mark soft rules as pending
4. Filter to states where hard rules pass (based on activation_logic)
5. If any remaining states have soft rules:
   a. Query LLM for soft rule evaluation
   b. Filter based on LLM verdicts
6. Select highest-priority state from remaining (or default)
7. Present to DM for approval with override option
8. Store approved states on Staging entity
9. Update ACTIVE_STATE edges
```

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| LocationState Entity | Pending | - | `entities/location_state.rs` |
| RegionState Entity | Pending | - | `entities/region_state.rs` |
| ActivationRule Value Object | Pending | - | `value_objects/activation_rules.rs` |
| LocationStateRepository | Pending | - | Neo4j CRUD |
| RegionStateRepository | Pending | - | Neo4j CRUD |
| StateResolutionService | Pending | - | Rule evaluation logic |
| Extended Staging Entity | Pending | - | Add visual state fields |
| Extended Staging Messages | Pending | Pending | Visual state in approval |
| Location State Editor UI | - | Pending | Creator mode |
| Region State Editor UI | - | Pending | Creator mode |
| Extended Staging Approval UI | - | Pending | State selection |
| State Preview | - | Pending | Timeline view |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/location_state.rs` | LocationState entity |
| Domain | `crates/domain/src/entities/region_state.rs` | RegionState entity |
| Domain | `crates/domain/src/entities/staging.rs` | Extended with visual state |
| Domain | `crates/domain/src/value_objects/activation_rules.rs` | Shared rule types |
| Domain | `crates/domain/src/ids.rs` | LocationStateId, RegionStateId |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| UI | `crates/player-ui/src/presentation/components/creator/location_state_editor.rs` | State editor (pending) |
| UI | `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs` | Extended approval UI |

---

## Related Systems

- **Depends on**: [Staging System](./staging-system.md) (visual state is part of staging approval), [Game Time System](./game-time-system.md) (TimeOfDay rules), [Narrative System](./narrative-system.md) (EventTriggered rules)
- **Used by**: [Scene System](./scene-system.md) (backdrop selection), [Asset System](./asset-system.md) (state-specific assets)

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-05 | Initial version - Phase 1 domain design |
