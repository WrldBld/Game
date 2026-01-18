# Calendar System Design

## Overview

This document describes the design for WrldBldr's calendar system, enabling DMs to define custom fantasy calendars for their worlds. The system decouples internal time tracking from calendar display, allowing rich narrative time (e.g., "15th of Mirtul, 1492 DR") while maintaining consistent time mechanics.

---

## Goals

1. **Abstract Time Tracking**: Internal time uses minutes since epoch (not real-world `DateTime`)
2. **Custom Calendars**: DMs can define fantasy calendars (Forgotten Realms, Eberron, homebrew)
3. **Default Gregorian**: Out-of-box experience uses familiar Gregorian calendar
4. **Historical Events**: Support negative time for "before campaign start" events
5. **Named Epoch**: DMs configure what "minute 0" represents (e.g., "1380 DR, Hammer 1")
6. **Per-World Calendars**: Each world can have its own calendar configuration
7. **Deterministic Tests**: Tests can create time at "Day 1, 9:00 AM" without real clock

---

## Design Principles

1. **Separation of Concerns**: Time tracking is separate from calendar display
2. **Calendar as Data**: Calendars are configuration, not code
3. **Graceful Defaults**: Missing calendar config falls back to Gregorian
4. **DM Authority**: DMs can reskin or replace calendars at any time

---

## Core Concepts

### Game Time (Internal)

Game time is tracked as **total minutes since epoch**:

```rust
pub struct GameTime {
    /// Total minutes since the campaign epoch (minute 0).
    /// Negative values represent time before the campaign start.
    total_minutes: i64,
    
    /// Whether time progression is paused.
    is_paused: bool,
}
```

**Why minutes?**
- Granular enough for gameplay (conversations, short actions)
- Large enough range: `i64` supports ±17.5 billion years of game time
- Integer math is simple and deterministic

**Why not `DateTime<Utc>`?**
- Gregorian-centric (assumes 12 months, 365/366 days)
- No support for fantasy calendars
- Real-world leap years don't apply to fantasy worlds

### Calendar Definition

A calendar defines how to convert raw minutes into named dates:

```rust
pub struct CalendarDefinition {
    /// Unique identifier (e.g., "gregorian", "harptos", "eberron")
    pub id: CalendarId,
    
    /// Display name (e.g., "Calendar of Harptos")
    pub name: String,
    
    /// Month definitions in order
    pub months: Vec<MonthDefinition>,
    
    /// Day names for the week (e.g., ["Sunday", "Monday", ...])
    pub day_names: Vec<String>,
    
    /// Hours per day (typically 24)
    pub hours_per_day: u8,
    
    /// Minutes per hour (typically 60)
    pub minutes_per_hour: u8,
    
    /// Special days that don't belong to any month (intercalary days)
    pub intercalary_days: Vec<IntercalaryDay>,
    
    /// Optional year numbering system
    pub era: Option<EraDefinition>,
}

pub struct MonthDefinition {
    /// Month name (e.g., "Hammer", "January")
    pub name: String,
    
    /// Number of days in this month
    pub days: u8,
    
    /// Optional season association
    pub season: Option<Season>,
}

pub struct IntercalaryDay {
    /// Day name (e.g., "Midwinter", "Shieldmeet")
    pub name: String,
    
    /// Inserted after which month (0-indexed)
    pub after_month: u8,
    
    /// How often this day occurs (every N years, or None for every year)
    pub frequency: Option<IntercalaryFrequency>,
}

pub struct EraDefinition {
    /// Era suffix (e.g., "DR", "YK", "AD")
    pub suffix: String,
    
    /// Year at epoch (minute 0)
    pub epoch_year: i32,
}
```

### Epoch Configuration

Each world configures what "minute 0" represents:

```rust
pub struct EpochConfig {
    /// The calendar to use for this world
    pub calendar_id: CalendarId,
    
    /// What year minute 0 falls in (e.g., 1492 for 1492 DR)
    pub epoch_year: i32,
    
    /// What month minute 0 falls in (1-indexed, e.g., 1 for Hammer)
    pub epoch_month: u8,
    
    /// What day minute 0 falls in (1-indexed)
    pub epoch_day: u8,
    
    /// What hour minute 0 falls in (0-23)
    pub epoch_hour: u8,
}
```

**Example: Forgotten Realms campaign starting 1492 DR**

```rust
EpochConfig {
    calendar_id: CalendarId::new("harptos"),
    epoch_year: 1492,
    epoch_month: 1,  // Hammer
    epoch_day: 1,
    epoch_hour: 0,
}
```

With this config:
- `total_minutes = 0` → "1st of Hammer, 1492 DR, 00:00"
- `total_minutes = -525600` (1 year before) → "1st of Hammer, 1491 DR, 00:00"
- `total_minutes = 540` (9 hours later) → "1st of Hammer, 1492 DR, 09:00"

---

## Calendar Formatting

### CalendarDate (Output Type)

When displaying time, `GameTime` is converted to a `CalendarDate`:

