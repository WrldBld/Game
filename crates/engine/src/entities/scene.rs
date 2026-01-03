//! Scene entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, CharacterId, RegionId, SceneId, WorldId};

use crate::infrastructure::ports::{RepoError, SceneRepo};

/// Scene entity operations.
pub struct Scene {
    repo: Arc<dyn SceneRepo>,
}

impl Scene {
    pub fn new(repo: Arc<dyn SceneRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: SceneId) -> Result<Option<domain::Scene>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, scene: &domain::Scene) -> Result<(), RepoError> {
        self.repo.save(scene).await
    }

    pub async fn get_current(&self, world_id: WorldId) -> Result<Option<domain::Scene>, RepoError> {
        self.repo.get_current(world_id).await
    }

    pub async fn set_current(&self, world_id: WorldId, scene_id: SceneId) -> Result<(), RepoError> {
        self.repo.set_current(world_id, scene_id).await
    }

    pub async fn list_for_region(&self, region_id: RegionId) -> Result<Vec<domain::Scene>, RepoError> {
        self.repo.list_for_region(region_id).await
    }

    pub async fn get_featured_characters(&self, scene_id: SceneId) -> Result<Vec<CharacterId>, RepoError> {
        self.repo.get_featured_characters(scene_id).await
    }

    pub async fn set_featured_characters(&self, scene_id: SceneId, characters: &[CharacterId]) -> Result<(), RepoError> {
        self.repo.set_featured_characters(scene_id, characters).await
    }
}
