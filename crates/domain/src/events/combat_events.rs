//! Combat-related domain events
//!
//! These enums communicate what happened during combat or challenge resolution,
//! allowing callers to react appropriately.

/// Outcome of a challenge/skill check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeOutcome {
    /// Challenge passed by the given margin
    Success { margin: i32 },
    /// Challenge failed by the given margin
    Failure { margin: i32 },
    /// Critical success (natural 20 or equivalent)
    CriticalSuccess,
    /// Critical failure (natural 1 or equivalent)
    CriticalFailure,
}
