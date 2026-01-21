//! Ad-hoc challenge outcomes for DM-created challenges
//!
//! Domain representation of custom outcome text for challenges
//! created on-the-fly by the DM.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};

/// Ad-hoc challenge outcomes for DM-created challenges
///
/// Domain representation of custom outcome text for challenges
/// created on-the-fly by the DM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    /// Outcome text for a successful roll
    success: String,
    /// Outcome text for a failed roll
    failure: String,
    /// Optional outcome text for a critical success
    critical_success: Option<String>,
    /// Optional outcome text for a critical failure
    critical_failure: Option<String>,
}

impl AdHocOutcomes {
    /// Create ad-hoc outcomes with all fields
    ///
    /// Returns an error if:
    /// - Success outcome is empty
    /// - Failure outcome is empty
    /// - Critical outcomes are not symmetric (both set or both None)
    pub fn new(
        success: impl Into<String>,
        failure: impl Into<String>,
        critical_success: Option<String>,
        critical_failure: Option<String>,
    ) -> Result<Self, DomainError> {
        let outcomes = Self {
            success: success.into(),
            failure: failure.into(),
            critical_success,
            critical_failure,
        };
        outcomes.validate()?;
        Ok(outcomes)
    }

    /// Create basic outcomes with only success/failure
    ///
    /// Returns an error if:
    /// - Success outcome is empty
    /// - Failure outcome is empty
    pub fn basic(
        success: impl Into<String>,
        failure: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let outcomes = Self {
            success: success.into(),
            failure: failure.into(),
            critical_success: None,
            critical_failure: None,
        };
        outcomes.validate()?;
        Ok(outcomes)
    }

    // --- Accessors ---

    /// Get the success outcome text
    pub fn success(&self) -> &str {
        &self.success
    }

    /// Get the failure outcome text
    pub fn failure(&self) -> &str {
        &self.failure
    }

    /// Get the critical success outcome text, if any
    pub fn critical_success(&self) -> Option<&str> {
        self.critical_success.as_deref()
    }

    /// Get the critical failure outcome text, if any
    pub fn critical_failure(&self) -> Option<&str> {
        self.critical_failure.as_deref()
    }

    // --- Builder methods ---

    /// Set the critical success outcome (builder pattern)
    pub fn with_critical_success(self, text: impl Into<String>) -> Self {
        Self {
            critical_success: Some(text.into()),
            ..self
        }
    }

    /// Set the critical failure outcome (builder pattern)
    pub fn with_critical_failure(self, text: impl Into<String>) -> Self {
        Self {
            critical_failure: Some(text.into()),
            ..self
        }
    }

    /// Set both critical outcomes (builder pattern)
    pub fn with_criticals(
        self,
        critical_success: impl Into<String>,
        critical_failure: impl Into<String>,
    ) -> Self {
        Self {
            critical_success: Some(critical_success.into()),
            critical_failure: Some(critical_failure.into()),
            ..self
        }
    }

    /// Validate the ad-hoc outcomes configuration.
    ///
    /// Checks that:
    /// - At least success or failure is non-empty
    /// - If critical_success is set, critical_failure is also set (symmetric criticals)
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.success.trim().is_empty() {
            return Err(DomainError::validation(
                "Success outcome cannot be empty".to_string(),
            ));
        }

        if self.failure.trim().is_empty() {
            return Err(DomainError::validation(
                "Failure outcome cannot be empty".to_string(),
            ));
        }

        // Check symmetric criticals (both set or both None)
        match (&self.critical_success, &self.critical_failure) {
            (None, None) | (Some(_), Some(_)) => {} // Both set or both None - OK
            (Some(_), None) => {
                return Err(DomainError::validation(
                    "Critical success requires critical failure to also be set".to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(DomainError::validation(
                    "Critical failure requires critical success to also be set".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let outcomes = AdHocOutcomes::new(
            "You succeed!",
            "You fail!",
            Some("Critical success!".to_string()),
            Some("Critical failure!".to_string()),
        )
        .unwrap();

        assert_eq!(outcomes.success(), "You succeed!");
        assert_eq!(outcomes.failure(), "You fail!");
        assert_eq!(outcomes.critical_success(), Some("Critical success!"));
        assert_eq!(outcomes.critical_failure(), Some("Critical failure!"));
    }

    #[test]
    fn test_basic() {
        let outcomes =
            AdHocOutcomes::basic("You pick the lock", "The lock resists your efforts").unwrap();

        assert_eq!(outcomes.success(), "You pick the lock");
        assert_eq!(outcomes.failure(), "The lock resists your efforts");
        assert_eq!(outcomes.critical_success(), None);
        assert_eq!(outcomes.critical_failure(), None);
    }

    #[test]
    fn test_builder_pattern() {
        let outcomes = AdHocOutcomes::basic("Success", "Failure")
            .unwrap()
            .with_critical_success("Epic success!")
            .with_critical_failure("Catastrophic failure!");

        assert_eq!(outcomes.success(), "Success");
        assert_eq!(outcomes.failure(), "Failure");
        assert_eq!(outcomes.critical_success(), Some("Epic success!"));
        assert_eq!(outcomes.critical_failure(), Some("Catastrophic failure!"));
    }

    #[test]
    fn test_with_criticals() {
        let outcomes = AdHocOutcomes::basic("Win", "Lose")
            .unwrap()
            .with_criticals("Big win", "Big lose");

        assert_eq!(outcomes.critical_success(), Some("Big win"));
        assert_eq!(outcomes.critical_failure(), Some("Big lose"));
    }

    #[test]
    fn test_outcomes_equality() {
        let outcomes = AdHocOutcomes::new(
            "Success",
            "Failure",
            Some("Crit success".to_string()),
            Some("Crit failure".to_string()),
        )
        .unwrap();

        let other = outcomes.clone();
        assert_eq!(outcomes, other);
    }

    #[test]
    fn test_validation_empty_success() {
        let result = AdHocOutcomes::new("", "Failure", None, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Success outcome cannot be empty"));
    }

    #[test]
    fn test_validation_empty_failure() {
        let result = AdHocOutcomes::new("Success", "", None, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failure outcome cannot be empty"));
    }

    #[test]
    fn test_validation_asymmetric_critical_success() {
        let result =
            AdHocOutcomes::new("Success", "Failure", Some("Crit success".to_string()), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Critical success requires critical failure"));
    }

    #[test]
    fn test_validation_asymmetric_critical_failure() {
        let result =
            AdHocOutcomes::new("Success", "Failure", None, Some("Crit failure".to_string()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Critical failure requires critical success"));
    }

    #[test]
    fn test_validation_whitespace_only() {
        let result = AdHocOutcomes::new("   ", "Failure", None, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Success outcome cannot be empty"));
    }

    #[test]
    fn test_validation_both_criticals_set() {
        let result = AdHocOutcomes::new(
            "Success",
            "Failure",
            Some("Crit success".to_string()),
            Some("Crit failure".to_string()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_both_criticals_none() {
        let result = AdHocOutcomes::new("Success", "Failure", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_validation_empty_success() {
        let result = AdHocOutcomes::basic("", "Failure");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Success outcome cannot be empty"));
    }

    #[test]
    fn test_basic_validation_empty_failure() {
        let result = AdHocOutcomes::basic("Success", "");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failure outcome cannot be empty"));
    }
}
