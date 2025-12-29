//! Session-related DTOs for player-app
//!
//! These types are re-exported from player-ports for use by the application layer.
//! The ports layer owns the canonical definitions to avoid circular dependencies.

// Re-export session types from player-ports (explicit list)
pub use wrldbldr_player_ports::session_types::{
    ParticipantRole, DiceInput, ApprovalDecision, DirectorialContext,
    NpcMotivationData, ApprovedNpcInfo, AdHocOutcomes, ChallengeOutcomeDecision,
};
