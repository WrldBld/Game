//! Game system content operations.

use std::sync::Arc;

use wrldbldr_domain::{self as domain, SkillId, WorldId};

use crate::infrastructure::ports::{ContentRepo, RepoError};

/// Game system content operations.
///
/// Currently provides skill CRUD, but intended to expand with
/// other system content (feats, spells, etc.) as needed.
pub struct ContentRepository {
    repo: Arc<dyn ContentRepo>,
}

impl ContentRepository {
    pub fn new(repo: Arc<dyn ContentRepo>) -> Self {
        Self { repo }
    }

    pub async fn get_skill(&self, id: SkillId) -> Result<Option<domain::Skill>, RepoError> {
        self.repo.get_skill(id).await
    }

    pub async fn list_skills_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::Skill>, RepoError> {
        self.repo.list_skills_in_world(world_id).await
    }

    pub async fn save_skill(&self, skill: &domain::Skill) -> Result<(), RepoError> {
        self.repo.save_skill(skill).await
    }

    pub async fn delete_skill(&self, id: SkillId) -> Result<(), RepoError> {
        self.repo.delete_skill(id).await
    }
}
