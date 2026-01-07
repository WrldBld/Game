//! Management use cases for CRUD-style operations.
//!
//! These use cases keep WebSocket handlers thin while coordinating entity modules.

use std::sync::Arc;

use wrldbldr_domain::{
    CharacterId, LocationId, PlayerCharacterId, RegionId, RelationshipId, WorldId,
};

use crate::entities::{Character, Location, Observation, PlayerCharacter, World};
use crate::infrastructure::ports::{ClockPort, RepoError};

/// Shared error type for management use cases.
#[derive(Debug, thiserror::Error)]
pub enum ManagementError {
    #[error("Not found")]
    NotFound,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Domain error: {0}")]
    Domain(String),
}

/// Container for management use cases.
pub struct ManagementUseCases {
    pub world: WorldCrud,
    pub character: CharacterCrud,
    pub location: LocationCrud,
    pub player_character: PlayerCharacterCrud,
    pub relationship: RelationshipCrud,
    pub observation: ObservationCrud,
}

impl ManagementUseCases {
    pub fn new(
        world: WorldCrud,
        character: CharacterCrud,
        location: LocationCrud,
        player_character: PlayerCharacterCrud,
        relationship: RelationshipCrud,
        observation: ObservationCrud,
    ) -> Self {
        Self {
            world,
            character,
            location,
            player_character,
            relationship,
            observation,
        }
    }
}

// =============================================================================
// World CRUD
// =============================================================================

pub struct WorldCrud {
    world: Arc<World>,
    clock: Arc<dyn ClockPort>,
}

impl WorldCrud {
    pub fn new(world: Arc<World>, clock: Arc<dyn ClockPort>) -> Self {
        Self { world, clock }
    }

    pub async fn list(&self) -> Result<Vec<wrldbldr_domain::World>, ManagementError> {
        Ok(self.world.list_all().await?)
    }

    pub async fn get(
        &self,
        world_id: WorldId,
    ) -> Result<Option<wrldbldr_domain::World>, ManagementError> {
        Ok(self.world.get(world_id).await?)
    }

    pub async fn create(
        &self,
        name: String,
        description: Option<String>,
        setting: Option<String>,
    ) -> Result<wrldbldr_domain::World, ManagementError> {
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "World name cannot be empty".to_string(),
            ));
        }

        let now = self.clock.now();
        let mut world = wrldbldr_domain::World::new(name, description.clone().unwrap_or_default(), now);

        if world.description.is_empty() {
            if let Some(setting) = setting {
                world.description = setting;
            }
        }

        self.world.save(&world).await?;
        Ok(world)
    }

    pub async fn update(
        &self,
        world_id: WorldId,
        name: Option<String>,
        description: Option<String>,
        setting: Option<String>,
    ) -> Result<wrldbldr_domain::World, ManagementError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        let now = self.clock.now();

        if let Some(name) = name {
            world.update_name(name, now);
        }
        if let Some(description) = description {
            world.update_description(description, now);
        } else if let Some(setting) = setting {
            world.update_description(setting, now);
        }

        self.world.save(&world).await?;
        Ok(world)
    }

    pub async fn delete(&self, world_id: WorldId) -> Result<(), ManagementError> {
        self.world.delete(world_id).await?;
        Ok(())
    }
}

// =============================================================================
// Character CRUD
// =============================================================================

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

// =============================================================================
// Location + Region CRUD
// =============================================================================

pub struct LocationCrud {
    location: Arc<Location>,
}

impl LocationCrud {
    pub fn new(location: Arc<Location>) -> Self {
        Self { location }
    }

