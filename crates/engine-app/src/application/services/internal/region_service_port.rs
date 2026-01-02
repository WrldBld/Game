//! Region service port - Interface for region operations
//!
//! This port abstracts region business logic from infrastructure adapters.
//! It exposes query methods for retrieving regions by various criteria.
//!
//! # Design Notes
//!
//! This port is designed for use by infrastructure adapters that need to query
//! region information. It focuses on read operations used by navigation systems,
//! prompt builders, and spawn point selection.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::Region;
use wrldbldr_domain::{LocationId, RegionId, WorldId};

/// Port for region service operations used by infrastructure adapters.
///
/// This trait provides read-only access to region data for use in
/// navigation, prompt building, and player spawn point selection.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait RegionServicePort: Send + Sync {
    /// Get a region by ID.
    ///
    /// Returns `Ok(None)` if the region is not found.
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>>;

    /// List all regions within a location.
    ///
    /// Returns regions in display order.
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>>;

    /// Get all spawn point regions in a world.
    ///
    /// Returns regions where `is_spawn_point` is true, used for
    /// initial player character placement.
    async fn get_spawn_regions(&self, world_id: WorldId) -> Result<Vec<Region>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of RegionServicePort for testing.
    pub RegionServicePort {}

    #[async_trait]
    impl RegionServicePort for RegionServicePort {
        async fn get_region(&self, id: RegionId) -> Result<Option<Region>>;
        async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>>;
        async fn get_spawn_regions(&self, world_id: WorldId) -> Result<Vec<Region>>;
    }
}
