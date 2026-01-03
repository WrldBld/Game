//! Location entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, LocationConnection, LocationId, RegionConnection, RegionId, WorldId};

use crate::infrastructure::ports::{LocationRepo, RepoError};

/// Location entity operations.
///
/// Encapsulates all location and region operations.
pub struct Location {
    repo: Arc<dyn LocationRepo>,
}

impl Location {
    pub fn new(repo: Arc<dyn LocationRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // Location CRUD
    // =========================================================================

    pub async fn get_location(&self, id: LocationId) -> Result<Option<domain::Location>, RepoError> {
        self.repo.get_location(id).await
    }

    pub async fn save_location(&self, location: &domain::Location) -> Result<(), RepoError> {
        self.repo.save_location(location).await
    }

    pub async fn list_locations_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Location>, RepoError> {
        self.repo.list_locations_in_world(world_id).await
    }

    // =========================================================================
    // Region CRUD
    // =========================================================================

    pub async fn get_region(&self, id: RegionId) -> Result<Option<domain::Region>, RepoError> {
        self.repo.get_region(id).await
    }

    pub async fn save_region(&self, region: &domain::Region) -> Result<(), RepoError> {
        self.repo.save_region(region).await
    }

    pub async fn list_regions_in_location(&self, location_id: LocationId) -> Result<Vec<domain::Region>, RepoError> {
        self.repo.list_regions_in_location(location_id).await
    }

    // =========================================================================
    // Connections
    // =========================================================================

    pub async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>, RepoError> {
        self.repo.get_connections(region_id).await
    }

    pub async fn save_connection(&self, connection: &RegionConnection) -> Result<(), RepoError> {
        self.repo.save_connection(connection).await
    }

    pub async fn get_location_exits(&self, location_id: LocationId) -> Result<Vec<LocationConnection>, RepoError> {
        self.repo.get_location_exits(location_id).await
    }

    // =========================================================================
    // Derived Operations
    // =========================================================================

    /// Check if a region connection exists and is not locked.
    pub async fn can_move_to(&self, from: RegionId, to: RegionId) -> Result<bool, RepoError> {
        let connections = self.get_connections(from).await?;
        Ok(connections.iter().any(|c| c.to_region == to && !c.is_locked))
    }
}
