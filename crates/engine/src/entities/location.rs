//! Location entity operations.

use std::sync::Arc;
use wrldbldr_domain::{
    self as domain, LocationConnection, LocationId, RegionConnection, RegionId, WorldId,
};

use crate::infrastructure::ports::{LocationRepo, RepoError};

/// Location entity operations.
///
/// Encapsulates all location and region operations.
pub struct Location {
    repo: Arc<dyn LocationRepo>,
}

/// An exit from a region to another location.
///
/// Used for navigation UI - enriched version of LocationConnection.
#[derive(Debug, Clone)]
pub struct RegionExit {
    pub location_id: LocationId,
    pub location_name: String,
    pub arrival_region_id: RegionId,
    pub description: Option<String>,
}

impl Location {
    pub fn new(repo: Arc<dyn LocationRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // Location CRUD
    // =========================================================================

    /// Get a location by ID.
    pub async fn get(&self, id: LocationId) -> Result<Option<domain::Location>, RepoError> {
        self.repo.get_location(id).await
    }

    /// Get a location by ID.
    ///
    /// **Deprecated**: Use `get()` instead for consistency with other entity modules.
    #[deprecated(since = "0.1.0", note = "Use get() instead for consistency")]
    pub async fn get_location(
        &self,
        id: LocationId,
    ) -> Result<Option<domain::Location>, RepoError> {
        self.repo.get_location(id).await
    }

    pub async fn save_location(&self, location: &domain::Location) -> Result<(), RepoError> {
        self.repo.save_location(location).await
    }

    /// List locations in a world.
    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::Location>, RepoError> {
        self.repo.list_locations_in_world(world_id).await
    }

    /// List locations in a world.
    ///
    /// **Deprecated**: Use `list_in_world()` instead for consistency with other entity modules.
    #[deprecated(since = "0.1.0", note = "Use list_in_world() instead for consistency")]
    pub async fn list_locations_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::Location>, RepoError> {
        self.repo.list_locations_in_world(world_id).await
    }

    /// Delete a location by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete(&self, id: LocationId) -> Result<(), RepoError> {
        self.repo.delete_location(id).await
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

    pub async fn list_regions_in_location(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<domain::Region>, RepoError> {
        self.repo.list_regions_in_location(location_id).await
    }

    /// Delete a region by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_region(&self, id: RegionId) -> Result<(), RepoError> {
        self.repo.delete_region(id).await
    }

    // =========================================================================
    // Connections
    // =========================================================================

    pub async fn get_connections(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<RegionConnection>, RepoError> {
        self.repo.get_connections(region_id).await
    }

    pub async fn save_connection(&self, connection: &RegionConnection) -> Result<(), RepoError> {
        self.repo.save_connection(connection).await
    }

    pub async fn delete_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), RepoError> {
        self.repo.delete_connection(from_region, to_region).await
    }

    pub async fn get_location_exits(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<LocationConnection>, RepoError> {
        self.repo.get_location_exits(location_id).await
    }

    pub async fn save_location_connection(
        &self,
        connection: &LocationConnection,
    ) -> Result<(), RepoError> {
        self.repo.save_location_connection(connection).await
    }

    pub async fn delete_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
    ) -> Result<(), RepoError> {
        self.repo
            .delete_location_connection(from_location, to_location)
            .await
    }

    pub async fn get_region_exits(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<domain::RegionExit>, RepoError> {
        self.repo.get_region_exits(region_id).await
    }

    pub async fn save_region_exit(&self, exit: &domain::RegionExit) -> Result<(), RepoError> {
        self.repo.save_region_exit(exit).await
    }

    pub async fn delete_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
    ) -> Result<(), RepoError> {
        self.repo.delete_region_exit(region_id, location_id).await
    }

    /// Get exits from a region to other locations.
    ///
    /// This finds the location for the given region, then finds connections to
    /// other locations, and enriches them with location names and default arrival regions.
    pub async fn get_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>, RepoError> {
        let region_exits = self.repo.get_region_exits(region_id).await?;
        if !region_exits.is_empty() {
            let mut exits = Vec::new();
            for exit in region_exits {
                if let Some(target_location) = self.repo.get_location(exit.to_location).await? {
                    exits.push(RegionExit {
                        location_id: exit.to_location,
                        location_name: target_location.name,
                        arrival_region_id: exit.arrival_region_id,
                        description: exit.description,
                    });
                } else {
                    tracing::warn!(
                        from_region = %region_id,
                        to_location = %exit.to_location,
                        "Skipping exit: target location not found"
                    );
                }
            }

            return Ok(exits);
        }

        // Get the region to find its location
        let region = match self.repo.get_region(region_id).await? {
            Some(r) => r,
            None => return Ok(vec![]),
        };

        // Get exits from this location
        let location_exits = self.repo.get_location_exits(region.location_id).await?;

        let mut exits = Vec::new();
        for exit in location_exits {
            // Get the target location details
            if let Some(target_location) = self.repo.get_location(exit.to_location).await? {
                // Determine arrival region
                let arrival_region_id = if let Some(default_region) =
                    target_location.default_region_id
                {
                    default_region
                } else {
                    // Try to find a spawn point in the target location
                    let regions = self.repo.list_regions_in_location(exit.to_location).await?;
                    match regions.into_iter().find(|r| r.is_spawn_point) {
                        Some(r) => r.id,
                        None => {
                            // Log warning for invalid exit configuration
                            tracing::warn!(
                                from_region = %region_id,
                                to_location = %exit.to_location,
                                target_location_name = %target_location.name,
                                "Skipping exit: target location has no default region and no spawn point"
                            );
                            continue;
                        }
                    }
                };

                exits.push(RegionExit {
                    location_id: exit.to_location,
                    location_name: target_location.name,
                    arrival_region_id,
                    description: exit.description,
                });
            }
        }

        Ok(exits)
    }

    // =========================================================================
    // Derived Operations
    // =========================================================================

    /// Check if a region connection exists and is not locked.
    pub async fn can_move_to(&self, from: RegionId, to: RegionId) -> Result<bool, RepoError> {
        let connections = self.get_connections(from).await?;
        Ok(connections
            .iter()
            .any(|c| c.to_region == to && !c.is_locked))
    }
}
