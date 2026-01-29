//! Time restriction value object for challenge availability.
//!
//! Challenges can be restricted to specific times of day.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};

/// Time restriction for challenge availability.
///
/// Controls when a challenge can be triggered based on game time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityTimeRestriction {
    /// Challenge is available during morning hours
    Morning,
    /// Challenge is available during afternoon hours
    Afternoon,
    /// Challenge is available during evening hours
    Evening,
    /// Challenge is available during night hours
    Night,
}

impl AvailabilityTimeRestriction {
    /// Parse a string into a time restriction.
    ///
    /// # Arguments
    /// * `s` - String to parse (case-insensitive)
    ///
    /// # Examples
    /// ```
    /// # use wrldbldr_domain::value_objects::AvailabilityTimeRestriction;
    /// assert!(AvailabilityTimeRestriction::parse("morning").is_ok());
    /// assert!(AvailabilityTimeRestriction::parse("Night").is_ok());
    /// assert!(AvailabilityTimeRestriction::parse("invalid").is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        match s.trim().to_lowercase().as_str() {
            "morning" => Ok(Self::Morning),
            "afternoon" => Ok(Self::Afternoon),
            "evening" => Ok(Self::Evening),
            "night" => Ok(Self::Night),
            _ => Err(DomainError::validation(format!(
                "Invalid time restriction: '{}'. Valid values: morning, afternoon, evening, night",
                s
            ))),
        }
    }

    /// Get the display name for this time restriction.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Morning => "Morning",
            Self::Afternoon => "Afternoon",
            Self::Evening => "Evening",
            Self::Night => "Night",
        }
    }
}

impl std::fmt::Display for AvailabilityTimeRestriction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl TryFrom<String> for AvailabilityTimeRestriction {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<AvailabilityTimeRestriction> for String {
    fn from(restriction: AvailabilityTimeRestriction) -> String {
        restriction.display_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_morning() {
        assert!(matches!(
            AvailabilityTimeRestriction::parse("morning"),
            Ok(AvailabilityTimeRestriction::Morning)
        ));
    }

    #[test]
    fn parse_afternoon() {
        assert!(matches!(
            AvailabilityTimeRestriction::parse("afternoon"),
            Ok(AvailabilityTimeRestriction::Afternoon)
        ));
    }

    #[test]
    fn parse_evening() {
        assert!(matches!(
            AvailabilityTimeRestriction::parse("evening"),
            Ok(AvailabilityTimeRestriction::Evening)
        ));
    }

    #[test]
    fn parse_night() {
        assert!(matches!(
            AvailabilityTimeRestriction::parse("night"),
            Ok(AvailabilityTimeRestriction::Night)
        ));
    }

    #[test]
    fn parse_case_insensitive() {
        assert!(matches!(
            AvailabilityTimeRestriction::parse("MORNING"),
            Ok(AvailabilityTimeRestriction::Morning)
        ));
        assert!(matches!(
            AvailabilityTimeRestriction::parse("Morning"),
            Ok(AvailabilityTimeRestriction::Morning)
        ));
        assert!(matches!(
            AvailabilityTimeRestriction::parse("mOrNiNg"),
            Ok(AvailabilityTimeRestriction::Morning)
        ));
    }

    #[test]
    fn parse_invalid() {
        assert!(AvailabilityTimeRestriction::parse("invalid").is_err());
        assert!(AvailabilityTimeRestriction::parse("midnight").is_err());
        assert!(AvailabilityTimeRestriction::parse("").is_err());
    }

    #[test]
    fn display_name() {
        assert_eq!(
            AvailabilityTimeRestriction::Morning.display_name(),
            "Morning"
        );
        assert_eq!(
            AvailabilityTimeRestriction::Afternoon.display_name(),
            "Afternoon"
        );
        assert_eq!(
            AvailabilityTimeRestriction::Evening.display_name(),
            "Evening"
        );
        assert_eq!(AvailabilityTimeRestriction::Night.display_name(), "Night");
    }

    #[test]
    fn to_string() {
        assert_eq!(
            String::from(AvailabilityTimeRestriction::Morning),
            "Morning"
        );
        assert_eq!(String::from(AvailabilityTimeRestriction::Night), "Night");
    }

    #[test]
    fn try_from_string() {
        assert!(matches!(
            AvailabilityTimeRestriction::try_from("morning".to_string()),
            Ok(AvailabilityTimeRestriction::Morning)
        ));
        assert!(AvailabilityTimeRestriction::try_from("invalid".to_string()).is_err());
    }
}
