//! Player Character management operations.

use std::sync::Arc;

use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, UserId, WorldId};

use crate::infrastructure::ports::{ClockPort, LocationRepo, PlayerCharacterRepo};

use super::ManagementError;

pub struct PlayerCharacterManagement {
    player_character: Arc<dyn PlayerCharacterRepo>,
    location: Arc<dyn LocationRepo>,
    clock: Arc<dyn ClockPort>,
}

impl PlayerCharacterManagement {
    pub fn new(
        player_character: Arc<dyn PlayerCharacterRepo>,
        location: Arc<dyn LocationRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            player_character,
            location,
            clock,
        }
    }

    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::PlayerCharacter>, ManagementError> {
        Ok(self.player_character.list_in_world(world_id).await?)
    }

    pub async fn get(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<wrldbldr_domain::PlayerCharacter>, ManagementError> {
        Ok(self.player_character.get(pc_id).await?)
    }

    pub async fn get_by_user(
        &self,
        world_id: WorldId,
        user_id: String,
    ) -> Result<Option<wrldbldr_domain::PlayerCharacter>, ManagementError> {
        Ok(self
            .player_character
            .get_by_user(world_id, &user_id)
            .await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        user_id: Option<String>,
        starting_region_id: Option<RegionId>,
        sheet_data: Option<wrldbldr_shared::character_sheet::CharacterSheetValues>,
    ) -> Result<wrldbldr_domain::PlayerCharacter, ManagementError> {
        let character_name: wrldbldr_domain::CharacterName = name
            .try_into()
            .map_err(|e| ManagementError::InvalidInput(format!("Invalid character name: {}", e)))?;

        let (starting_location_id, resolved_region_id) =
            self.resolve_spawn(world_id, starting_region_id).await?;

        let now = self.clock.now();
        // Convert user_id String to UserId, defaulting to "anonymous" if not provided
        let user_id = UserId::new(user_id.unwrap_or_else(|| "anonymous".to_string()))
            .unwrap_or_else(|_| UserId::from_trusted("anonymous".to_string()));
        let mut pc = wrldbldr_domain::PlayerCharacter::new(
            user_id,
            world_id,
            character_name,
            starting_location_id,
            now,
        );

        if let Some(region_id) = resolved_region_id {
            pc = pc.with_starting_region(region_id);
        }
        if let Some(sheet_data) = sheet_data {
            pc = pc.with_sheet_data(sheet_data);
        }

        self.player_character.save(&pc).await?;
        Ok(pc)
    }

    pub async fn update(
        &self,
        pc_id: PlayerCharacterId,
        name: Option<String>,
        sheet_data: Option<wrldbldr_shared::character_sheet::CharacterSheetValues>,
    ) -> Result<wrldbldr_domain::PlayerCharacter, ManagementError> {
        let mut pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "PlayerCharacter",
                id: pc_id.to_string(),
            })?;

        if let Some(name) = name {
            let character_name: wrldbldr_domain::CharacterName = name.try_into().map_err(|e| {
                ManagementError::InvalidInput(format!("Invalid character name: {}", e))
            })?;
            pc.set_name(character_name);
        }
        if let Some(sheet_data) = sheet_data {
            pc.set_sheet_data(Some(sheet_data));
        }
        pc.touch(self.clock.now());

        self.player_character.save(&pc).await?;
        Ok(pc)
    }

    pub async fn update_location(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<(), ManagementError> {
        let region =
            self.location
                .get_region(region_id)
                .await?
                .ok_or(ManagementError::NotFound {
                    entity_type: "Region",
                    id: region_id.to_string(),
                })?;

        self.player_character
            .update_position(pc_id, region.location_id(), region_id)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, pc_id: PlayerCharacterId) -> Result<(), ManagementError> {
        self.player_character.delete(pc_id).await?;
        Ok(())
    }

    async fn resolve_spawn(
        &self,
        world_id: WorldId,
        starting_region_id: Option<RegionId>,
    ) -> Result<(LocationId, Option<RegionId>), ManagementError> {
        if let Some(region_id) = starting_region_id {
            let region =
                self.location
                    .get_region(region_id)
                    .await?
                    .ok_or(ManagementError::NotFound {
                        entity_type: "Region",
                        id: region_id.to_string(),
                    })?;
            return Ok((region.location_id(), Some(region.id())));
        }

        let locations = self.location.list_locations_in_world(world_id, None, None).await?;
        for location in &locations {
            let regions = self
                .location
                .list_regions_in_location(location.id(), None, None)
                .await?;
            if let Some(spawn) = regions.iter().find(|r| r.is_spawn_point()) {
                return Ok((location.id(), Some(spawn.id())));
            }
        }

        let fallback_location = locations
            .first()
            .ok_or_else(|| ManagementError::InvalidInput("No locations in world".to_string()))?;
        let regions = self
            .location
            .list_regions_in_location(fallback_location.id(), None, None)
            .await?;
        let region = regions
            .first()
            .ok_or_else(|| ManagementError::InvalidInput("No regions in world".to_string()))?;

        Ok((fallback_location.id(), Some(region.id())))
    }
}
