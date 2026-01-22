# Visual State System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


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

### Resolution Precedence

- **RegionState overrides LocationState** for region-specific properties (backdrop, atmosphere, ambience).
- **LocationState provides defaults** when a RegionState omits a property.
- **Fallback states** apply when no other state matches (default state per location/region).

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
| `Custom` | LLM evaluates description (DM approval required) | "When tension is high", "If party is being stealthy" |

---

## User Stories

### Backend Implemented (UI Pending)

- [x] **US-VS-001**: As a DM, I can create location states with visual overrides so that city-wide scenes shift with events
  - *Acceptance Criteria*:
    - A location state can be created with optional overrides for backdrop, atmosphere, ambience, and map overlays
    - A location can list all its available location states ordered by priority
    - A location state can be updated and deleted without breaking existing staging records
  - *Implementation Notes*: `LocationState` entity + repository CRUD in `crates/domain/src/entities/location_state.rs` and `crates/engine/src/repositories/location_state.rs`
  - *Required Data Fields*: `id`, `location_id`, `world_id`, `name`, `description`, `priority`, `is_default`, `activation_rules`, `activation_logic`, `backdrop_override`, `atmosphere_override`, `ambient_sound`, `map_overlay`
  - *UI States*: No creator UI yet; stored data is not user-editable in Player or DM panels
  - *Error Handling*: Reject invalid IDs or missing location; validation errors for empty name/invalid rule payloads
  - *Cross-Refs*: [Asset System](./asset-system.md) for asset paths, [Staging System](./staging-system.md) for activation lifecycle

- [x] **US-VS-002**: As a DM, I can create region states with visual overrides so that specific regions react to time and events
  - *Acceptance Criteria*:
    - A region state can be created with the same override fields as a location state
    - Region states are scoped to a single region and sortable by priority
    - Region state updates preserve activation rules and default flags
  - *Implementation Notes*: `RegionState` entity + repository CRUD in `crates/domain/src/entities/region_state.rs` and `crates/engine/src/repositories/region_state.rs`
  - *Required Data Fields*: `id`, `region_id`, `location_id`, `world_id`, `name`, `description`, `priority`, `is_default`, `activation_rules`, `activation_logic`, `backdrop_override`, `atmosphere_override`, `ambient_sound`
  - *UI States*: No creator UI yet; region state metadata is backend-only
  - *Error Handling*: Reject invalid region IDs, duplicate defaults, or invalid rule payloads
  - *Cross-Refs*: [Scene System](./scene-system.md) for region rendering, [Navigation System](./navigation-system.md) for region context

- [x] **US-VS-003**: As a DM, I can define hard activation rules so that visual states activate deterministically
  - *Acceptance Criteria*:
    - Hard rule types (Always, DateExact, DateRange, TimeOfDay, EventTriggered, FlagSet, CharacterPresent) are available
    - Activation logic supports `all`, `any`, and `at_least` semantics
    - Rule evaluation returns pass/fail based on current game time, flags, and staged NPCs
  - *Implementation Notes*: `ActivationRule` enum and evaluation utilities in `crates/domain/src/value_objects/activation_rules.rs`
  - *Required Data Fields*: `activation_rules`, `activation_logic`, and rule-specific fields (date range, time period, event id, flag key, npc id)
  - *UI States*: Rule editing UI not yet implemented; rules are authored via data import or tooling
  - *Error Handling*: Validation errors for malformed rule definitions or unknown rule type
  - *Cross-Refs*: [Game Time System](./game-time-system.md), [Narrative System](./narrative-system.md), [Staging System](./staging-system.md)