    pub async fn list_locations(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Location>, ManagementError> {
        Ok(self.location.list_in_world(world_id).await?)
    }

    pub async fn get_location(
        &self,
        location_id: LocationId,
    ) -> Result<Option<wrldbldr_domain::Location>, ManagementError> {
        Ok(self.location.get(location_id).await?)
    }

    pub async fn create_location(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        setting: Option<String>,
    ) -> Result<wrldbldr_domain::Location, ManagementError> {
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "Location name cannot be empty".to_string(),
            ));
        }

        let mut location =
            wrldbldr_domain::Location::new(world_id, name, wrldbldr_domain::LocationType::Unknown);
        if let Some(description) = description {
            location = location.with_description(description);
        }
        if let Some(setting) = setting {
            location = location.with_atmosphere(setting);
        }

        self.location.save_location(&location).await?;
        Ok(location)
    }

    pub async fn update_location(
        &self,
        location_id: LocationId,
        name: Option<String>,
        description: Option<String>,
        setting: Option<String>,
    ) -> Result<wrldbldr_domain::Location, ManagementError> {
        let mut location = self
            .location
            .get(location_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(ManagementError::InvalidInput(
                    "Location name cannot be empty".to_string(),
                ));
            }
            location.name = name;
        }
        if let Some(description) = description {
            location.description = description;
        }
        if let Some(setting) = setting {
            location.atmosphere = Some(setting);
        }

        self.location.save_location(&location).await?;
        Ok(location)
    }

    pub async fn delete_location(&self, location_id: LocationId) -> Result<(), ManagementError> {
        self.location.delete(location_id).await?;
        Ok(())
    }

    pub async fn list_regions(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<wrldbldr_domain::Region>, ManagementError> {
        Ok(self.location.list_regions_in_location(location_id).await?)
    }

    pub async fn get_region(
        &self,
        region_id: RegionId,
    ) -> Result<Option<wrldbldr_domain::Region>, ManagementError> {
        Ok(self.location.get_region(region_id).await?)
    }

    pub async fn create_region(
        &self,
        location_id: LocationId,
        name: String,
        description: Option<String>,
        is_spawn_point: Option<bool>,
    ) -> Result<wrldbldr_domain::Region, ManagementError> {
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "Region name cannot be empty".to_string(),
            ));
        }

        let mut region = wrldbldr_domain::Region::new(location_id, name);
        if let Some(description) = description {
            region = region.with_description(description);
        }
        if is_spawn_point.unwrap_or(false) {
            region = region.as_spawn_point();
        }

        self.location.save_region(&region).await?;
        Ok(region)
    }

    pub async fn update_region(
        &self,
        region_id: RegionId,
        name: Option<String>,
        description: Option<String>,
        is_spawn_point: Option<bool>,
    ) -> Result<wrldbldr_domain::Region, ManagementError> {
        let mut region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(ManagementError::InvalidInput(
                    "Region name cannot be empty".to_string(),
                ));
            }
            region.name = name;
        }
        if let Some(description) = description {
            region.description = description;
        }
        if let Some(is_spawn_point) = is_spawn_point {
            region.is_spawn_point = is_spawn_point;
        }

        self.location.save_region(&region).await?;
        Ok(region)
    }

    pub async fn delete_region(&self, region_id: RegionId) -> Result<(), ManagementError> {
        self.location.delete_region(region_id).await?;
        Ok(())
    }

    pub async fn list_spawn_points(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Region>, ManagementError> {
        let mut spawn_points = Vec::new();
        let locations = self.location.list_in_world(world_id).await?;
        for location in locations {
            let regions = self.location.list_regions_in_location(location.id).await?;
            spawn_points.extend(regions.into_iter().filter(|r| r.is_spawn_point));
        }
        Ok(spawn_points)
    }

    pub async fn list_location_connections(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<wrldbldr_domain::LocationConnection>, ManagementError> {
        Ok(self.location.get_location_exits(location_id).await?)
    }

    pub async fn create_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
        bidirectional: bool,
    ) -> Result<(), ManagementError> {
        let connection = wrldbldr_domain::LocationConnection {
            from_location,
            to_location,
            connection_type: "Connection".to_string(),
            description: None,
            bidirectional,
            travel_time: 0,
            is_locked: false,
            lock_description: None,
        };

        self.location.save_location_connection(&connection).await?;
        Ok(())
    }

    pub async fn delete_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
    ) -> Result<(), ManagementError> {
        self.location
            .delete_location_connection(from_location, to_location)
            .await?;
        Ok(())
    }

    pub async fn list_region_connections(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<wrldbldr_domain::RegionConnection>, ManagementError> {
        Ok(self.location.get_connections(region_id).await?)
    }

    pub async fn create_region_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
        description: Option<String>,
        bidirectional: Option<bool>,
        locked: Option<bool>,
        lock_description: Option<String>,
    ) -> Result<(), ManagementError> {
        let mut connection = wrldbldr_domain::RegionConnection::new(from_region, to_region);
        if let Some(description) = description {
            connection = connection.with_description(description);
        }
        if bidirectional == Some(false) {
            connection = connection.one_way();
        }
        if locked.unwrap_or(false) {
            connection = connection.locked(lock_description.unwrap_or_else(|| "Locked".to_string()));
        }

        self.location.save_connection(&connection).await?;
        Ok(())
    }

    pub async fn delete_region_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), ManagementError> {
        self.location.delete_connection(from_region, to_region).await?;
        Ok(())
    }

    pub async fn unlock_region_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), ManagementError> {
        let connections = self.location.get_connections(from_region).await?;
        let existing = connections
            .into_iter()
            .find(|c| c.to_region == to_region)
            .ok_or(ManagementError::NotFound)?;

        let mut updated = wrldbldr_domain::RegionConnection::new(existing.from_region, existing.to_region);
        updated.description = existing.description;
        updated.bidirectional = existing.bidirectional;
        updated.is_locked = false;
        updated.lock_description = None;

        self.location.save_connection(&updated).await?;
        Ok(())
    }

    pub async fn list_region_exits(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<wrldbldr_domain::RegionExit>, ManagementError> {
        Ok(self.location.get_region_exits(region_id).await?)
    }

    pub async fn create_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        arrival_region_id: RegionId,
        description: Option<String>,
        bidirectional: Option<bool>,
    ) -> Result<(), ManagementError> {
        let exit = wrldbldr_domain::RegionExit {
            from_region: region_id,
            to_location: location_id,
            arrival_region_id,
            description,
            bidirectional: bidirectional.unwrap_or(true),
        };
        self.location.save_region_exit(&exit).await?;
        Ok(())
    }

    pub async fn delete_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
    ) -> Result<(), ManagementError> {
        self.location.delete_region_exit(region_id, location_id).await?;
        Ok(())
    }
}

