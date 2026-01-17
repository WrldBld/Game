//! Challenge entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, ChallengeId, SceneId, WorldId};

use crate::infrastructure::ports::{ChallengeRepo, RepoError};

/// Challenge entity operations.
pub struct ChallengeRepository {
    repo: Arc<dyn ChallengeRepo>,
}

impl ChallengeRepository {
    pub fn new(repo: Arc<dyn ChallengeRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: ChallengeId) -> Result<Option<domain::Challenge>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, challenge: &domain::Challenge) -> Result<(), RepoError> {
        self.repo.save(challenge).await
    }

    pub async fn delete(&self, id: ChallengeId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    pub async fn list_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::Challenge>, RepoError> {
        self.repo.list_for_world(world_id).await
    }

    pub async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<domain::Challenge>, RepoError> {
        self.repo.list_for_scene(scene_id).await
    }

    pub async fn list_pending(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::Challenge>, RepoError> {
        self.repo.list_pending_for_world(world_id).await
    }

    pub async fn mark_resolved(&self, id: ChallengeId) -> Result<(), RepoError> {
        self.repo.mark_resolved(id).await
    }

    /// Enable or disable a challenge.
    ///
    /// Enabled challenges can be triggered, disabled ones cannot.
    /// This is used by EnableChallenge/DisableChallenge triggers.
    pub async fn set_enabled(&self, id: ChallengeId, enabled: bool) -> Result<(), RepoError> {
        self.repo.set_enabled(id, enabled).await
    }

    /// Get all resolved challenge IDs in a world.
    ///
    /// Used for building trigger context.
    pub async fn get_resolved(&self, world_id: WorldId) -> Result<Vec<ChallengeId>, RepoError> {
        self.repo.get_resolved_challenges(world_id).await
    }
}
