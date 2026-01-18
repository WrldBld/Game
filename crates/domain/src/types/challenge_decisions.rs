//! Challenge outcome decision types for DM approval workflows.
//!
//! These are domain types used in use cases. The protocol layer has its own
//! versions with Unknown variants for forward compatibility.

use serde::{Deserialize, Serialize};

/// DM's decision on a challenge outcome.
///
/// This is the domain version without the Unknown variant - the protocol
/// layer handles unknown decisions at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ChallengeOutcomeDecision {
    /// Accept the outcome as-is
    Accept,
    /// Edit the outcome description
    Edit {
        /// The modified outcome description
        modified_description: String,
    },
    /// Request LLM to suggest alternative outcome descriptions
    Suggest {
        /// Optional guidance for the LLM when generating suggestions
        #[serde(default)]
        guidance: Option<String>,
    },
}

impl ChallengeOutcomeDecision {
    /// Create an Accept decision.
    pub fn accept() -> Self {
        Self::Accept
    }

    /// Create an Edit decision with the specified description.
    pub fn edit(modified_description: impl Into<String>) -> Self {
        Self::Edit {
            modified_description: modified_description.into(),
        }
    }

    /// Create a Suggest decision with optional guidance.
    pub fn suggest(guidance: Option<String>) -> Self {
        Self::Suggest { guidance }
    }

    /// Returns true if this decision immediately resolves the challenge.
    pub fn is_immediate(&self) -> bool {
        matches!(self, Self::Accept | Self::Edit { .. })
    }

    /// Returns true if this decision requests LLM suggestions.
    pub fn requests_suggestions(&self) -> bool {
        matches!(self, Self::Suggest { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_is_immediate() {
        let decision = ChallengeOutcomeDecision::accept();
        assert!(decision.is_immediate());
        assert!(!decision.requests_suggestions());
    }

    #[test]
    fn edit_is_immediate() {
        let decision = ChallengeOutcomeDecision::edit("New outcome");
        assert!(decision.is_immediate());
        assert!(!decision.requests_suggestions());
    }

    #[test]
    fn suggest_requests_suggestions() {
        let decision = ChallengeOutcomeDecision::suggest(Some("Be dramatic".to_string()));
        assert!(!decision.is_immediate());
        assert!(decision.requests_suggestions());
    }

    #[test]
    fn suggest_without_guidance() {
        let decision = ChallengeOutcomeDecision::suggest(None);
        assert!(!decision.is_immediate());
        assert!(decision.requests_suggestions());
    }

    #[test]
    fn decision_equality_accept() {
        let decision = ChallengeOutcomeDecision::Accept;
        let other = ChallengeOutcomeDecision::Accept;
        assert_eq!(decision, other);
    }

    #[test]
    fn decision_equality_edit() {
        let decision = ChallengeOutcomeDecision::Edit {
            modified_description: "Custom outcome".to_string(),
        };
        let other = decision.clone();
        assert_eq!(decision, other);
    }

    #[test]
    fn decision_equality_suggest_with_guidance() {
        let decision = ChallengeOutcomeDecision::Suggest {
            guidance: Some("Make it epic".to_string()),
        };
        let other = decision.clone();
        assert_eq!(decision, other);
    }

    #[test]
    fn decision_equality_suggest_without_guidance() {
        let decision = ChallengeOutcomeDecision::Suggest { guidance: None };
        let other = decision.clone();
        assert_eq!(decision, other);
    }
}