// =============================================================================
// Player Character CRUD
// =============================================================================

pub struct PlayerCharacterCrud {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    clock: Arc<dyn ClockPort>,
}

impl PlayerCharacterCrud {
    pub fn new(
        player_character: Arc<PlayerCharacter>,
        location: Arc<Location>,
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
        Ok(self.player_character.get_by_user(world_id, &user_id).await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        user_id: Option<String>,
        starting_region_id: Option<RegionId>,
        sheet_data: Option<serde_json::Value>,
    ) -> Result<wrldbldr_domain::PlayerCharacter, ManagementError> {
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "Player character name cannot be empty".to_string(),
            ));
        }

        let (starting_location_id, resolved_region_id) =
            self.resolve_spawn(world_id, starting_region_id).await?;

        let now = self.clock.now();
        let mut pc = wrldbldr_domain::PlayerCharacter::new(
            user_id.unwrap_or_else(|| "anonymous".to_string()),
            world_id,
            name,
            starting_location_id,
            now,
        );

        if let Some(region_id) = resolved_region_id {
            pc = pc.with_starting_region(region_id);
        }
        if let Some(sheet_data) = sheet_data {
            let data: wrldbldr_domain::CharacterSheetData =
                serde_json::from_value(sheet_data).map_err(|e| {
                    ManagementError::InvalidInput(format!(
                        "Invalid sheet_data: {}",
                        e.to_string()
                    ))
                })?;
            pc = pc.with_sheet_data(data);
        }

        self.player_character.save(&pc).await?;
        Ok(pc)
    }

    pub async fn update(
        &self,
        pc_id: PlayerCharacterId,
        name: Option<String>,
        sheet_data: Option<serde_json::Value>,
    ) -> Result<wrldbldr_domain::PlayerCharacter, ManagementError> {
        let mut pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(ManagementError::InvalidInput(
                    "Player character name cannot be empty".to_string(),
                ));
            }
            pc.name = name;
        }
        if let Some(sheet_data) = sheet_data {
            let data: wrldbldr_domain::CharacterSheetData =
                serde_json::from_value(sheet_data).map_err(|e| {
                    ManagementError::InvalidInput(format!(
                        "Invalid sheet_data: {}",
                        e.to_string()
                    ))
                })?;
            pc.sheet_data = Some(data);
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
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        self.player_character
            .update_position(pc_id, region.location_id, region_id)
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
            let region = self
                .location
                .get_region(region_id)
                .await?
                .ok_or(ManagementError::NotFound)?;
            return Ok((region.location_id, Some(region.id)));
        }

        let locations = self.location.list_in_world(world_id).await?;
        for location in &locations {
            let regions = self.location.list_regions_in_location(location.id).await?;
            if let Some(spawn) = regions.iter().find(|r| r.is_spawn_point) {
                return Ok((location.id, Some(spawn.id)));
            }
        }

        let fallback_location = locations
            .first()
            .ok_or_else(|| ManagementError::InvalidInput("No locations in world".to_string()))?;
        let regions = self
            .location
            .list_regions_in_location(fallback_location.id)
            .await?;
        let region = regions
            .first()
            .ok_or_else(|| ManagementError::InvalidInput("No regions in world".to_string()))?;

        Ok((fallback_location.id, Some(region.id)))
    }
}

// =============================================================================
// Relationship CRUD
// =============================================================================

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
}

// =============================================================================
// Observation CRUD
// =============================================================================

pub struct ObservationCrud {
    observation: Arc<Observation>,
    player_character: Arc<PlayerCharacter>,
    character: Arc<Character>,
    location: Arc<Location>,
    world: Arc<World>,
    clock: Arc<dyn ClockPort>,
}

