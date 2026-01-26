//! Calendar system value objects
//!
//! This module provides types for custom fantasy calendars, enabling DMs to define
//! rich narrative time (e.g., "15th of Mirtul, 1492 DR") while maintaining consistent
//! time mechanics internally via `GameTime` (total seconds since epoch).
//!
//! Key types:
//! - `CalendarId` - Validated identifier for calendars (e.g., "gregorian", "harptos")
//! - `CalendarDefinition` - Full calendar configuration (months, weeks, intercalary days)
//! - `CalendarDate` - Formatted output from GameTime conversion
//! - `EpochConfig` - What second 0 represents in the calendar

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::DomainError;
use crate::game_time::TimeOfDay;

/// Maximum length for calendar identifiers
const MAX_CALENDAR_ID_LENGTH: usize = 50;

// ============================================================================
// CalendarId
// ============================================================================

/// A validated calendar identifier (e.g., "gregorian", "harptos")
///
/// Validation rules:
/// - Non-empty
/// - Maximum 50 characters
/// - Lowercase alphanumeric with underscores only
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CalendarId(String);

impl CalendarId {
    /// Create a new validated calendar identifier.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The identifier is empty after trimming
    /// - The identifier exceeds 50 characters
    /// - The identifier contains characters other than lowercase alphanumeric or underscore
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        let trimmed = id.trim().to_lowercase();

        if trimmed.is_empty() {
            return Err(DomainError::validation("Calendar ID cannot be empty"));
        }
        if trimmed.len() > MAX_CALENDAR_ID_LENGTH {
            return Err(DomainError::validation(format!(
                "Calendar ID cannot exceed {} characters",
                MAX_CALENDAR_ID_LENGTH
            )));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(DomainError::validation(
                "Calendar ID must contain only lowercase alphanumeric characters and underscores",
            ));
        }

        Ok(Self(trimmed))
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CalendarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CalendarId {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<CalendarId> for String {
    fn from(id: CalendarId) -> String {
        id.0
    }
}

// ============================================================================
// Season
// ============================================================================

/// The four seasons of the year
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    /// Returns the display name for this season.
    pub fn display_name(&self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Autumn => "Autumn",
            Season::Winter => "Winter",
        }
    }
}

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// MonthDefinition
// ============================================================================

/// Configuration for a single month in a calendar
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthDefinition {
    /// Month name (e.g., "Hammer", "January")
    pub name: String,
    /// Number of days in this month
    pub days: u8,
    /// Optional season association
    pub season: Option<Season>,
}

impl MonthDefinition {
    /// Create a new month definition.
    pub fn new(name: impl Into<String>, days: u8) -> Self {
        Self {
            name: name.into(),
            days,
            season: None,
        }
    }

    /// Create a new month definition with a season.
    pub fn with_season(name: impl Into<String>, days: u8, season: Season) -> Self {
        Self {
            name: name.into(),
            days,
            season: Some(season),
        }
    }
}

// ============================================================================
// IntercalaryDay
// ============================================================================

/// A special day that doesn't belong to any month (e.g., Midwinter, Shieldmeet)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntercalaryDay {
    /// Day name (e.g., "Midwinter", "Shieldmeet")
    pub name: String,
    /// Inserted after which month (0-indexed)
    pub after_month: u8,
}

impl IntercalaryDay {
    /// Create a new intercalary day definition.
    pub fn new(name: impl Into<String>, after_month: u8) -> Self {
        Self {
            name: name.into(),
            after_month,
        }
    }
}

// ============================================================================
// EraDefinition
// ============================================================================

/// Era configuration for year numbering (e.g., "DR", "AD")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EraDefinition {
    /// Era suffix (e.g., "DR", "YK", "AD")
    pub suffix: String,
    /// Year at epoch (second 0)
    pub epoch_year: i32,
}

impl EraDefinition {
    /// Create a new era definition.
    pub fn new(suffix: impl Into<String>, epoch_year: i32) -> Self {
        Self {
            suffix: suffix.into(),
            epoch_year,
        }
    }
}

// ============================================================================
// CalendarDefinition
// ============================================================================

/// Full calendar configuration defining months, weeks, and special days
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDefinition {
    /// Unique identifier (e.g., "gregorian", "harptos")
    id: CalendarId,
    /// Display name (e.g., "Calendar of Harptos")
    name: String,
    /// Month definitions in order
    months: Vec<MonthDefinition>,
    /// Day names for the week (e.g., ["Sunday", "Monday", ...])
    day_names: Vec<String>,
    /// Hours per day (typically 24)
    hours_per_day: u8,
    /// Minutes per hour (typically 60)
    minutes_per_hour: u8,
    /// Special days that don't belong to any month
    intercalary_days: Vec<IntercalaryDay>,
    /// Optional year numbering system
    era: Option<EraDefinition>,
}

