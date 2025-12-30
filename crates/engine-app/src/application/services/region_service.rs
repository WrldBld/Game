//! Region Service - Application service for region management
//!
//! This service provides use case implementations for creating, updating,
//! and managing regions within locations.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::entities::{Character, Region, RegionConnection, RegionExit};
use wrldbldr_domain::value_objects::RegionRelationshipType;
use wrldbldr_domain::{LocationId, RegionId, WorldId};
use wrldbldr_engine_ports::outbound::{
    LocationCrudPort, LocationMapPort, RegionConnectionPort, RegionCrudPort, RegionExitPort,
    RegionNpcPort, RegionServicePort,
};

// Validation constants
const MAX_REGION_NAME_LENGTH: usize = 255;
const MAX_REGION_DESCRIPTION_LENGTH: usize = 10000;

/// Region service trait defining the application use cases
#[async_trait]
pub trait RegionService: Send + Sync {
    /// Get a region by ID
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>>;

    /// List all regions in a location
    async fn list_regions(&self, location_id: LocationId) -> Result<Vec<Region>>;

    /// List all spawn point regions in a world
    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>>;

    /// Create a new region within a location
    async fn create_region(
        &self,
        location_id: LocationId,
        name: String,
        description: String,
        is_spawn_point: bool,
    ) -> Result<Region>;

    /// Update an existing region
    async fn update_region(
        &self,
        id: RegionId,
        name: Option<String>,
        description: Option<String>,
        is_spawn_point: Option<bool>,
    ) -> Result<Region>;

    /// Delete a region
    async fn delete_region(&self, id: RegionId) -> Result<()>;

