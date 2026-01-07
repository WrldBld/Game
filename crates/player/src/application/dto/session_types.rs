//! Session-related DTOs for player-app
//!
//! These types are re-exported from player-ports for use by the application layer.
//! The ports layer owns the canonical definitions to avoid circular dependencies.

// Re-export session types from player-ports (explicit list)
pub use crate::ports::session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DiceInput,
    DirectorialContext, NpcMotivationData, ParticipantRole,
};
