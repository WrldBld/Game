//! Application Request Handler - Routes WebSocket requests to services
//!
//! This module implements the `RequestHandler` trait from `engine-ports`,
//! routing incoming `RequestPayload` messages to the appropriate services.
//!
//! # Architecture
//!
//! The handler follows hexagonal architecture:
//! - Inbound: `RequestHandler` trait (from engine-ports)
//! - Outbound: Repository ports, services
//! - Application: This handler orchestrates between them

use std::sync::Arc;
use async_trait::async_trait;
use chrono::Timelike;
use uuid::Uuid;

use wrldbldr_engine_ports::inbound::{BroadcastSink, RequestContext, RequestHandler};
use wrldbldr_engine_ports::outbound::{ObservationRepositoryPort, RegionRepositoryPort};
use wrldbldr_protocol::{
    EntityChangedData, EntityType, ErrorCode, RequestPayload, ResponseResult,
};

use crate::application::dto::{
    ActResponseDto, ChallengeResponseDto, CharacterResponseDto, ChainStatusResponseDto,
    ConnectionResponseDto, EventChainResponseDto, InteractionResponseDto, LocationResponseDto,
    NarrativeEventResponseDto, PlayerCharacterResponseDto, SceneResponseDto, SkillResponseDto,
    WorldResponseDto,
};
use crate::application::services::{
    WorldService, CharacterService, LocationService, SkillService,
    SceneService, InteractionService, ChallengeService, NarrativeEventService,
    EventChainService, PlayerCharacterService, RelationshipService,
    ActantialContextService, MoodService, StoryEventService,
};

// =============================================================================
// App Request Handler
// =============================================================================

/// Application-layer request handler
///
/// This handler receives `RequestPayload` from the WebSocket infrastructure,
/// routes to the appropriate service, and returns a `ResponseResult`.
pub struct AppRequestHandler {
    // Core services
    world_service: Arc<dyn WorldService>,
    character_service: Arc<dyn CharacterService>,
    location_service: Arc<dyn LocationService>,
    skill_service: Arc<dyn SkillService>,
    scene_service: Arc<dyn SceneService>,
    interaction_service: Arc<dyn InteractionService>,
    challenge_service: Arc<dyn ChallengeService>,
    narrative_event_service: Arc<dyn NarrativeEventService>,
    event_chain_service: Arc<dyn EventChainService>,
    player_character_service: Arc<dyn PlayerCharacterService>,
    relationship_service: Arc<dyn RelationshipService>,
    actantial_service: Arc<dyn ActantialContextService>,
    mood_service: Arc<dyn MoodService>,
    story_event_service: Arc<dyn StoryEventService>,

    // Repository ports (for simple CRUD that doesn't need a full service)
    observation_repo: Arc<dyn ObservationRepositoryPort>,
    region_repo: Arc<dyn RegionRepositoryPort>,

    // Broadcast sink for entity change notifications
    broadcast_sink: Option<Arc<dyn BroadcastSink>>,
}

impl AppRequestHandler {
    /// Create a new request handler with all service dependencies
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_service: Arc<dyn WorldService>,
        character_service: Arc<dyn CharacterService>,
        location_service: Arc<dyn LocationService>,
        skill_service: Arc<dyn SkillService>,
        scene_service: Arc<dyn SceneService>,
        interaction_service: Arc<dyn InteractionService>,
        challenge_service: Arc<dyn ChallengeService>,
        narrative_event_service: Arc<dyn NarrativeEventService>,
        event_chain_service: Arc<dyn EventChainService>,
        player_character_service: Arc<dyn PlayerCharacterService>,
        relationship_service: Arc<dyn RelationshipService>,
        actantial_service: Arc<dyn ActantialContextService>,
        mood_service: Arc<dyn MoodService>,
        story_event_service: Arc<dyn StoryEventService>,
        observation_repo: Arc<dyn ObservationRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
    ) -> Self {
        Self {
            world_service,
            character_service,
            location_service,
            skill_service,
            scene_service,
            interaction_service,
            challenge_service,
            narrative_event_service,
            event_chain_service,
            player_character_service,
            relationship_service,
            actantial_service,
            mood_service,
            story_event_service,
            observation_repo,
            region_repo,
            broadcast_sink: None,
        }
    }

    /// Set the broadcast sink for entity change notifications
    pub fn with_broadcast_sink(mut self, sink: Arc<dyn BroadcastSink>) -> Self {
        self.broadcast_sink = Some(sink);
        self
    }

    /// Broadcast an entity change to the world
    #[allow(dead_code)]
    async fn broadcast_change(&self, world_id: Uuid, change: EntityChangedData) {
        if let Some(sink) = &self.broadcast_sink {
            sink.broadcast_entity_change(world_id, change).await;
        }
    }

    // =========================================================================
    // ID Parsing Helpers
    // =========================================================================

    fn parse_uuid(id: &str, entity_name: &str) -> Result<Uuid, ResponseResult> {
        Uuid::parse_str(id).map_err(|_| {
            ResponseResult::error(
                ErrorCode::BadRequest,
                format!("Invalid {} ID: {}", entity_name, id),
            )
        })
    }

    fn parse_world_id(id: &str) -> Result<wrldbldr_domain::WorldId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "world")?;
        Ok(wrldbldr_domain::WorldId::from_uuid(uuid))
    }

    fn parse_character_id(id: &str) -> Result<wrldbldr_domain::CharacterId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "character")?;
        Ok(wrldbldr_domain::CharacterId::from_uuid(uuid))
    }

    fn parse_location_id(id: &str) -> Result<wrldbldr_domain::LocationId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "location")?;
        Ok(wrldbldr_domain::LocationId::from_uuid(uuid))
    }

    fn parse_skill_id(id: &str) -> Result<wrldbldr_domain::SkillId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "skill")?;
        Ok(wrldbldr_domain::SkillId::from_uuid(uuid))
    }

    fn parse_scene_id(id: &str) -> Result<wrldbldr_domain::SceneId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "scene")?;
        Ok(wrldbldr_domain::SceneId::from_uuid(uuid))
    }

    fn parse_act_id(id: &str) -> Result<wrldbldr_domain::ActId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "act")?;
        Ok(wrldbldr_domain::ActId::from_uuid(uuid))
    }

    fn parse_challenge_id(id: &str) -> Result<wrldbldr_domain::ChallengeId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "challenge")?;
        Ok(wrldbldr_domain::ChallengeId::from_uuid(uuid))
    }

    fn parse_narrative_event_id(id: &str) -> Result<wrldbldr_domain::NarrativeEventId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "narrative_event")?;
        Ok(wrldbldr_domain::NarrativeEventId::from_uuid(uuid))
    }

    fn parse_event_chain_id(id: &str) -> Result<wrldbldr_domain::EventChainId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "event_chain")?;
        Ok(wrldbldr_domain::EventChainId::from_uuid(uuid))
    }

    fn parse_player_character_id(id: &str) -> Result<wrldbldr_domain::PlayerCharacterId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "player_character")?;
        Ok(wrldbldr_domain::PlayerCharacterId::from_uuid(uuid))
    }

    fn parse_interaction_id(id: &str) -> Result<wrldbldr_domain::InteractionId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "interaction")?;
        Ok(wrldbldr_domain::InteractionId::from_uuid(uuid))
    }

    #[allow(dead_code)]
    fn parse_goal_id(id: &str) -> Result<wrldbldr_domain::GoalId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "goal")?;
        Ok(wrldbldr_domain::GoalId::from_uuid(uuid))
    }

    #[allow(dead_code)]
    fn parse_want_id(id: &str) -> Result<wrldbldr_domain::WantId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "want")?;
        Ok(wrldbldr_domain::WantId::from_uuid(uuid))
    }

    #[allow(dead_code)]
    fn parse_region_id(id: &str) -> Result<wrldbldr_domain::RegionId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "region")?;
        Ok(wrldbldr_domain::RegionId::from_uuid(uuid))
    }

    #[allow(dead_code)]
    fn parse_relationship_id(id: &str) -> Result<wrldbldr_domain::RelationshipId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "relationship")?;
        Ok(wrldbldr_domain::RelationshipId::from_uuid(uuid))
    }

    #[allow(dead_code)]
    fn parse_story_event_id(id: &str) -> Result<wrldbldr_domain::StoryEventId, ResponseResult> {
        let uuid = Self::parse_uuid(id, "story_event")?;
        Ok(wrldbldr_domain::StoryEventId::from_uuid(uuid))
    }
}

