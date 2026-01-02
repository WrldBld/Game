//! Item service port - Interface for item operations
//!
//! This port abstracts item business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::Item;
use wrldbldr_domain::{ItemId, RegionId, WorldId};

/// Port for item service operations
///
/// This trait defines the read operations for item management.
/// Adapters implement this trait by wrapping the ItemService.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ItemServicePort: Send + Sync {
    /// Get an item by ID
    ///
    /// Returns the item if found, or None if not found.
    async fn get_item(&self, id: ItemId) -> Result<Option<Item>>;

    /// List all items in a world
    ///
    /// Returns all items belonging to the specified world.
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Item>>;

    /// List all items in a region
    ///
    /// Returns all items placed in the specified region.
    async fn list_by_region(&self, region_id: RegionId) -> Result<Vec<Item>>;
}