impl ObservationCrud {
    pub fn new(
        observation: Arc<Observation>,
        player_character: Arc<PlayerCharacter>,
        character: Arc<Character>,
        location: Arc<Location>,
        world: Arc<World>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            observation,
            player_character,
            character,
            location,
            world,
            clock,
        }
    }

    pub async fn list(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<wrldbldr_domain::NpcObservation>, ManagementError> {
        Ok(self.observation.get_observations(pc_id).await?)
    }

    pub async fn list_summaries(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<ObservationSummaryData>, ManagementError> {
        let observations = self.observation.get_observations(pc_id).await?;
        let mut summaries = Vec::new();

        for observation in observations {
            let npc = self.character.get(observation.npc_id).await?;
            let region = self.location.get_region(observation.region_id).await?;
            let location = self.location.get(observation.location_id).await?;

            let (npc_name, npc_portrait) = if observation.is_revealed_to_player {
                (
                    npc.as_ref()
                        .map(|n| n.name.clone())
                        .unwrap_or_else(|| "Unknown NPC".to_string()),
                    npc.as_ref().and_then(|n| n.portrait_asset.clone()),
                )
            } else {
                ("Unknown Figure".to_string(), None)
            };

            let location_name = location
                .as_ref()
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Unknown Location".to_string());
            let region_name = region
                .as_ref()
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "Unknown Region".to_string());

            let (obs_type, obs_icon) = match observation.observation_type {
                wrldbldr_domain::ObservationType::Direct => ("direct", "eye"),
                wrldbldr_domain::ObservationType::HeardAbout => ("heard_about", "ear"),
                wrldbldr_domain::ObservationType::Deduced => ("deduced", "brain"),
            };

            summaries.push(ObservationSummaryData {
                npc_id: observation.npc_id.to_string(),
                npc_name,
                npc_portrait,
                location_name,
                region_name,
                game_time: observation.game_time.to_rfc3339(),
                observation_type: obs_type.to_string(),
                observation_type_icon: obs_icon.to_string(),
                notes: observation.notes.clone(),
            });
        }

        Ok(summaries)
    }

    pub async fn create(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        observation_type: String,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
        notes: Option<String>,
    ) -> Result<wrldbldr_domain::NpcObservation, ManagementError> {
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        let (location_id, region_id) =
            self.resolve_observation_location(location_id, region_id).await?;

        let world = self
            .world
            .get(pc.world_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        let obs_type =
            observation_type.parse::<wrldbldr_domain::ObservationType>().map_err(|e| {
                ManagementError::InvalidInput(format!("Invalid observation type: {}", e))
            })?;

        let now = self.clock.now();
        let game_time = world.game_time.current();
        let observation = match obs_type {
            wrldbldr_domain::ObservationType::Direct => {
                wrldbldr_domain::NpcObservation::direct(
                    pc_id,
                    npc_id,
                    location_id,
                    region_id,
                    game_time,
                    now,
                )
            }
            wrldbldr_domain::ObservationType::HeardAbout => {
                wrldbldr_domain::NpcObservation::heard_about(
                    pc_id,
                    npc_id,
                    location_id,
                    region_id,
                    game_time,
                    notes.clone(),
                    now,
                )
            }
            wrldbldr_domain::ObservationType::Deduced => {
                wrldbldr_domain::NpcObservation::deduced(
                    pc_id,
                    npc_id,
                    location_id,
                    region_id,
                    game_time,
                    notes.clone(),
                    now,
                )
            }
        };

        self.observation.save_observation(&observation).await?;
        Ok(observation)
    }

    pub async fn delete(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<(), ManagementError> {
        self.observation.delete_observation(pc_id, npc_id).await?;
        Ok(())
    }

    async fn resolve_observation_location(
        &self,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
    ) -> Result<(LocationId, RegionId), ManagementError> {
        match (location_id, region_id) {
            (Some(location_id), Some(region_id)) => Ok((location_id, region_id)),
            (None, Some(region_id)) => {
                let region = self
                    .location
                    .get_region(region_id)
                    .await?
                    .ok_or(ManagementError::NotFound)?;
                Ok((region.location_id, region_id))
            }
            _ => Err(ManagementError::InvalidInput(
                "location_id and/or region_id required".to_string(),
            )),
        }
    }
}

/// Summary of an NPC observation for UI consumption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObservationSummaryData {
    pub npc_id: String,
    pub npc_name: String,
    pub npc_portrait: Option<String>,
    pub location_name: String,
    pub region_name: String,
    pub game_time: String,
    pub observation_type: String,
    pub observation_type_icon: String,
    pub notes: Option<String>,
}
