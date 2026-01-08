use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};

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
}

impl Default for GameTimeConfig {
    fn default() -> Self {
        Self {
            mode: TimeMode::default(),
            time_costs: TimeCostConfig::default(),
            show_time_to_players: true,
            time_format: TimeFormat::default(),
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameTime {
    current: DateTime<Utc>,
    is_paused: bool,
}

// NOTE: Default impl removed for hexagonal architecture purity.
// Domain layer should not call Utc::now(). Callers should use GameTime::new(clock.now()).

impl GameTime {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            current: now,
            is_paused: true,
        }
    }

    pub fn starting_at(start: DateTime<Utc>) -> Self {
        Self {
            current: start,
            is_paused: true,
        }
    }

    pub fn current(&self) -> DateTime<Utc> {
        self.current
    }

    pub fn set_time(&mut self, new_time: DateTime<Utc>) {
        self.current = new_time;
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
    }

    pub fn advance(&mut self, duration: Duration) {
        self.current += duration;
    }

    pub fn advance_hours(&mut self, hours: u32) {
        self.advance(Duration::hours(hours as i64));
    }

    pub fn advance_days(&mut self, days: u32) {
        self.advance(Duration::days(days as i64));
    }

    /// Advance time by a number of minutes.
    pub fn advance_minutes(&mut self, minutes: u32) {
        self.advance(Duration::minutes(minutes as i64));
    }

    /// Set the time to a specific day and hour.
    /// Minutes are reset to 0.
    pub fn set_day_and_hour(&mut self, day: u32, hour: u32) {
        // Calculate the base date (day 1 = current year Jan 1)
        let base = self.current.with_ordinal(1).unwrap_or(self.current);
        let target_ordinal = day.min(365); // Clamp to valid range

        // Set to the target day
        let target_date = base.with_ordinal(target_ordinal).unwrap_or(base);

        // Set hour and zero out minutes/seconds
        self.current = target_date
            .with_hour(hour.min(23))
            .unwrap_or(target_date)
            .with_minute(0)
            .unwrap_or(target_date)
            .with_second(0)
            .unwrap_or(target_date);
    }

    /// Skip to the next occurrence of a time period.
    /// If currently in that period, advances to the next day's occurrence.
    pub fn skip_to_period(&mut self, target_period: TimeOfDay) {
        let current_period = self.time_of_day();
        let current_hour = self.current.hour() as u8;
        let target_hour = target_period.start_hour();

        if current_period == target_period {
            // Already in this period - skip to tomorrow's occurrence
            self.advance_days(1);
            self.current = self
                .current
                .with_hour(target_hour as u32)
                .unwrap_or(self.current)
                .with_minute(0)
                .unwrap_or(self.current);
        } else if target_hour > current_hour {
            // Target is later today
            self.current = self
                .current
                .with_hour(target_hour as u32)
                .unwrap_or(self.current)
                .with_minute(0)
                .unwrap_or(self.current);
        } else {
            // Target is tomorrow (earlier hour than current)
            self.advance_days(1);
            self.current = self
                .current
                .with_hour(target_hour as u32)
                .unwrap_or(self.current)
                .with_minute(0)
                .unwrap_or(self.current);
        }
    }

    /// Calculate minutes until the start of a target period.
    /// If currently in that period, returns 0.
    /// If target is tomorrow, includes overnight hours.
    pub fn minutes_until_period(&self, target_period: TimeOfDay) -> u32 {
        let current_period = self.time_of_day();
        if current_period == target_period {
            return 0;
        }

        let current_hour = self.current.hour();
        let current_minute = self.current.minute();
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

    /// Get the current hour (0-23).
    pub fn hour(&self) -> u8 {
        self.current.hour() as u8
    }

    /// Get the current minute (0-59).
    pub fn minute(&self) -> u8 {
        self.current.minute() as u8
    }

    /// Get the current day number (1-based ordinal).
    pub fn day(&self) -> u32 {
        self.current.ordinal()
    }

    pub fn time_of_day(&self) -> TimeOfDay {
        let hour = self.current.hour();
        match hour {
            5..=11 => TimeOfDay::Morning,
            12..=17 => TimeOfDay::Afternoon,
            18..=21 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }

    pub fn day_ordinal(&self) -> u32 {
        self.current.ordinal()
    }

    pub fn display_time(&self) -> String {
        let hour = self.current.hour();
        let minute = self.current.minute();

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

    pub fn display_date(&self) -> String {
        format!("Day {}, {}", self.day_ordinal(), self.display_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_time_is_paused() {
        let gt = GameTime::new(Utc::now());
        assert!(gt.is_paused());
    }

    #[test]
    fn time_of_day_mapping_is_standardized() {
        let morning = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T05:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(morning.time_of_day(), TimeOfDay::Morning);

        let afternoon = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(afternoon.time_of_day(), TimeOfDay::Afternoon);

        let evening = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T18:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(evening.time_of_day(), TimeOfDay::Evening);

        let night = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T03:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(night.time_of_day(), TimeOfDay::Night);
    }
}