/// Parse a string archetype name into a CampbellArchetype
fn parse_archetype(s: &str) -> wrldbldr_domain::value_objects::CampbellArchetype {
    use wrldbldr_domain::value_objects::CampbellArchetype;
    match s.to_lowercase().as_str() {
        "hero" => CampbellArchetype::Hero,
        "mentor" => CampbellArchetype::Mentor,
        "herald" => CampbellArchetype::Herald,
        "shadow" => CampbellArchetype::Shadow,
        "shapeshifter" => CampbellArchetype::Shapeshifter,
        "threshold_guardian" | "thresholdguardian" | "threshold guardian" => CampbellArchetype::ThresholdGuardian,
        "trickster" => CampbellArchetype::Trickster,
        "ally" => CampbellArchetype::Ally,
        _ => CampbellArchetype::Ally, // Default to Ally for unknown archetypes
    }
}

/// Parse a difficulty string into a Difficulty enum
fn parse_difficulty(s: &str) -> wrldbldr_domain::entities::Difficulty {
    use wrldbldr_domain::entities::Difficulty;
    // Check for DC format first (e.g., "DC 15", "dc15", "15")
    let s_lower = s.to_lowercase();
    if s_lower.starts_with("dc") {
        if let Ok(dc) = s_lower.trim_start_matches("dc").trim().parse::<u32>() {
            return Difficulty::DC(dc);
        }
    }
    // Try to parse as plain number (assume DC)
    if let Ok(dc) = s.parse::<u32>() {
        return Difficulty::DC(dc);
    }
    // Try percentage format
    if s_lower.ends_with('%') {
        if let Ok(pct) = s_lower.trim_end_matches('%').trim().parse::<u32>() {
            return Difficulty::Percentage(pct);
        }
    }
    // Match descriptive difficulties
    match s_lower.as_str() {
        "easy" => Difficulty::d20_easy(),
        "medium" | "moderate" => Difficulty::d20_medium(),
        "hard" => Difficulty::d20_hard(),
        "very hard" | "veryhard" | "very_hard" => Difficulty::d20_very_hard(),
        "opposed" => Difficulty::Opposed,
        _ => Difficulty::Custom(s.to_string()),
    }
}

/// Parse a relationship type string into a RelationshipType enum
fn parse_relationship_type(s: &str) -> wrldbldr_domain::value_objects::RelationshipType {
    use wrldbldr_domain::value_objects::{FamilyRelation, RelationshipType};
    match s.to_lowercase().as_str() {
        "romantic" => RelationshipType::Romantic,
        "professional" => RelationshipType::Professional,
        "rivalry" => RelationshipType::Rivalry,
        "friendship" | "friend" => RelationshipType::Friendship,
        "mentorship" | "mentor" => RelationshipType::Mentorship,
        "enmity" | "enemy" => RelationshipType::Enmity,
        // Family relations
        "parent" => RelationshipType::Family(FamilyRelation::Parent),
        "child" => RelationshipType::Family(FamilyRelation::Child),
        "sibling" => RelationshipType::Family(FamilyRelation::Sibling),
        "spouse" => RelationshipType::Family(FamilyRelation::Spouse),
        "grandparent" => RelationshipType::Family(FamilyRelation::Grandparent),
        "grandchild" => RelationshipType::Family(FamilyRelation::Grandchild),
        "aunt" | "uncle" | "auntuncle" => RelationshipType::Family(FamilyRelation::AuntUncle),
        "niece" | "nephew" | "niecenephew" => RelationshipType::Family(FamilyRelation::NieceNephew),
        "cousin" => RelationshipType::Family(FamilyRelation::Cousin),
        // Default to custom
        _ => RelationshipType::Custom(s.to_string()),
    }
}

/// Parse a mood level string into a MoodLevel enum
fn parse_mood_level(s: &str) -> wrldbldr_domain::value_objects::MoodLevel {
    use wrldbldr_domain::value_objects::MoodLevel;
    match s.to_lowercase().as_str() {
        "friendly" => MoodLevel::Friendly,
        "neutral" => MoodLevel::Neutral,
        "suspicious" => MoodLevel::Suspicious,
        "hostile" => MoodLevel::Hostile,
        "afraid" => MoodLevel::Afraid,
        "grateful" => MoodLevel::Grateful,
        "annoyed" => MoodLevel::Annoyed,
        "curious" => MoodLevel::Curious,
        "melancholic" => MoodLevel::Melancholic,
        _ => MoodLevel::Neutral, // Default to neutral
    }
}

/// Parse a relationship level string into a RelationshipLevel enum
fn parse_relationship_level(s: &str) -> wrldbldr_domain::value_objects::RelationshipLevel {
    use wrldbldr_domain::value_objects::RelationshipLevel;
    match s.to_lowercase().as_str() {
        "ally" => RelationshipLevel::Ally,
        "friend" => RelationshipLevel::Friend,
        "acquaintance" => RelationshipLevel::Acquaintance,
        "stranger" => RelationshipLevel::Stranger,
        "rival" => RelationshipLevel::Rival,
        "enemy" => RelationshipLevel::Enemy,
        "nemesis" => RelationshipLevel::Nemesis,
        _ => RelationshipLevel::Stranger, // Default to stranger
    }
}

/// Convert ActorTypeData to ActorTargetType
fn convert_actor_type(data: wrldbldr_protocol::ActorTypeData) -> crate::application::services::ActorTargetType {
    match data {
        wrldbldr_protocol::ActorTypeData::Npc => crate::application::services::ActorTargetType::Npc,
        wrldbldr_protocol::ActorTypeData::Pc => crate::application::services::ActorTargetType::Pc,
    }
}

