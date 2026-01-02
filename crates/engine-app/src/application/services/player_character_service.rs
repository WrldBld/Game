//! Player Character Service - Application service for player character management
//!
//! This service provides use case implementations for creating, updating,
//! and managing player characters (PCs) in sessions.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::entities::CharacterSheetData;
use wrldbldr_domain::entities::PlayerCharacter;
use wrldbldr_domain::{LocationId, PlayerCharacterId, SkillId, WorldId};
use crate::application::services::internal::PlayerCharacterServicePort;
use wrldbldr_engine_ports::outbound::{
    ClockPort, LocationCrudPort, PlayerCharacterCrudPort, PlayerCharacterPositionPort,
    PlayerCharacterQueryPort, WorldRepositoryPort,
};

/// Request to create a new player character
#[derive(Debug, Clone)]
pub struct CreatePlayerCharacterRequest {
    pub user_id: String,
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub starting_location_id: LocationId,
    pub sheet_data: Option<CharacterSheetData>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Request to update an existing player character
#[derive(Debug, Clone)]
pub struct UpdatePlayerCharacterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sheet_data: Option<CharacterSheetData>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Player character service trait defining the application use cases
#[async_trait]
pub trait PlayerCharacterService: Send + Sync {
    /// Create a new player character
    async fn create_pc(&self, request: CreatePlayerCharacterRequest) -> Result<PlayerCharacter>;

    /// Get a player character by ID
    async fn get_pc(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Get a player character by user ID and world ID
    async fn get_pc_by_user_and_world(
        &self,
        user_id: &str,
        world_id: &WorldId,
    ) -> Result<Option<PlayerCharacter>>;

    /// Get all player characters in a world
    async fn get_pcs_by_world(&self, world_id: &WorldId) -> Result<Vec<PlayerCharacter>>;

    /// Update a player character
    async fn update_pc(
        &self,
        id: PlayerCharacterId,
        request: UpdatePlayerCharacterRequest,
    ) -> Result<PlayerCharacter>;

    /// Update a player character's location
    async fn update_pc_location(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Delete a player character
    async fn delete_pc(&self, id: PlayerCharacterId) -> Result<()>;

    /// Get a player character's modifier for a specific skill.
    /// Returns 0 if the PC doesn't have the skill or doesn't have sheet data.
    async fn get_skill_modifier(&self, id: PlayerCharacterId, skill_id: SkillId) -> Result<i32>;
}

/// Default implementation of PlayerCharacterService using port abstractions
#[derive(Clone)]
pub struct PlayerCharacterServiceImpl {
    pc_crud: Arc<dyn PlayerCharacterCrudPort>,
    pc_query: Arc<dyn PlayerCharacterQueryPort>,
    pc_position: Arc<dyn PlayerCharacterPositionPort>,
    location_repository: Arc<dyn LocationCrudPort>,
    world_repository: Arc<dyn WorldRepositoryPort>,
    clock: Arc<dyn ClockPort>,
}

impl PlayerCharacterServiceImpl {
    /// Create a new PlayerCharacterServiceImpl with the given repositories
    pub fn new(
        pc_crud: Arc<dyn PlayerCharacterCrudPort>,
        pc_query: Arc<dyn PlayerCharacterQueryPort>,
        pc_position: Arc<dyn PlayerCharacterPositionPort>,
        location_repository: Arc<dyn LocationCrudPort>,
        world_repository: Arc<dyn WorldRepositoryPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            pc_crud,
            pc_query,
            pc_position,
            location_repository,
            world_repository,
            clock,
        }
    }

    /// Validate create request
    fn validate_create_request(request: &CreatePlayerCharacterRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Character name cannot be empty"));
        }
        Ok(())
    }
}

#[async_trait]
impl PlayerCharacterService for PlayerCharacterServiceImpl {
    #[instrument(skip(self), fields(user_id = %request.user_id, name = %request.name))]
    async fn create_pc(&self, request: CreatePlayerCharacterRequest) -> Result<PlayerCharacter> {
        Self::validate_create_request(&request)?;

        // Verify the world exists
        let _ = self
            .world_repository
            .get(request.world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("World not found: {}", request.world_id))?;

        // Verify the location exists
        let _ = self
            .location_repository
            .get(request.starting_location_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Location not found: {}", request.starting_location_id)
            })?;

        let mut pc = PlayerCharacter::new(
            request.user_id.clone(),
            request.world_id,
            request.name.clone(),
            request.starting_location_id,
            self.clock.now(),
        );

        if let Some(description) = request.description {
            pc = pc.with_description(description);
        }
        if let Some(sheet_data) = request.sheet_data {
            pc = pc.with_sheet_data(sheet_data);
        }
        if let Some(sprite) = request.sprite_asset {
            pc = pc.with_sprite(sprite);
        }
        if let Some(portrait) = request.portrait_asset {
            pc = pc.with_portrait(portrait);
        }

        // Validate the PC
        pc.validate().map_err(|e| anyhow::anyhow!(e))?;

        self.pc_crud
            .create(&pc)
            .await
            .context("Failed to create player character in repository")?;

        info!(
            pc_id = %pc.id,
            user_id = %pc.user_id,
            world_id = %pc.world_id,
            "Created player character: {}",
            pc.name
        );
        Ok(pc)
    }

