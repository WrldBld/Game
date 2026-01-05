//! LocationState entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, LocationId, LocationStateId};

use crate::infrastructure::ports::{LocationStateRepo, RepoError};

/// LocationState entity operations.
pub struct LocationStateEntity {
    repo: Arc<dyn LocationStateRepo>,
}

impl LocationStateEntity {
    pub fn new(repo: Arc<dyn LocationStateRepo>) -> Self {
        Self { repo }
    }

    // CRUD operations

    pub async fn get(&self, id: LocationStateId) -> Result<Option<domain::LocationState>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, state: &domain::LocationState) -> Result<(), RepoError> {
        self.repo.save(state).await
    }

    pub async fn delete(&self, id: LocationStateId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    // Query operations

    pub async fn list_for_location(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<domain::LocationState>, RepoError> {
        self.repo.list_for_location(location_id).await
    }

    pub async fn get_default(
        &self,
        location_id: LocationId,
    ) -> Result<Option<domain::LocationState>, RepoError> {
        self.repo.get_default(location_id).await
    }

    // Active state management

    pub async fn set_active(
        &self,
        location_id: LocationId,
        state_id: LocationStateId,
    ) -> Result<(), RepoError> {
        self.repo.set_active(location_id, state_id).await
    }

    pub async fn get_active(
        &self,
        location_id: LocationId,
    ) -> Result<Option<domain::LocationState>, RepoError> {
        self.repo.get_active(location_id).await
    }

    pub async fn clear_active(&self, location_id: LocationId) -> Result<(), RepoError> {
        self.repo.clear_active(location_id).await
    }
}
