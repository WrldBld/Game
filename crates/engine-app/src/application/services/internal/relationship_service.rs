//! Relationship service port - Interface for relationship operations
//!
//! This port abstracts relationship business logic from infrastructure adapters.
//! It provides methods for managing relationships between characters in the
//! social network.
//!
//! # Design Notes
//!
//! This port exposes CRUD operations for relationships plus social network
//! queries. Infrastructure adapters should depend on this trait rather than
//! importing the service directly from engine-app.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::value_objects::Relationship;
use wrldbldr_domain::{CharacterId, RelationshipId, WorldId};

use wrldbldr_engine_ports::outbound::SocialNetwork;

/// Port for relationship service operations.
///
/// This trait provides access to relationship management functionality
/// including CRUD operations and social network queries.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait RelationshipServicePort: Send + Sync {
    /// Get all relationships for a character (outgoing).
    ///
    /// Returns relationships where the character is the source.
    ///
    /// # Arguments
    ///
    /// * `character_id` - The ID of the character whose relationships to fetch
    ///
    /// # Returns
    ///
    /// A vector of relationships originating from this character.
    async fn get_relationships(&self, character_id: CharacterId) -> Result<Vec<Relationship>>;

    /// Create a new relationship between characters.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship to create
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship cannot be created (e.g., duplicate,
    /// invalid character IDs).
    async fn create_relationship(&self, relationship: &Relationship) -> Result<()>;

    /// Update an existing relationship.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship with updated fields
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship doesn't exist or cannot be updated.
    async fn update_relationship(&self, relationship: &Relationship) -> Result<()>;

    /// Delete a relationship.
    ///
    /// # Arguments
    ///
    /// * `relationship_id` - The ID of the relationship to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship doesn't exist or cannot be deleted.
    async fn delete_relationship(&self, relationship_id: RelationshipId) -> Result<()>;

    /// Get a specific relationship by ID.
    ///
    /// # Arguments
    ///
    /// * `relationship_id` - The ID of the relationship to retrieve
    ///
    /// # Returns
    ///
    /// `Ok(Some(relationship))` if found, `Ok(None)` if not found.
    async fn get_relationship(
        &self,
        relationship_id: RelationshipId,
    ) -> Result<Option<Relationship>>;

    /// Get the social network graph for a world.
    ///
    /// Returns a graph representation of all character relationships
    /// in the specified world.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world whose social network to fetch
    ///
    /// # Returns
    ///
    /// A `SocialNetwork` containing character nodes and relationship edges.
    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of RelationshipServicePort for testing.
    pub RelationshipServicePort {}

    #[async_trait]
    impl RelationshipServicePort for RelationshipServicePort {
        async fn get_relationships(&self, character_id: CharacterId) -> Result<Vec<Relationship>>;
        async fn create_relationship(&self, relationship: &Relationship) -> Result<()>;
        async fn update_relationship(&self, relationship: &Relationship) -> Result<()>;
        async fn delete_relationship(&self, relationship_id: RelationshipId) -> Result<()>;
        async fn get_relationship(&self, relationship_id: RelationshipId) -> Result<Option<Relationship>>;
        async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork>;
    }
}
