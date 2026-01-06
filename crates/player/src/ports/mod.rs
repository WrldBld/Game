//! Player port definitions.
//!
//! NOTE: This module is transitional while we remove most internal ports.

// Re-export session types from the unified crate root for convenience.
pub use crate::session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DiceInput,
    DirectorialContext, NpcMotivationData, ParticipantRole,
};

// Keep `crate::ports::session_types::...` working during the move.
pub use crate::session_types as session_types;

pub mod outbound;
