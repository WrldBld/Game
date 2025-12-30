//! Core CRUD operations for Region entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::ids::{LocationId, RegionId, WorldId};
use wrldbldr_domain::Region;

/// Core CRUD operations for Region entities.
///
/// This trait covers:
/// - Basic entity operations (get, update, delete)
/// - Listing regions by location
/// - Listing spawn point regions in a world
///
/// # Used By
/// - `RegionServiceImpl` - For all CRUD operations
/// - Navigation services - For retrieving and managing regions
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait RegionCrudPort: Send + Sync {
    /// Get a region by ID
    async fn get(&self, id: RegionId) -> Result<Option<Region>>;

    /// Update a region
    async fn update(&self, region: &Region) -> Result<()>;

    /// Delete a region
    async fn delete(&self, id: RegionId) -> Result<()>;

    /// List all regions in a location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>>;

    /// List all spawn point regions in a world
    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>>;
}
