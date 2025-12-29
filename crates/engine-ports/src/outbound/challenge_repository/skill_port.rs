//! Skill relationship management for Challenge entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{ChallengeId, SkillId};

/// Skill relationship operations for Challenge entities.
///
/// This trait manages the REQUIRES_SKILL edge between Challenge and Skill nodes.
///
/// # Used By
/// - `ChallengeServiceImpl` - For managing skill requirements
/// - `ChallengeResolutionService` - For getting required skill during resolution
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengeSkillPort: Send + Sync {
    /// Set the required skill for a challenge (creates REQUIRES_SKILL edge)
    async fn set_required_skill(&self, challenge_id: ChallengeId, skill_id: SkillId) -> Result<()>;

    /// Get the required skill for a challenge
    async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>>;

    /// Remove the required skill from a challenge
    async fn remove_required_skill(&self, challenge_id: ChallengeId) -> Result<()>;
}
