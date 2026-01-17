//! RegionState entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, RegionId, RegionStateId};

use crate::infrastructure::ports::{RegionStateRepo, RepoError};

/// RegionState entity operations.
pub struct RegionStateRepository {
    repo: Arc<dyn RegionStateRepo>,
}

impl RegionStateRepository {
    pub fn new(repo: Arc<dyn RegionStateRepo>) -> Self {
        Self { repo }
    }

    // CRUD operations

    pub async fn get(&self, id: RegionStateId) -> Result<Option<domain::RegionState>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, state: &domain::RegionState) -> Result<(), RepoError> {
        self.repo.save(state).await
    }

    pub async fn delete(&self, id: RegionStateId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    // Query operations

    pub async fn list_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<domain::RegionState>, RepoError> {
        self.repo.list_for_region(region_id).await
    }

    pub async fn get_default(
        &self,
        region_id: RegionId,
    ) -> Result<Option<domain::RegionState>, RepoError> {
        self.repo.get_default(region_id).await
    }

    // Active state management

    pub async fn set_active(
        &self,
        region_id: RegionId,
        state_id: RegionStateId,
    ) -> Result<(), RepoError> {
        self.repo.set_active(region_id, state_id).await
    }

    pub async fn get_active(
        &self,
        region_id: RegionId,
    ) -> Result<Option<domain::RegionState>, RepoError> {
        self.repo.get_active(region_id).await
    }

    pub async fn clear_active(&self, region_id: RegionId) -> Result<(), RepoError> {
        self.repo.clear_active(region_id).await
    }
}