- [x] **US-VS-005**: As a DM, I can set priorities for states so that conflicts resolve predictably
  - *Acceptance Criteria*:
    - State resolution chooses the highest-priority matching state within a scope
    - Priority ties use consistent ordering (e.g., stable by name or creation order)
    - Priority can be updated without changing state IDs
  - *Implementation Notes*: `priority` field on `LocationState` and `RegionState`; resolution logic in `crates/engine/src/use_cases/visual_state/resolve_state.rs`
  - *Required Data Fields*: `priority` (integer)
  - *UI States*: Priority input not yet exposed in UI; stored via backend tooling
  - *Error Handling*: Invalid priority values default to zero or reject during validation
  - *Cross-Refs*: [Staging System](./staging-system.md), [Scene System](./scene-system.md)

- [x] **US-VS-006**: As a DM, I can mark a state as the default so that fallback visuals always exist
  - *Acceptance Criteria*:
    - Each location/region can have exactly one default state
    - Resolution falls back to default when no rule matches
    - Default designation persists through updates and deletions
  - *Implementation Notes*: `is_default` flag and `get_default()` lookup in repositories
  - *Required Data Fields*: `is_default`
  - *UI States*: Default toggle not yet available; defaults set via backend tooling
  - *Error Handling*: Reject multiple defaults for the same scope; error when no default exists and no match
  - *Cross-Refs*: [Staging System](./staging-system.md), [Scene System](./scene-system.md)

### UI Pending

- [ ] **US-VS-004**: As a DM, I can define soft activation rules so that AI can evaluate nuanced conditions
  - *Acceptance Criteria*:
    - DM can enter free-text conditions and optional LLM prompts per rule
    - Soft rules are evaluated by LLM only after hard rules pass
    - LLM reasoning is stored and visible in staging approval
  - *Implementation Notes*: Extend `ActivationRule` with `Custom` payload and route to AI evaluation in `crates/engine/src/use_cases/visual_state/resolve_state.rs`
  - *Required Data Fields*: `custom_condition`, `llm_prompt`, `visual_state_reasoning`, `visual_state_source`
  - *UI States*: Rule editor supports adding/removing custom rules; shows validation errors inline
  - *Error Handling*: LLM timeout returns fallback to default state and logs reasoning failure
  - *Cross-Refs*: [Prompt Template System](./prompt-template-system.md), [Dialogue System](./dialogue-system.md)

- [ ] **US-VS-007**: As a DM, I see resolved visual states in the staging approval popup so that I can approve the full scene
  - *Acceptance Criteria*:
    - Staging approval UI displays resolved location and region states with reasons
    - DM can see the effective backdrop/atmosphere preview before approving
    - Approved states are persisted on the staging record
  - *Implementation Notes*: Extend staging approval payloads and UI in `crates/player/src/ui/presentation/components/dm_panel/staging_approval.rs`
  - *Required Data Fields*: `resolved_location_state`, `resolved_region_state`, `available_location_states`, `available_region_states`, `visual_state_reasoning`
  - *UI States*: Loading while preview assets resolve; empty state when no visual state data
  - *Error Handling*: If payload missing, UI shows fallback message and allows approval without visual state
  - *Cross-Refs*: [Staging System](./staging-system.md), [Scene System](./scene-system.md)

- [ ] **US-VS-008**: As a DM, I can override the auto-resolved visual state during staging so that I can direct the scene
  - *Acceptance Criteria*:
    - DM can select a different location or region state from the available list
    - Override selection is saved with staging approval and marked as `DmOverride`
    - Override changes trigger a `VisualStateChanged` broadcast to players
  - *Implementation Notes*: Extend staging approval request/response to include optional overrides; update resolver to accept overrides
  - *Required Data Fields*: `location_state_id`, `region_state_id`, `visual_state_source`
  - *UI States*: Selector dropdowns with default values; disabled state when no alternatives
  - *Error Handling*: Reject override IDs not in available options; show validation error in DM UI
  - *Cross-Refs*: [Staging System](./staging-system.md), [Navigation System](./navigation-system.md)

