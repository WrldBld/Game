//! Player port definitions and shared cross-layer types.

pub mod inbound;
pub mod session_types;

// Re-export session types at crate root for convenience
pub use session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DiceInput,
    DirectorialContext, NpcMotivationData, ParticipantRole,
};

pub mod outbound;
