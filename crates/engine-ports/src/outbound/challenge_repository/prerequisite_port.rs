//! Prerequisite relationship management for Challenge entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::ChallengePrerequisite;
use wrldbldr_domain::ChallengeId;

/// Prerequisite relationship operations for Challenge entities.
///
/// This trait manages the REQUIRES_COMPLETION_OF edges between Challenge nodes,
/// creating prerequisite chains for challenge progression.
///
/// # Used By
/// - `ChallengeServiceImpl` - For managing challenge prerequisites
/// - `ChallengeResolutionService` - For checking if prerequisites are met
/// - `TriggerEvaluationService` - For evaluating challenge availability
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengePrerequisitePort: Send + Sync {
    /// Add a prerequisite challenge (creates REQUIRES_COMPLETION_OF edge)
    async fn add_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite: ChallengePrerequisite,
    ) -> Result<()>;

    /// Get all prerequisites for a challenge
    async fn get_prerequisites(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengePrerequisite>>;

    /// Remove a prerequisite from a challenge
    async fn remove_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite_id: ChallengeId,
    ) -> Result<()>;

    /// Get challenges that require this challenge as a prerequisite
    async fn get_dependent_challenges(&self, challenge_id: ChallengeId)
        -> Result<Vec<ChallengeId>>;
}
