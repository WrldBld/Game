//! Time suggestion decision types for DM approval workflows.
//!
//! These are domain types used in use cases. The protocol layer has its own
//! versions with Unknown variants for forward compatibility.

use serde::{Deserialize, Serialize};

/// DM's decision on a time suggestion.
///
/// This is the domain version without the Unknown variant - the protocol
/// layer handles unknown decisions at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum TimeSuggestionDecision {
    /// Accept the suggested time cost
    Approve,
    /// Modify the time cost to a different value
    Modify {
        /// The new time cost in minutes
        minutes: u32,
    },
    /// Skip this time suggestion (no time advancement)
    Skip,
}

impl TimeSuggestionDecision {
    /// Create an Approve decision.
    pub fn approve() -> Self {
        Self::Approve
    }

    /// Create a Modify decision with the specified minutes.
    pub fn modify(minutes: u32) -> Self {
        Self::Modify { minutes }
    }

    /// Create a Skip decision.
    pub fn skip() -> Self {
        Self::Skip
    }

    /// Returns the minutes to advance, if any.
    ///
    /// - `Approve` returns `None` (use suggested minutes)
    /// - `Modify { minutes }` returns `Some(minutes)`
    /// - `Skip` returns `Some(0)`
    pub fn resolved_minutes(&self, suggested: u32) -> u32 {
        match self {
            Self::Approve => suggested,
            Self::Modify { minutes } => *minutes,
            Self::Skip => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_resolves_to_suggested_minutes() {
        let decision = TimeSuggestionDecision::approve();
        assert_eq!(decision.resolved_minutes(30), 30);
    }

    #[test]
    fn modify_resolves_to_specified_minutes() {
        let decision = TimeSuggestionDecision::modify(15);
        assert_eq!(decision.resolved_minutes(30), 15);
    }

    #[test]
    fn skip_resolves_to_zero() {
        let decision = TimeSuggestionDecision::skip();
        assert_eq!(decision.resolved_minutes(30), 0);
    }

    #[test]
    fn decision_equality_approve() {
        let decision = TimeSuggestionDecision::Approve;
        let other = TimeSuggestionDecision::Approve;
        assert_eq!(decision, other);
    }

    #[test]
    fn decision_equality_modify() {
        let decision = TimeSuggestionDecision::Modify { minutes: 45 };
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