- [ ] **US-VS-009**: As a player, I see the appropriate backdrop based on current visual state so that the scene feels responsive
  - *Acceptance Criteria*:
    - Player view renders location/region overrides based on active visual state
    - Region overrides take precedence over location overrides
    - Backdrop updates when a `VisualStateChanged` message arrives
  - *Implementation Notes*: Use resolved visual state fields in scene rendering components; integrate with scene state store
  - *Required Data Fields*: `location_state`, `region_state`, `backdrop_override`, `atmosphere_override`
  - *UI States*: Loading placeholder while asset downloads; fallback to default location/region backdrop
  - *Error Handling*: Missing assets fall back to default images with warning logging
  - *Cross-Refs*: [Scene System](./scene-system.md), [Asset System](./asset-system.md)

- [ ] **US-VS-010**: As a player, I hear ambient sounds based on current visual state so that the location feels alive
  - *Acceptance Criteria*:
    - Ambient sound changes when visual state changes
    - Region ambience overrides location ambience
    - Audio respects mute/volume settings
  - *Implementation Notes*: Integrate audio playback with visual state updates in player app services
  - *Required Data Fields*: `ambient_sound`, client audio settings
  - *UI States*: Audio loading indicator in settings panel; fallback to silence when no sound is defined
  - *Error Handling*: Audio load failure logs and continues without blocking UI
  - *Cross-Refs*: [Scene System](./scene-system.md), [Asset System](./asset-system.md)

- [ ] **US-VS-011**: As a DM, I can preview what a region looks like in different states so that I can author content confidently
  - *Acceptance Criteria*:
    - DM can switch between states and see combined backdrop + atmosphere preview
    - Preview reflects time-of-day and location defaults
    - Preview works without staging a live scene
  - *Implementation Notes*: Add preview UI in creator mode using state resolution with mock context
  - *Required Data Fields*: `state_id`, preview context (time, flags), asset paths
  - *UI States*: Preview loading, empty state when no assets, error state for missing assets
  - *Error Handling*: Preview failures do not affect live staging; show errors inline
  - *Cross-Refs*: [Asset System](./asset-system.md), [Scene System](./scene-system.md)

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
| LocationState Entity | ✅ | - | `crates/domain/src/entities/location_state.rs` |
| RegionState Entity | ✅ | - | `crates/domain/src/entities/region_state.rs` |
| ActivationRule Value Object | ✅ | - | `crates/domain/src/value_objects/activation_rules.rs` |
| LocationStateRepository | ✅ | - | `crates/engine/src/repositories/location_state.rs` |
| RegionStateRepository | ✅ | - | `crates/engine/src/repositories/region_state.rs` |
| StateResolutionService | ✅ | - | `crates/engine/src/use_cases/visual_state/resolve_state.rs` |
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
| Repository | `crates/engine/src/repositories/location_state.rs` | LocationState persistence |
| Repository | `crates/engine/src/repositories/region_state.rs` | RegionState persistence |
| Use Case | `crates/engine/src/use_cases/visual_state/resolve_state.rs` | State resolution |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| UI | Planned (TBD) | Location state editor |
| UI | `crates/player/src/ui/presentation/components/dm_panel/staging_approval.rs` | Extended approval UI |

---

## Related Systems

- **Depends on**: [Staging System](./staging-system.md) (visual state is part of staging approval), [Game Time System](./game-time-system.md) (TimeOfDay rules), [Narrative System](./narrative-system.md) (EventTriggered rules)
- **Used by**: [Scene System](./scene-system.md) (backdrop selection), [Asset System](./asset-system.md) (state-specific assets)

---

## UI Design Documentation

For detailed UI/UX mockups covering:
- Visual State Catalog browser and selection
- Pre-stage modal with state selection + generation
- Staging approval with state override
- Visual State details modal
- Generation workflow

See: [Visual State Catalog UI Design](../designs/visual-state-catalog-ui.md)

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-22 | Added UI design documentation link |
| 2026-01-05 | Initial version - Phase 1 domain design |
