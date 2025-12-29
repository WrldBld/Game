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
use serde::{Deserialize, Serialize};

use wrldbldr_domain::value_objects::ChallengeOutcomeData;
use wrldbldr_domain::WorldId;

// Re-export OutcomeDecision from use_case_types for convenience
pub use super::use_case_types::OutcomeDecision;

/// Result of a challenge approval operation
#[derive(Debug, Clone)]
pub enum ChallengeApprovalResult {
    /// Item queued for DM approval
    Queued {
        /// Resolution ID for tracking
        resolution_id: String,
    },
    /// Challenge resolved (approved by DM)
    Resolved {
        /// Challenge ID
        challenge_id: String,
        /// Outcome details
        outcome: ResolvedOutcome,
        /// State changes applied
        state_changes: Vec<StateChangeInfo>,
    },
    /// LLM suggestions ready
    SuggestionsReady {
        /// Resolution ID
        resolution_id: String,
        /// Generated suggestions
        suggestions: Vec<String>,
    },
    /// Outcome branches ready for selection
    BranchesReady {
        /// Resolution ID
        resolution_id: String,
        /// Available branches
        branches: Vec<OutcomeBranchInfo>,
    },
}

/// Resolved outcome details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedOutcome {
    /// Type of outcome (success, failure, critical, etc.)
    pub outcome_type: String,
    /// Description of the outcome
    pub outcome_description: String,
    /// Raw roll value
    pub roll: i32,
    /// Modifier applied
    pub modifier: i32,
    /// Total result
    pub total: i32,
    /// Roll breakdown string
    pub roll_breakdown: Option<String>,
    /// Individual dice results
    pub individual_rolls: Option<Vec<i32>>,
    /// Challenge name
    pub challenge_name: String,
    /// Character name who rolled
    pub character_name: String,
}

/// Outcome branch for DM selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeBranchInfo {
    /// Unique branch ID
    pub branch_id: String,
    /// Branch title
    pub title: String,
    /// Branch description
    pub description: String,
    /// Effects this branch would apply
    pub effects: Vec<String>,
}

/// State change information from trigger execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChangeInfo {
    /// Type of state change
    pub change_type: String,
    /// Description of the change
    pub description: String,
}

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
