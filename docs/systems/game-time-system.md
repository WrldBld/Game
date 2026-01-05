# Game Time System

## Overview

The Game Time System manages in-game time progression for narrative TTRPGs. Unlike real-time systems, game time advances through player actions and DM decisions, creating a "suggested time" model where the system proposes time costs for actions and the DM approves, modifies, or skips them. This enables time-sensitive mechanics (NPC schedules, scene availability, staging TTL) while keeping the DM in control of narrative pacing.

---

## Game Design

### Why Game Time Matters

1. **NPC Presence**: NPCs have schedules - the blacksmith works during the day, the thief prowls at night
2. **Scene Availability**: Some scenes only trigger at specific times ("midnight ritual", "dawn departure")
3. **Narrative Pacing**: Travel should feel meaningful - crossing the city takes time
4. **Staging TTL**: NPC presence in regions expires after in-game hours, not real minutes
5. **Atmosphere**: Time of day affects descriptions, mood, available interactions

### Design Principles

1. **DM has ultimate control** - Can advance, set, pause, or skip time suggestions
2. **System assists, doesn't dictate** - Suggests appropriate time costs, DM decides
3. **Granular but not tedious** - Hour-based precision, but DM can skip to periods
4. **Visible to players** - Current time displayed, time passage communicated clearly

### Time Model

- **Granularity**: Hours (0-23), with derived `TimeOfDay` periods
- **Periods**: Morning (5-11), Afternoon (12-17), Evening (18-21), Night (22-4)
- **Day tracking**: Day counter (Day 1, Day 2, etc.)
- **Display**: "Day 3, Evening (19:00)" or "Day 3, 7:00 PM"

---

## User Stories

### Implemented (Existing)

- [x] **US-TIME-001**: As a DM, I can manually advance game time by hours so that I control pacing
  - *Implementation*: `AdvanceGameTime` request, `GameTimeUpdated` broadcast
  - *Files*: `crates/protocol/src/requests.rs`, `crates/engine/src/api/websocket.rs`

- [x] **US-TIME-002**: As a DM, I can see current game time so that I know the narrative context
  - *Implementation*: `World.game_time` persisted, included in world data
  - *Files*: `crates/domain/src/game_time.rs`, `crates/domain/src/entities/world.rs`

### Pending

- [ ] **US-TIME-003**: As a DM, I can set the exact time (hour and day) so that I can jump to specific moments
- [ ] **US-TIME-004**: As a DM, I can skip to the next time period so that I don't count hours manually
- [ ] **US-TIME-005**: As a DM, I can pause time progression so that suggested time doesn't accumulate
- [ ] **US-TIME-006**: As a DM, I can configure default time costs per action type so that time flows consistently
- [ ] **US-TIME-007**: As a DM, I receive time suggestions when players take time-consuming actions so that I can approve/modify them
- [ ] **US-TIME-008**: As a player, I see time passage notifications so that I understand when time moves
- [ ] **US-TIME-009**: As a player, I can see the current game time so that I can plan time-sensitive actions

---

## Core Concepts

### Time Suggestion Flow

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         TIME SUGGESTION FLOW                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. Player performs action (travel, rest, etc.)                              │
│                        │                                                     │
│                        ▼                                                     │
│  2. System calculates time cost from TimeCostConfig                          │
│     ┌─────────────────────────────────────┐                                  │
│     │ Travel to location: 1 hour          │                                  │
│     │ Current: Day 2, Morning (9:00)      │                                  │
│     │ After: Day 2, Morning (10:00)       │                                  │
│     └─────────────────────────────────────┘                                  │
│                        │                                                     │
│                        ▼                                                     │
│  3. If time_mode == "suggested": Send TimeSuggestion to DM                   │
│     If time_mode == "auto": Apply automatically                              │
│     If time_mode == "manual": Skip (DM advances manually)                    │
│                        │                                                     │
│                        ▼                                                     │
│  4. DM receives TimeSuggestion (if suggested mode)                           │
│     ┌─────────────────────────────────────────────────────────────────┐      │
│     │ "Kira traveled to The Rusty Anchor"                             │      │
│     │ Suggested: +1 hour (9:00 → 10:00)                               │      │
│     │                                                                 │      │
│     │ [Approve]  [Modify: ___]  [Skip]  [Pause Time]                 │      │
│     └─────────────────────────────────────────────────────────────────┘      │
│                        │                                                     │
│                        ▼                                                     │
│  5. DM decision applied, GameTimeAdvanced broadcast to all                   │
│     (includes reason: "Travel to The Rusty Anchor")                          │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Time Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `manual` | Time only advances via explicit DM action | Maximum control, narrative-heavy sessions |
| `suggested` | System suggests, DM approves/modifies | Balanced - consistent time with oversight |
| `auto` | Time advances automatically on actions | Fast-paced, less DM overhead |

