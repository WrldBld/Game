# Game Time System - Implementation Plan

**Created**: 2026-01-04  
**Status**: Implementation Complete (Phases 1-8)  
**Reference**: [System Design](../systems/game-time-system.md)

---

## Overview

This plan implements the "Suggested Time" model where player actions generate time suggestions that the DM can approve, modify, or skip.

---

## Phase 1: Domain Layer - Config Types

**Goal**: Add configuration types to the domain layer.

### Tasks

1. **Add `TimeMode` enum** to `crates/domain/src/game_time.rs`
   ```rust
   pub enum TimeMode { Manual, Suggested, Auto }
   ```

2. **Add `TimeCostConfig` struct** to `crates/domain/src/game_time.rs`
   ```rust
   pub struct TimeCostConfig {
       pub travel_location: u32,   // minutes
       pub travel_region: u32,
       pub rest_short: u32,
       pub rest_long: u32,
       pub conversation: u32,
       pub challenge: u32,
       pub scene_transition: u32,
   }
   ```

3. **Add `TimeFormat` enum** to `crates/domain/src/game_time.rs`
   ```rust
   pub enum TimeFormat { TwelveHour, TwentyFourHour, PeriodOnly }
   ```

4. **Add `GameTimeConfig` struct** to `crates/domain/src/game_time.rs`
   ```rust
   pub struct GameTimeConfig {
       pub mode: TimeMode,
       pub time_costs: TimeCostConfig,
       pub show_time_to_players: bool,
       pub time_format: TimeFormat,
   }
   ```

5. **Add `TimeAdvanceReason` enum** to `crates/domain/src/game_time.rs`
   ```rust
   pub enum TimeAdvanceReason {
       DmManual,
       TravelLocation { from: String, to: String },
       TravelRegion { from: String, to: String },
       RestShort,
       RestLong,
       Challenge { name: String },
       SceneTransition { scene_name: String },
       DmSetTime,
       DmSkipToPeriod { period: TimeOfDay },
   }
   ```

6. **Add helper methods to `GameTime`**:
   - `advance_minutes(minutes: u32)`
   - `skip_to_period(period: TimeOfDay)` - advances to next occurrence
   - `set_day_and_hour(day: u32, hour: u32)`
   - `minutes_until_period(period: TimeOfDay) -> u32`

7. **Add helper to `TimeOfDay`**:
   - `start_hour(&self) -> u8` - returns 5, 12, 18, or 22
   - `next(&self) -> TimeOfDay` - returns next period

8. **Update exports** in `crates/domain/src/lib.rs`

### Files Changed
- `crates/domain/src/game_time.rs`
- `crates/domain/src/lib.rs`

### Estimated Effort: 1 hour

---

## Phase 2: Domain Layer - World Integration

**Goal**: Add time config to World entity.

### Tasks

1. **Add `time_config` field to `World`** struct in `crates/domain/src/entities/world.rs`
   ```rust
   pub struct World {
       // ... existing fields ...
       pub time_config: GameTimeConfig,
   }
   ```

2. **Update `World::new()`** to initialize with default config

3. **Add methods to `World`**:
   - `set_time_mode(&mut self, mode: TimeMode)`
   - `set_time_costs(&mut self, costs: TimeCostConfig)`
   - `advance_time(&mut self, minutes: u32, reason: TimeAdvanceReason) -> TimeAdvanceResult`
   - `time_cost_for_action(&self, action: &str) -> u32`

### Files Changed
- `crates/domain/src/entities/world.rs`

### Estimated Effort: 30 minutes

---

## Phase 3: Protocol Layer - Wire Types

**Goal**: Add protocol types for time-related messages.

### Tasks

1. **Extend `types.rs`** with new types:
   ```rust
   pub struct TimeSuggestionData { ... }
   pub struct TimeAdvanceData { ... }
   pub struct TimeCostConfig { ... }  // Wire version
   pub struct GameTimeConfig { ... }  // Wire version
   ```

2. **Add converters** from domain to protocol types

3. **Add new `ClientMessage` variants** in `messages.rs`:
   - `SetGameTime { world_id, day, hour, notify_players }`
   - `SkipToPeriod { world_id, period }`
   - `PauseGameTime { world_id, paused }`
   - `SetTimeMode { world_id, mode }`
   - `SetTimeCosts { world_id, costs }`
   - `RespondToTimeSuggestion { suggestion_id, decision }`

4. **Add `TimeSuggestionDecision` enum**:
   ```rust
   pub enum TimeSuggestionDecision {
       Approve,
       Modify { minutes: u32 },
       Skip,
   }
   ```

5. **Add new `ServerMessage` variants** in `messages.rs`:
   - `TimeSuggestion { data: TimeSuggestionData }`
   - `GameTimeAdvanced { data: TimeAdvanceData }` (richer than existing `GameTimeUpdated`)
   - `TimeModeChanged { world_id, mode }`
   - `GameTimePaused { world_id, paused }`

