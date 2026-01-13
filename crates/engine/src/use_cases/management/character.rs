//! Character CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, WorldId};

use crate::entities::Character;
use crate::infrastructure::ports::ClockPort;

use super::ManagementError;

pub struct CharacterCrud {
    character: Arc<Character>,
    clock: Arc<dyn ClockPort>,
}

impl CharacterCrud {
    pub fn new(character: Arc<Character>, clock: Arc<dyn ClockPort>) -> Self {
        Self { character, clock }
    }

    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Character>, ManagementError> {
        Ok(self.character.list_in_world(world_id).await?)
    }

    pub async fn get(
        &self,
        character_id: CharacterId,
    ) -> Result<Option<wrldbldr_domain::Character>, ManagementError> {
        Ok(self.character.get(character_id).await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        archetype: Option<String>,
        sprite_asset: Option<String>,
        portrait_asset: Option<String>,
    ) -> Result<wrldbldr_domain::Character, ManagementError> {
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "Character name cannot be empty".to_string(),
            ));
        }

        let archetype_value = archetype
            .as_deref()
            .unwrap_or("Unknown")
            .parse::<wrldbldr_domain::CampbellArchetype>()
            .map_err(ManagementError::Domain)?;

        let mut character = wrldbldr_domain::Character::new(world_id, name, archetype_value);

        if let Some(description) = description {
            character = character.with_description(description);
        }
        if let Some(sprite) = sprite_asset {
            character = character.with_sprite(sprite);
        }
        if let Some(portrait) = portrait_asset {
            character = character.with_portrait(portrait);
        }

        self.character.save(&character).await?;
        Ok(character)
    }

    pub async fn update(
        &self,
        character_id: CharacterId,
        name: Option<String>,
        description: Option<String>,
        sprite_asset: Option<String>,
        portrait_asset: Option<String>,
        is_alive: Option<bool>,
        is_active: Option<bool>,
    ) -> Result<wrldbldr_domain::Character, ManagementError> {
        let mut character = self
            .character
            .get(character_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(ManagementError::InvalidInput(
                    "Character name cannot be empty".to_string(),
                ));
            }
            character.name = name;
        }
        if let Some(description) = description {
            character.description = description;
        }
        if let Some(sprite) = sprite_asset {
            character.sprite_asset = Some(sprite);
        }
        if let Some(portrait) = portrait_asset {
            character.portrait_asset = Some(portrait);
        }
        if let Some(is_alive) = is_alive {
            character.is_alive = is_alive;
        }
        if let Some(is_active) = is_active {
            character.is_active = is_active;
        }

        self.character.save(&character).await?;
        Ok(character)
    }

    pub async fn delete(&self, character_id: CharacterId) -> Result<(), ManagementError> {
        self.character.delete(character_id).await?;
        Ok(())
    }

    pub async fn change_archetype(
        &self,
        character_id: CharacterId,
        new_archetype: String,
        reason: String,
    ) -> Result<(), ManagementError> {
        let mut character = self
            .character
            .get(character_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        let archetype_value = new_archetype
            .parse::<wrldbldr_domain::CampbellArchetype>()
            .map_err(ManagementError::Domain)?;

        character.change_archetype(archetype_value, reason, self.clock.now());
        self.character.save(&character).await?;
        Ok(())
    }
}
