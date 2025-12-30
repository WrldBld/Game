//! Parent-child hierarchy operations for Location entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{Location, LocationId};

/// Manages parent-child hierarchy relationships between locations.
///
/// This trait manages CONTAINS_LOCATION edges in Neo4j, enabling
/// hierarchical location structures (e.g., a building containing rooms,
/// a city containing districts).
///
/// # Neo4j Edge Pattern
/// ```cypher
/// (parent:Location)-[:CONTAINS_LOCATION]->(child:Location)
/// ```
///
/// # Used By
/// - `LocationServiceImpl` - For managing location hierarchies
/// - Navigation services - For traversing location trees
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait LocationHierarchyPort: Send + Sync {
    /// Set the parent of a location, creating a CONTAINS_LOCATION edge.
    ///
    /// If the child already has a parent, the existing relationship
    /// is replaced with the new one.
    async fn set_parent(&self, child_id: LocationId, parent_id: LocationId) -> Result<()>;

    /// Remove the parent relationship from a location.
    ///
    /// Deletes the CONTAINS_LOCATION edge pointing to this location.
    /// No-op if the location has no parent.
    async fn remove_parent(&self, child_id: LocationId) -> Result<()>;

    /// Get the parent location of a given location.
    ///
    /// Returns `None` if the location has no parent (is a root location).
    async fn get_parent(&self, location_id: LocationId) -> Result<Option<Location>>;

    /// Get all child locations contained within a location.
    ///
    /// Returns an empty vector if the location has no children.
    async fn get_children(&self, location_id: LocationId) -> Result<Vec<Location>>;
}
