use serde::{Deserialize, Serialize};

use crate::value_objects::{
    calculate_calendar_date, CalendarDate, CalendarDefinition, CalendarId, EpochConfig,
};

// =============================================================================
// Time Constants
// =============================================================================

/// Minutes per hour (standard)
pub const MINUTES_PER_HOUR: i64 = 60;

/// Hours per day (standard)
pub const HOURS_PER_DAY: i64 = 24;

/// Minutes per day (standard: 24 * 60 = 1440)
pub const MINUTES_PER_DAY: i64 = MINUTES_PER_HOUR * HOURS_PER_DAY;

// =============================================================================
// Time of Day
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl TimeOfDay {
    pub fn display_name(&self) -> &'static str {
        match self {
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::Evening => "Evening",
            TimeOfDay::Night => "Night",
        }
    }

    /// Returns the starting hour for this time period.
    pub fn start_hour(&self) -> u8 {
        match self {
            TimeOfDay::Morning => 5,
            TimeOfDay::Afternoon => 12,
            TimeOfDay::Evening => 18,
            TimeOfDay::Night => 22,
        }
    }

    /// Returns the next time period in sequence.
    pub fn next(&self) -> TimeOfDay {
        match self {
            TimeOfDay::Morning => TimeOfDay::Afternoon,
            TimeOfDay::Afternoon => TimeOfDay::Evening,
            TimeOfDay::Evening => TimeOfDay::Night,
            TimeOfDay::Night => TimeOfDay::Morning,
        }
    }

    /// Returns all periods in order.
    pub fn all() -> [TimeOfDay; 4] {
        [
            TimeOfDay::Morning,
            TimeOfDay::Afternoon,
            TimeOfDay::Evening,
            TimeOfDay::Night,
        ]
    }
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// Time Mode
// =============================================================================

/// How time suggestions are handled in this world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeMode {
    /// Time only advances via explicit DM action.
    /// No suggestions are generated.
    Manual,
    /// System suggests time passage, DM approves/modifies/skips.
    /// This is the default - provides consistency with DM oversight.
    #[default]
    Suggested,
    /// Time advances automatically when players take actions.
    /// Fast-paced, less DM overhead.
    Auto,
}

impl TimeMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            TimeMode::Manual => "Manual",
            TimeMode::Suggested => "Suggested",
            TimeMode::Auto => "Automatic",
        }
    }
}

impl std::fmt::Display for TimeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// Time Format
// =============================================================================

/// How time is displayed to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeFormat {
    /// "9:00 AM"
    #[default]
    TwelveHour,
    /// "09:00"
    TwentyFourHour,
    /// "Morning" (period only, no specific time)
    PeriodOnly,
}

// =============================================================================
// Time Cost Configuration
// =============================================================================

/// Default time costs for various actions, in minutes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeCostConfig {
    /// Minutes for travel between locations (default: 60)
    pub travel_location: u32,
    /// Minutes for travel between regions within a location (default: 10)
    pub travel_region: u32,
    /// Minutes for short rest (default: 60)
    pub rest_short: u32,
    /// Minutes for long rest / sleep (default: 480 = 8 hours)
    pub rest_long: u32,
    /// Minutes per conversation exchange (default: 0 = no time cost)
    pub conversation: u32,
    /// Minutes per challenge attempt (default: 10)
    pub challenge: u32,
    /// Minutes for scene transitions (default: 0)
    pub scene_transition: u32,
}

impl Default for TimeCostConfig {
    fn default() -> Self {
        Self {
            travel_location: 60,
            travel_region: 10,
            rest_short: 60,
            rest_long: 480,
            conversation: 0,
            challenge: 10,
            scene_transition: 0,
        }
    }
}

impl TimeCostConfig {
    /// Get the time cost for a given action type.
    pub fn cost_for_action(&self, action: &str) -> u32 {
        match action {
            "travel_location" => self.travel_location,
            "travel_region" => self.travel_region,
            "rest_short" => self.rest_short,
            "rest_long" => self.rest_long,
            "conversation" => self.conversation,
            "challenge" => self.challenge,
            "scene_transition" => self.scene_transition,
            _ => 0,
        }
    }
}

// =============================================================================
// Game Time Configuration
// =============================================================================