6. **Add new `RequestPayload` variants** in `requests.rs`:
   - `SetGameTime { world_id, day, hour, notify_players }`
   - `SkipToPeriod { world_id, period }`
   - `GetTimeConfig { world_id }`
   - `UpdateTimeConfig { world_id, config }`

### Files Changed
- `crates/protocol/src/types.rs`
- `crates/protocol/src/messages.rs`
- `crates/protocol/src/requests.rs`

### Estimated Effort: 1.5 hours

---

## Phase 4: Engine - Repository Layer

**Goal**: Persist time config in Neo4j.

### Tasks

1. **Update `WorldRepo` port** in `crates/engine/src/infrastructure/ports.rs`:
   - Ensure `save` and `get` handle `time_config`

2. **Update `Neo4jWorldRepo`** in `crates/engine/src/infrastructure/neo4j/world_repo.rs`:
   - Serialize `time_config` to `time_config_json` field
   - Deserialize on read with fallback to defaults

3. **Add `TimeSuggestionRepo` port** (optional - could use in-memory for v1):
   - `save_suggestion(suggestion: &TimeSuggestion) -> Uuid`
   - `get_pending_for_world(world_id: WorldId) -> Vec<TimeSuggestion>`
   - `resolve_suggestion(id: Uuid, decision: TimeSuggestionDecision)`

### Files Changed
- `crates/engine/src/infrastructure/ports.rs`
- `crates/engine/src/infrastructure/neo4j/world_repo.rs`

### Estimated Effort: 1 hour

---

## Phase 5: Engine - Time Use Case

**Goal**: Create use case for time operations.

### Tasks

1. **Create `crates/engine/src/use_cases/time/mod.rs`**:
   ```rust
   pub struct TimeUseCases {
       pub advance_time: Arc<AdvanceTime>,
       pub suggest_time: Arc<SuggestTime>,
   }
   ```

2. **Create `AdvanceTime` use case**:
   - Input: `world_id`, `minutes`, `reason`, `notify_players`
   - Checks time mode (manual/suggested/auto)
   - Updates world
   - Returns `TimeAdvanceData` for broadcast

3. **Create `SuggestTime` use case**:
   - Input: `world_id`, `pc_id`, `action_type`, `description`
   - Looks up time cost from config
   - If mode is `Suggested`: creates TimeSuggestion, returns it
   - If mode is `Auto`: calls `AdvanceTime` directly
   - If mode is `Manual`: does nothing, returns None

4. **Wire into `App`** in `crates/engine/src/app.rs`

5. **Update `UseCases` struct** to include `time`

### Files Changed
- `crates/engine/src/use_cases/time/mod.rs` (new)
- `crates/engine/src/use_cases/mod.rs`
- `crates/engine/src/app.rs`

### Estimated Effort: 2 hours

---

## Phase 6: Engine - WebSocket Handlers

**Goal**: Add handlers for time-related messages.

### Tasks

1. **Add handlers** in `crates/engine/src/api/websocket.rs`:
   - `handle_set_game_time` - DM sets exact time
   - `handle_skip_to_period` - DM skips to period
   - `handle_pause_game_time` - DM pauses/unpauses
   - `handle_set_time_mode` - DM changes mode
   - `handle_set_time_costs` - DM configures costs
   - `handle_respond_to_time_suggestion` - DM approves/modifies/skips

2. **Update existing `handle_advance_game_time`**:
   - Add `reason` field support
   - Use new `TimeAdvanceData` for broadcast

3. **Add request handlers**:
   - `RequestPayload::SetGameTime`
   - `RequestPayload::SkipToPeriod`
   - `RequestPayload::GetTimeConfig`
   - `RequestPayload::UpdateTimeConfig`

4. **Create helper** `broadcast_time_advance(state, world_id, data)`

### Files Changed
- `crates/engine/src/api/websocket.rs`

### Estimated Effort: 2 hours

---

## Phase 7: Engine - Integration with Movement

**Goal**: Generate time suggestions when players travel.

### Tasks

1. **Update `EnterRegion` use case** in `crates/engine/src/use_cases/movement/enter_region.rs`:
   - After successful movement, call `suggest_time` use case
   - Determine if it's location change or just region change
   - Pass appropriate action type

2. **Update `ExitLocation` use case** similarly

3. **Create helper** `determine_travel_type(from_region, to_region) -> TravelType`

4. **Return time suggestion** in movement result for handler to broadcast

### Files Changed
- `crates/engine/src/use_cases/movement/enter_region.rs`
- `crates/engine/src/use_cases/movement/exit_location.rs`
- `crates/engine/src/use_cases/movement/mod.rs`

### Estimated Effort: 1.5 hours

---

## Phase 8: Engine - Fix Existing Time Usage

**Goal**: Use game time instead of real time where appropriate.

### Tasks

1. **Update `Observation` entity** in `crates/engine/src/entities/observation.rs`:
   - Pass game time from World instead of `clock.now()`
   - Update `record_observation` signature to accept `game_time`

