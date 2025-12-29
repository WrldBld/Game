//! Relationship Service - Application service for relationship management
//!
//! This service provides use case implementations for managing relationships
//! between characters in the social network.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::value_objects::Relationship;
use wrldbldr_domain::{CharacterId, RelationshipId, WorldId};
use wrldbldr_engine_ports::outbound::{
    RelationshipRepositoryPort, RelationshipServicePort, SocialNetwork,
};

/// Relationship service trait defining the application use cases
#[async_trait]
pub trait RelationshipService: Send + Sync {
    /// Get all relationships for a character (outgoing)
    async fn get_relationships(&self, character_id: CharacterId) -> Result<Vec<Relationship>>;

    /// Create a new relationship between characters
    async fn create_relationship(&self, relationship: &Relationship) -> Result<()>;

    /// Update an existing relationship
    async fn update_relationship(&self, relationship: &Relationship) -> Result<()>;

    /// Delete a relationship
    async fn delete_relationship(&self, relationship_id: RelationshipId) -> Result<()>;

    /// Get a specific relationship by ID
    async fn get_relationship(
        &self,
        relationship_id: RelationshipId,
    ) -> Result<Option<Relationship>>;

    /// Get the social network graph for a world
    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork>;
}

/// Default implementation of RelationshipService using port abstractions
#[derive(Clone)]
pub struct RelationshipServiceImpl {
    repository: Arc<dyn RelationshipRepositoryPort>,
}

impl RelationshipServiceImpl {
    /// Create a new RelationshipServiceImpl with the given repository
    pub fn new(repository: Arc<dyn RelationshipRepositoryPort>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl RelationshipService for RelationshipServiceImpl {
    #[instrument(skip(self))]
    async fn get_relationships(&self, character_id: CharacterId) -> Result<Vec<Relationship>> {
        debug!(character_id = %character_id, "Fetching relationships for character");
        self.repository
            .get_for_character(character_id)
            .await
            .context("Failed to get relationships from repository")
    }

    #[instrument(skip(self, relationship))]
    async fn create_relationship(&self, relationship: &Relationship) -> Result<()> {
        info!(
            from = %relationship.from_character,
            to = %relationship.to_character,
            "Creating relationship"
        );

        self.repository
            .create(relationship)
            .await
            .context("Failed to create relationship in repository")?;

        info!(relationship_id = %relationship.id, "Created relationship");
        Ok(())
    }

    #[instrument(skip(self, relationship))]
    async fn update_relationship(&self, relationship: &Relationship) -> Result<()> {
        info!(relationship_id = %relationship.id, "Updating relationship");

        self.repository
            .update(relationship)
            .await
            .context("Failed to update relationship in repository")?;

        info!(relationship_id = %relationship.id, "Updated relationship");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_relationship(&self, relationship_id: RelationshipId) -> Result<()> {
        info!(relationship_id = %relationship_id, "Deleting relationship");

        self.repository
            .delete(relationship_id)
            .await
            .context("Failed to delete relationship from repository")?;

        info!(relationship_id = %relationship_id, "Deleted relationship");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_relationship(
        &self,
        relationship_id: RelationshipId,
    ) -> Result<Option<Relationship>> {
        debug!(relationship_id = %relationship_id, "Fetching relationship");
        self.repository
            .get(relationship_id)
            .await
            .context("Failed to get relationship from repository")
    }

    #[instrument(skip(self))]
    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork> {
        debug!(world_id = %world_id, "Fetching social network for world");
        self.repository
            .get_social_network(world_id)
            .await
            .context("Failed to get social network from repository")
    }
}

// =============================================================================
// RelationshipServicePort Implementation
// =============================================================================

#[async_trait]
impl RelationshipServicePort for RelationshipServiceImpl {
    async fn get_relationships(&self, character_id: CharacterId) -> Result<Vec<Relationship>> {
        RelationshipService::get_relationships(self, character_id).await
    }

    async fn create_relationship(&self, relationship: &Relationship) -> Result<()> {
        RelationshipService::create_relationship(self, relationship).await
    }

    async fn update_relationship(&self, relationship: &Relationship) -> Result<()> {
        RelationshipService::update_relationship(self, relationship).await
    }

    async fn delete_relationship(&self, relationship_id: RelationshipId) -> Result<()> {
        RelationshipService::delete_relationship(self, relationship_id).await
    }

    async fn get_relationship(
        &self,
        relationship_id: RelationshipId,
    ) -> Result<Option<Relationship>> {
        RelationshipService::get_relationship(self, relationship_id).await
    }

    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork> {
        RelationshipService::get_social_network(self, world_id).await
    }
}
