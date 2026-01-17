//! Character CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, WorldId};

use crate::repositories::{CharacterRepository, ClockService};

use super::ManagementError;

pub struct CharacterCrud {
    character: Arc<CharacterRepository>,
    clock: Arc<ClockService>,
}

impl CharacterCrud {
    pub fn new(character: Arc<CharacterRepository>, clock: Arc<ClockService>) -> Self {
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
        // Create validated CharacterName (handles empty/whitespace validation)
        let character_name = wrldbldr_domain::CharacterName::new(name)
            .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;

        let archetype_value = archetype
            .as_deref()
            .unwrap_or("Unknown")
            .parse::<wrldbldr_domain::CampbellArchetype>()
            .map_err(|e| ManagementError::Domain(e.to_string()))?;

        let mut character =
            wrldbldr_domain::Character::new(world_id, character_name, archetype_value);

        if let Some(description) = description {
            let desc = wrldbldr_domain::Description::new(description)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            character = character.with_description(desc);
        }
        if let Some(sprite) = sprite_asset {
            let asset_path = wrldbldr_domain::AssetPath::new(sprite)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            character = character.with_sprite(asset_path);
        }
        if let Some(portrait) = portrait_asset {
            let asset_path = wrldbldr_domain::AssetPath::new(portrait)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            character = character.with_portrait(asset_path);
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

        // For name/description, we need to rebuild the character using builder pattern
        // since fields are private. We use the with_* methods for immutable updates.
        if let Some(name) = name {
            let validated_name = wrldbldr_domain::CharacterName::new(name)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            // Rebuild character with new name using aggregate's constructor + builder
            let new_character = wrldbldr_domain::Character::new(
                character.world_id(),
                validated_name,
                character.base_archetype(),
            )
            .with_id(character.id())
            .with_description(character.description().clone())
            .with_stats(character.stats().clone())
            .with_state(character.state())
            .with_current_archetype(character.current_archetype())
            .with_archetype_history(character.archetype_history().to_vec())
            .with_default_disposition(character.default_disposition())
            .with_default_mood(*character.default_mood())
            .with_expression_config(character.expression_config().clone());
            // Copy assets if present
            let new_character = if let Some(sprite) = character.sprite_asset() {
                new_character.with_sprite(sprite.clone())
            } else {
                new_character
            };
            let new_character = if let Some(portrait) = character.portrait_asset() {
                new_character.with_portrait(portrait.clone())
            } else {
                new_character
            };
            character = new_character;
        }
        if let Some(description) = description {
            let desc = wrldbldr_domain::Description::new(description)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            character.set_description(desc);
        }
        if let Some(sprite) = sprite_asset {
            let asset_path = wrldbldr_domain::AssetPath::new(sprite)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            character.set_sprite(Some(asset_path));
        }
        if let Some(portrait) = portrait_asset {
            let asset_path = wrldbldr_domain::AssetPath::new(portrait)
                .map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            character.set_portrait(Some(asset_path));
        }
        if let Some(is_alive) = is_alive {
            if is_alive && character.is_dead() {
                // Resurrect if was dead and should be alive
                character.resurrect();
            } else if !is_alive && character.is_alive() {
                // Kill if was alive and should be dead - apply max damage
                if let Some(max_hp) = character.stats().max_hp() {
                    character.apply_damage(max_hp + 1000);
                } else {
                    // No HP tracking, manually set state via with_state
                    character = wrldbldr_domain::Character::new(
                        character.world_id(),
                        character.name().clone(),
                        character.base_archetype(),
                    )
                    .with_id(character.id())
                    .with_description(character.description().clone())
                    .with_stats(character.stats().clone())
                    .with_state(wrldbldr_domain::CharacterState::Dead)
                    .with_current_archetype(character.current_archetype())
                    .with_archetype_history(character.archetype_history().to_vec())
                    .with_default_disposition(character.default_disposition())
                    .with_default_mood(*character.default_mood())
                    .with_expression_config(character.expression_config().clone());
                }
            }
        }
        if let Some(is_active) = is_active {
            if is_active {
                character.activate();
            } else {
                character.deactivate();
            }
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
            .map_err(|e| ManagementError::Domain(e.to_string()))?;

        character.change_archetype(archetype_value, reason, self.clock.now());
        self.character.save(&character).await?;
        Ok(())
    }
}
