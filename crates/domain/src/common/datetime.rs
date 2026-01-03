//! DateTime parsing utilities with consistent error handling.

use chrono::{DateTime, Utc};

/// Parses an RFC3339 timestamp string, returning an error if parsing fails.
///
/// # Examples
///
/// ```
/// use wrldbldr_common::parse_datetime;
/// use chrono::Datelike;
///
/// let dt = parse_datetime("2024-01-15T10:30:00Z").unwrap();
/// assert_eq!(dt.year(), 2024);
/// ```
///
/// # Errors
///
/// Returns `chrono::ParseError` if the string is not valid RFC3339.
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

/// Parses an RFC3339 timestamp string, falling back to provided default on error.
///
/// This is useful for database fields that should have a valid timestamp
/// even if the stored value is malformed. Use with ClockPort for testability:
///
/// ```ignore
/// parse_datetime_or(&timestamp_str, self.clock.now())
/// ```
///
/// # Examples
///
/// ```
/// use wrldbldr_common::parse_datetime_or;
/// use chrono::{DateTime, Utc, TimeZone, Datelike};
///
/// let default = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
///
/// // Valid timestamp returns parsed value
/// let dt = parse_datetime_or("2024-01-15T10:30:00Z", default);
/// assert_eq!(dt.year(), 2024);
///
/// // Invalid timestamp returns default
/// let dt = parse_datetime_or("not-a-date", default);
/// assert_eq!(dt.year(), 2020);
/// ```
pub fn parse_datetime_or(s: &str, default: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Timelike};

    #[test]
    fn test_parse_datetime_valid() {
        let dt = parse_datetime("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_datetime_with_timezone() {
        let dt = parse_datetime("2024-01-15T10:30:00+05:00").unwrap();
        // Should be converted to UTC
        assert_eq!(dt.hour(), 5); // 10:30 +05:00 = 05:30 UTC
    }

    #[test]
    fn test_parse_datetime_invalid() {
        assert!(parse_datetime("not-a-date").is_err());
        assert!(parse_datetime("").is_err());
        assert!(parse_datetime("2024-01-15").is_err()); // Missing time component
    }

    #[test]
    fn test_parse_datetime_or_valid() {
        let default = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let dt = parse_datetime_or("2024-01-15T10:30:00Z", default);
        assert_eq!(dt.year(), 2024);
    }

    #[test]
    fn test_parse_datetime_or_invalid_returns_default() {
        let default = Utc.with_ymd_and_hms(2020, 6, 15, 12, 0, 0).unwrap();
        let dt = parse_datetime_or("invalid", default);
        assert_eq!(dt, default);
    }

    #[test]
    fn test_parse_datetime_or_empty_returns_default() {
        let default = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let dt = parse_datetime_or("", default);
        assert_eq!(dt, default);
    }
}
