//! Region item port for managing CONTAINS_ITEM edges.
//!
//! This module contains stub implementations for future region item placement
//! functionality. See US-REGION-ITEMS for implementation details.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::ids::{ItemId, RegionId};
use wrldbldr_domain::Item;

/// Port for managing items placed in regions (CONTAINS_ITEM edges).
///
/// This trait handles item placement within regions, stored as
/// `CONTAINS_ITEM` edges in Neo4j. Items can be placed in regions
/// up to the region's max_items capacity.
///
/// # Status
/// **Stub implementation** - Not yet implemented. See US-REGION-ITEMS.
///
/// # Future Used By
/// - `ItemService` - For item placement and retrieval
/// - `RegionService` - For region inventory management
/// - `InteractionService` - For picking up/dropping items
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait RegionItemPort: Send + Sync {
    /// Add an item to a region (stub - not yet implemented).
    ///
    /// This will create a `(Region)-[:CONTAINS_ITEM]->(Item)` edge.
    /// Future implementation should enforce region.max_items capacity.
    async fn add_item_to_region(&self, _region_id: RegionId, _item_id: ItemId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Region item placement not yet implemented - see US-REGION-ITEMS"
        ))
    }

    /// Get all items in a region (stub - not yet implemented).
    ///
    /// Returns items linked via `(Region)-[:CONTAINS_ITEM]->(Item)` edge.
    async fn get_region_items(&self, _region_id: RegionId) -> Result<Vec<Item>> {
        Err(anyhow::anyhow!(
            "Region item placement not yet implemented - see US-REGION-ITEMS"
        ))
    }

    /// Remove an item from a region (stub - not yet implemented).
    ///
    /// Deletes the `(Region)-[:CONTAINS_ITEM]->(Item)` edge.
    async fn remove_item_from_region(&self, _region_id: RegionId, _item_id: ItemId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Region item placement not yet implemented - see US-REGION-ITEMS"
        ))
    }
}
