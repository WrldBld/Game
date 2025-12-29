//! Location service port - Interface for location operations
//!
//! This port abstracts location business logic from infrastructure adapters.
//! It exposes query methods for retrieving locations by various criteria.
//!
//! # Design Notes
//!
//! This port is designed for use by infrastructure adapters that need to query
//! location information. It focuses on read operations used by navigation systems,
//! prompt builders, and scene setup.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::Location;
use wrldbldr_domain::{LocationId, WorldId};

/// Port for location service operations used by infrastructure adapters.
///
/// This trait provides read-only access to location data for use in
/// navigation, prompt building, and scene context.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait LocationServicePort: Send + Sync {
    /// Get a location by ID.
    ///
    /// Returns `Ok(None)` if the location is not found.
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>>;

    /// List all locations in a world.
    ///
    /// Returns locations sorted by name.
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Location>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of LocationServicePort for testing.
    pub LocationServicePort {}

    #[async_trait]
    impl LocationServicePort for LocationServicePort {
        async fn get_location(&self, id: LocationId) -> Result<Option<Location>>;
        async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Location>>;
    }
}