```rust
pub struct CalendarDate {
    /// Year in the calendar (can be negative for "before era")
    pub year: i32,
    
    /// Month index (1-indexed)
    pub month: u8,
    
    /// Month name (e.g., "Hammer", "January")
    pub month_name: String,
    
    /// Day of month (1-indexed)
    pub day: u8,
    
    /// Day of week index (0-indexed)
    pub day_of_week: u8,
    
    /// Day of week name (e.g., "Monday", "Swords")
    pub day_of_week_name: String,
    
    /// Hour (0-23)
    pub hour: u8,
    
    /// Minute (0-59)
    pub minute: u8,
    
    /// Time period (Morning, Afternoon, Evening, Night)
    pub period: TimeOfDay,
    
    /// If this is an intercalary day, its name
    pub intercalary_day: Option<String>,
    
    /// Era suffix (e.g., "DR", "AD")
    pub era_suffix: Option<String>,
}
```

### Display Formats

```rust
impl CalendarDate {
    /// "15th of Hammer, 1492 DR"
    pub fn display_full(&self) -> String;
    
    /// "Hammer 15, 1492"
    pub fn display_short(&self) -> String;
    
    /// "9:00 AM"
    pub fn display_time(&self) -> String;
    
    /// "Day 15, 9:00 AM" (ordinal style, for simple display)
    pub fn display_ordinal(&self) -> String;
    
    /// "Morning"
    pub fn display_period(&self) -> String;
}
```

---

## Built-in Calendars

### Gregorian (Default)

Standard real-world calendar:
- 12 months: January (31), February (28/29), March (31), ...
- 7-day week: Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday
- Era: AD/BC (optional)

### Harptos (Forgotten Realms)

The Calendar of Harptos used in the Forgotten Realms:
- 12 months of 30 days each: Hammer, Alturiak, Ches, Tarsakh, Mirtul, Kythorn, Flamerule, Eleasis, Eleint, Marpenoth, Uktar, Nightal
- 5 intercalary days: Midwinter (after Hammer), Greengrass (after Tarsakh), Midsummer (after Flamerule), Highharvestide (after Eleint), Feast of the Moon (after Uktar)
- 10-day "tendays" instead of 7-day weeks
- Era: DR (Dalereckoning)

### Eberron (Future)

The Galifar Calendar:
- 12 months, 28 days each
- 7-day weeks
- Era: YK (Year of the Kingdom)

---

## Data Model

### Domain Types

```
crates/domain/src/
  game_time.rs          # GameTime struct (refactored to use i64 minutes)
  value_objects/
    calendar.rs         # CalendarDefinition, CalendarDate, EpochConfig
    calendar_id.rs      # CalendarId newtype
```

### GameTimeConfig Extension

```rust
pub struct GameTimeConfig {
    pub mode: TimeMode,
    pub time_costs: TimeCostConfig,
    pub show_time_to_players: bool,
    pub time_format: TimeFormat,
    
    // NEW: Calendar configuration
    pub calendar_id: CalendarId,
    pub epoch_config: EpochConfig,
}
```

### Neo4j Storage

```cypher
// World node - time as minutes
(:World {
    id: "uuid",
    game_time_minutes: 540,           // Total minutes since epoch (was: datetime string)
    game_time_paused: true,
    time_config: "{...}"              // JSON with calendar_id, epoch_config
})

// Calendar definitions stored as JSON or separate nodes
(:Calendar {
    id: "harptos",
    name: "Calendar of Harptos",
    definition_json: "{...}"
})

// GameTime nodes for event anchoring
(:GameTime {
    id: "uuid",
    world_id: "uuid",
    total_minutes: 540,               // Raw minutes
    // Denormalized for queries:
    year: 1492,
    month: 1,
    day: 1,
    hour: 9,
    period: "Morning"
})
```

### Wire Format

```rust
// crates/shared/src/types.rs

/// Wire format for game time
pub struct GameTime {
    /// Total minutes since epoch
    pub total_minutes: i64,
    
    /// Paused state
    pub is_paused: bool,
    
    /// Pre-formatted display values (computed by server)
    pub display: GameTimeDisplay,
}

pub struct GameTimeDisplay {
    /// Day number (ordinal-style for simple display)
    pub day: u32,
    
    /// Hour (0-23)
    pub hour: u8,
    
    /// Minute (0-59)
    pub minute: u8,
    
    /// Period name ("Morning", "Afternoon", etc.)
    pub period: String,
    
    /// Full formatted date (e.g., "15th of Hammer, 1492 DR")
    pub formatted_date: String,
    
    /// Formatted time (e.g., "9:00 AM")
    pub formatted_time: String,
}
```

---

## Time Operations

### Creating Time

```rust
// At campaign start (minute 0)
let time = GameTime::at_epoch();

// At a specific offset from epoch
let time = GameTime::from_minutes(540);  // 9 hours after epoch

// Historical event (100 years before campaign)
let time = GameTime::from_minutes(-52_560_000);  // Negative minutes
```

### Converting to Calendar Date