2. **Update `Staging` TTL checks**:
   - Use world's game time for expiration checks
   - Update `get_active_staging` to accept game time

3. **Update `Narrative` entity**:
   - Use game time for story event timestamps
   - Update `record_dialogue_event` to accept game time

4. **Update WebSocket handlers** that call these:
   - Fetch world's game time
   - Pass to entity methods

### Files Changed
- `crates/engine/src/entities/observation.rs`
- `crates/engine/src/entities/staging.rs` (or staging repo)
- `crates/engine/src/entities/narrative.rs`
- `crates/engine/src/api/websocket.rs`
- `crates/engine/src/use_cases/movement/enter_region.rs`

### Estimated Effort: 2 hours

---

## Phase 9: Player - State Updates

**Goal**: Handle new time messages in player state.

### Tasks

1. **Update `PlayerEvent` enum** in `crates/player-ports/src/outbound/player_events.rs`:
   - Add `TimeSuggestion { data }` (DM only)
   - Add `GameTimeAdvanced { data }`
   - Add `TimeModeChanged { mode }`
   - Add `GameTimePaused { paused }`

2. **Update message translator** in `crates/player-adapters/src/infrastructure/message_translator.rs`

3. **Update session message handler** in `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

4. **Update game state** in `crates/player-ui/src/presentation/state/game_state.rs`:
   - Store current time config
   - Store pending time suggestions (DM only)

### Files Changed
- `crates/player-ports/src/outbound/player_events.rs`
- `crates/player-adapters/src/infrastructure/message_translator.rs`
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
- `crates/player-ui/src/presentation/state/game_state.rs`

### Estimated Effort: 2 hours

---

## Phase 10: Player - UI Components (Optional for v1)

**Goal**: Add DM time controls.

### Tasks

1. **Create `TimeControlPanel` component**:
   - Shows current time
   - +1 Hour, +4 Hours, Next Period buttons
   - Set Time button (opens modal)
   - Pause toggle
   - Time mode selector

2. **Create `TimeSuggestionToast` component**:
   - Shows pending suggestion
   - Approve/Modify/Skip buttons

3. **Create `SetTimeModal` component**:
   - Day/Hour inputs
   - Quick-set period buttons

4. **Integrate into DM Director view**

### Files Changed
- `crates/player-ui/src/presentation/components/dm/time_control.rs` (new)
- `crates/player-ui/src/presentation/components/dm/mod.rs`
- `crates/player-ui/src/routes/dm_routes.rs`

### Estimated Effort: 3 hours

---

## Implementation Order

```
Phase 1 (Domain Types)
    │
    ▼
Phase 2 (World Integration)
    │
    ▼
Phase 3 (Protocol Types)
    │
    ├─────────────────────────┐
    ▼                         ▼
Phase 4 (Repository)     Phase 9 (Player State)
    │                         │
    ▼                         │
Phase 5 (Use Cases)           │
    │                         │
    ▼                         │
Phase 6 (WebSocket)           │
    │                         │
    ├─────────────────────────┘
    ▼
Phase 7 (Movement Integration)
    │
    ▼
Phase 8 (Fix Existing Usage)
    │
    ▼
Phase 10 (UI - Optional)
```

---

## Summary

| Phase | Description | Effort | Priority | Status |
|-------|-------------|--------|----------|--------|
| 1 | Domain config types | 1h | Critical | DONE |
| 2 | World integration | 30m | Critical | DONE |
| 3 | Protocol types | 1.5h | Critical | DONE |
| 4 | Repository | 1h | Critical | DONE |
| 5 | Time use cases | 2h | Critical | DONE |
| 6 | WebSocket handlers | 2h | Critical | DONE |
| 7 | Movement integration | 1.5h | High | DONE |
| 8 | Fix existing time usage | 2h | High | DONE |
| 9 | Player state | 2h | Medium | DONE (Phase 6) |
| 10 | UI components | 3h | Low (v2) | Pending |

**Total Core (Phases 1-8)**: ~11.5 hours - COMPLETE  
**Total with UI (All phases)**: ~16.5 hours

---

## Testing Strategy

1. **Unit tests** for `GameTime` helper methods
2. **Unit tests** for time cost calculations
3. **Integration tests** for time suggestion flow
4. **Manual testing** of DM controls

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Protocol changes break clients | High | Version messages, provide defaults |
| Existing time usage scattered | Medium | Search thoroughly, test extensively |
| Neo4j schema migration | Low | New field with default, no migration needed |
| UI complexity | Low | Defer UI to v2, use existing toast pattern |

---

## Success Criteria

1. DM can set exact time via WebSocket
2. DM can skip to next time period
3. DM can pause/unpause time
4. DM can configure time mode and costs
5. Travel generates time suggestions in `suggested` mode
6. Staging TTL uses game time, not real time
7. Observations record game time
8. Time is broadcast correctly to all players
