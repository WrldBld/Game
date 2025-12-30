//! Grid map and region operations for Location entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::Region;
use wrldbldr_domain::{GridMapId, LocationId};

/// Grid map and region management for Location entities.
///
/// This trait covers:
/// - Grid map association (HAS_TACTICAL_MAP edge)
/// - Region management (HAS_REGION edges)
///
/// # Used By
/// - `LocationServiceImpl` - For map and region operations
/// - Navigation services - For retrieving location structure
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait LocationMapPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Grid Map Operations (HAS_TACTICAL_MAP edge)
    // -------------------------------------------------------------------------

    /// Associate a grid map with a location.
    ///
    /// Creates a `HAS_TACTICAL_MAP` edge from the location to the grid map.
    async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()>;

    /// Remove the grid map association from a location.
    ///
    /// Deletes the `HAS_TACTICAL_MAP` edge if it exists.
    async fn remove_grid_map(&self, location_id: LocationId) -> Result<()>;

    /// Get the grid map ID associated with a location.
    ///
    /// Returns `None` if no grid map is associated.
    async fn get_grid_map_id(&self, location_id: LocationId) -> Result<Option<GridMapId>>;

    // -------------------------------------------------------------------------
    // Region Operations (HAS_REGION edges)
    // -------------------------------------------------------------------------

    /// Create a region within a location.
    ///
    /// Creates the Region node and a `HAS_REGION` edge from the location.
    async fn create_region(&self, location_id: LocationId, region: &Region) -> Result<()>;

    /// Get all regions within a location.
    ///
    /// Retrieves all regions connected via `HAS_REGION` edges.
    async fn get_regions(&self, location_id: LocationId) -> Result<Vec<Region>>;
}