```rust
// Get calendar definition
let calendar = world.calendar();
let epoch = world.epoch_config();

// Convert game time to calendar date
let date = time.to_calendar_date(&calendar, &epoch);

println!("{}", date.display_full());  // "1st of Hammer, 1492 DR, 9:00 AM"
```

### Time Arithmetic

```rust
// Advance by hours
time.advance_hours(3);

// Advance by days (uses calendar's hours_per_day)
time.advance_days(1, &calendar);

// Set to specific calendar date
time.set_to_date(1492, 1, 15, 9, 0, &calendar, &epoch);
```

---

## Migration Strategy

### Phase 1: Internal Refactor (Non-Breaking)
1. Change `GameTime` internal storage from `DateTime<Utc>` to `i64` minutes
2. Keep existing API surface (`day()`, `hour()`, `minute()`)
3. Use Gregorian calendar implicitly for formatting
4. Update Neo4j storage from RFC3339 strings to integer minutes

### Phase 2: Calendar Support (Additive)
1. Add `CalendarDefinition` value object
2. Add `EpochConfig` to `GameTimeConfig`
3. Add built-in calendars (Gregorian, Harptos)
4. Update display methods to use calendar

### Phase 3: DM Configuration (UI)
1. World settings: Calendar selection
2. World settings: Epoch configuration
3. Custom calendar editor (future)

---

## Test Strategy

### Unit Tests

```rust
#[test]
fn epoch_is_day_one() {
    let time = GameTime::at_epoch();
    assert_eq!(time.total_minutes(), 0);
    
    let calendar = CalendarDefinition::gregorian();
    let epoch = EpochConfig::default();  // Jan 1, Year 1
    let date = time.to_calendar_date(&calendar, &epoch);
    
    assert_eq!(date.day, 1);
    assert_eq!(date.month, 1);
    assert_eq!(date.hour, 0);
}

#[test]
fn negative_time_represents_past() {
    let time = GameTime::from_minutes(-1440);  // 1 day before epoch
    let calendar = CalendarDefinition::gregorian();
    let epoch = EpochConfig::new(2024, 1, 2, 0);  // Epoch is Jan 2
    
    let date = time.to_calendar_date(&calendar, &epoch);
    assert_eq!(date.month, 1);
    assert_eq!(date.day, 1);  // Day before Jan 2 = Jan 1
}

#[test]
fn harptos_intercalary_days() {
    let calendar = CalendarDefinition::harptos();
    let epoch = EpochConfig::harptos_default();  // 1492 DR, Hammer 1
    
    // 31st day of year (after Hammer's 30 days + Midwinter)
    let time = GameTime::from_minutes(30 * 24 * 60);  // 30 days after epoch
    let date = time.to_calendar_date(&calendar, &epoch);
    
    assert_eq!(date.intercalary_day, Some("Midwinter".to_string()));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn time_persists_as_minutes() {
    let world = create_test_world().await;
    let time = GameTime::from_minutes(540);
    
    world.advance_time(540);
    save_world(&world).await;
    
    let loaded = load_world(world.id()).await;
    assert_eq!(loaded.game_time().total_minutes(), 540);
}
```

---

## Example: Setting Up a Forgotten Realms Campaign

```rust
// 1. Create world with Harptos calendar
let world = World::new(
    WorldName::new("Sword Coast Adventures")?,
    ClockPort::now(),
)
.with_time_config(GameTimeConfig {
    mode: TimeMode::Suggested,
    time_costs: TimeCostConfig::default(),
    show_time_to_players: true,
    time_format: TimeFormat::TwelveHour,
    calendar_id: CalendarId::new("harptos"),
    epoch_config: EpochConfig {
        calendar_id: CalendarId::new("harptos"),
        epoch_year: 1492,
        epoch_month: 1,  // Hammer
        epoch_day: 1,
        epoch_hour: 9,   // Campaign starts at 9 AM
    },
});

// 2. Time starts at epoch (minute 0)
let time = world.game_time();
assert_eq!(time.total_minutes(), 0);

// 3. Display shows calendar date
let calendar = calendars.get(&world.calendar_id())?;
let date = time.to_calendar_date(calendar, world.epoch_config());
println!("{}", date.display_full());
// Output: "1st of Hammer, 1492 DR, 9:00 AM"

// 4. Advance time
world.advance_hours(3);
let date = world.game_time().to_calendar_date(calendar, world.epoch_config());
println!("{}", date.display_full());
// Output: "1st of Hammer, 1492 DR, 12:00 PM"
```

---

## Open Questions

1. **Calendar Storage**: Store built-in calendars in code or Neo4j?
   - **Decision**: Code for built-in, Neo4j for custom calendars

2. **Multi-Calendar Worlds**: Can different locations have different calendars?
   - **Decision**: v1 is per-world; v2 could support per-location

3. **Leap Years**: Support leap years in custom calendars?
   - **Decision**: v1 supports intercalary days with frequency; full leap year rules are v2

4. **Calendar Migration**: What happens if DM changes calendar mid-campaign?
   - **Decision**: Total minutes stays the same; display changes

---

## Revision History

| Date       | Change                           |
|------------|----------------------------------|
| 2026-01-18 | Initial design document          |