    #[instrument(skip(self), fields(pc_id = %id))]
    async fn get_pc(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>> {
        debug!(pc_id = %id, "Fetching player character");
        self.pc_crud
            .get(id)
            .await
            .context("Failed to get player character from repository")
    }

    #[instrument(skip(self), fields(user_id = %user_id, world_id = %world_id))]
    async fn get_pc_by_user_and_world(
        &self,
        user_id: &str,
        world_id: &WorldId,
    ) -> Result<Option<PlayerCharacter>> {
        debug!(user_id = %user_id, world_id = %world_id, "Fetching player character by user and world");
        // Get all PCs for user in this world and return the first one (active PC)
        let pcs = self
            .pc_query
            .get_by_user_and_world(user_id, *world_id)
            .await
            .context("Failed to get player character from repository")?;
        Ok(pcs.into_iter().next())
    }

    #[instrument(skip(self), fields(world_id = %world_id))]
    async fn get_pcs_by_world(&self, world_id: &WorldId) -> Result<Vec<PlayerCharacter>> {
        debug!(world_id = %world_id, "Fetching player characters for world");
        self.pc_query
            .get_all_by_world(*world_id)
            .await
            .context("Failed to get player characters from repository")
    }

    #[instrument(skip(self), fields(pc_id = %id))]
    async fn update_pc(
        &self,
        id: PlayerCharacterId,
        request: UpdatePlayerCharacterRequest,
    ) -> Result<PlayerCharacter> {
        let mut pc = self
            .pc_crud
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        if let Some(name) = request.name {
            if name.trim().is_empty() {
                return Err(anyhow::anyhow!("Character name cannot be empty"));
            }
            pc.name = name;
        }
        if let Some(description) = request.description {
            pc.description = if description.trim().is_empty() {
                None
            } else {
                Some(description)
            };
        }
        if let Some(sheet_data) = request.sheet_data {
            pc.sheet_data = Some(sheet_data);
        }
        if let Some(sprite) = request.sprite_asset {
            pc.sprite_asset = if sprite.trim().is_empty() {
                None
            } else {
                Some(sprite)
            };
        }
        if let Some(portrait) = request.portrait_asset {
            pc.portrait_asset = if portrait.trim().is_empty() {
                None
            } else {
                Some(portrait)
            };
        }

        pc.touch(self.clock.now()); // Update last_active_at
        pc.validate().map_err(|e| anyhow::anyhow!(e))?;

        self.pc_crud
            .update(&pc)
            .await
            .context("Failed to update player character in repository")?;

        info!(pc_id = %pc.id, "Updated player character: {}", pc.name);
        Ok(pc)
    }

    #[instrument(skip(self), fields(pc_id = %id, location_id = %location_id))]
    async fn update_pc_location(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        // Verify the location exists
        let _ = self
            .location_repository
            .get(location_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Location not found: {}", location_id))?;

        // Verify the PC exists
        let _ = self
            .pc_crud
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        self.pc_position
            .update_location(id, location_id)
            .await
            .context("Failed to update player character location in repository")?;

        info!(pc_id = %id, location_id = %location_id, "Updated player character location");
        Ok(())
    }

    #[instrument(skip(self), fields(pc_id = %id))]
    async fn delete_pc(&self, id: PlayerCharacterId) -> Result<()> {
        let pc = self
            .pc_crud
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        self.pc_crud
            .delete(id)
            .await
            .context("Failed to delete player character from repository")?;

        info!(pc_id = %id, "Deleted player character: {}", pc.name);
        Ok(())
    }

    #[instrument(skip(self), fields(pc_id = %id, skill_id = %skill_id))]
    async fn get_skill_modifier(&self, id: PlayerCharacterId, skill_id: SkillId) -> Result<i32> {
        let pc = self
            .pc_crud
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", id))?;

        // If the PC has sheet data, look up the skill modifier
        if let Some(sheet_data) = &pc.sheet_data {
            let skill_id_str = skill_id.to_string();
            if let Some(modifier) = sheet_data.get_skill_modifier(&skill_id_str) {
                debug!(pc_id = %id, skill_id = %skill_id, modifier = modifier, "Found skill modifier");
                return Ok(modifier);
            }
        }

        // Default to 0 if no sheet data or skill not found
        debug!(pc_id = %id, skill_id = %skill_id, "No skill modifier found, defaulting to 0");
        Ok(0)
    }
}

// Implementation of the port trait for hexagonal architecture compliance
#[async_trait]
impl PlayerCharacterServicePort for PlayerCharacterServiceImpl {
    async fn get_pc(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>> {
        PlayerCharacterService::get_pc(self, id).await
    }

    async fn get_pc_by_user_and_world(
        &self,
        user_id: &str,
        world_id: &WorldId,
    ) -> Result<Option<PlayerCharacter>> {
        PlayerCharacterService::get_pc_by_user_and_world(self, user_id, world_id).await
    }

    async fn get_pcs_by_world(&self, world_id: &WorldId) -> Result<Vec<PlayerCharacter>> {
        PlayerCharacterService::get_pcs_by_world(self, world_id).await
    }

    async fn get_skill_modifier(&self, id: PlayerCharacterId, skill_id: SkillId) -> Result<i32> {
        PlayerCharacterService::get_skill_modifier(self, id, skill_id).await
    }
}