/// Complete time configuration for a world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameTimeConfig {
    /// How time suggestions are handled
    pub mode: TimeMode,
    /// Default time costs per action type
    pub time_costs: TimeCostConfig,
    /// Whether to show exact time to players (vs just period)
    pub show_time_to_players: bool,
    /// Time format preference for display
    pub time_format: TimeFormat,
    /// Calendar system to use for this world (default: "gregorian")
    #[serde(default = "default_calendar_id")]
    pub calendar_id: CalendarId,
    /// Epoch configuration defining what minute 0 represents
    #[serde(default)]
    pub epoch_config: EpochConfig,
}

/// Default calendar ID (Gregorian)
fn default_calendar_id() -> CalendarId {
    CalendarId::new("gregorian").expect("gregorian is a valid calendar ID")
}

impl Default for GameTimeConfig {
    fn default() -> Self {
        Self {
            mode: TimeMode::default(),
            time_costs: TimeCostConfig::default(),
            show_time_to_players: true,
            time_format: TimeFormat::default(),
            calendar_id: default_calendar_id(),
            epoch_config: EpochConfig::default(),
        }
    }
}

impl GameTimeConfig {
    /// Create a new GameTimeConfig with Gregorian calendar defaults.
    pub fn gregorian() -> Self {
        Self::default()
    }

    /// Create a new GameTimeConfig for Forgotten Realms (Harptos calendar).
    pub fn harptos() -> Self {
        Self {
            calendar_id: CalendarId::new("harptos").expect("harptos is a valid calendar ID"),
            epoch_config: EpochConfig::harptos_default(),
            ..Default::default()
        }
    }

    /// Create a GameTimeConfig with a custom calendar and epoch.
    pub fn with_calendar(calendar_id: CalendarId, epoch_config: EpochConfig) -> Self {
        Self {
            calendar_id,
            epoch_config,
            ..Default::default()
        }
    }
}

// =============================================================================
// Time Advance Reason
// =============================================================================

/// Reason for time advancement, used for logging and player notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TimeAdvanceReason {
    /// DM manually advanced time via +hours button
    DmManual { hours: u32 },
    /// Travel between locations
    TravelLocation { from: String, to: String },
    /// Travel between regions within the same location
    TravelRegion { from: String, to: String },
    /// Short rest
    RestShort,
    /// Long rest / sleep
    RestLong,
    /// Challenge attempt
    Challenge { name: String },
    /// Scene transition
    SceneTransition { scene_name: String },
    /// DM set time directly via Set Time modal
    DmSetTime,
    /// DM skipped to a specific time period
    DmSkipToPeriod { period: TimeOfDay },
}

impl TimeAdvanceReason {
    /// Returns a human-readable description of the reason.
    pub fn description(&self) -> String {
        match self {
            TimeAdvanceReason::DmManual { hours } => {
                format!(
                    "Time advanced by {} hour{}",
                    hours,
                    if *hours == 1 { "" } else { "s" }
                )
            }
            TimeAdvanceReason::TravelLocation { from, to } => {
                format!("Traveled from {} to {}", from, to)
            }
            TimeAdvanceReason::TravelRegion { from, to } => {
                format!("Moved from {} to {}", from, to)
            }
            TimeAdvanceReason::RestShort => "Took a short rest".to_string(),
            TimeAdvanceReason::RestLong => "Rested for the night".to_string(),
            TimeAdvanceReason::Challenge { name } => format!("Attempted: {}", name),
            TimeAdvanceReason::SceneTransition { scene_name } => {
                format!("Scene transition: {}", scene_name)
            }
            TimeAdvanceReason::DmSetTime => "Time set by DM".to_string(),
            TimeAdvanceReason::DmSkipToPeriod { period } => {
                format!("Skipped to {}", period.display_name())
            }
        }
    }
}

// =============================================================================
// Game Time
// =============================================================================

/// Game time represented as total minutes since the campaign epoch (minute 0).
///
/// This struct provides a simple, integer-based time representation that is
/// independent of any calendar system. Use `to_calendar_date()` to convert
/// to a human-readable calendar date.
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::game_time::{GameTime, MINUTES_PER_DAY};
///
/// // Create time at epoch (minute 0)
/// let mut time = GameTime::at_epoch();
/// assert_eq!(time.total_minutes(), 0);
/// assert_eq!(time.day(), 1);
///
/// // Advance by one day
/// time.advance_days(1);
/// assert_eq!(time.total_minutes(), MINUTES_PER_DAY);
/// assert_eq!(time.day(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTime {
    /// Total minutes since the campaign epoch (minute 0).
    /// Negative values represent time before the campaign start.
    total_minutes: i64,
    is_paused: bool,
}