### Default Time Costs

| Action Type | Default Cost | Configurable |
|-------------|--------------|--------------|
| `travel_location` | 60 minutes | Yes |
| `travel_region` | 10 minutes | Yes |
| `rest_short` | 60 minutes | Yes |
| `rest_long` | 480 minutes (8 hours) | Yes |
| `conversation` | 0 minutes | Yes |
| `challenge` | 10 minutes | Yes |
| `scene_transition` | 0 minutes | Yes |

---

## UI Mockups

### DM Time Control Panel (Director Mode)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  GAME TIME                                           ⏸ Paused / ▶ Running  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│     ☀️ Day 3, Morning                                                       │
│        9:00 AM                                                              │
│                                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │  +1 Hour    │  │  +4 Hours   │  │  Next       │  │  Set Time   │        │
│  │             │  │             │  │  Period     │  │     ⚙️      │        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
│                                                                             │
│  Time Mode: [Manual ▼]     Pending Suggestions: 2                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

### DM Time Suggestion Toast

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ⏱️ TIME SUGGESTION                                                    [x]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Kira traveled from Market Square to The Docks                              │
│                                                                             │
│  Suggested: +1 hour                                                         │
│  9:00 AM → 10:00 AM (still Morning)                                         │
│                                                                             │
│  ┌──────────┐  ┌─────────────────────┐  ┌──────────┐                       │
│  │ Approve  │  │ Modify: [1] hours   │  │   Skip   │                       │
│  └──────────┘  └─────────────────────┘  └──────────┘                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

### Player Time Display

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Scene Header                                                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  The Rusty Anchor Tavern                              Day 3, Morning 🌅     │
│  ─────────────────────────────────                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending (time display exists but needs refinement)

### Set Time Modal (DM)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  SET GAME TIME                                                         [x]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Current: Day 3, 9:00 AM (Morning)                                          │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Day: [3    ]   Hour: [14   ]   (= 2:00 PM, Afternoon)              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Quick Set:                                                                 │
│  ┌─────────┐ ┌───────────┐ ┌─────────┐ ┌───────┐                           │
│  │ Morning │ │ Afternoon │ │ Evening │ │ Night │                           │
│  │  (9:00) │ │  (14:00)  │ │ (19:00) │ │(22:00)│                           │
│  └─────────┘ └───────────┘ └─────────┘ └───────┘                           │
│                                                                             │
│  Notify players: [✓] "Time advances to afternoon..."                       │
│                                                                             │
│                                         ┌──────────┐  ┌──────────┐         │
│                                         │  Cancel  │  │   Set    │         │
│                                         └──────────┘  └──────────┘         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

---

## Data Model

### Domain Types

```rust
// crates/domain/src/game_time.rs (existing, to be extended)

/// Game time configuration for a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTimeConfig {
    /// How time suggestions are handled
    pub mode: TimeMode,
    /// Default time costs per action type (minutes)
    pub time_costs: TimeCostConfig,
    /// Whether to show time to players
    pub show_time_to_players: bool,
    /// Time format preference
    pub time_format: TimeFormat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TimeMode {
    /// Time only advances via explicit DM action
    Manual,
    /// System suggests, DM approves (default)
    #[default]
    Suggested,
    /// Time advances automatically on actions
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCostConfig {
    /// Minutes for travel between locations
    pub travel_location: u32,
    /// Minutes for travel between regions within a location
    pub travel_region: u32,
    /// Minutes for short rest
    pub rest_short: u32,
    /// Minutes for long rest (typically overnight)
    pub rest_long: u32,
    /// Minutes per conversation exchange (0 = no cost)
    pub conversation: u32,
    /// Minutes per challenge attempt
    pub challenge: u32,
    /// Minutes for scene transitions
    pub scene_transition: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TimeFormat {
    /// "9:00 AM"
    #[default]
    TwelveHour,
    /// "09:00"
    TwentyFourHour,
    /// "Morning" (period only)
    PeriodOnly,
}

/// Reason for time advancement (for logging and display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeAdvanceReason {
    /// DM manually advanced time
    DmManual,
    /// Travel between locations
    TravelLocation { from: String, to: String },
    /// Travel between regions
    TravelRegion { from: String, to: String },
    /// Short rest
    RestShort,
    /// Long rest / sleep
    RestLong,
    /// Challenge attempt
    Challenge { name: String },
    /// Scene transition
    SceneTransition { scene_name: String },
    /// DM set time directly
    DmSetTime,
    /// DM skipped to period
    DmSkipToPeriod { period: TimeOfDay },
}
```

