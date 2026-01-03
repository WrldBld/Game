//! Inventory entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, ItemId, RegionId, WorldId};

use crate::infrastructure::ports::{ItemRepo, RepoError};

/// Inventory entity operations.
///
/// Handles items in the game world.
pub struct Inventory {
    repo: Arc<dyn ItemRepo>,
}

impl Inventory {
    pub fn new(repo: Arc<dyn ItemRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: ItemId) -> Result<Option<domain::Item>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, item: &domain::Item) -> Result<(), RepoError> {
        self.repo.save(item).await
    }

    pub async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<domain::Item>, RepoError> {
        self.repo.list_in_region(region_id).await
    }

    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Item>, RepoError> {
        self.repo.list_in_world(world_id).await
    }
}
