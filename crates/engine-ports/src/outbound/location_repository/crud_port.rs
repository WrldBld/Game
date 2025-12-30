//! Core CRUD operations for Location entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{Location, LocationId, WorldId};

/// Core CRUD operations for Location entities.
///
/// This trait covers:
/// - Basic entity operations (create, get, list, update, delete)
///
/// # Used By
/// - `LocationServiceImpl` - For all CRUD operations
/// - Navigation services - For retrieving and managing locations
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait LocationCrudPort: Send + Sync {
    /// Create a new location
    async fn create(&self, location: &Location) -> Result<()>;

    /// Get a location by ID
    async fn get(&self, id: LocationId) -> Result<Option<Location>>;

    /// List all locations in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Location>>;

    /// Update a location
    async fn update(&self, location: &Location) -> Result<()>;

    /// Delete a location
    async fn delete(&self, id: LocationId) -> Result<()>;
}