    /// Get all NPCs with their relationship types for a region
    async fn get_region_npcs(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>>;

    // -------------------------------------------------------------------------
    // Region Connections
    // -------------------------------------------------------------------------

    /// Create a connection between two regions
    async fn create_connection(&self, connection: RegionConnection) -> Result<()>;

    /// Get all connections from a region
    async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>>;

    /// Delete a connection between two regions
    async fn delete_connection(&self, from: RegionId, to: RegionId) -> Result<()>;

    /// Unlock a locked connection between two regions
    async fn unlock_connection(&self, from: RegionId, to: RegionId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Region Exits
    // -------------------------------------------------------------------------

    /// Create an exit from a region to another location
    async fn create_exit(&self, exit: RegionExit) -> Result<()>;

    /// Get all exits from a region
    async fn get_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>>;

    /// Delete an exit from a region to a location
    async fn delete_exit(&self, from_region: RegionId, to_location: LocationId) -> Result<()>;
}

/// Default implementation of RegionService using port abstractions
pub struct RegionServiceImpl {
    region_crud: Arc<dyn RegionCrudPort>,
    region_connection: Arc<dyn RegionConnectionPort>,
    region_exit: Arc<dyn RegionExitPort>,
    region_npc: Arc<dyn RegionNpcPort>,
    location_crud: Arc<dyn LocationCrudPort>,
    location_map: Arc<dyn LocationMapPort>,
}

impl RegionServiceImpl {
    /// Create a new RegionServiceImpl with the given repositories
    ///
    /// Note: `location_crud` and `location_map` can be clones of the same underlying
    /// Arc if the concrete type implements both traits (e.g., Neo4jLocationRepository).
    /// Similarly, region_crud, region_connection, region_exit, and region_npc can all
    /// be clones of the same Arc if the concrete type implements all traits.
    pub fn new(
        region_crud: Arc<dyn RegionCrudPort>,
        region_connection: Arc<dyn RegionConnectionPort>,
        region_exit: Arc<dyn RegionExitPort>,
        region_npc: Arc<dyn RegionNpcPort>,
        location_crud: Arc<dyn LocationCrudPort>,
        location_map: Arc<dyn LocationMapPort>,
    ) -> Self {
        Self {
            region_crud,
            region_connection,
            region_exit,
            region_npc,
            location_crud,
            location_map,
        }
    }

    /// Validate region name
    fn validate_name(name: &str) -> Result<()> {
        if name.trim().is_empty() {
            anyhow::bail!("Region name cannot be empty");
        }
        if name.len() > MAX_REGION_NAME_LENGTH {
            anyhow::bail!(
                "Region name cannot exceed {} characters",
                MAX_REGION_NAME_LENGTH
            );
        }
        Ok(())
    }

    /// Validate region description
    fn validate_description(description: &str) -> Result<()> {
        if description.len() > MAX_REGION_DESCRIPTION_LENGTH {
            anyhow::bail!(
                "Region description cannot exceed {} characters",
                MAX_REGION_DESCRIPTION_LENGTH
            );
        }
        Ok(())
    }
}

#[async_trait]
impl RegionService for RegionServiceImpl {
    #[instrument(skip(self))]
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>> {
        debug!(region_id = %id, "Fetching region");
        self.region_crud
            .get(id)
            .await
            .context("Failed to get region from repository")
    }

    #[instrument(skip(self))]
    async fn list_regions(&self, location_id: LocationId) -> Result<Vec<Region>> {
        debug!(location_id = %location_id, "Listing regions in location");
        self.region_crud
            .list_by_location(location_id)
            .await
            .context("Failed to list regions from repository")
    }

    #[instrument(skip(self))]
    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>> {
        debug!(world_id = %world_id, "Listing spawn point regions in world");
        self.region_crud
            .list_spawn_points(world_id)
            .await
            .context("Failed to list spawn points from repository")
    }

    #[instrument(skip(self), fields(location_id = %location_id, name = %name))]
    async fn create_region(
        &self,
        location_id: LocationId,
        name: String,
        description: String,
        is_spawn_point: bool,
    ) -> Result<Region> {
        // Validate inputs
        Self::validate_name(&name)?;
        Self::validate_description(&description)?;

        // Verify the location exists
        let _ = self
            .location_crud
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        // Build the region
        let mut region = Region::new(location_id, &name).with_description(&description);

        if is_spawn_point {
            region = region.as_spawn_point();
        }

        // Create the region via the location map port
        self.location_map
            .create_region(location_id, &region)
            .await
            .context("Failed to create region in repository")?;

        info!(
            region_id = %region.id,
            location_id = %location_id,
            "Created region: {}",
            name
        );

        Ok(region)
    }

    #[instrument(skip(self), fields(region_id = %id))]
    async fn update_region(
        &self,
        id: RegionId,
        name: Option<String>,
        description: Option<String>,
        is_spawn_point: Option<bool>,
    ) -> Result<Region> {
        // Validate inputs if provided
        if let Some(ref n) = name {
            Self::validate_name(n)?;
        }
        if let Some(ref d) = description {
            Self::validate_description(d)?;
        }

        // Get existing region
        let mut region = self
            .region_crud
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", id))?;

        // Apply updates
        if let Some(n) = name {
            region.name = n;
        }
        if let Some(d) = description {
            region.description = d;
        }
        if let Some(spawn) = is_spawn_point {
            region.is_spawn_point = spawn;
        }

        // Update in repository
        self.region_crud
            .update(&region)
            .await
            .context("Failed to update region in repository")?;

        info!(region_id = %id, "Updated region: {}", region.name);

        Ok(region)
    }

    #[instrument(skip(self))]
    async fn delete_region(&self, id: RegionId) -> Result<()> {
        // Verify region exists
        let region = self
            .region_crud
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", id))?;

        // Delete the region
        self.region_crud
            .delete(id)
            .await
            .context("Failed to delete region from repository")?;

        info!(region_id = %id, "Deleted region: {}", region.name);

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_region_npcs(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        debug!(region_id = %region_id, "Getting NPCs related to region");

        // Verify region exists
        let _ = self
            .region_crud
            .get(region_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", region_id))?;

        self.region_npc
            .get_npcs_related_to_region(region_id)
            .await
            .context("Failed to get NPCs for region from repository")
    }

    // -------------------------------------------------------------------------
    // Region Connections
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn create_connection(&self, connection: RegionConnection) -> Result<()> {
        debug!(
            from = %connection.from_region,
            to = %connection.to_region,
            "Creating region connection"
        );

        // Verify both regions exist
        self.region_crud
            .get(connection.from_region)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Source region not found: {}", connection.from_region)
            })?;
        self.region_crud
            .get(connection.to_region)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target region not found: {}", connection.to_region))?;

        self.region_connection
            .create_connection(&connection)
            .await
            .context("Failed to create region connection")
    }

    #[instrument(skip(self))]
    async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>> {
        debug!(region_id = %region_id, "Getting region connections");
        self.region_connection
            .get_connections(region_id)
            .await
            .context("Failed to get region connections")
    }

    #[instrument(skip(self))]
    async fn delete_connection(&self, from: RegionId, to: RegionId) -> Result<()> {
        debug!(from = %from, to = %to, "Deleting region connection");
        self.region_connection
            .delete_connection(from, to)
            .await
            .context("Failed to delete region connection")
    }

    #[instrument(skip(self))]
    async fn unlock_connection(&self, from: RegionId, to: RegionId) -> Result<()> {
        debug!(from = %from, to = %to, "Unlocking region connection");
        self.region_connection
            .unlock_connection(from, to)
            .await
            .context("Failed to unlock region connection")
    }

    // -------------------------------------------------------------------------
    // Region Exits
    // -------------------------------------------------------------------------

    #[instrument(skip(self))]
    async fn create_exit(&self, exit: RegionExit) -> Result<()> {
        debug!(
            from = %exit.from_region,
            to = %exit.to_location,
            "Creating region exit"
        );

        // Verify region exists
        self.region_crud
            .get(exit.from_region)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Source region not found: {}", exit.from_region))?;

        // Verify target location exists
        self.location_crud
            .get(exit.to_location)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target location not found: {}", exit.to_location))?;

        self.region_exit
            .create_exit(&exit)
            .await
            .context("Failed to create region exit")
    }

    #[instrument(skip(self))]
    async fn get_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>> {
        debug!(region_id = %region_id, "Getting region exits");
        self.region_exit
            .get_exits(region_id)
            .await
            .context("Failed to get region exits")
    }

    #[instrument(skip(self))]
    async fn delete_exit(&self, from_region: RegionId, to_location: LocationId) -> Result<()> {
        debug!(from = %from_region, to = %to_location, "Deleting region exit");
        self.region_exit
            .delete_exit(from_region, to_location)
            .await
            .context("Failed to delete region exit")
    }
}

// Implementation of the port trait for hexagonal architecture compliance
#[async_trait]
impl RegionServicePort for RegionServiceImpl {
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>> {
        RegionService::get_region(self, id).await
    }

    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>> {
        RegionService::list_regions(self, location_id).await
    }

    async fn get_spawn_regions(&self, world_id: WorldId) -> Result<Vec<Region>> {
        RegionService::list_spawn_points(self, world_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name() {
        // Empty name should fail
        assert!(RegionServiceImpl::validate_name("").is_err());
        assert!(RegionServiceImpl::validate_name("   ").is_err());

        // Valid name should pass
        assert!(RegionServiceImpl::validate_name("Town Square").is_ok());

        // Too long name should fail
        let long_name = "a".repeat(MAX_REGION_NAME_LENGTH + 1);
        assert!(RegionServiceImpl::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_description() {
        // Empty description is valid
        assert!(RegionServiceImpl::validate_description("").is_ok());

        // Normal description should pass
        assert!(RegionServiceImpl::validate_description("A bustling marketplace").is_ok());

        // Too long description should fail
        let long_desc = "a".repeat(MAX_REGION_DESCRIPTION_LENGTH + 1);
        assert!(RegionServiceImpl::validate_description(&long_desc).is_err());
    }
}