impl CalendarDefinition {
    /// Create a new calendar definition.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: CalendarId,
        name: impl Into<String>,
        months: Vec<MonthDefinition>,
        day_names: Vec<String>,
        hours_per_day: u8,
        minutes_per_hour: u8,
        intercalary_days: Vec<IntercalaryDay>,
        era: Option<EraDefinition>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            months,
            day_names,
            hours_per_day,
            minutes_per_hour,
            intercalary_days,
            era,
        }
    }

    // Accessors

    /// Returns the calendar identifier.
    pub fn id(&self) -> &CalendarId {
        &self.id
    }

    /// Returns the calendar display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the month definitions.
    pub fn months(&self) -> &[MonthDefinition] {
        &self.months
    }

    /// Returns the day names for the week.
    pub fn day_names(&self) -> &[String] {
        &self.day_names
    }

    /// Returns the number of hours per day.
    pub fn hours_per_day(&self) -> u8 {
        self.hours_per_day
    }

    /// Returns the number of minutes per hour.
    pub fn minutes_per_hour(&self) -> u8 {
        self.minutes_per_hour
    }

    /// Returns the intercalary (special) days.
    pub fn intercalary_days(&self) -> &[IntercalaryDay] {
        &self.intercalary_days
    }

    /// Returns the era definition, if any.
    pub fn era(&self) -> Option<&EraDefinition> {
        self.era.as_ref()
    }

    // Computed properties

    /// Returns the total number of days in a year (including intercalary days).
    pub fn days_in_year(&self) -> u32 {
        let month_days: u32 = self.months.iter().map(|m| m.days as u32).sum();
        let intercalary_count = self.intercalary_days.len() as u32;
        month_days + intercalary_count
    }

    /// Returns the number of minutes per day.
    pub fn minutes_per_day(&self) -> u32 {
        self.hours_per_day as u32 * self.minutes_per_hour as u32
    }

    /// Returns the number of minutes per year.
    pub fn minutes_per_year(&self) -> i64 {
        self.days_in_year() as i64 * self.minutes_per_day() as i64
    }

    // Built-in calendars

    /// Creates the standard Gregorian calendar.
    ///
    /// - 12 months: January (31), February (28), March (31), etc.
    /// - 7-day week: Sunday through Saturday
    /// - 24 hours per day, 60 minutes per hour
    pub fn gregorian() -> Self {
        Self {
            id: CalendarId::new("gregorian").expect("gregorian is a valid calendar ID"),
            name: "Gregorian Calendar".to_string(),
            months: vec![
                MonthDefinition::with_season("January", 31, Season::Winter),
                MonthDefinition::with_season("February", 28, Season::Winter),
                MonthDefinition::with_season("March", 31, Season::Spring),
                MonthDefinition::with_season("April", 30, Season::Spring),
                MonthDefinition::with_season("May", 31, Season::Spring),
                MonthDefinition::with_season("June", 30, Season::Summer),
                MonthDefinition::with_season("July", 31, Season::Summer),
                MonthDefinition::with_season("August", 31, Season::Summer),
                MonthDefinition::with_season("September", 30, Season::Autumn),
                MonthDefinition::with_season("October", 31, Season::Autumn),
                MonthDefinition::with_season("November", 30, Season::Autumn),
                MonthDefinition::with_season("December", 31, Season::Winter),
            ],
            day_names: vec![
                "Sunday".to_string(),
                "Monday".to_string(),
                "Tuesday".to_string(),
                "Wednesday".to_string(),
                "Thursday".to_string(),
                "Friday".to_string(),
                "Saturday".to_string(),
            ],
            hours_per_day: 24,
            minutes_per_hour: 60,
            intercalary_days: vec![],
            era: Some(EraDefinition::new("AD", 1)),
        }
    }

    /// Creates the Calendar of Harptos (Forgotten Realms).
    ///
    /// - 12 months of 30 days each: Hammer, Alturiak, Ches, Tarsakh, Mirtul, Kythorn,
    ///   Flamerule, Eleasis, Eleint, Marpenoth, Uktar, Nightal
    /// - 5 intercalary days: Midwinter, Greengrass, Midsummer, Highharvestide, Feast of the Moon
    /// - 10-day "tendays" instead of 7-day weeks
    /// - Era: DR (Dalereckoning)
    pub fn harptos() -> Self {
        Self {
            id: CalendarId::new("harptos").expect("harptos is a valid calendar ID"),
            name: "Calendar of Harptos".to_string(),
            months: vec![
                MonthDefinition::with_season("Hammer", 30, Season::Winter),
                MonthDefinition::with_season("Alturiak", 30, Season::Winter),
                MonthDefinition::with_season("Ches", 30, Season::Spring),
                MonthDefinition::with_season("Tarsakh", 30, Season::Spring),
                MonthDefinition::with_season("Mirtul", 30, Season::Spring),
                MonthDefinition::with_season("Kythorn", 30, Season::Summer),
                MonthDefinition::with_season("Flamerule", 30, Season::Summer),
                MonthDefinition::with_season("Eleasis", 30, Season::Summer),
                MonthDefinition::with_season("Eleint", 30, Season::Autumn),
                MonthDefinition::with_season("Marpenoth", 30, Season::Autumn),
                MonthDefinition::with_season("Uktar", 30, Season::Autumn),
                MonthDefinition::with_season("Nightal", 30, Season::Winter),
            ],
            day_names: vec![
                "First-day".to_string(),
                "Second-day".to_string(),
                "Third-day".to_string(),
                "Fourth-day".to_string(),
                "Fifth-day".to_string(),
                "Sixth-day".to_string(),
                "Seventh-day".to_string(),
                "Eighth-day".to_string(),
                "Ninth-day".to_string(),
                "Tenth-day".to_string(),
            ],
            hours_per_day: 24,
            minutes_per_hour: 60,
            intercalary_days: vec![
                IntercalaryDay::new("Midwinter", 0),  // After Hammer (index 0)
                IntercalaryDay::new("Greengrass", 3), // After Tarsakh (index 3)
                IntercalaryDay::new("Midsummer", 6),  // After Flamerule (index 6)
                IntercalaryDay::new("Highharvestide", 8), // After Eleint (index 8)
                IntercalaryDay::new("Feast of the Moon", 10), // After Uktar (index 10)
            ],
            era: Some(EraDefinition::new("DR", 1)),
        }
    }
}

