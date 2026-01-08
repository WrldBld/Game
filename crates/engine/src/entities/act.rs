//! Act entity operations.

use std::sync::Arc;

use wrldbldr_domain::{self as domain, ActId, WorldId};

use crate::infrastructure::ports::{ActRepo, RepoError};

/// Act entity operations.
pub struct Act {
    repo: Arc<dyn ActRepo>,
}

impl Act {
    pub fn new(repo: Arc<dyn ActRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: ActId) -> Result<Option<domain::Act>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Act>, RepoError> {
        self.repo.list_in_world(world_id).await
    }

    pub async fn save(&self, act: &domain::Act) -> Result<(), RepoError> {
        self.repo.save(act).await
    }

    pub async fn delete(&self, id: ActId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }
}
