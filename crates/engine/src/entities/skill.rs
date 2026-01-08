//! Skill entity operations.

use std::sync::Arc;

use wrldbldr_domain::{self as domain, SkillId, WorldId};

use crate::infrastructure::ports::{RepoError, SkillRepo};

/// Skill entity operations.
pub struct Skill {
    repo: Arc<dyn SkillRepo>,
}

impl Skill {
    pub fn new(repo: Arc<dyn SkillRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: SkillId) -> Result<Option<domain::Skill>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Skill>, RepoError> {
        self.repo.list_in_world(world_id).await
    }

    pub async fn save(&self, skill: &domain::Skill) -> Result<(), RepoError> {
        self.repo.save(skill).await
    }

    pub async fn delete(&self, id: SkillId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }
}