// ============================================================================
// EpochConfig
// ============================================================================

/// Configuration for what "second 0" represents in the calendar
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochConfig {
    /// The calendar to use for this world
    pub calendar_id: CalendarId,
    /// What year second 0 falls in (e.g., 1492 for 1492 DR)
    pub epoch_year: i32,
    /// What month second 0 falls in (1-indexed, e.g., 1 for Hammer/January)
    pub epoch_month: u8,
    /// What day second 0 falls in (1-indexed)
    pub epoch_day: u8,
    /// What hour second 0 falls in (0-23)
    pub epoch_hour: u8,
}

impl EpochConfig {
    /// Create a new epoch configuration.
    pub fn new(
        calendar_id: CalendarId,
        epoch_year: i32,
        epoch_month: u8,
        epoch_day: u8,
        epoch_hour: u8,
    ) -> Self {
        Self {
            calendar_id,
            epoch_year,
            epoch_month,
            epoch_day,
            epoch_hour,
        }
    }

    /// Create a default Gregorian epoch (January 1, Year 1, 00:00).
    pub fn gregorian_default() -> Self {
        Self {
            calendar_id: CalendarId::new("gregorian").expect("gregorian is a valid calendar ID"),
            epoch_year: 1,
            epoch_month: 1,
            epoch_day: 1,
            epoch_hour: 0,
        }
    }

    /// Create a default Harptos epoch for a Forgotten Realms campaign.
    /// Defaults to Hammer 1, 1492 DR, 00:00.
    pub fn harptos_default() -> Self {
        Self {
            calendar_id: CalendarId::new("harptos").expect("harptos is a valid calendar ID"),
            epoch_year: 1492,
            epoch_month: 1,
            epoch_day: 1,
            epoch_hour: 0,
        }
    }
}

impl Default for EpochConfig {
    fn default() -> Self {
        Self::gregorian_default()
    }
}

// ============================================================================
// CalendarDate
// ============================================================================

/// Formatted calendar date output from GameTime conversion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Second (0-59)
    pub second: u8,
    /// Time period (Morning, Afternoon, Evening, Night)
    pub period: TimeOfDay,
    /// If this is an intercalary day, its name
    pub intercalary_day: Option<String>,
    /// Era suffix (e.g., "DR", "AD")
    pub era_suffix: Option<String>,
}

impl CalendarDate {
    /// Display the full date (e.g., "15th of Hammer, 1492 DR").
    pub fn display_full(&self) -> String {
        let day_suffix = ordinal_suffix(self.day);

        if let Some(ref intercalary) = self.intercalary_day {
            // Intercalary days are special - they don't belong to a month
            if let Some(ref era) = self.era_suffix {
                format!("{}, {} {}", intercalary, self.year, era)
            } else {
                format!("{}, {}", intercalary, self.year)
            }
        } else if let Some(ref era) = self.era_suffix {
            format!(
                "{}{} of {}, {} {}",
                self.day, day_suffix, self.month_name, self.year, era
            )
        } else {
            format!(
                "{}{} of {}, {}",
                self.day, day_suffix, self.month_name, self.year
            )
        }
    }

    /// Display the short date (e.g., "Hammer 15, 1492").
    pub fn display_short(&self) -> String {
        if let Some(ref intercalary) = self.intercalary_day {
            format!("{}, {}", intercalary, self.year)
        } else {
            format!("{} {}, {}", self.month_name, self.day, self.year)
        }
    }

