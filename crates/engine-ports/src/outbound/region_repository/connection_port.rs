//! Region connection port for managing CONNECTED_TO_REGION edges.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::ids::RegionId;
use wrldbldr_domain::RegionConnection;

/// Port for managing region connections (CONNECTED_TO_REGION edges).
///
/// This trait handles navigation connections between regions within the same
/// location, stored as `CONNECTED_TO_REGION` edges in Neo4j. Connections can
/// be unidirectional or bidirectional and may be locked/unlocked.
///
/// # Used By
/// - `RegionService` - For managing region connectivity
/// - `NavigationService` - For intra-location pathfinding and movement
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait RegionConnectionPort: Send + Sync {
    /// Create a connection between two regions.
    ///
    /// Creates a CONNECTED_TO_REGION edge from the source to the destination region.
    async fn create_connection(&self, connection: &RegionConnection) -> Result<()>;

    /// Get all connections from a region.
    ///
    /// Returns all outgoing CONNECTED_TO_REGION edges from the given region,
    /// representing valid navigation targets within the location.
    async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>>;

    /// Delete a connection between two regions.
    ///
    /// Removes the CONNECTED_TO_REGION edge from the source to the destination.
    /// Note: This only removes the edge in the specified direction.
    async fn delete_connection(&self, from: RegionId, to: RegionId) -> Result<()>;

    /// Unlock a locked connection between two regions.
    ///
    /// Sets is_locked to false on the CONNECTED_TO_REGION edge, allowing navigation
    /// through this connection. Returns an error if the connection doesn't exist.
    async fn unlock_connection(&self, from: RegionId, to: RegionId) -> Result<()>;
}