### Neo4j Schema Changes

```cypher
// World node - add time_config JSON
(:World {
    id: "uuid",
    name: "string",
    // ... existing fields ...
    game_time_json: "{...}",      // GameTime serialized
    time_config_json: "{...}"     // GameTimeConfig serialized (NEW)
})

// TimeSuggestion node (pending suggestions for DM)
(:TimeSuggestion {
    id: "uuid",
    world_id: "uuid",
    pc_id: "uuid",               // Which PC's action triggered this
    action_type: "string",       // "travel_location", "rest_short", etc.
    action_description: "string", // Human-readable description
    suggested_minutes: 32,       // Suggested time cost
    current_time_json: "{...}",  // GameTime at suggestion creation
    created_at: datetime(),
    status: "pending" | "approved" | "modified" | "skipped"
})
```

### Protocol Types

```rust
// crates/protocol/src/types.rs (extend existing)

/// Time suggestion for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSuggestionData {
    pub suggestion_id: String,
    pub pc_id: String,
    pub pc_name: String,
    pub action_type: String,
    pub action_description: String,
    pub suggested_minutes: u32,
    pub current_time: GameTime,
    pub resulting_time: GameTime,
    pub period_change: Option<(String, String)>, // ("Morning", "Afternoon") if period changes
}

/// Time advance notification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAdvanceData {
    pub previous_time: GameTime,
    pub new_time: GameTime,
    pub minutes_advanced: u32,
    pub reason: String,
    pub period_changed: bool,
    pub new_period: Option<String>,
}
```

---

## API

### WebSocket Messages

#### Client → Server (New)

| Message | Fields | Purpose |
|---------|--------|---------|
| `SetGameTime` | `world_id`, `day`, `hour`, `notify_players` | DM sets exact time |
| `SkipToPeriod` | `world_id`, `period` | DM skips to next occurrence of period |
| `PauseGameTime` | `world_id`, `paused` | DM pauses/unpauses time |
| `SetTimeMode` | `world_id`, `mode` | DM changes time mode |
| `SetTimeCosts` | `world_id`, `costs` | DM configures time costs |
| `RespondToTimeSuggestion` | `suggestion_id`, `decision` | DM approves/modifies/skips suggestion |

#### Client → Server (Existing, Modified)

| Message | Change |
|---------|--------|
| `AdvanceGameTime` | Add optional `reason` field |

#### Server → Client (New)

| Message | Fields | Purpose |
|---------|--------|---------|
| `TimeSuggestion` | `TimeSuggestionData` | Sent to DM when action suggests time passage |
| `GameTimeAdvanced` | `TimeAdvanceData` | Broadcast to all when time advances (replaces simple `GameTimeUpdated`) |
| `TimeModeChanged` | `world_id`, `mode` | Broadcast when DM changes time mode |
| `GameTimePaused` | `world_id`, `paused` | Broadcast when time paused/unpaused |

#### Server → Client (Existing, Keep)

| Message | Notes |
|---------|-------|
| `GameTimeUpdated` | Keep for backward compat, but prefer `GameTimeAdvanced` |

---

## Integration Points

### Systems That Generate Time Suggestions

| System | Action | Trigger Point |
|--------|--------|---------------|
| Movement | `travel_location` | `EnterRegion` use case when location changes |
| Movement | `travel_region` | `EnterRegion` use case when only region changes |
| Challenge | `challenge` | `RollChallenge` use case after resolution |
| Scene | `scene_transition` | Scene change handlers |
| (Future) | `rest_short` | Rest action when implemented |
| (Future) | `rest_long` | Sleep action when implemented |