    /// Display the time (e.g., "9:00 AM").
    pub fn display_time(&self) -> String {
        let period = if self.hour >= 12 { "PM" } else { "AM" };
        let display_hour = if self.hour == 0 {
            12
        } else if self.hour > 12 {
            self.hour - 12
        } else {
            self.hour
        };

        format!(
            "{}:{:02}:{:02} {}",
            display_hour, self.minute, self.second, period
        )
    }

    /// Display ordinal style (e.g., "Day 15, 9:00 AM").
    pub fn display_ordinal(&self) -> String {
        format!("Day {}, {}", self.day, self.display_time())
    }

    /// Display the time period (e.g., "Morning").
    pub fn display_period(&self) -> String {
        self.period.display_name().to_string()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Returns the ordinal suffix for a day number (st, nd, rd, th).
fn ordinal_suffix(day: u8) -> &'static str {
    match day {
        1 | 21 | 31 => "st",
        2 | 22 => "nd",
        3 | 23 => "rd",
        _ => "th",
    }
}

/// Calculates a CalendarDate from total seconds since epoch.
///
/// This function converts raw seconds (from GameTime) into a human-readable
/// calendar date using the provided calendar definition and epoch configuration.
pub fn calculate_calendar_date(
    total_seconds: i64,
    calendar: &CalendarDefinition,
    epoch: &EpochConfig,
) -> CalendarDate {
    let seconds_per_day = (calendar.minutes_per_day() as i64) * 60;
    let seconds_per_year = calendar.minutes_per_year() * 60;
    let days_in_year = calendar.days_in_year() as i64;

    // Calculate epoch offset in seconds
    // The epoch config tells us what calendar date second 0 represents
    let epoch_day_of_year = calculate_day_of_year(epoch.epoch_month, epoch.epoch_day, calendar);
    let epoch_second_of_day = epoch.epoch_hour as i64 * calendar.minutes_per_hour() as i64 * 60;
    let epoch_offset_seconds =
        (epoch_day_of_year as i64 - 1) * seconds_per_day + epoch_second_of_day;

    // Adjust total_seconds to be relative to year start
    let adjusted_seconds = total_seconds + epoch_offset_seconds;

    // Calculate year offset from epoch year
    let year_offset = if adjusted_seconds >= 0 {
        adjusted_seconds / seconds_per_year
    } else {
        // For negative seconds, we need to round towards negative infinity
        (adjusted_seconds - seconds_per_year + 1) / seconds_per_year
    };

    let year = epoch.epoch_year + year_offset as i32;

    // Calculate remaining seconds within the year
    let mut seconds_in_year = adjusted_seconds - (year_offset * seconds_per_year);
    if seconds_in_year < 0 {
        seconds_in_year += seconds_per_year;
    }

    // Calculate day of year (0-indexed)
    let day_of_year = (seconds_in_year / seconds_per_day) as u32;
    let second_of_day = (seconds_in_year % seconds_per_day) as u32;

    // Calculate hour, minute, and second
    let seconds_per_hour = calendar.minutes_per_hour() as u32 * 60;
    let hour = (second_of_day / seconds_per_hour) as u8;
    let remaining_seconds = second_of_day % seconds_per_hour;
    let minute = (remaining_seconds / 60) as u8;
    let second = (remaining_seconds % 60) as u8;

    // Determine time of day period
    let period = time_of_day_from_hour(hour);

    // Convert day_of_year to month/day, accounting for intercalary days
    let (month_index, day, intercalary_day) = day_of_year_to_month_day(day_of_year, calendar);

    let month_name = if month_index < calendar.months.len() {
        calendar.months[month_index].name.clone()
    } else {
        "Unknown".to_string()
    };

    // Calculate day of week
    // We need a consistent starting point - day 0 of year 1 is day 0 of the week
    let total_days = if year >= 1 {
        (year - 1) as i64 * days_in_year + day_of_year as i64
    } else {
        // Negative years: year 0 is before year 1
        year as i64 * days_in_year + day_of_year as i64
    };

    let day_of_week = if !calendar.day_names.is_empty() {
        ((total_days % calendar.day_names.len() as i64 + calendar.day_names.len() as i64)
            % calendar.day_names.len() as i64) as u8
    } else {
        0
    };

    let day_of_week_name = if !calendar.day_names.is_empty() {
        calendar.day_names[day_of_week as usize].clone()
    } else {
        "Day".to_string()
    };

    let era_suffix = calendar.era.as_ref().map(|e| e.suffix.clone());

    CalendarDate {
        year,
        month: (month_index + 1) as u8,
        month_name,
        day,
        day_of_week,
        day_of_week_name,
        hour,
        minute,
        second,
        period,
        intercalary_day,
        era_suffix,
    }
}

/// Converts a day of year (0-indexed) to month index, day of month, and optional intercalary day name.
fn day_of_year_to_month_day(
    day_of_year: u32,
    calendar: &CalendarDefinition,
) -> (usize, u8, Option<String>) {
    let mut remaining_days = day_of_year;

    for (month_idx, month) in calendar.months.iter().enumerate() {
        if remaining_days < month.days as u32 {
            // We're in this month
            return (month_idx, (remaining_days + 1) as u8, None);
        }

        remaining_days -= month.days as u32;

        // Check for intercalary days after this month
        for intercalary in &calendar.intercalary_days {
            if intercalary.after_month as usize == month_idx {
                if remaining_days == 0 {
                    // This is the intercalary day
                    return (month_idx, 0, Some(intercalary.name.clone()));
                }
                remaining_days -= 1;
            }
        }
    }

    // Fallback - shouldn't happen with valid calendar
    (0, 1, None)
}

/// Calculates the day of year (1-indexed) for a given month and day.
fn calculate_day_of_year(month: u8, day: u8, calendar: &CalendarDefinition) -> u32 {
    if month == 0 || day == 0 {
        return 1;
    }

    let mut day_of_year: u32 = 0;

    // Add days from previous months
    for (idx, month_def) in calendar.months.iter().enumerate() {
        if idx + 1 < month as usize {
            day_of_year += month_def.days as u32;

            // Add intercalary days after this month
            for intercalary in &calendar.intercalary_days {
                if intercalary.after_month as usize == idx {
                    day_of_year += 1;
                }
            }
        }
    }

    // Add the day of the current month
    day_of_year += day as u32;

    day_of_year
}

/// Determines the time of day from an hour value.
fn time_of_day_from_hour(hour: u8) -> TimeOfDay {
    match hour {
        5..=11 => TimeOfDay::Morning,
        12..=17 => TimeOfDay::Afternoon,
        18..=21 => TimeOfDay::Evening,
        _ => TimeOfDay::Night,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod calendar_id {
        use super::*;

        #[test]
        fn valid_id() {
            let id = CalendarId::new("gregorian").unwrap();
            assert_eq!(id.as_str(), "gregorian");
            assert_eq!(id.to_string(), "gregorian");
        }

        #[test]
        fn valid_id_with_underscore() {
            let id = CalendarId::new("forgotten_realms").unwrap();
            assert_eq!(id.as_str(), "forgotten_realms");
        }

        #[test]
        fn valid_id_with_numbers() {
            let id = CalendarId::new("calendar_v2").unwrap();
            assert_eq!(id.as_str(), "calendar_v2");
        }

        #[test]
        fn empty_id_rejected() {
            let result = CalendarId::new("");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("cannot be empty"));
        }

        #[test]
        fn whitespace_only_rejected() {
            let result = CalendarId::new("   ");
            assert!(result.is_err());
        }

        #[test]
        fn id_is_lowercased() {
            let id = CalendarId::new("GREGORIAN").unwrap();
            assert_eq!(id.as_str(), "gregorian");
        }

        #[test]
        fn id_is_trimmed() {
            let id = CalendarId::new("  harptos  ").unwrap();
            assert_eq!(id.as_str(), "harptos");
        }

        #[test]
        fn too_long_rejected() {
            let long_id = "a".repeat(51);
            let result = CalendarId::new(long_id);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("50"));
        }

        #[test]
        fn max_length_accepted() {
            let max_id = "a".repeat(50);
            let id = CalendarId::new(max_id).unwrap();
            assert_eq!(id.as_str().len(), 50);
        }

        #[test]
        fn special_chars_rejected() {
            let result = CalendarId::new("cal-endar");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("lowercase alphanumeric"));
        }