/// Convert ActantialRoleData to ActantialRole
fn convert_actantial_role(data: wrldbldr_protocol::ActantialRoleData) -> wrldbldr_domain::entities::ActantialRole {
    match data {
        wrldbldr_protocol::ActantialRoleData::Helper => wrldbldr_domain::entities::ActantialRole::Helper,
        wrldbldr_protocol::ActantialRoleData::Opponent => wrldbldr_domain::entities::ActantialRole::Opponent,
        wrldbldr_protocol::ActantialRoleData::Sender => wrldbldr_domain::entities::ActantialRole::Sender,
        wrldbldr_protocol::ActantialRoleData::Receiver => wrldbldr_domain::entities::ActantialRole::Receiver,
    }
}

/// Convert WantTargetTypeData to target type string
fn convert_want_target_type(data: wrldbldr_protocol::WantTargetTypeData) -> &'static str {
    match data {
        wrldbldr_protocol::WantTargetTypeData::Character => "Character",
        wrldbldr_protocol::WantTargetTypeData::Item => "Item",
        wrldbldr_protocol::WantTargetTypeData::Goal => "Goal",
    }
}

/// Convert WantVisibilityData to domain WantVisibility
fn convert_want_visibility(data: wrldbldr_protocol::WantVisibilityData) -> wrldbldr_domain::entities::WantVisibility {
    match data {
        wrldbldr_protocol::WantVisibilityData::Known => wrldbldr_domain::entities::WantVisibility::Known,
        wrldbldr_protocol::WantVisibilityData::Suspected => wrldbldr_domain::entities::WantVisibility::Suspected,
        wrldbldr_protocol::WantVisibilityData::Hidden => wrldbldr_domain::entities::WantVisibility::Hidden,
    }
}

