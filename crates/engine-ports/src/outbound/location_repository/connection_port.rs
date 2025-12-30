//! Location connection port for managing CONNECTED_TO edges in the navigation graph.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::LocationConnection;
use wrldbldr_domain::LocationId;

/// Port for managing location connections (CONNECTED_TO edges) in the navigation graph.
///
/// This trait handles navigation connections between locations, stored as
/// `CONNECTED_TO` edges in Neo4j. Connections can be unidirectional or
/// bidirectional and may be locked/unlocked.
///
/// # Used By
/// - `NavigationService` - For pathfinding and movement
/// - `LocationService` - For managing location connectivity
/// - `WorldBuilderService` - For setting up initial world connections
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait LocationConnectionPort: Send + Sync {
    /// Create a new connection between two locations.
    ///
    /// Creates a CONNECTED_TO edge from the source to the destination location.
    /// If the connection is bidirectional, the reverse edge should also be created.
    async fn create_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Get all connections from a specific location.
    ///
    /// Returns all outgoing CONNECTED_TO edges from the given location,
    /// representing valid navigation targets from that location.
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>>;

    /// Update an existing connection's properties.
    ///
    /// Updates properties like description, travel_time, is_locked, etc.
    /// The connection is identified by its from_location and to_location.
    async fn update_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Delete a connection between two locations.
    ///
    /// Removes the CONNECTED_TO edge from the source to the destination.
    /// Note: This only removes the edge in the specified direction.
    /// For bidirectional connections, call twice with reversed parameters.
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    /// Unlock a locked connection between two locations.
    ///
    /// Sets is_locked to false on the CONNECTED_TO edge, allowing navigation
    /// through this connection. Returns an error if the connection doesn't exist.
    async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()>;
}