        #[test]
        fn spaces_rejected() {
            let result = CalendarId::new("my calendar");
            assert!(result.is_err());
        }

        #[test]
        fn try_from_string() {
            let id: CalendarId = "harptos".to_string().try_into().unwrap();
            assert_eq!(id.as_str(), "harptos");
        }

        #[test]
        fn into_string() {
            let id = CalendarId::new("eberron").unwrap();
            let s: String = id.into();
            assert_eq!(s, "eberron");
        }

        #[test]
        fn clone_preserves_id() {
            let id = CalendarId::new("custom").unwrap();
            let cloned = id.clone();
            assert_eq!(cloned.as_str(), "custom");
        }
    }

    mod season {
        use super::*;

        #[test]
        fn display_names() {
            assert_eq!(Season::Spring.display_name(), "Spring");
            assert_eq!(Season::Summer.display_name(), "Summer");
            assert_eq!(Season::Autumn.display_name(), "Autumn");
            assert_eq!(Season::Winter.display_name(), "Winter");
        }

        #[test]
        fn display_trait() {
            assert_eq!(Season::Spring.to_string(), "Spring");
        }
    }

    mod calendar_definition {
        use super::*;

        #[test]
        fn gregorian_has_365_days() {
            let calendar = CalendarDefinition::gregorian();
            assert_eq!(calendar.days_in_year(), 365);
        }

        #[test]
        fn gregorian_has_12_months() {
            let calendar = CalendarDefinition::gregorian();
            assert_eq!(calendar.months().len(), 12);
        }

        #[test]
        fn gregorian_has_7_day_week() {
            let calendar = CalendarDefinition::gregorian();
            assert_eq!(calendar.day_names().len(), 7);
        }

        #[test]
        fn gregorian_minutes_per_day() {
            let calendar = CalendarDefinition::gregorian();
            assert_eq!(calendar.minutes_per_day(), 24 * 60);
        }

        #[test]
        fn harptos_has_365_days() {
            let calendar = CalendarDefinition::harptos();
            // 12 months * 30 days + 5 intercalary days = 365
            assert_eq!(calendar.days_in_year(), 365);
        }

        #[test]
        fn harptos_has_12_months() {
            let calendar = CalendarDefinition::harptos();
            assert_eq!(calendar.months().len(), 12);
        }

        #[test]
        fn harptos_has_10_day_week() {
            let calendar = CalendarDefinition::harptos();
            assert_eq!(calendar.day_names().len(), 10);
        }

        #[test]
        fn harptos_has_5_intercalary_days() {
            let calendar = CalendarDefinition::harptos();
            assert_eq!(calendar.intercalary_days().len(), 5);
        }

        #[test]
        fn harptos_era_is_dr() {
            let calendar = CalendarDefinition::harptos();
            assert_eq!(calendar.era().unwrap().suffix, "DR");
        }

        #[test]
        fn accessors_work() {
            let calendar = CalendarDefinition::gregorian();
            assert_eq!(calendar.id().as_str(), "gregorian");
            assert_eq!(calendar.name(), "Gregorian Calendar");
            assert_eq!(calendar.hours_per_day(), 24);
            assert_eq!(calendar.minutes_per_hour(), 60);
        }
    }

    mod epoch_config {
        use super::*;

        #[test]
        fn gregorian_default() {
            let epoch = EpochConfig::gregorian_default();
            assert_eq!(epoch.calendar_id.as_str(), "gregorian");
            assert_eq!(epoch.epoch_year, 1);
            assert_eq!(epoch.epoch_month, 1);
            assert_eq!(epoch.epoch_day, 1);
            assert_eq!(epoch.epoch_hour, 0);
        }

        #[test]
        fn harptos_default() {
            let epoch = EpochConfig::harptos_default();
            assert_eq!(epoch.calendar_id.as_str(), "harptos");
            assert_eq!(epoch.epoch_year, 1492);
            assert_eq!(epoch.epoch_month, 1);
            assert_eq!(epoch.epoch_day, 1);
            assert_eq!(epoch.epoch_hour, 0);
        }

        #[test]
        fn default_is_gregorian() {
            let epoch = EpochConfig::default();
            assert_eq!(epoch.calendar_id.as_str(), "gregorian");
        }
    }

    mod calendar_date {
        use super::*;

        #[test]
        fn display_short() {
            let date = CalendarDate {
                year: 1492,
                month: 5,
                month_name: "Mirtul".to_string(),
                day: 15,
                day_of_week: 4,
                day_of_week_name: "Fifth-day".to_string(),
                hour: 14,
                minute: 30,
                second: 0,
                period: TimeOfDay::Afternoon,
                intercalary_day: None,
                era_suffix: Some("DR".to_string()),
            };
            assert_eq!(date.display_short(), "Mirtul 15, 1492");
        }

        #[test]
        fn display_full_without_era() {
            let date = CalendarDate {
                year: 1492,
                month: 1,
                month_name: "Hammer".to_string(),
                day: 1,
                day_of_week: 0,
                day_of_week_name: "First-day".to_string(),
                hour: 0,
                minute: 0,
                second: 0,
                period: TimeOfDay::Night,
                intercalary_day: None,
                era_suffix: None,
            };
            assert_eq!(date.display_full(), "1st of Hammer, 1492");
        }

        #[test]
        fn display_full_with_era() {
            let date = CalendarDate {
                year: 1492,
                month: 1,
                month_name: "Hammer".to_string(),
                day: 0,
                day_of_week: 0,
                day_of_week_name: "First-day".to_string(),
                hour: 12,
                minute: 0,
                second: 0,
                period: TimeOfDay::Afternoon,
                intercalary_day: Some("Midwinter".to_string()),
                era_suffix: Some("DR".to_string()),
            };
            assert_eq!(date.display_full(), "Midwinter, 1492 DR");
        }

        #[test]
        fn display_time_am() {
            let date = CalendarDate {
                year: 1492,
                month: 1,
                month_name: "Hammer".to_string(),
                day: 1,
                day_of_week: 0,
                day_of_week_name: "First-day".to_string(),
                hour: 9,
                minute: 30,
                second: 0,
                period: TimeOfDay::Morning,
                intercalary_day: None,
                era_suffix: None,
            };
            assert_eq!(date.display_ordinal(), "Day 1, 9:30:00 AM");
        }

        #[test]
        fn display_period() {
            let date = CalendarDate {
                year: 1492,
                month: 1,
                month_name: "Hammer".to_string(),
                day: 1,
                day_of_week: 0,
                day_of_week_name: "First-day".to_string(),
                hour: 9,
                minute: 0,
                second: 0,
                period: TimeOfDay::Morning,
                intercalary_day: None,
                era_suffix: None,
            };
            assert_eq!(date.display_period(), "Morning");
        }

        #[test]
        fn ordinal_suffixes() {
            assert_eq!(ordinal_suffix(1), "st");
            assert_eq!(ordinal_suffix(2), "nd");
            assert_eq!(ordinal_suffix(3), "rd");
            assert_eq!(ordinal_suffix(4), "th");
            assert_eq!(ordinal_suffix(11), "th");
            assert_eq!(ordinal_suffix(12), "th");
            assert_eq!(ordinal_suffix(13), "th");
            assert_eq!(ordinal_suffix(21), "st");
            assert_eq!(ordinal_suffix(22), "nd");
            assert_eq!(ordinal_suffix(23), "rd");
            assert_eq!(ordinal_suffix(31), "st");
        }
    }

    mod calculate_calendar_date {
        use super::*;

        #[test]
        fn epoch_is_day_one() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let date = calculate_calendar_date(0, &calendar, &epoch);

            assert_eq!(date.year, 1);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 1);
            assert_eq!(date.hour, 0);
            assert_eq!(date.minute, 0);
        }

        #[test]
        fn nine_hours_after_epoch() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let date = calculate_calendar_date(9 * 60 * 60, &calendar, &epoch); // 9 hours = 32400 seconds

            assert_eq!(date.year, 1);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 1);
            assert_eq!(date.hour, 9);
            assert_eq!(date.minute, 0);
            assert_eq!(date.period, TimeOfDay::Morning);
        }

        #[test]
        fn one_day_after_epoch() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let date = calculate_calendar_date(24 * 60 * 60, &calendar, &epoch); // 1 day = 86400 seconds

            assert_eq!(date.year, 1);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 2);
            assert_eq!(date.hour, 0);
        }

        #[test]
        fn end_of_january() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            // 30 days after epoch = January 31
            let date = calculate_calendar_date(30 * 24 * 60 * 60, &calendar, &epoch);

            assert_eq!(date.year, 1);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 31);
        }

        #[test]
        fn february_first() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            // 31 days after epoch = February 1
            let date = calculate_calendar_date(31 * 24 * 60 * 60, &calendar, &epoch);

            assert_eq!(date.year, 1);
            assert_eq!(date.month, 2);
            assert_eq!(date.day, 1);
            assert_eq!(date.month_name, "February");
        }

        #[test]
        fn negative_time_before_epoch() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::new(
                CalendarId::new("gregorian").unwrap(),
                2024,
                1,
                2, // January 2
                0,
            );

            // 1 day before epoch = January 1
            let date = calculate_calendar_date(-24 * 60 * 60, &calendar, &epoch);

            assert_eq!(date.year, 2024);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 1);
        }

        #[test]
        fn harptos_epoch() {
            let calendar = CalendarDefinition::harptos();
            let epoch = EpochConfig::harptos_default();

            let date = calculate_calendar_date(0, &calendar, &epoch);

            assert_eq!(date.year, 1492);
            assert_eq!(date.month, 1);
            assert_eq!(date.month_name, "Hammer");
            assert_eq!(date.day, 1);
            assert_eq!(date.era_suffix, Some("DR".to_string()));
        }

        #[test]
        fn harptos_after_hammer() {
            let calendar = CalendarDefinition::harptos();
            let epoch = EpochConfig::harptos_default();

            // 30 days after epoch = last day of Hammer
            let date = calculate_calendar_date(29 * 24 * 60 * 60, &calendar, &epoch);
            assert_eq!(date.month_name, "Hammer");
            assert_eq!(date.day, 30);

            // 31st day = Midwinter (intercalary)
            let date = calculate_calendar_date(30 * 24 * 60 * 60, &calendar, &epoch);
            assert_eq!(date.intercalary_day, Some("Midwinter".to_string()));
        }

        #[test]
        fn harptos_alturiak_first() {
            let calendar = CalendarDefinition::harptos();
            let epoch = EpochConfig::harptos_default();

            // 32nd day = Alturiak 1 (after 30 days of Hammer + 1 Midwinter)
            let date = calculate_calendar_date(31 * 24 * 60 * 60, &calendar, &epoch);

            assert_eq!(date.month, 2);
            assert_eq!(date.month_name, "Alturiak");
            assert_eq!(date.day, 1);
            assert_eq!(date.intercalary_day, None);
        }

        #[test]
        fn year_rollover() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            // 365 days = start of year 2
            let date = calculate_calendar_date(365 * 24 * 60 * 60, &calendar, &epoch);

            assert_eq!(date.year, 2);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 1);
        }

        #[test]
        fn custom_epoch_hour() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::new(
                CalendarId::new("gregorian").unwrap(),
                2024,
                1,
                1,
                9, // 9 AM
            );

            // At minute 0, it should be 9 AM
            let date = calculate_calendar_date(0, &calendar, &epoch);

            assert_eq!(date.year, 2024);
            assert_eq!(date.month, 1);
            assert_eq!(date.day, 1);
            assert_eq!(date.hour, 9);
        }

        #[test]
        fn custom_epoch_mid_year() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::new(
                CalendarId::new("gregorian").unwrap(),
                2024,
                6,
                15, // June 15
                12, // Noon
            );

            let date = calculate_calendar_date(0, &calendar, &epoch);

            assert_eq!(date.year, 2024);
            assert_eq!(date.month, 6);
            assert_eq!(date.month_name, "June");
            assert_eq!(date.day, 15);
            assert_eq!(date.hour, 12);
        }

        #[test]
        fn day_of_week_increments() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let day0 = calculate_calendar_date(0, &calendar, &epoch);
            let day1 = calculate_calendar_date(24 * 60 * 60, &calendar, &epoch); // 1 day
            let day7 = calculate_calendar_date(7 * 24 * 60 * 60, &calendar, &epoch); // 7 days

            // Day of week should increment
            assert_eq!(
                (day1.day_of_week as i32 - day0.day_of_week as i32 + 7) % 7,
                1
            );

            // After 7 days, should be same day of week
            assert_eq!(day7.day_of_week, day0.day_of_week);
        }

        #[test]
        fn time_periods() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            // Morning (5-11)
            let morning = calculate_calendar_date(5 * 60 * 60, &calendar, &epoch); // 5 AM
            assert_eq!(morning.period, TimeOfDay::Morning);

            // Afternoon (12-17)
            let afternoon = calculate_calendar_date(14 * 60 * 60, &calendar, &epoch); // 2 PM
            assert_eq!(afternoon.period, TimeOfDay::Afternoon);

            // Evening (18-21)
            let evening = calculate_calendar_date(19 * 60 * 60, &calendar, &epoch); // 7 PM
            assert_eq!(evening.period, TimeOfDay::Evening);

            // Night (22-4)
            let night = calculate_calendar_date(23 * 60 * 60, &calendar, &epoch); // 11 PM
            assert_eq!(night.period, TimeOfDay::Night);
        }

        #[test]
        fn minutes_precision() {
            let calendar = CalendarDefinition::gregorian();
            let epoch = EpochConfig::gregorian_default();

            let date = calculate_calendar_date(5400, &calendar, &epoch); // 90 minutes = 5400 seconds = 1:30 AM

            assert_eq!(date.hour, 1);
            assert_eq!(date.minute, 30);
            assert_eq!(date.second, 0);
        }
    }
}

