//! Outbound ports - Interfaces for external services
//!
//! These ports define the contracts that infrastructure adapters must implement,
//! allowing application services to interact with external systems without
//! depending on concrete implementations.

pub mod api_port;
pub mod game_connection_port;
pub mod platform;

pub use api_port::{ApiError, ApiPort};
pub use game_connection_port::{
    ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecisionData, ConnectionState, DiceInputType,
    DirectorialContext, GameConnectionPort, NpcMotivation, ParticipantRole,
};
pub use platform::{
    Platform, storage_keys,
};