impl GameTime {
    /// Creates a new GameTime at the epoch (minute 0), paused.
    ///
    /// This is the preferred constructor for new games.
    pub fn at_epoch() -> Self {
        Self {
            total_minutes: 0,
            is_paused: true,
        }
    }

    /// Creates a new GameTime from total minutes since epoch.
    ///
    /// Negative values represent time before the campaign start.
    pub fn from_minutes(minutes: i64) -> Self {
        Self {
            total_minutes: minutes,
            is_paused: true,
        }
    }

    /// Creates a new GameTime at the epoch (minute 0), paused.
    ///
    /// # Deprecated
    ///
    /// Use `GameTime::at_epoch()` instead. This method is kept for backward
    /// compatibility but will be removed in a future version.
    #[deprecated(since = "0.2.0", note = "Use GameTime::at_epoch() instead")]
    pub fn new() -> Self {
        Self::at_epoch()
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Returns the total minutes since the campaign epoch.
    ///
    /// Negative values represent time before the campaign start.
    pub fn total_minutes(&self) -> i64 {
        self.total_minutes
    }

    /// Returns the day number (1-based).
    ///
    /// Day 1 is the first day of the campaign (minutes 0-1439).
    /// For negative minutes, this still returns a positive day number
    /// representing days before the campaign start.
    pub fn day(&self) -> u32 {
        // For both positive and negative values, we want:
        // - minutes 0..1439 -> day 1
        // - minutes 1440..2879 -> day 2
        // - minutes -1440..-1 -> day 1 (before epoch, same "day" concept)
        // - minutes -2880..-1441 -> day 2 (before epoch)
        if self.total_minutes >= 0 {
            (self.total_minutes / MINUTES_PER_DAY) as u32 + 1
        } else {
            // For negative minutes, calculate how many full days back
            ((-self.total_minutes - 1) / MINUTES_PER_DAY) as u32 + 1
        }
    }

    /// Returns the hour of day (0-23).
    pub fn hour(&self) -> u8 {
        let minute_of_day = self.total_minutes.rem_euclid(MINUTES_PER_DAY);
        (minute_of_day / MINUTES_PER_HOUR) as u8
    }

    /// Returns the minute of hour (0-59).
    pub fn minute(&self) -> u8 {
        let minute_of_hour = self.total_minutes.rem_euclid(MINUTES_PER_HOUR);
        minute_of_hour as u8
    }

    /// Returns the current time of day period.
    pub fn time_of_day(&self) -> TimeOfDay {
        let hour = self.hour();
        match hour {
            5..=11 => TimeOfDay::Morning,
            12..=17 => TimeOfDay::Afternoon,
            18..=21 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }

    /// Returns the day number (1-based ordinal).
    ///
    /// This is an alias for `day()` for backward compatibility.
    pub fn day_ordinal(&self) -> u32 {
        self.day()
    }

    /// Returns whether the game time is paused.
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    // =========================================================================
    // Mutation Methods
    // =========================================================================

    /// Sets the paused state.
    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
    }

    /// Sets the total minutes directly.
    pub fn set_total_minutes(&mut self, minutes: i64) {
        self.total_minutes = minutes;
    }

    /// Advances time by the specified number of minutes.
    pub fn advance_minutes(&mut self, minutes: u32) {
        self.total_minutes += minutes as i64;
    }

    /// Advances time by the specified number of hours.
    pub fn advance_hours(&mut self, hours: u32) {
        self.total_minutes += hours as i64 * MINUTES_PER_HOUR;
    }

    /// Advances time by the specified number of days.
    pub fn advance_days(&mut self, days: u32) {
        self.total_minutes += days as i64 * MINUTES_PER_DAY;
    }

    /// Sets the time to a specific day and hour.
    ///
    /// Day is 1-based (day 1 = first day). Hour is 0-23.
    /// Minutes are reset to 0.
    pub fn set_day_and_hour(&mut self, day: u32, hour: u32) {
        let day = day.max(1); // Ensure day is at least 1
        let hour = hour.min(23); // Clamp hour to valid range
        self.total_minutes = ((day as i64 - 1) * HOURS_PER_DAY + hour as i64) * MINUTES_PER_HOUR;
    }

    /// Skip to the next occurrence of a time period.
    ///
    /// If currently in that period, advances to the next day's occurrence.
    pub fn skip_to_period(&mut self, target_period: TimeOfDay) {
        let current_period = self.time_of_day();
        let current_hour = self.hour();
        let target_hour = target_period.start_hour();

        if current_period == target_period {
            // Already in this period - skip to tomorrow's occurrence
            self.advance_days(1);
            // Set to target hour, zero minutes
            let day_start = (self.total_minutes / MINUTES_PER_DAY) * MINUTES_PER_DAY;
            self.total_minutes = day_start + target_hour as i64 * MINUTES_PER_HOUR;
        } else if target_hour > current_hour {
            // Target is later today - advance to target hour
            let current_minute_of_day = self.total_minutes.rem_euclid(MINUTES_PER_DAY);
            let target_minute_of_day = target_hour as i64 * MINUTES_PER_HOUR;
            self.total_minutes += target_minute_of_day - current_minute_of_day;
        } else {
            // Target is tomorrow (earlier hour than current)
            self.advance_days(1);
            let day_start = (self.total_minutes / MINUTES_PER_DAY) * MINUTES_PER_DAY;
            self.total_minutes = day_start + target_hour as i64 * MINUTES_PER_HOUR;
        }
    }

    /// Calculate minutes until the start of a target period.
    ///
    /// If currently in that period, returns 0.
    /// If target is tomorrow, includes overnight hours.
    pub fn minutes_until_period(&self, target_period: TimeOfDay) -> u32 {
        let current_period = self.time_of_day();
        if current_period == target_period {
            return 0;
        }

        let current_hour = self.hour() as u32;
        let current_minute = self.minute() as u32;
        let target_hour = target_period.start_hour() as u32;

        let hours_until = if target_hour > current_hour {
            target_hour - current_hour
        } else {
            // Wraps to next day
            24 - current_hour + target_hour
        };

        // Subtract current minutes (we're partway through this hour)
        let total_minutes = hours_until * 60;
        total_minutes.saturating_sub(current_minute)
    }

    // =========================================================================
    // Calendar Integration
    // =========================================================================

    /// Convert to a calendar date using the given calendar and epoch configuration.
    ///
    /// This allows the same `GameTime` to be displayed differently depending on
    /// the world's calendar system (e.g., Gregorian, Harptos for Forgotten Realms).
    pub fn to_calendar_date(
        &self,
        calendar: &CalendarDefinition,
        epoch: &EpochConfig,
    ) -> CalendarDate {
        calculate_calendar_date(self.total_minutes, calendar, epoch)
    }

    // =========================================================================
    // Compatibility Methods
    // =========================================================================

    /// Convert to a DateTime for compatibility with systems that still require DateTime.
    ///
    /// This creates a synthetic DateTime using a fixed epoch (2000-01-01 00:00:00 UTC)
    /// plus the total_minutes. This is useful for:
    /// - Storing observations with a timestamp
    /// - Comparing game times using chrono's comparison operators
    /// - Legacy code that hasn't been migrated yet
    ///
    /// Note: The actual date is arbitrary and should not be displayed to users.
    /// Use `display_date()` or `to_calendar_date()` for user-facing time display.
    pub fn to_datetime(&self) -> chrono::DateTime<chrono::Utc> {
        use chrono::{Duration, TimeZone, Utc};
        // Use a fixed epoch: 2000-01-01 00:00:00 UTC
        let epoch = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        epoch + Duration::minutes(self.total_minutes)
    }

    // =========================================================================
    // Display Methods
    // =========================================================================

    /// Display the time in 12-hour format (e.g., "9:00 AM").
    pub fn display_time(&self) -> String {
        let hour = self.hour();
        let minute = self.minute();

        let period = if hour >= 12 { "PM" } else { "AM" };
        let display_hour = if hour == 0 {
            12
        } else if hour > 12 {
            hour - 12
        } else {
            hour
        };

        format!("{}:{:02} {}", display_hour, minute, period)
    }

    /// Display as ordinal date with time (e.g., "Day 3, 9:00 AM").
    pub fn display_date(&self) -> String {
        format!("Day {}, {}", self.day(), self.display_time())
    }

    /// Display the current time period name (e.g., "Morning").
    pub fn display_period(&self) -> String {
        self.time_of_day().display_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod game_time_creation {
        use super::*;

        #[test]
        fn at_epoch_creates_minute_zero() {
            let gt = GameTime::at_epoch();
            assert_eq!(gt.total_minutes(), 0);
            assert!(gt.is_paused());
        }

        #[test]
        fn from_minutes_positive() {
            let gt = GameTime::from_minutes(1440);
            assert_eq!(gt.total_minutes(), 1440);
            assert!(gt.is_paused());
        }

        #[test]
        fn from_minutes_negative() {
            let gt = GameTime::from_minutes(-720);
            assert_eq!(gt.total_minutes(), -720);
        }

        #[test]
        #[allow(deprecated)]
        fn deprecated_new_creates_at_epoch() {
            let gt = GameTime::new();
            assert_eq!(gt.total_minutes(), 0);
            assert!(gt.is_paused());
        }
    }

    mod day_calculation {
        use super::*;

        #[test]
        fn day_at_epoch_is_one() {
            let gt = GameTime::at_epoch();
            assert_eq!(gt.day(), 1);
        }

        #[test]
        fn day_at_minute_1439_is_one() {
            let gt = GameTime::from_minutes(1439);
            assert_eq!(gt.day(), 1);
        }

        #[test]
        fn day_at_minute_1440_is_two() {
            let gt = GameTime::from_minutes(1440);
            assert_eq!(gt.day(), 2);
        }

        #[test]
        fn day_at_minute_2880_is_three() {
            let gt = GameTime::from_minutes(2880);
            assert_eq!(gt.day(), 3);
        }

        #[test]
        fn day_ordinal_equals_day() {
            let gt = GameTime::from_minutes(5000);
            assert_eq!(gt.day_ordinal(), gt.day());
        }
    }

    mod negative_time {
        use super::*;

        #[test]
        fn negative_one_minute_is_day_one() {
            let gt = GameTime::from_minutes(-1);
            assert_eq!(gt.day(), 1);
        }

        #[test]
        fn negative_1440_minutes_is_day_one() {
            let gt = GameTime::from_minutes(-1440);
            assert_eq!(gt.day(), 1);
        }

        #[test]
        fn negative_1441_minutes_is_day_two() {
            let gt = GameTime::from_minutes(-1441);
            assert_eq!(gt.day(), 2);
        }

        #[test]
        fn negative_time_hour_wraps() {
            // -60 minutes = 23:00 of the previous day
            let gt = GameTime::from_minutes(-60);
            assert_eq!(gt.hour(), 23);
        }

        #[test]
        fn negative_time_minute_wraps() {
            // -30 minutes = XX:30 of the previous hour
            let gt = GameTime::from_minutes(-30);
            assert_eq!(gt.minute(), 30);
        }
    }

    mod hour_and_minute {
        use super::*;

        #[test]
        fn hour_at_epoch_is_zero() {
            let gt = GameTime::at_epoch();
            assert_eq!(gt.hour(), 0);
        }

        #[test]
        fn minute_at_epoch_is_zero() {
            let gt = GameTime::at_epoch();
            assert_eq!(gt.minute(), 0);
        }

        #[test]
        fn hour_at_540_minutes_is_nine() {
            let gt = GameTime::from_minutes(540); // 9 hours
            assert_eq!(gt.hour(), 9);
        }

        #[test]
        fn minute_at_90_is_thirty() {
            let gt = GameTime::from_minutes(90); // 1 hour 30 minutes
            assert_eq!(gt.hour(), 1);
            assert_eq!(gt.minute(), 30);
        }

        #[test]
        fn hour_wraps_at_24() {
            let gt = GameTime::from_minutes(25 * 60); // 25 hours = 1 hour next day
            assert_eq!(gt.hour(), 1);
        }
    }

    mod time_of_day {
        use super::*;

        #[test]
        fn morning_at_5am() {
            let gt = GameTime::from_minutes(5 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Morning);
        }

        #[test]
        fn morning_at_11am() {
            let gt = GameTime::from_minutes(11 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Morning);
        }

        #[test]
        fn afternoon_at_noon() {
            let gt = GameTime::from_minutes(12 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Afternoon);
        }

        #[test]
        fn afternoon_at_5pm() {
            let gt = GameTime::from_minutes(17 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Afternoon);
        }

        #[test]
        fn evening_at_6pm() {
            let gt = GameTime::from_minutes(18 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Evening);
        }

        #[test]
        fn evening_at_9pm() {
            let gt = GameTime::from_minutes(21 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Evening);
        }

        #[test]
        fn night_at_10pm() {
            let gt = GameTime::from_minutes(22 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Night);
        }

        #[test]
        fn night_at_3am() {
            let gt = GameTime::from_minutes(3 * 60);
            assert_eq!(gt.time_of_day(), TimeOfDay::Night);
        }

        #[test]
        fn night_at_midnight() {
            let gt = GameTime::at_epoch();
            assert_eq!(gt.time_of_day(), TimeOfDay::Night);
        }
    }

    mod advance_methods {
        use super::*;

        #[test]
        fn advance_minutes() {
            let mut gt = GameTime::at_epoch();
            gt.advance_minutes(90);
            assert_eq!(gt.total_minutes(), 90);
        }

        #[test]
        fn advance_hours() {
            let mut gt = GameTime::at_epoch();
            gt.advance_hours(3);
            assert_eq!(gt.total_minutes(), 180);
        }

        #[test]
        fn advance_days() {
            let mut gt = GameTime::at_epoch();
            gt.advance_days(2);
            assert_eq!(gt.total_minutes(), 2880);
        }

        #[test]
        fn set_total_minutes() {
            let mut gt = GameTime::at_epoch();
            gt.set_total_minutes(5000);
            assert_eq!(gt.total_minutes(), 5000);
        }

        #[test]
        fn set_total_minutes_negative() {
            let mut gt = GameTime::at_epoch();
            gt.set_total_minutes(-1000);
            assert_eq!(gt.total_minutes(), -1000);
        }
    }

    mod set_day_and_hour {
        use super::*;

        #[test]
        fn set_day_1_hour_9() {
            let mut gt = GameTime::from_minutes(5000);
            gt.set_day_and_hour(1, 9);
            assert_eq!(gt.day(), 1);
            assert_eq!(gt.hour(), 9);
            assert_eq!(gt.minute(), 0);
        }

        #[test]
        fn set_day_3_hour_14() {
            let mut gt = GameTime::at_epoch();
            gt.set_day_and_hour(3, 14);
            assert_eq!(gt.day(), 3);
            assert_eq!(gt.hour(), 14);
            // (3-1) * 24 * 60 + 14 * 60 = 2880 + 840 = 3720
            assert_eq!(gt.total_minutes(), 3720);
        }

        #[test]
        fn set_day_clamps_hour_to_23() {
            let mut gt = GameTime::at_epoch();
            gt.set_day_and_hour(1, 30);
            assert_eq!(gt.hour(), 23);
        }

        #[test]
        fn set_day_clamps_day_to_1() {
            let mut gt = GameTime::at_epoch();
            gt.set_day_and_hour(0, 9);
            assert_eq!(gt.day(), 1);
        }
    }

    mod skip_to_period {
        use super::*;

        #[test]
        fn skip_from_morning_to_afternoon() {
            let mut gt = GameTime::from_minutes(9 * 60); // 9 AM
            gt.skip_to_period(TimeOfDay::Afternoon);
            assert_eq!(gt.hour(), 12);
            assert_eq!(gt.minute(), 0);
            assert_eq!(gt.time_of_day(), TimeOfDay::Afternoon);
        }

        #[test]
        fn skip_from_morning_to_morning_advances_day() {
            let mut gt = GameTime::from_minutes(9 * 60); // 9 AM day 1
            gt.skip_to_period(TimeOfDay::Morning);
            assert_eq!(gt.hour(), 5);
            assert_eq!(gt.day(), 2);
        }

        #[test]
        fn skip_from_evening_to_morning_advances_day() {
            let mut gt = GameTime::from_minutes(19 * 60); // 7 PM
            gt.skip_to_period(TimeOfDay::Morning);
            assert_eq!(gt.hour(), 5);
            assert_eq!(gt.day(), 2);
        }

        #[test]
        fn skip_from_night_to_morning_same_day() {
            let mut gt = GameTime::from_minutes(3 * 60); // 3 AM day 1
            gt.skip_to_period(TimeOfDay::Morning);
            assert_eq!(gt.hour(), 5);
            assert_eq!(gt.day(), 1);
        }
    }

    mod minutes_until_period {
        use super::*;

        #[test]
        fn same_period_returns_zero() {
            let gt = GameTime::from_minutes(9 * 60); // 9 AM = Morning
            assert_eq!(gt.minutes_until_period(TimeOfDay::Morning), 0);
        }

        #[test]
        fn morning_to_afternoon() {
            let gt = GameTime::from_minutes(9 * 60); // 9 AM
                                                     // Until noon = 3 hours = 180 minutes
            assert_eq!(gt.minutes_until_period(TimeOfDay::Afternoon), 180);
        }

        #[test]
        fn evening_to_morning_wraps() {
            let gt = GameTime::from_minutes(19 * 60); // 7 PM
                                                      // Until 5 AM = 10 hours = 600 minutes
            assert_eq!(gt.minutes_until_period(TimeOfDay::Morning), 600);
        }

        #[test]
        fn partial_hour_subtracted() {
            let gt = GameTime::from_minutes(9 * 60 + 30); // 9:30 AM
                                                          // Until noon = 2.5 hours = 150 minutes
            assert_eq!(gt.minutes_until_period(TimeOfDay::Afternoon), 150);
        }
    }

    mod display_methods {
        use super::*;

        #[test]
        fn display_time_morning() {
            let gt = GameTime::from_minutes(9 * 60 + 30); // 9:30 AM
            assert_eq!(gt.display_time(), "9:30 AM");
        }

        #[test]
        fn display_time_afternoon() {
            let gt = GameTime::from_minutes(14 * 60 + 15); // 2:15 PM
            assert_eq!(gt.display_time(), "2:15 PM");
        }

        #[test]
        fn display_time_midnight() {
            let gt = GameTime::at_epoch();
            assert_eq!(gt.display_time(), "12:00 AM");
        }

        #[test]
        fn display_time_noon() {
            let gt = GameTime::from_minutes(12 * 60);
            assert_eq!(gt.display_time(), "12:00 PM");
        }

        #[test]
        fn display_date() {
            let gt = GameTime::from_minutes(MINUTES_PER_DAY * 2 + 9 * 60); // Day 3, 9 AM
            assert_eq!(gt.display_date(), "Day 3, 9:00 AM");
        }

        #[test]
        fn display_period() {
            let gt = GameTime::from_minutes(9 * 60);
            assert_eq!(gt.display_period(), "Morning");
        }
    }

    mod paused_state {
        use super::*;

        #[test]
        fn new_game_time_is_paused() {
            let gt = GameTime::at_epoch();
            assert!(gt.is_paused());
        }

        #[test]
        fn can_unpause() {
            let mut gt = GameTime::at_epoch();
            gt.set_paused(false);
            assert!(!gt.is_paused());
        }

        #[test]
        fn can_pause_again() {
            let mut gt = GameTime::at_epoch();
            gt.set_paused(false);
            gt.set_paused(true);
            assert!(gt.is_paused());
        }
    }

    mod calendar_integration {
        use super::*;
        use crate::value_objects::{CalendarDefinition, EpochConfig};

        #[test]
        fn to_calendar_date_at_epoch() {
            let gt = GameTime::at_epoch();
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let date = gt.to_calendar_date(&calendar, &epoch);

            assert_eq!(date.year, 1);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 1);
            assert_eq!(date.hour, 0);
            assert_eq!(date.minute, 0);
        }

        #[test]
        fn to_calendar_date_one_day_later() {
            let gt = GameTime::from_minutes(MINUTES_PER_DAY);
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let date = gt.to_calendar_date(&calendar, &epoch);

            assert_eq!(date.day, 2);
        }

        #[test]
        fn to_calendar_date_with_harptos() {
            let gt = GameTime::at_epoch();
            let calendar = CalendarDefinition::harptos();
            let epoch = EpochConfig::harptos_default();

            let date = gt.to_calendar_date(&calendar, &epoch);

            assert_eq!(date.year, 1492);
            assert_eq!(date.month_name, "Hammer");
            assert_eq!(date.era_suffix, Some("DR".to_string()));
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn round_trip_json() {
            let gt = GameTime::from_minutes(12345);
            let json = serde_json::to_string(&gt).unwrap();
            let restored: GameTime = serde_json::from_str(&json).unwrap();
            assert_eq!(gt, restored);
        }

        #[test]
        fn round_trip_negative_time() {
            let gt = GameTime::from_minutes(-5000);
            let json = serde_json::to_string(&gt).unwrap();
            let restored: GameTime = serde_json::from_str(&json).unwrap();
            assert_eq!(gt, restored);
        }
    }
}
