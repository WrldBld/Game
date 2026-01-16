//! Ad-hoc challenge outcomes for DM-created challenges
//!
//! Domain representation of custom outcome text for challenges
//! created on-the-fly by the DM.

use serde::{Deserialize, Serialize};

/// Ad-hoc challenge outcomes for DM-created challenges
///
/// Domain representation of custom outcome text for challenges
/// created on-the-fly by the DM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub fn new(
        success: impl Into<String>,
        failure: impl Into<String>,
        critical_success: Option<String>,
        critical_failure: Option<String>,
    ) -> Self {
        Self {
            success: success.into(),
            failure: failure.into(),
            critical_success,
            critical_failure,
        }
    }

    /// Create basic outcomes with only success/failure
    pub fn basic(success: impl Into<String>, failure: impl Into<String>) -> Self {
        Self {
            success: success.into(),
            failure: failure.into(),
            critical_success: None,
            critical_failure: None,
        }
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
        );

        assert_eq!(outcomes.success(), "You succeed!");
        assert_eq!(outcomes.failure(), "You fail!");
        assert_eq!(outcomes.critical_success(), Some("Critical success!"));
        assert_eq!(outcomes.critical_failure(), Some("Critical failure!"));
    }

    #[test]
    fn test_basic() {
        let outcomes = AdHocOutcomes::basic("You pick the lock", "The lock resists your efforts");

        assert_eq!(outcomes.success(), "You pick the lock");
        assert_eq!(outcomes.failure(), "The lock resists your efforts");
        assert_eq!(outcomes.critical_success(), None);
        assert_eq!(outcomes.critical_failure(), None);
    }

    #[test]
    fn test_builder_pattern() {
        let outcomes = AdHocOutcomes::basic("Success", "Failure")
            .with_critical_success("Epic success!")
            .with_critical_failure("Catastrophic failure!");

        assert_eq!(outcomes.success(), "Success");
        assert_eq!(outcomes.failure(), "Failure");
        assert_eq!(outcomes.critical_success(), Some("Epic success!"));
        assert_eq!(outcomes.critical_failure(), Some("Catastrophic failure!"));
    }

    #[test]
    fn test_with_criticals() {
        let outcomes = AdHocOutcomes::basic("Win", "Lose").with_criticals("Big win", "Big lose");

        assert_eq!(outcomes.critical_success(), Some("Big win"));
        assert_eq!(outcomes.critical_failure(), Some("Big lose"));
    }

    #[test]
    fn test_outcomes_equality() {
        let outcomes =
            AdHocOutcomes::new("Success", "Failure", Some("Crit success".to_string()), None);

        let other = outcomes.clone();
        assert_eq!(outcomes, other);
    }
}
