//! Time suggestion decision types for DM approval workflows.
//!
//! These are domain types used in use cases. The protocol layer has its own
//! versions with Unknown variants for forward compatibility.

use serde::{Deserialize, Serialize};

/// DM's decision on a time suggestion.
///
/// This is a domain version without unknown variant - protocol
/// layer handles unknown decisions at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum TimeSuggestionDecision {
    /// Accept's suggested time cost
    Approve,
    /// Modify's time cost to a different value
    Modify {
        /// The new time cost in seconds
        seconds: u32,
    },
    /// Skip this time suggestion (no time advancement)
    Skip,
}

impl TimeSuggestionDecision {
    /// Create an Approve decision.
    pub fn approve_suggestion() -> Self {
        Self::Approve
    }

    /// Create a Modify decision with a specified seconds.
    pub fn modify_seconds(seconds: u32) -> Self {
        Self::Modify { seconds }
    }

    /// Create a Skip decision.
    pub fn skip() -> Self {
        Self::Skip
    }

    /// Returns seconds to advance, if any.
    ///
    /// - `Approve` returns `None` (use suggested seconds)
    /// - `Modify { seconds }` returns `Some(seconds)`
    /// - `Skip` returns `Some(0)`
    pub fn resolved_seconds(&self, suggested: u32) -> u32 {
        match self {
            Self::Approve => suggested,
            Self::Modify { seconds } => *seconds,
            Self::Skip => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_suggestion_resolves_to_suggested_seconds() {
        let decision = TimeSuggestionDecision::approve_suggestion();
        assert_eq!(decision.resolved_seconds(30), 30);
    }

    #[test]
    fn modify_suggestion_resolves_to_specified_seconds() {
        let decision = TimeSuggestionDecision::modify_seconds(15);
        assert_eq!(decision.resolved_seconds(30), 15);
    }

    #[test]
    fn skip_suggestion_resolves_to_zero() {
        let decision = TimeSuggestionDecision::skip();
        assert_eq!(decision.resolved_seconds(30), 0);
    }

    #[test]
    fn decision_equality_approve() {
        let decision = TimeSuggestionDecision::Approve;
        let other = TimeSuggestionDecision::Approve;
        assert_eq!(decision, other);
    }

    #[test]
    fn decision_equality_modify() {
        let decision = TimeSuggestionDecision::Modify { seconds: 45 };
        let other = decision.clone();
        assert_eq!(decision, other);
    }

    #[test]
    fn decision_equality_skip() {
        let decision = TimeSuggestionDecision::Skip;
        let other = TimeSuggestionDecision::Skip;
        assert_eq!(decision, other);
    }
}
