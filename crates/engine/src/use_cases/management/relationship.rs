//! Relationship CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, RelationshipId, WorldId};

use crate::use_cases::character_operations::Character;
use crate::infrastructure::ports::ClockPort;

use super::ManagementError;

pub struct RelationshipCrud {
    character: Arc<Character>,
    clock: Arc<dyn ClockPort>,
}

impl RelationshipCrud {
    pub fn new(character: Arc<Character>, clock: Arc<dyn ClockPort>) -> Self {
        Self { character, clock }
    }

    pub async fn list_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Relationship>, ManagementError> {
        let characters = self.character.list_in_world(world_id).await?;
        let mut relationships = Vec::new();
        for character in characters {
            relationships.extend(self.character.get_relationships(character.id).await?);
        }
        Ok(relationships)
    }

    pub async fn create(
        &self,
        from_id: CharacterId,
        to_id: CharacterId,
        relationship_type: String,
        description: Option<String>,
    ) -> Result<wrldbldr_domain::Relationship, ManagementError> {
        let rel_type = relationship_type
            .parse::<wrldbldr_domain::RelationshipType>()
            .map_err(ManagementError::Domain)?;

        let mut relationship = wrldbldr_domain::Relationship::new(from_id, to_id, rel_type);

        if let Some(description) = description {
            relationship.add_event(wrldbldr_domain::RelationshipEvent {
                description,
                sentiment_change: 0.0,
                timestamp: self.clock.now(),
            });
        }

        self.character.save_relationship(&relationship).await?;
        Ok(relationship)
    }

    pub async fn delete(&self, relationship_id: RelationshipId) -> Result<(), ManagementError> {
        self.character.delete_relationship(relationship_id).await?;
        Ok(())
    }

    pub async fn get_for_character(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<wrldbldr_domain::Relationship>, ManagementError> {
        Ok(self.character.get_relationships(character_id).await?)
    }

    pub async fn add_event(
        &self,
        from_id: CharacterId,
        to_id: CharacterId,
        description: String,
        sentiment_change: f32,
    ) -> Result<wrldbldr_domain::Relationship, ManagementError> {
        // Find the existing relationship
        let relationships = self.character.get_relationships(from_id).await?;
        let mut relationship = relationships
            .into_iter()
            .find(|r| r.to_character == to_id)
            .ok_or(ManagementError::NotFound)?;

        // Add the event
        relationship.add_event(wrldbldr_domain::RelationshipEvent {
            description,
            sentiment_change,
            timestamp: self.clock.now(),
        });

        self.character.save_relationship(&relationship).await?;
        Ok(relationship)
    }
}
