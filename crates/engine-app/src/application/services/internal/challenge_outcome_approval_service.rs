//! Challenge outcome approval service port - Interface for DM approval of challenge resolutions
//!
//! This port abstracts the DM approval workflow for challenge outcomes from infrastructure.
//! After a player rolls a challenge, the outcome is queued for DM approval before being
//! broadcast to players.
//!
//! # Architecture Note
//!
//! This port handles:
//! - Queueing challenge resolutions for DM approval
//! - Processing DM decisions (accept, edit, request suggestions)
//! - Managing pending resolutions
//!
//! Broadcasting is handled by the use case layer via event channels.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::value_objects::ChallengeOutcomeData;
use wrldbldr_domain::WorldId;

// Re-export OutcomeDecision from engine-ports use_case_types
pub use wrldbldr_engine_ports::outbound::OutcomeDecision;

/// Port for challenge outcome approval service operations
///
/// This trait defines the application use cases for DM approval of challenge outcomes.
/// It handles queueing resolutions, processing DM decisions, and managing pending items.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengeOutcomeApprovalServicePort: Send + Sync {
    /// Queue a challenge resolution for DM approval
    ///
    /// Returns the resolution_id for tracking.
    async fn queue_for_approval(
        &self,
        world_id: WorldId,
        resolution: ChallengeOutcomeData,
    ) -> Result<String>;

    /// Process DM's decision on a challenge outcome
    ///
    /// Handles accept, edit, or suggest decisions from the DM.
    async fn process_decision(
        &self,
        world_id: WorldId,
        resolution_id: &str,
        decision: OutcomeDecision,
    ) -> Result<()>;

    /// Update suggestions for a pending resolution
    ///
    /// Used when LLM suggestions are ready to be attached to a pending resolution.
    async fn update_suggestions(&self, resolution_id: &str, suggestions: Vec<String>)
        -> Result<()>;

    /// Get all pending resolutions for a world
    ///
    /// Returns all challenge outcomes awaiting DM approval.
    async fn get_pending_for_world(&self, world_id: WorldId) -> Vec<ChallengeOutcomeData>;

    /// Request LLM to generate outcome branches for DM selection
    ///
    /// Similar to suggestion generation but returns structured branches
    /// instead of simple text suggestions.
    async fn request_branches(
        &self,
        world_id: WorldId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<()>;

    /// Select an outcome branch and resolve the challenge
    ///
    /// The DM picks a branch by ID, optionally modifying the description.
    async fn select_branch(
        &self,
        world_id: WorldId,
        resolution_id: &str,
        branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<()>;
}