#[async_trait]
impl RequestHandler for AppRequestHandler {
    async fn handle(&self, payload: RequestPayload, ctx: RequestContext) -> ResponseResult {
        // Log the request for debugging
        tracing::debug!(
            connection_id = %ctx.connection_id,
            user_id = %ctx.user_id,
            world_id = ?ctx.world_id,
            is_dm = ctx.is_dm,
            payload_type = ?std::mem::discriminant(&payload),
            "Handling WebSocket request"
        );

        match payload {
            // =================================================================
            // World Operations
            // =================================================================
            RequestPayload::ListWorlds => {
                match self.world_service.list_worlds().await {
                    Ok(worlds) => {
                        let dtos: Vec<WorldResponseDto> = worlds.into_iter().map(|w| w.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetWorld { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.world_service.get_world(id).await {
                    Ok(Some(world)) => {
                        let dto: WorldResponseDto = world.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::ExportWorld { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.world_service.export_world_snapshot(id).await {
                    Ok(snapshot) => ResponseResult::success(snapshot),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateWorld { data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let request = crate::application::services::CreateWorldRequest {
                    name: data.name,
                    description: data.description.unwrap_or_default(),
                    rule_system: None,
                };
                match self.world_service.create_world(request).await {
                    Ok(world) => {
                        let dto: WorldResponseDto = world.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateWorld { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdateWorldRequest {
                    name: data.name,
                    description: data.description,
                    rule_system: None,
                };
                match self.world_service.update_world(id, request).await {
                    Ok(world) => {
                        let dto: WorldResponseDto = world.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteWorld { world_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.world_service.delete_world(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Character Operations
            // =================================================================
            RequestPayload::ListCharacters { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.character_service.list_characters(id).await {
                    Ok(characters) => {
                        let dtos: Vec<CharacterResponseDto> = characters.into_iter().map(|c| c.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetCharacter { character_id } => {
                let id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.character_service.get_character(id).await {
                    Ok(Some(character)) => {
                        let dto: CharacterResponseDto = character.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteCharacter { character_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.character_service.delete_character(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateCharacter { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Parse archetype (default to Ally if not specified)
                let archetype = data.archetype
                    .as_deref()
                    .map(|s| parse_archetype(s))
                    .unwrap_or(wrldbldr_domain::value_objects::CampbellArchetype::Ally);
                
                let request = crate::application::services::CreateCharacterRequest {
                    world_id: id,
                    name: data.name,
                    description: data.description,
                    archetype,
                    sprite_asset: data.sprite_asset,
                    portrait_asset: data.portrait_asset,
                    stats: None,
                    initial_wants: vec![],
                };
                match self.character_service.create_character(request).await {
                    Ok(character) => {
                        let dto: CharacterResponseDto = character.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateCharacter { character_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdateCharacterRequest {
                    name: data.name,
                    description: data.description,
                    sprite_asset: data.sprite_asset,
                    portrait_asset: data.portrait_asset,
                    stats: None,
                    is_alive: data.is_alive,
                    is_active: data.is_active,
                };
                match self.character_service.update_character(id, request).await {
                    Ok(character) => {
                        let dto: CharacterResponseDto = character.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::ChangeArchetype { character_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let archetype = parse_archetype(&data.new_archetype);
                let request = crate::application::services::ChangeArchetypeRequest {
                    new_archetype: archetype,
                    reason: data.reason,
                };
                match self.character_service.change_archetype(id, request).await {
                    Ok(character) => {
                        let dto: CharacterResponseDto = character.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetCharacterInventory { character_id: _ } => {
                // TODO: Need ItemService to be added to handler
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "GetCharacterInventory pending ItemService integration",
                )
            }

            // =================================================================
            // Location Operations
            // =================================================================
            RequestPayload::ListLocations { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.location_service.list_locations(id).await {
                    Ok(locations) => {
                        let dtos: Vec<LocationResponseDto> = locations.into_iter().map(|l| l.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetLocation { location_id } => {
                let id = match Self::parse_location_id(&location_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.location_service.get_location(id).await {
                    Ok(Some(location)) => {
                        let dto: LocationResponseDto = location.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Location not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteLocation { location_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_location_id(&location_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.location_service.delete_location(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateLocation { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::CreateLocationRequest {
                    world_id: id,
                    name: data.name,
                    description: data.description,
                    location_type: wrldbldr_domain::entities::LocationType::Interior,
                    parent_id: None,
                    backdrop_asset: None,
                    atmosphere: data.setting,
                    presence_cache_ttl_hours: None,
                    use_llm_presence: None,
                };
                match self.location_service.create_location(request).await {
                    Ok(location) => {
                        let dto: LocationResponseDto = location.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateLocation { location_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_location_id(&location_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdateLocationRequest {
                    name: data.name,
                    description: data.description,
                    location_type: None,
                    backdrop_asset: None,
                    atmosphere: data.setting.map(Some),
                    presence_cache_ttl_hours: None,
                    use_llm_presence: None,
                };
                match self.location_service.update_location(id, request).await {
                    Ok(location) => {
                        let dto: LocationResponseDto = location.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetLocationConnections { location_id } => {
                let id = match Self::parse_location_id(&location_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.location_service.get_connections(id).await {
                    Ok(connections) => {
                        let dtos: Vec<ConnectionResponseDto> = connections.into_iter().map(|c| c.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateLocationConnection { data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let from_id = match Self::parse_location_id(&data.from_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let to_id = match Self::parse_location_id(&data.to_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::CreateConnectionRequest {
                    from_location: from_id,
                    to_location: to_id,
                    connection_type: "path".to_string(), // Default connection type
                    description: None,
                    bidirectional: data.bidirectional.unwrap_or(true),
                    travel_time: 1,
                    is_locked: false,
                    lock_description: None,
                };
                match self.location_service.create_connection(request).await {
                    Ok(()) => ResponseResult::success(serde_json::json!({
                        "from_id": data.from_id,
                        "to_id": data.to_id,
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteLocationConnection { from_id, to_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let fid = match Self::parse_location_id(&from_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let tid = match Self::parse_location_id(&to_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.location_service.delete_connection(fid, tid).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Region Operations
            // =================================================================
            // Note: Regions are sub-entities of Locations. Full CRUD requires
            // RegionService which doesn't exist yet. Currently using LocationService
            // for what's available.

            RequestPayload::ListRegions { location_id } => {
                let id = match Self::parse_location_id(&location_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Use location service to get location with regions
                match self.location_service.get_location_with_connections(id).await {
                    Ok(Some(loc_with_conn)) => {
                        let dtos: Vec<serde_json::Value> = loc_with_conn.regions.iter().map(|r| {
                            serde_json::json!({
                                "id": r.id.to_string(),
                                "name": r.name,
                                "description": r.description,
                                "is_spawn_point": r.is_spawn_point,
                            })
                        }).collect();
                        ResponseResult::success(dtos)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Location not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetRegion { region_id } => {
                let id = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.region_repo.get(id).await {
                    Ok(Some(region)) => {
                        let dto = serde_json::json!({
                            "id": region.id.to_string(),
                            "location_id": region.location_id.to_string(),
                            "name": region.name,
                            "description": region.description,
                            "backdrop_asset": region.backdrop_asset,
                            "atmosphere": region.atmosphere,
                            "is_spawn_point": region.is_spawn_point,
                            "order": region.order,
                        });
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Region not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateRegion { location_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let lid = match Self::parse_location_id(&location_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Create region entity
                let mut region = wrldbldr_domain::entities::Region::new(lid, data.name)
                    .with_description(data.description.unwrap_or_default());
                // Set spawn point if specified
                if data.is_spawn_point.unwrap_or(false) {
                    region.is_spawn_point = true;
                }
                match self.location_service.add_region(lid, region.clone()).await {
                    Ok(()) => ResponseResult::success(serde_json::json!({
                        "id": region.id.to_string(),
                        "name": region.name,
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateRegion { region_id, data: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _id = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "UpdateRegion requires RegionService - pending implementation",
                )
            }

            RequestPayload::DeleteRegion { region_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _id = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "DeleteRegion requires RegionService - pending implementation",
                )
            }

            RequestPayload::GetRegionConnections { region_id } => {
                let _id = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "GetRegionConnections requires RegionService - pending implementation",
                )
            }

            RequestPayload::CreateRegionConnection { from_id: _, to_id: _, data: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "CreateRegionConnection requires RegionService - pending implementation",
                )
            }

            RequestPayload::DeleteRegionConnection { from_id: _, to_id: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "DeleteRegionConnection requires RegionService - pending implementation",
                )
            }

            RequestPayload::UnlockRegionConnection { from_id: _, to_id: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "UnlockRegionConnection requires RegionService - pending implementation",
                )
            }

            RequestPayload::GetRegionExits { region_id } => {
                let _id = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "GetRegionExits requires RegionService - pending implementation",
                )
            }

            RequestPayload::CreateRegionExit { region_id: _, location_id: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "CreateRegionExit requires RegionService - pending implementation",
                )
            }

            RequestPayload::DeleteRegionExit { region_id: _, location_id: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "DeleteRegionExit requires RegionService - pending implementation",
                )
            }

            RequestPayload::ListSpawnPoints { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.region_repo.list_spawn_points(id).await {
                    Ok(regions) => {
                        let dtos: Vec<serde_json::Value> = regions.iter().map(|r| {
                            serde_json::json!({
                                "id": r.id.to_string(),
                                "location_id": r.location_id.to_string(),
                                "name": r.name,
                                "description": r.description,
                                "is_spawn_point": r.is_spawn_point,
                            })
                        }).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Skill Operations
            // =================================================================
            RequestPayload::ListSkills { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.skill_service.list_skills(id).await {
                    Ok(skills) => {
                        let dtos: Vec<SkillResponseDto> = skills.into_iter().map(|s| s.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetSkill { skill_id } => {
                let id = match Self::parse_skill_id(&skill_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.skill_service.get_skill(id).await {
                    Ok(Some(skill)) => {
                        let dto: SkillResponseDto = skill.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Skill not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteSkill { skill_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_skill_id(&skill_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.skill_service.delete_skill(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateSkill { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::CreateSkillRequest {
                    name: data.name,
                    description: data.description.unwrap_or_default(),
                    category: wrldbldr_domain::entities::SkillCategory::Physical, // Default
                    base_attribute: data.attribute,
                };
                match self.skill_service.create_skill(id, request).await {
                    Ok(skill) => {
                        let dto: SkillResponseDto = skill.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateSkill { skill_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_skill_id(&skill_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdateSkillRequest {
                    name: data.name,
                    description: data.description,
                    category: None, // No category in protocol data
                    base_attribute: data.attribute,
                    is_hidden: None,
                    order: None,
                };
                match self.skill_service.update_skill(id, request).await {
                    Ok(skill) => {
                        let dto: SkillResponseDto = skill.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Scene Operations
            // =================================================================
            RequestPayload::ListScenes { act_id } => {
                let id = match Self::parse_act_id(&act_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.scene_service.list_scenes_by_act(id).await {
                    Ok(scenes) => {
                        let dtos: Vec<SceneResponseDto> = scenes.into_iter().map(|s| s.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetScene { scene_id } => {
                let id = match Self::parse_scene_id(&scene_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.scene_service.get_scene(id).await {
                    Ok(Some(scene)) => {
                        let dto: SceneResponseDto = scene.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Scene not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteScene { scene_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_scene_id(&scene_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.scene_service.delete_scene(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateScene { act_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let aid = match Self::parse_act_id(&act_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Parse location_id if provided
                let location_id = match data.location_id {
                    Some(ref lid) => match Self::parse_location_id(lid) {
                        Ok(id) => id,
                        Err(e) => return e,
                    },
                    None => {
                        return ResponseResult::error(
                            ErrorCode::BadRequest,
                            "location_id is required for creating a scene",
                        );
                    }
                };
                let request = crate::application::services::CreateSceneRequest {
                    act_id: aid,
                    name: data.name,
                    location_id,
                    time_context: None,
                    backdrop_override: None,
                    featured_characters: vec![],
                    directorial_notes: data.description,
                    entry_conditions: vec![],
                    order: 0,
                };
                match self.scene_service.create_scene(request).await {
                    Ok(scene) => {
                        let dto: SceneResponseDto = scene.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateScene { scene_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_scene_id(&scene_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdateSceneRequest {
                    name: data.name,
                    time_context: None,
                    backdrop_override: None,
                    entry_conditions: None,
                    order: None,
                };
                match self.scene_service.update_scene(id, request).await {
                    Ok(scene) => {
                        let dto: SceneResponseDto = scene.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Act Operations
            // =================================================================
            RequestPayload::ListActs { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.world_service.get_acts(id).await {
                    Ok(acts) => {
                        let dtos: Vec<ActResponseDto> = acts.into_iter().map(|a| a.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateAct { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::CreateActRequest {
                    name: data.name,
                    stage: wrldbldr_domain::entities::MonomythStage::OrdinaryWorld, // Default stage
                    description: data.description,
                    order: data.order.unwrap_or(0),
                };
                match self.world_service.create_act(id, request).await {
                    Ok(act) => {
                        let dto: ActResponseDto = act.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Interaction Operations
            // =================================================================
            RequestPayload::ListInteractions { scene_id } => {
                let id = match Self::parse_scene_id(&scene_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.interaction_service.list_interactions(id).await {
                    Ok(interactions) => {
                        let dtos: Vec<InteractionResponseDto> = interactions.into_iter().map(|i| i.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetInteraction { interaction_id } => {
                let id = match Self::parse_interaction_id(&interaction_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.interaction_service.get_interaction(id).await {
                    Ok(Some(interaction)) => {
                        let dto: InteractionResponseDto = interaction.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteInteraction { interaction_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_interaction_id(&interaction_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.interaction_service.delete_interaction(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetInteractionAvailability { interaction_id, available } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_interaction_id(&interaction_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.interaction_service.set_interaction_availability(id, available).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateInteraction { scene_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let sid = match Self::parse_scene_id(&scene_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Create a new InteractionTemplate entity
                let interaction = wrldbldr_domain::entities::InteractionTemplate::new(
                    sid,
                    data.name,
                    wrldbldr_domain::entities::InteractionType::Dialogue, // Default type
                    wrldbldr_domain::entities::InteractionTarget::None,
                )
                .with_prompt_hints(data.description.unwrap_or_default());
                
                // Set availability if specified
                let interaction = if data.available == Some(false) {
                    interaction.disabled()
                } else {
                    interaction
                };

                match self.interaction_service.create_interaction(&interaction).await {
                    Ok(()) => {
                        let dto: InteractionResponseDto = interaction.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateInteraction { interaction_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_interaction_id(&interaction_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Fetch existing interaction first
                let existing = match self.interaction_service.get_interaction(id).await {
                    Ok(Some(i)) => i,
                    Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
                    Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                };
                // Apply updates
                let mut updated = existing;
                if let Some(name) = data.name {
                    updated.name = name;
                }
                if let Some(description) = data.description {
                    updated.prompt_hints = description;
                }
                if let Some(available) = data.available {
                    updated.is_available = available;
                }
                match self.interaction_service.update_interaction(&updated).await {
                    Ok(()) => {
                        let dto: InteractionResponseDto = updated.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Challenge Operations
            // =================================================================
            RequestPayload::ListChallenges { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.challenge_service.list_challenges(id).await {
                    Ok(challenges) => {
                        let dtos: Vec<ChallengeResponseDto> = challenges
                            .into_iter()
                            .map(ChallengeResponseDto::from_challenge_minimal)
                            .collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetChallenge { challenge_id } => {
                let id = match Self::parse_challenge_id(&challenge_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.challenge_service.get_challenge(id).await {
                    Ok(Some(challenge)) => {
                        let dto = ChallengeResponseDto::from_challenge_minimal(challenge);
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Challenge not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteChallenge { challenge_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_challenge_id(&challenge_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.challenge_service.delete_challenge(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetChallengeActive { challenge_id, active } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_challenge_id(&challenge_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.challenge_service.set_active(id, active).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetChallengeFavorite { challenge_id, favorite: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_challenge_id(&challenge_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.challenge_service.toggle_favorite(id).await {
                    Ok(is_favorite) => ResponseResult::success(serde_json::json!({ "is_favorite": is_favorite })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateChallenge { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let wid = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Parse difficulty from string
                let difficulty = parse_difficulty(&data.difficulty);
                // Create the challenge entity
                let challenge = wrldbldr_domain::entities::Challenge::new(wid, data.name, difficulty)
                    .with_description(data.description.unwrap_or_default());
                // Set outcomes if provided
                let challenge = if data.success_outcome.is_some() || data.failure_outcome.is_some() {
                    let outcomes = wrldbldr_domain::entities::ChallengeOutcomes::simple(
                        data.success_outcome.unwrap_or_default(),
                        data.failure_outcome.unwrap_or_default(),
                    );
                    challenge.with_outcomes(outcomes)
                } else {
                    challenge
                };
                match self.challenge_service.create_challenge(challenge.clone()).await {
                    Ok(created) => {
                        // If skill_id was provided, set the required skill relationship
                        if !data.skill_id.is_empty() {
                            if let Ok(skill_id) = Self::parse_skill_id(&data.skill_id) {
                                let _ = self.challenge_service.set_required_skill(created.id, skill_id).await;
                            }
                        }
                        let dto = ChallengeResponseDto::from_challenge_minimal(created);
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateChallenge { challenge_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_challenge_id(&challenge_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Fetch existing challenge first
                let existing = match self.challenge_service.get_challenge(id).await {
                    Ok(Some(c)) => c,
                    Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Challenge not found"),
                    Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                };
                // Apply updates
                let mut updated = existing;
                if let Some(name) = data.name {
                    updated.name = name;
                }
                if let Some(description) = data.description {
                    updated.description = description;
                }
                if let Some(ref difficulty_str) = data.difficulty {
                    updated.difficulty = parse_difficulty(difficulty_str);
                }
                if data.success_outcome.is_some() || data.failure_outcome.is_some() {
                    let outcomes = wrldbldr_domain::entities::ChallengeOutcomes::simple(
                        data.success_outcome.unwrap_or_else(|| updated.outcomes.success.description.clone()),
                        data.failure_outcome.unwrap_or_else(|| updated.outcomes.failure.description.clone()),
                    );
                    updated.outcomes = outcomes;
                }
                match self.challenge_service.update_challenge(updated.clone()).await {
                    Ok(result) => {
                        // Update skill relationship if provided
                        if let Some(ref skill_id_str) = data.skill_id {
                            if let Ok(skill_id) = Self::parse_skill_id(skill_id_str) {
                                let _ = self.challenge_service.set_required_skill(result.id, skill_id).await;
                            }
                        }
                        let dto = ChallengeResponseDto::from_challenge_minimal(result);
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Narrative Event Operations
            // =================================================================
            RequestPayload::ListNarrativeEvents { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.list_by_world(id).await {
                    Ok(events) => {
                        let dtos: Vec<NarrativeEventResponseDto> = events.into_iter().map(|e| e.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetNarrativeEvent { event_id } => {
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.get(id).await {
                    Ok(Some(event)) => {
                        let dto: NarrativeEventResponseDto = event.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Narrative event not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteNarrativeEvent { event_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.delete(id).await {
                    Ok(_) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetNarrativeEventActive { event_id, active } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.set_active(id, active).await {
                    Ok(_) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetNarrativeEventFavorite { event_id, favorite: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.toggle_favorite(id).await {
                    Ok(is_favorite) => ResponseResult::success(serde_json::json!({ "is_favorite": is_favorite })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::TriggerNarrativeEvent { event_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.mark_triggered(id, None).await {
                    Ok(_) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::ResetNarrativeEvent { event_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.narrative_event_service.reset_triggered(id).await {
                    Ok(_) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateNarrativeEvent { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let wid = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Create the narrative event entity
                let mut event = wrldbldr_domain::entities::NarrativeEvent::new(wid, data.name);
                event.description = data.description.unwrap_or_default();
                match self.narrative_event_service.create(event).await {
                    Ok(created) => {
                        let dto: NarrativeEventResponseDto = created.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateNarrativeEvent { event_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Fetch existing event first
                let existing = match self.narrative_event_service.get(id).await {
                    Ok(Some(e)) => e,
                    Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Narrative event not found"),
                    Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                };
                // Apply updates
                let mut updated = existing;
                if let Some(name) = data.name {
                    updated.name = name;
                }
                if let Some(description) = data.description {
                    updated.description = description;
                }
                match self.narrative_event_service.update(updated).await {
                    Ok(result) => {
                        let dto: NarrativeEventResponseDto = result.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Event Chain Operations
            // =================================================================
            RequestPayload::ListEventChains { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.list_event_chains(id).await {
                    Ok(chains) => {
                        let dtos: Vec<EventChainResponseDto> = chains.into_iter().map(|c| c.into()).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetEventChain { chain_id } => {
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.get_event_chain(id).await {
                    Ok(Some(chain)) => {
                        let dto: EventChainResponseDto = chain.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Event chain not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteEventChain { chain_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.delete_event_chain(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateEventChain { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let wid = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Create the event chain entity
                let mut chain = wrldbldr_domain::entities::EventChain::new(wid, data.name);
                chain.description = data.description.unwrap_or_default();
                match self.event_chain_service.create_event_chain(chain).await {
                    Ok(created) => {
                        let dto: EventChainResponseDto = created.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateEventChain { chain_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Fetch existing chain first
                let existing = match self.event_chain_service.get_event_chain(id).await {
                    Ok(Some(c)) => c,
                    Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Event chain not found"),
                    Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                };
                // Apply updates
                let mut updated = existing;
                if let Some(name) = data.name {
                    updated.name = name;
                }
                if let Some(description) = data.description {
                    updated.description = description;
                }
                match self.event_chain_service.update_event_chain(updated).await {
                    Ok(result) => {
                        let dto: EventChainResponseDto = result.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetEventChainActive { chain_id, active } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.set_active(id, active).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetEventChainFavorite { chain_id, favorite: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.toggle_favorite(id).await {
                    Ok(is_favorite) => ResponseResult::success(serde_json::json!({ "is_favorite": is_favorite })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::AddEventToChain { chain_id, event_id, position: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let cid = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let eid = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.add_event_to_chain(cid, eid).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::RemoveEventFromChain { chain_id, event_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let cid = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let eid = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.remove_event_from_chain(cid, eid).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CompleteChainEvent { chain_id, event_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let cid = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let eid = match Self::parse_narrative_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.complete_event(cid, eid).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::ResetEventChain { chain_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.reset_chain(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetEventChainStatus { chain_id } => {
                let id = match Self::parse_event_chain_id(&chain_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.event_chain_service.get_status(id).await {
                    Ok(Some(status)) => {
                        let dto: ChainStatusResponseDto = status.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Event chain not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Player Character Operations
            // =================================================================
            RequestPayload::ListPlayerCharacters { world_id: _ } => {
                // Player character service uses session_id, not world_id
                // This will need to be updated when sessions are removed
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "ListPlayerCharacters needs world-based query - pending session removal",
                )
            }

            RequestPayload::GetPlayerCharacter { pc_id } => {
                let id = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.player_character_service.get_pc(id).await {
                    Ok(Some(pc)) => {
                        let dto: PlayerCharacterResponseDto = pc.into();
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Player character not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeletePlayerCharacter { pc_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.player_character_service.delete_pc(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreatePlayerCharacter { world_id, data } => {
                let wid = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // For now, we need a starting location - this is a limitation
                // In a real implementation, we'd need to pick a spawn point or default location
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "CreatePlayerCharacter requires starting_location_id - use session join flow instead",
                )
            }

            RequestPayload::UpdatePlayerCharacter { pc_id, data } => {
                let id = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdatePlayerCharacterRequest {
                    name: data.name,
                    description: None,
                    sheet_data: None, // TODO: Parse sheet_data from protocol
                    sprite_asset: None,
                    portrait_asset: None,
                };
                match self.player_character_service.update_pc(id, request).await {
                    Ok(pc) => {
                        let dto: PlayerCharacterResponseDto = pc.into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdatePlayerCharacterLocation { pc_id, region_id } => {
                let pid = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Note: The service uses LocationId, but protocol passes region_id
                // For now, we'll try to parse as LocationId and update
                let lid = match Self::parse_location_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.player_character_service.update_pc_location(pid, lid).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Relationship Operations
            // =================================================================
            RequestPayload::GetSocialNetwork { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.relationship_service.get_social_network(id).await {
                    Ok(network) => ResponseResult::success(network),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteRelationship { relationship_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_relationship_id(&relationship_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.relationship_service.delete_relationship(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateRelationship { data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let from_id = match Self::parse_character_id(&data.from_character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let to_id = match Self::parse_character_id(&data.to_character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let relationship_type = parse_relationship_type(&data.relationship_type);
                let relationship = wrldbldr_domain::value_objects::Relationship::new(
                    from_id,
                    to_id,
                    relationship_type,
                );
                match self.relationship_service.create_relationship(&relationship).await {
                    Ok(()) => ResponseResult::success(serde_json::json!({
                        "id": relationship.id.to_string(),
                        "from_character_id": data.from_character_id,
                        "to_character_id": data.to_character_id,
                        "relationship_type": data.relationship_type,
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Actantial Context Operations
            // =================================================================
            RequestPayload::GetActantialContext { character_id } => {
                let id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.get_context(id).await {
                    Ok(context) => ResponseResult::success(context),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::AddActantialView { character_id, want_id, target_id, target_type, role, reason } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let cid = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let wid = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let target_type_converted = convert_actor_type(target_type);
                let role_converted = convert_actantial_role(role);
                match self.actantial_service.add_actantial_view(
                    cid, wid, &target_id, target_type_converted, role_converted, reason
                ).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::RemoveActantialView { character_id, want_id, target_id, target_type, role } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let cid = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let wid = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let target_type_converted = convert_actor_type(target_type);
                let role_converted = convert_actantial_role(role);
                match self.actantial_service.remove_actantial_view(
                    cid, wid, &target_id, target_type_converted, role_converted
                ).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // NPC Mood Operations
            // =================================================================
            RequestPayload::GetNpcMoods { pc_id } => {
                let id = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.mood_service.get_all_relationships(id).await {
                    Ok(moods) => {
                        let dtos: Vec<wrldbldr_domain::value_objects::NpcMoodStateDto> = moods
                            .iter()
                            .map(|m| m.into())
                            .collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetNpcMood { npc_id, pc_id, mood, reason } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let pid = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let mood_level = parse_mood_level(&mood);
                match self.mood_service.set_mood(nid, pid, mood_level, reason).await {
                    Ok(state) => {
                        let dto: wrldbldr_domain::value_objects::NpcMoodStateDto = (&state).into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetNpcRelationship { npc_id, pc_id, relationship } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let pid = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let rel_level = parse_relationship_level(&relationship);
                match self.mood_service.set_relationship(nid, pid, rel_level).await {
                    Ok(state) => {
                        let dto: wrldbldr_domain::value_objects::NpcMoodStateDto = (&state).into();
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Story Event Operations
            // =================================================================

            RequestPayload::ListStoryEvents { world_id, page, page_size } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let page = page.unwrap_or(0);
                let page_size = page_size.unwrap_or(50);
                match self.story_event_service.list_by_world_paginated(id, page, page_size).await {
                    Ok(events) => {
                        let dtos: Vec<serde_json::Value> = events.iter().map(|e| {
                            serde_json::json!({
                                "id": e.id.to_string(),
                                "world_id": e.world_id.to_string(),
                                "event_type": format!("{:?}", e.event_type),
                                "summary": e.summary,
                                "timestamp": e.timestamp.to_rfc3339(),
                                "game_time": e.game_time,
                                "is_hidden": e.is_hidden,
                            })
                        }).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetStoryEvent { event_id } => {
                let id = match Self::parse_story_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.story_event_service.get_event(id).await {
                    Ok(Some(event)) => {
                        let dto = serde_json::json!({
                            "id": event.id.to_string(),
                            "world_id": event.world_id.to_string(),
                            "event_type": format!("{:?}", event.event_type),
                            "summary": event.summary,
                            "timestamp": event.timestamp.to_rfc3339(),
                            "game_time": event.game_time,
                            "is_hidden": event.is_hidden,
                        });
                        ResponseResult::success(dto)
                    }
                    Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Story event not found"),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateStoryEvent { event_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_story_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Update summary if provided
                if let Some(summary) = data.summary {
                    if let Err(e) = self.story_event_service.update_summary(id, &summary).await {
                        return ResponseResult::error(ErrorCode::InternalError, e.to_string());
                    }
                }
                // Update tags if provided
                if let Some(tags) = data.tags {
                    if let Err(e) = self.story_event_service.update_tags(id, tags).await {
                        return ResponseResult::error(ErrorCode::InternalError, e.to_string());
                    }
                }
                ResponseResult::success_empty()
            }

            RequestPayload::SetStoryEventVisibility { event_id, visible } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_story_event_id(&event_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.story_event_service.set_visibility(id, visible).await {
                    Ok(_) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateDmMarker { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let wid = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.story_event_service.create_dm_marker(wid, data.title, data.content).await {
                    Ok(event_id) => ResponseResult::success(serde_json::json!({
                        "id": event_id.to_string(),
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Observation Operations
            // =================================================================
            // Note: Observations track when a PC has seen/met an NPC.

            RequestPayload::ListObservations { pc_id } => {
                let id = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.observation_repo.get_for_pc(id).await {
                    Ok(observations) => {
                        let dtos: Vec<serde_json::Value> = observations.iter().map(|obs| {
                            serde_json::json!({
                                "pc_id": obs.pc_id.to_string(),
                                "npc_id": obs.npc_id.to_string(),
                                "location_id": obs.location_id.to_string(),
                                "region_id": obs.region_id.to_string(),
                                "game_time": obs.game_time.to_rfc3339(),
                                "observation_type": obs.observation_type.to_string(),
                                "is_revealed_to_player": obs.is_revealed_to_player,
                                "notes": obs.notes,
                                "created_at": obs.created_at.to_rfc3339(),
                            })
                        }).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::CreateObservation { pc_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let pid = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let npc_id = match Self::parse_character_id(&data.npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Parse location_id and region_id (required for creating observation)
                let location_id = match data.location_id {
                    Some(ref lid) => match Self::parse_location_id(lid) {
                        Ok(id) => id,
                        Err(e) => return e,
                    },
                    None => {
                        return ResponseResult::error(
                            ErrorCode::BadRequest,
                            "location_id is required for creating an observation",
                        );
                    }
                };
                let region_id = match data.region_id {
                    Some(ref rid) => match Self::parse_region_id(rid) {
                        Ok(id) => id,
                        Err(e) => return e,
                    },
                    None => {
                        return ResponseResult::error(
                            ErrorCode::BadRequest,
                            "region_id is required for creating an observation",
                        );
                    }
                };
                // Parse observation type
                let observation_type = data.observation_type.parse::<wrldbldr_domain::entities::ObservationType>()
                    .unwrap_or(wrldbldr_domain::entities::ObservationType::Direct);
                // Use current time as game_time (in a real implementation, this might come from world state)
                let game_time = chrono::Utc::now();
                // Create the observation based on type
                let observation = match observation_type {
                    wrldbldr_domain::entities::ObservationType::Direct => {
                        wrldbldr_domain::entities::NpcObservation::direct(pid, npc_id, location_id, region_id, game_time)
                    }
                    wrldbldr_domain::entities::ObservationType::HeardAbout => {
                        wrldbldr_domain::entities::NpcObservation::heard_about(pid, npc_id, location_id, region_id, game_time, data.notes.clone())
                    }
                    wrldbldr_domain::entities::ObservationType::Deduced => {
                        wrldbldr_domain::entities::NpcObservation::deduced(pid, npc_id, location_id, region_id, game_time, data.notes.clone())
                    }
                };
                match self.observation_repo.upsert(&observation).await {
                    Ok(()) => ResponseResult::success(serde_json::json!({
                        "pc_id": observation.pc_id.to_string(),
                        "npc_id": observation.npc_id.to_string(),
                        "location_id": observation.location_id.to_string(),
                        "region_id": observation.region_id.to_string(),
                        "observation_type": observation.observation_type.to_string(),
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteObservation { pc_id, npc_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _pid = match Self::parse_player_character_id(&pc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Note: ObservationRepositoryPort doesn't have a delete method
                // This would require adding one to the port or keeping as stub
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "DeleteObservation not yet implemented - ObservationRepositoryPort lacks delete method",
                )
            }

            // =================================================================
            // Character-Region Relationship Operations
            // =================================================================
            // Note: These require RegionRepositoryPort or a dedicated service.

            RequestPayload::ListCharacterRegionRelationships { character_id } => {
                let _id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "ListCharacterRegionRelationships requires RegionService - pending implementation",
                )
            }

            RequestPayload::SetCharacterHomeRegion { character_id, region_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _cid = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _rid = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "SetCharacterHomeRegion requires RegionService - pending implementation",
                )
            }

            RequestPayload::SetCharacterWorkRegion { character_id, region_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _cid = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _rid = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "SetCharacterWorkRegion requires RegionService - pending implementation",
                )
            }

            RequestPayload::RemoveCharacterRegionRelationship { character_id, region_id, relationship_type: _ } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _cid = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _rid = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "RemoveCharacterRegionRelationship requires RegionService - pending implementation",
                )
            }

            RequestPayload::ListRegionNpcs { region_id } => {
                let _id = match Self::parse_region_id(&region_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "ListRegionNpcs requires RegionService - pending implementation",
                )
            }

            // =================================================================
            // Goal Operations
            // =================================================================
            RequestPayload::ListGoals { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.get_world_goals(id).await {
                    Ok(goals) => {
                        let dtos: Vec<serde_json::Value> = goals.iter().map(|g| {
                            serde_json::json!({
                                "id": g.id.to_string(),
                                "name": g.name,
                                "description": g.description,
                            })
                        }).collect();
                        ResponseResult::success(dtos)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetGoal { goal_id } => {
                // Goal get is not directly supported by ActantialContextService
                // We'll need to list and filter, or add a direct method
                let id = match Self::parse_goal_id(&goal_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // For now, return not implemented until we add a get method
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "GetGoal requires direct repository access - pending service method",
                )
            }

            RequestPayload::CreateGoal { world_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let wid = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.create_goal(wid, data.name, data.description).await {
                    Ok(goal_id) => ResponseResult::success(serde_json::json!({
                        "id": goal_id.to_string(),
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateGoal { goal_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_goal_id(&goal_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.update_goal(id, data.name, data.description).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteGoal { goal_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_goal_id(&goal_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.delete_goal(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Want Operations
            // =================================================================
            RequestPayload::ListWants { character_id } => {
                let id = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                // Get full context which includes wants
                match self.actantial_service.get_context(id).await {
                    Ok(context) => {
                        // Extract wants from context
                        ResponseResult::success(context.wants)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::GetWant { want_id } => {
                // Want get is not directly supported by ActantialContextService
                let _id = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "GetWant requires direct repository access - pending service method",
                )
            }

            RequestPayload::CreateWant { character_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let cid = match Self::parse_character_id(&character_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let visibility = convert_want_visibility(data.visibility);
                let request = crate::application::services::CreateWantRequest {
                    description: data.description,
                    intensity: data.intensity,
                    priority: data.priority,
                    visibility,
                    target_id: data.target_id,
                    target_type: data.target_type.map(|t| convert_want_target_type(t).to_string()),
                    deflection_behavior: data.deflection_behavior,
                    tells: data.tells,
                };
                match self.actantial_service.create_want(cid, request).await {
                    Ok(want_id) => ResponseResult::success(serde_json::json!({
                        "id": want_id.to_string(),
                    })),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::UpdateWant { want_id, data } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let request = crate::application::services::UpdateWantRequest {
                    description: data.description,
                    intensity: data.intensity,
                    priority: data.priority,
                    visibility: data.visibility.map(convert_want_visibility),
                    deflection_behavior: data.deflection_behavior,
                    tells: data.tells,
                };
                match self.actantial_service.update_want(id, request).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::DeleteWant { want_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.delete_want(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::SetWantTarget { want_id, target_id, target_type } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let target_type_str = convert_want_target_type(target_type);
                match self.actantial_service.set_want_target(id, &target_id, target_type_str).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::RemoveWantTarget { want_id } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.actantial_service.remove_want_target(id).await {
                    Ok(()) => ResponseResult::success_empty(),
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // Game Time Operations
            // =================================================================
            RequestPayload::GetGameTime { world_id } => {
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.world_service.get_game_time(id).await {
                    Ok(game_time) => {
                        let current = game_time.current();
                        let dto = wrldbldr_protocol::GameTime {
                            day: game_time.day_ordinal(),
                            hour: current.hour() as u8,
                            minute: current.minute() as u8,
                            is_paused: game_time.is_paused(),
                        };
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            RequestPayload::AdvanceGameTime { world_id, hours } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let id = match Self::parse_world_id(&world_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                match self.world_service.advance_game_time(id, hours).await {
                    Ok(game_time) => {
                        let current = game_time.current();
                        let dto = wrldbldr_protocol::GameTime {
                            day: game_time.day_ordinal(),
                            hour: current.hour() as u8,
                            minute: current.minute() as u8,
                            is_paused: game_time.is_paused(),
                        };
                        ResponseResult::success(dto)
                    }
                    Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                }
            }

            // =================================================================
            // AI Suggestion Operations
            // =================================================================
            // Note: These require LLM integration via suggestion services.

            RequestPayload::SuggestDeflectionBehavior { npc_id, want_id, want_description } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _wid = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    format!("SuggestDeflectionBehavior for '{}' requires LLM service - pending implementation", want_description),
                )
            }

            RequestPayload::SuggestBehavioralTells { npc_id, want_id, want_description } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _wid = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    format!("SuggestBehavioralTells for '{}' requires LLM service - pending implementation", want_description),
                )
            }

            RequestPayload::SuggestWantDescription { npc_id, context } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    format!("SuggestWantDescription requires LLM service - pending implementation (context: {:?})", context),
                )
            }

            RequestPayload::SuggestActantialReason { npc_id, want_id, target_id, role } => {
                if let Err(e) = ctx.require_dm() { return e; }
                let _nid = match Self::parse_character_id(&npc_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                let _wid = match Self::parse_want_id(&want_id) {
                    Ok(id) => id,
                    Err(e) => return e,
                };
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    format!("SuggestActantialReason for target {} ({:?}) requires LLM service - pending implementation", target_id, role),
                )
            }

            // =================================================================
            // Catch-all for truly unhandled operations
            // =================================================================
            // This should ideally never be reached if all RequestPayload variants are handled above.
            #[allow(unreachable_patterns)]
            _ => {
                tracing::error!(
                    payload_type = ?std::mem::discriminant(&payload),
                    "UNHANDLED Request payload type in AppRequestHandler - this is a bug!"
                );
                ResponseResult::error(
                    ErrorCode::ServiceUnavailable,
                    "This operation is not yet fully implemented",
                )
            }
        }
    }
}
