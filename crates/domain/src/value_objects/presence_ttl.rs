//! Presence cache TTL (time-to-live) in hours
//!
//! A newtype representing the duration (in hours) that
//! NPC presence information should be cached before requiring refresh.
//!
//! # Tier Classification
//!
//! - **Tier 2: Validated Newtype** - Wraps `i32` with validation rules
//!
//! See [docs/architecture/tier-levels.md](../../../../docs/architecture/tier-levels.md)
//! for complete tier classification system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Presence cache TTL in hours (validated newtype)
///
/// This newtype represents the time-to-live for NPC presence cache,
/// ensuring invalid values cannot be constructed.
///
/// # Validation Rules
///
/// - Value must be >= 0 (non-negative)
/// - Value must be <= 8760 (1 year in hours) - prevents unreasonably long caches
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::value_objects::PresenceTtlHours;
///
/// // Valid values
/// let ttl = PresenceTtlHours::new(3).unwrap();
/// assert_eq!(ttl.value(), 3);
///
/// let zero = PresenceTtlHours::new(0).unwrap();
/// assert_eq!(zero.value(), 0);
///
/// // Invalid values
/// assert!(PresenceTtlHours::new(-1).is_err());
/// assert!(PresenceTtlHours::new(8761).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "i32", into = "i32")]
pub struct PresenceTtlHours(i32);

impl PresenceTtlHours {
    /// Minimum valid value: 0 hours
    pub const MIN: i32 = 0;

    /// Maximum valid value: 8760 hours (1 year)
    pub const MAX: i32 = 8760;

    /// Default value: 3 hours
    pub const DEFAULT: i32 = 3;

    /// Create a new `PresenceTtlHours` value.
    ///
    /// # Errors
    ///
    /// Returns `DomainError` if the value is outside valid range:
    /// - Must be >= 0
    /// - Must be <= 8760 (1 year in hours)
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::PresenceTtlHours;
    ///
    /// let valid = PresenceTtlHours::new(3).unwrap();
    /// assert_eq!(valid.value(), 3);
    ///
    /// let invalid = PresenceTtlHours::new(-1);
    /// assert!(invalid.is_err());
    /// ```
    pub fn new(hours: i32) -> Result<Self, crate::DomainError> {
        if hours < Self::MIN {
            return Err(crate::DomainError::validation(format!(
                "Presence TTL must be >= {} hours, got {}",
                Self::MIN,
                hours
            )));
        }

        if hours > Self::MAX {
            return Err(crate::DomainError::validation(format!(
                "Presence TTL must be <= {} hours (1 year), got {}",
                Self::MAX,
                hours
            )));
        }

        Ok(Self(hours))
    }

    /// Create a new `PresenceTtlHours` value, clamping to valid range.
    ///
    /// This is a convenience method for cases where you want to ensure
    /// validity without explicit error handling (e.g., from user input).
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::PresenceTtlHours;
    ///
    /// // Too large - clamped to MAX
    /// let clamped = PresenceTtlHours::clamped(9000);
    /// assert_eq!(clamped.value(), 8760);
    ///
    /// // Negative - clamped to MIN
    /// let clamped2 = PresenceTtlHours::clamped(-5);
    /// assert_eq!(clamped2.value(), 0);
    ///
    /// // Valid - unchanged
    /// let valid = PresenceTtlHours::clamped(6);
    /// assert_eq!(valid.value(), 6);
    /// ```
    pub fn clamped(hours: i32) -> Self {
        Self(hours.clamp(Self::MIN, Self::MAX))
    }

    /// Returns the underlying `i32` value.
    ///
    /// This is a read-only accessor since the value is validated
    /// at construction time.
    #[inline]
    pub const fn value(self) -> i32 {
        self.0
    }

    /// Returns the default value (3 hours).
    #[inline]
    pub const fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

impl Default for PresenceTtlHours {
    fn default() -> Self {
        Self::default()
    }
}

impl fmt::Display for PresenceTtlHours {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} hours", self.0)
    }
}

// Implement conversions for interop with code using raw i32

impl From<PresenceTtlHours> for i32 {
    fn from(ttl: PresenceTtlHours) -> Self {
        ttl.0
    }
}

impl TryFrom<i32> for PresenceTtlHours {
    type Error = crate::DomainError;

    fn try_from(hours: i32) -> Result<Self, Self::Error> {
        Self::new(hours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_valid_values() {
        let ttl = PresenceTtlHours::new(3).unwrap();
        assert_eq!(ttl.value(), 3);
    }

    #[test]
    fn new_rejects_negative() {
        let result = PresenceTtlHours::new(-1);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("must be >= 0"));
        }
    }

    #[test]
    fn new_rejects_too_large() {
        let result = PresenceTtlHours::new(8761);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("must be <= 8760"));
        }
    }

    #[test]
    fn clamped_brings_negative_to_min() {
        let ttl = PresenceTtlHours::clamped(-5);
        assert_eq!(ttl.value(), 0);
    }

    #[test]
    fn clamped_brings_large_to_max() {
        let ttl = PresenceTtlHours::clamped(9000);
        assert_eq!(ttl.value(), 8760);
    }

    #[test]
    fn clamped_preserves_valid_values() {
        let ttl = PresenceTtlHours::clamped(6);
        assert_eq!(ttl.value(), 6);
    }

    #[test]
    fn default_returns_3_hours() {
        let ttl = PresenceTtlHours::default();
        assert_eq!(ttl.value(), 3);
    }

    #[test]
    fn display_formats_correctly() {
        let ttl = PresenceTtlHours::new(5).unwrap();
        assert_eq!(ttl.to_string(), "5 hours");
    }
}