#[allow(unused_imports)]
mod integration {
    use super::{calculate_calendar_date, CalendarDefinition, CalendarId, EpochConfig, TimeOfDay};

    #[test]
    fn forgotten_realms_campaign_start() {
        // Typical FR campaign: 1492 DR, Hammer 1, 9:00 AM
        let calendar = CalendarDefinition::harptos();
        let epoch = EpochConfig::new(
            CalendarId::new("harptos").unwrap(),
            1492,
            1, // Hammer
            1,
            9, // 9 AM
        );

        let date = calculate_calendar_date(0, &calendar, &epoch);

        assert_eq!(date.display_full(), "1st of Hammer, 1492 DR");
        assert_eq!(date.display_time(), "9:00:00 AM");
        assert_eq!(date.display_period(), "Morning");
    }

    #[test]
    fn forgotten_realms_three_hours_later() {
        let calendar = CalendarDefinition::harptos();
        let epoch = EpochConfig::new(CalendarId::new("harptos").unwrap(), 1492, 1, 1, 9);

        // 3 hours later = noon
        let date = calculate_calendar_date(3 * 60 * 60, &calendar, &epoch);

        assert_eq!(date.hour, 12);
        assert_eq!(date.display_time(), "12:00:00 PM");
        assert_eq!(date.period, TimeOfDay::Afternoon);
    }

    #[test]
    fn gregorian_full_year_cycle() {
        let calendar = CalendarDefinition::gregorian();
        let epoch = EpochConfig::gregorian_default();

        // Check each month boundary
        let months_data = [
            (0, 1, "January"),
            (31, 2, "February"),
            (31 + 28, 3, "March"),
            (31 + 28 + 31, 4, "April"),
            (31 + 28 + 31 + 30, 5, "May"),
            (31 + 28 + 31 + 30 + 31, 6, "June"),
        ];

        for (days, expected_month, expected_name) in months_data {
            let date = calculate_calendar_date(days * 24 * 60 * 60, &calendar, &epoch);
            assert_eq!(
                date.month, expected_month,
                "Month mismatch for day {}",
                days
            );
            assert_eq!(
                date.month_name, expected_name,
                "Month name mismatch for day {}",
                days
            );
            assert_eq!(date.day, 1, "Day should be 1 for day {}", days);
        }
    }
}
