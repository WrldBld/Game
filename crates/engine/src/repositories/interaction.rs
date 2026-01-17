//! Interaction entity operations.

use std::sync::Arc;

use wrldbldr_domain::{self as domain, InteractionId, SceneId};

use crate::infrastructure::ports::{InteractionRepo, RepoError};

/// Interaction entity operations.
pub struct InteractionRepository {
    repo: Arc<dyn InteractionRepo>,
}

impl InteractionRepository {
    pub fn new(repo: Arc<dyn InteractionRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(
        &self,
        id: InteractionId,
    ) -> Result<Option<domain::InteractionTemplate>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<domain::InteractionTemplate>, RepoError> {
        self.repo.list_for_scene(scene_id).await
    }

    pub async fn save(&self, interaction: &domain::InteractionTemplate) -> Result<(), RepoError> {
        self.repo.save(interaction).await
    }

    pub async fn delete(&self, id: InteractionId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }
}
