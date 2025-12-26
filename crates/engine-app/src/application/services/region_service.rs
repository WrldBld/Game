//! Region Service - Application service for region management
//!
//! This service provides use case implementations for creating, updating,
//! and managing regions within locations.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::entities::{Character, Region};
use wrldbldr_domain::value_objects::RegionRelationshipType;
use wrldbldr_domain::{LocationId, RegionId, WorldId};
use wrldbldr_engine_ports::outbound::{LocationRepositoryPort, RegionRepositoryPort};

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
}

/// Default implementation of RegionService using port abstractions
pub struct RegionServiceImpl {
    region_repository: Arc<dyn RegionRepositoryPort>,
    location_repository: Arc<dyn LocationRepositoryPort>,
}

impl RegionServiceImpl {
    /// Create a new RegionServiceImpl with the given repositories
    pub fn new(
        region_repository: Arc<dyn RegionRepositoryPort>,
        location_repository: Arc<dyn LocationRepositoryPort>,
    ) -> Self {
        Self {
            region_repository,
            location_repository,
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
        self.region_repository
            .get(id)
            .await
            .context("Failed to get region from repository")
    }

    #[instrument(skip(self))]
    async fn list_regions(&self, location_id: LocationId) -> Result<Vec<Region>> {
        debug!(location_id = %location_id, "Listing regions in location");
        self.region_repository
            .list_by_location(location_id)
            .await
            .context("Failed to list regions from repository")
    }

    #[instrument(skip(self))]
    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>> {
        debug!(world_id = %world_id, "Listing spawn point regions in world");
        self.region_repository
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
            .location_repository
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        // Build the region
        let mut region = Region::new(location_id, &name).with_description(&description);

        if is_spawn_point {
            region = region.as_spawn_point();
        }

        // Create the region via the location repository
        self.location_repository
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
            .region_repository
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
        // Note: RegionRepositoryPort currently doesn't have an update method.
        // This will need to be added to the port interface. For now, we use
        // a pattern similar to other services that delegate to the repository.
        // The repository implementation will need to provide this method.
        self.region_repository
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
            .region_repository
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", id))?;

        // Delete the region
        // Note: RegionRepositoryPort currently doesn't have a delete method.
        // This will need to be added to the port interface.
        self.region_repository
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
            .region_repository
            .get(region_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", region_id))?;

        self.region_repository
            .get_npcs_related_to_region(region_id)
            .await
            .context("Failed to get NPCs for region from repository")
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