### Systems That Consume Game Time

| System | How It Uses Time |
|--------|------------------|
| Staging | TTL expiration based on game time, not real time |
| Scene Resolution | Filter scenes by `TimeOfDay` |
| NPC Presence | `is_npc_present(time_of_day)` for shifts/frequency |
| Observations | Record game time when PC observes something |
| Story Events | Include game time context in event records |
| Narrative Triggers | `TimeAtLocation` trigger evaluation |

---

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| `GameTime` struct | ✅ | Exists in domain |
| `TimeOfDay` enum | ✅ | Exists in domain |
| `World.game_time` | ✅ | Persisted |
| `AdvanceGameTime` | ✅ | DM can advance hours |
| `GameTimeUpdated` | ✅ | Broadcast exists |
| `GameTimeConfig` | ⏳ | Need to add |
| `TimeMode` enum | ⏳ | Need to add |
| `TimeCostConfig` | ⏳ | Need to add |
| `TimeSuggestion` flow | ⏳ | Need to implement |
| `SetGameTime` | ⏳ | Need to add |
| `SkipToPeriod` | ⏳ | Need to add |
| Time suggestion UI | ⏳ | Need to add |
| Time control panel | ⏳ | Need to add |
| Integration: Staging | ⏳ | Uses real time, needs game time |
| Integration: Observations | ⏳ | Uses real time, needs game time |
| Integration: Movement | ⏳ | Needs to generate suggestions |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/game_time.rs` | GameTime, TimeOfDay, config types |
| Domain | `crates/domain/src/entities/world.rs` | World with game_time field |
| Ports | `crates/engine/src/infrastructure/ports.rs` | WorldRepo with time methods |
| Entities | `crates/engine/src/entities/world.rs` | World entity operations |
| Use Cases | `crates/engine/src/use_cases/time/mod.rs` | Time suggestion use cases (NEW) |
| API | `crates/engine/src/api/websocket.rs` | Time-related handlers |
| Neo4j | `crates/engine/src/infrastructure/neo4j/world_repo.rs` | Persist time config |

### Protocol

| File | Purpose |
|------|---------|
| `crates/protocol/src/types.rs` | GameTime wire format |
| `crates/protocol/src/messages.rs` | Time-related messages |
| `crates/protocol/src/requests.rs` | Time-related requests |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| UI | `crates/player-ui/src/presentation/components/time_display.rs` | Time display component |
| UI | `crates/player-ui/src/presentation/components/dm/time_control.rs` | DM time controls (NEW) |
| State | `crates/player-ui/src/presentation/state/game_state.rs` | Current time state |
| Utils | `crates/player-ui/src/presentation/utils/game_time_format.rs` | Time formatting |

---

## Migration Notes

### Breaking Changes

None - all changes are additive. Existing `GameTime` and `AdvanceGameTime` continue to work.

### Default Values

When `time_config` is missing from a World:
- `mode`: `Suggested` (safest default - DM sees suggestions)
- `time_costs`: Use sensible defaults (60/10/60/480/0/10/0 minutes)
- `show_time_to_players`: `true`
- `time_format`: `TwelveHour`

### Data Migration

No migration needed - new fields have defaults. Old worlds will use default config.

---

## Related Systems

- **Depends on**: [Scene System](./scene-system.md) (time context), [Navigation System](./navigation-system.md) (travel triggers)
- **Used by**: [Staging System](./staging-system.md) (TTL), [NPC System](./npc-system.md) (presence), [Observation System](./observation-system.md) (timestamps), [Narrative System](./narrative-system.md) (triggers)

---

## Future Considerations

### Not In Scope (v1)

1. **Undo time changes** - Would require event sourcing architecture
2. **Calendar system** - Named days/months, holidays
3. **Weather tied to time** - Atmospheric changes
4. **Automatic long rest** - "Rest until morning" button
5. **Time-locked items** - Items that only appear at certain times

### Potential v2 Features

1. **Time presets** - DM saves "dawn at the docks" for quick recall
2. **Scheduled events** - "At midnight, trigger event X"
3. **Time-based NPC dialogue** - Different greetings by time of day
4. **Session time tracking** - How much game time passed this session

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-04 | Initial design document |
