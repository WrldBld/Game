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
    pub success: String,
    /// Outcome text for a failed roll
    pub failure: String,
    /// Optional outcome text for a critical success
    pub critical_success: Option<String>,
    /// Optional outcome text for a critical failure
    pub critical_failure: Option<String>,
}

impl AdHocOutcomes {
    /// Create ad-hoc outcomes with all fields
    pub fn new(
        success: String,
        failure: String,
        critical_success: Option<String>,
        critical_failure: Option<String>,
    ) -> Self {
        Self {
            success,
            failure,
            critical_success,
            critical_failure,
        }
    }

    /// Create basic outcomes with only success/failure
    pub fn basic(success: String, failure: String) -> Self {
        Self {
            success,
            failure,
            critical_success: None,
            critical_failure: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let outcomes = AdHocOutcomes::new(
            "You succeed!".to_string(),
            "You fail!".to_string(),
            Some("Critical success!".to_string()),
            Some("Critical failure!".to_string()),
        );

        assert_eq!(outcomes.success, "You succeed!");
        assert_eq!(outcomes.failure, "You fail!");
        assert_eq!(
            outcomes.critical_success,
            Some("Critical success!".to_string())
        );
        assert_eq!(
            outcomes.critical_failure,
            Some("Critical failure!".to_string())
        );
    }

    #[test]
    fn test_basic() {
        let outcomes = AdHocOutcomes::basic(
            "You pick the lock".to_string(),
            "The lock resists your efforts".to_string(),
        );

        assert_eq!(outcomes.success, "You pick the lock");
        assert_eq!(outcomes.failure, "The lock resists your efforts");
        assert_eq!(outcomes.critical_success, None);
        assert_eq!(outcomes.critical_failure, None);
    }

    #[test]
    fn test_outcomes_equality() {
        let outcomes = AdHocOutcomes::new(
            "Success".to_string(),
            "Failure".to_string(),
            Some("Crit success".to_string()),
            None,
        );

        let other = outcomes.clone();
        assert_eq!(outcomes, other);
    }
}
