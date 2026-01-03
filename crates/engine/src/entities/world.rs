//! World entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, WorldId};

use crate::infrastructure::ports::{RepoError, WorldRepo};

/// World entity operations.
pub struct World {
    repo: Arc<dyn WorldRepo>,
}

impl World {
    pub fn new(repo: Arc<dyn WorldRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: WorldId) -> Result<Option<domain::World>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, world: &domain::World) -> Result<(), RepoError> {
        self.repo.save(world).await
    }

    pub async fn list_all(&self) -> Result<Vec<domain::World>, RepoError> {
        self.repo.list_all().await
    }

    pub async fn delete(&self, id: WorldId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }
}
