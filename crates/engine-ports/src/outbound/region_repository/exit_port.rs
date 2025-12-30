//! Region exit port for managing EXITS_TO_LOCATION edges.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::ids::{LocationId, RegionId};
use wrldbldr_domain::RegionExit;

/// Port for managing region exits (EXITS_TO_LOCATION edges).
///
/// This trait handles exits from regions to other locations, stored as
/// `EXITS_TO_LOCATION` edges in Neo4j. These edges represent navigation
/// points where a character can travel to a different location.
///
/// # Used By
/// - `RegionService` - For managing region exits
/// - `NavigationService` - For inter-location travel
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait RegionExitPort: Send + Sync {
    /// Create an exit from a region to another location.
    ///
    /// Creates an EXITS_TO_LOCATION edge from the region to the target location.
    async fn create_exit(&self, exit: &RegionExit) -> Result<()>;

    /// Get all exits from a region.
    ///
    /// Returns all outgoing EXITS_TO_LOCATION edges from the given region,
    /// representing valid travel destinations to other locations.
    async fn get_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>>;

    /// Delete an exit from a region to a location.
    ///
    /// Removes the EXITS_TO_LOCATION edge from the region to the target location.
    async fn delete_exit(&self, from_region: RegionId, to_location: LocationId) -> Result<()>;
}
