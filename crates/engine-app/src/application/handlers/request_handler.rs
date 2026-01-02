//! Application Request Handler - Routes WebSocket requests to domain handlers
//!
//! This module implements the `RequestHandler` trait from `engine-ports`,
//! routing incoming `RequestPayload` messages to domain-specific handlers.
//!
//! # Architecture
//!
//! The handler follows hexagonal architecture:
//! - Inbound: `RequestHandler` trait (from engine-ports)
//! - Outbound: Repository ports, services
//! - Application: This handler orchestrates between them
//!
//! Request handling is delegated to domain-specific modules:
//! - `world_handler` - World, Act, SheetTemplate, GameTime
//! - `character_handler` - Character, Inventory, Archetype
//! - `location_handler` - Location, Connection
//! - `region_handler` - Region, RegionConnection, Exit, SpawnPoints
//! - `scene_handler` - Scene, Interaction
//! - `challenge_handler` - Challenge
//! - `narrative_handler` - NarrativeEvent, EventChain
//! - `player_handler` - PlayerCharacter, Observation, Character-Region relationships
//! - `story_handler` - StoryEvent
//! - `misc_handler` - Skill, Goal, Want, Relationship, Disposition, Actantial
//! - `generation_handler` - AI suggestions, generation queue

use async_trait::async_trait;
use std::sync::Arc;

use wrldbldr_engine_ports::inbound::{RequestContext, RequestHandler};
use crate::application::services::internal::GenerationQueueProjectionServicePort;
use wrldbldr_engine_ports::outbound::{
    CharacterLocationPort, ClockPort, GenerationReadStatePort, ObservationRepositoryPort,
    RegionCrudPort, SuggestionEnqueuePort,
};
use wrldbldr_protocol::{ErrorCode, RequestPayload, ResponseResult};

use crate::application::services::{
    ActantialContextService, ChallengeService, CharacterService, DispositionService,
    EventChainService, InteractionService, ItemService, LocationService, NarrativeEventService,
    PlayerCharacterService, RegionService, RelationshipService, SceneService, SkillService,
    StoryEventService, WorldService,
};
use crate::application::services::internal::SheetTemplateServicePort;

use super::{
    challenge_handler, character_handler, generation_handler, location_handler, misc_handler,
    narrative_handler, player_handler, region_handler, scene_handler, story_handler, world_handler,
};

// =============================================================================
// App Request Handler
// =============================================================================

/// Application-layer request handler
///
/// This handler receives `RequestPayload` from the WebSocket infrastructure,
/// routes to the appropriate domain handler, and returns a `ResponseResult`.
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
    disposition_service: Arc<dyn DispositionService>,
    story_event_service: Arc<dyn StoryEventService>,
    item_service: Arc<dyn ItemService>,
    region_service: Arc<dyn RegionService>,
    sheet_template_service: Arc<dyn SheetTemplateServicePort>,

    // Repository ports (for simple CRUD that doesn't need a full service)
    character_location: Arc<dyn CharacterLocationPort>,
    observation_repo: Arc<dyn ObservationRepositoryPort>,
    region_crud: Arc<dyn RegionCrudPort>,

    // AI suggestion enqueue port (for async LLM suggestions)
    suggestion_enqueue: Arc<dyn SuggestionEnqueuePort>,

    // Generation queue services (for WebSocket hydration)
    generation_queue_projection: Arc<dyn GenerationQueueProjectionServicePort>,
    generation_read_state: Arc<dyn GenerationReadStatePort>,

    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl AppRequestHandler {
    /// Create a new request handler with all service dependencies
    ///
    /// All dependencies are required - there are no optional features.
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
        disposition_service: Arc<dyn DispositionService>,
        story_event_service: Arc<dyn StoryEventService>,
        item_service: Arc<dyn ItemService>,
        region_service: Arc<dyn RegionService>,
        sheet_template_service: Arc<dyn SheetTemplateServicePort>,
        character_location: Arc<dyn CharacterLocationPort>,
        observation_repo: Arc<dyn ObservationRepositoryPort>,
        region_crud: Arc<dyn RegionCrudPort>,
        suggestion_enqueue: Arc<dyn SuggestionEnqueuePort>,
        generation_queue_projection: Arc<dyn GenerationQueueProjectionServicePort>,
        generation_read_state: Arc<dyn GenerationReadStatePort>,
        clock: Arc<dyn ClockPort>,
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
            disposition_service,
            story_event_service,
            item_service,
            region_service,
            sheet_template_service,
            character_location,
            observation_repo,
            region_crud,
            suggestion_enqueue,
            generation_queue_projection,
            generation_read_state,
            clock,
        }
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
            RequestPayload::ListWorlds => world_handler::list_worlds(&self.world_service).await,
            RequestPayload::GetWorld { world_id } => {
                world_handler::get_world(&self.world_service, &world_id).await
            }
            RequestPayload::ExportWorld { world_id } => {
                world_handler::export_world(&self.world_service, &world_id).await
            }
            RequestPayload::GetSheetTemplate { world_id } => {
                world_handler::get_sheet_template(&self.sheet_template_service, &world_id).await
            }
            RequestPayload::CreateWorld { data } => {
                world_handler::create_world(&self.world_service, &ctx, data).await
            }
            RequestPayload::UpdateWorld { world_id, data } => {
                world_handler::update_world(&self.world_service, &ctx, &world_id, data).await
            }
            RequestPayload::DeleteWorld { world_id } => {
                world_handler::delete_world(&self.world_service, &ctx, &world_id).await
            }
            RequestPayload::ListActs { world_id } => {
                world_handler::list_acts(&self.world_service, &world_id).await
            }
            RequestPayload::CreateAct { world_id, data } => {
                world_handler::create_act(&self.world_service, &ctx, &world_id, data).await
            }
            RequestPayload::GetGameTime { world_id } => {
                world_handler::get_game_time(&self.world_service, &world_id).await
            }
            RequestPayload::AdvanceGameTime { world_id, hours } => {
                world_handler::advance_game_time(&self.world_service, &ctx, &world_id, hours).await
            }

            // =================================================================
            // Character Operations
            // =================================================================
            RequestPayload::ListCharacters { world_id } => {
                character_handler::list_characters(&self.character_service, &world_id).await
            }
            RequestPayload::GetCharacter { character_id } => {
                character_handler::get_character(&self.character_service, &character_id).await
            }
            RequestPayload::DeleteCharacter { character_id } => {
                character_handler::delete_character(&self.character_service, &ctx, &character_id)
                    .await
            }
            RequestPayload::CreateCharacter { world_id, data } => {
                character_handler::create_character(&self.character_service, &ctx, &world_id, data)
                    .await
            }
            RequestPayload::UpdateCharacter { character_id, data } => {
                character_handler::update_character(
                    &self.character_service,
                    &ctx,
                    &character_id,
                    data,
                )
                .await
            }
            RequestPayload::ChangeArchetype { character_id, data } => {
                character_handler::change_archetype(
                    &self.character_service,
                    &ctx,
                    &character_id,
                    data,
                )
                .await
            }
            RequestPayload::GetCharacterInventory { character_id } => {
                character_handler::get_character_inventory(&self.item_service, &character_id).await
            }

            // =================================================================
            // Location Operations
            // =================================================================
            RequestPayload::ListLocations { world_id } => {
                location_handler::list_locations(&self.location_service, &world_id).await
            }
            RequestPayload::GetLocation { location_id } => {
                location_handler::get_location(&self.location_service, &location_id).await
            }
            RequestPayload::DeleteLocation { location_id } => {
                location_handler::delete_location(&self.location_service, &ctx, &location_id).await
            }
            RequestPayload::CreateLocation { world_id, data } => {
                location_handler::create_location(&self.location_service, &ctx, &world_id, data)
                    .await
            }
            RequestPayload::UpdateLocation { location_id, data } => {
                location_handler::update_location(&self.location_service, &ctx, &location_id, data)
                    .await
            }
            RequestPayload::GetLocationConnections { location_id } => {
                location_handler::get_location_connections(&self.location_service, &location_id)
                    .await
            }
            RequestPayload::CreateLocationConnection { data } => {
                location_handler::create_location_connection(&self.location_service, &ctx, data)
                    .await
            }
            RequestPayload::DeleteLocationConnection { from_id, to_id } => {
                location_handler::delete_location_connection(
                    &self.location_service,
                    &ctx,
                    &from_id,
                    &to_id,
                )
                .await
            }

            // =================================================================
            // Region Operations
            // =================================================================
            RequestPayload::ListRegions { location_id } => {
                region_handler::list_regions(&self.location_service, &location_id).await
            }
            RequestPayload::GetRegion { region_id } => {
                region_handler::get_region(&self.region_crud, &region_id).await
            }
            RequestPayload::CreateRegion { location_id, data } => {
                region_handler::create_region(&self.location_service, &ctx, &location_id, data)
                    .await
            }
            RequestPayload::UpdateRegion { region_id, data } => {
                region_handler::update_region(&self.region_service, &ctx, &region_id, data).await
            }
            RequestPayload::DeleteRegion { region_id } => {
                region_handler::delete_region(&self.region_service, &ctx, &region_id).await
            }
            RequestPayload::GetRegionConnections { region_id } => {
                region_handler::get_region_connections(&self.region_service, &region_id).await
            }
            RequestPayload::CreateRegionConnection {
                from_id,
                to_id,
                data,
            } => {
                region_handler::create_region_connection(
                    &self.region_service,
                    &ctx,
                    &from_id,
                    &to_id,
                    data,
                )
                .await
            }
            RequestPayload::DeleteRegionConnection { from_id, to_id } => {
                region_handler::delete_region_connection(
                    &self.region_service,
                    &ctx,
                    &from_id,
                    &to_id,
                )
                .await
            }
            RequestPayload::UnlockRegionConnection { from_id, to_id } => {
                region_handler::unlock_region_connection(
                    &self.region_service,
                    &ctx,
                    &from_id,
                    &to_id,
                )
                .await
            }
            RequestPayload::GetRegionExits { region_id } => {
                region_handler::get_region_exits(&self.region_service, &region_id).await
            }
            RequestPayload::CreateRegionExit {
                region_id,
                location_id,
                arrival_region_id,
                description,
                bidirectional,
            } => {
                region_handler::create_region_exit(
                    &self.region_service,
                    &ctx,
                    &region_id,
                    &location_id,
                    &arrival_region_id,
                    description,
                    bidirectional,
                )
                .await
            }
            RequestPayload::DeleteRegionExit {
                region_id,
                location_id,
            } => {
                region_handler::delete_region_exit(
                    &self.region_service,
                    &ctx,
                    &region_id,
                    &location_id,
                )
                .await
            }
            RequestPayload::ListSpawnPoints { world_id } => {
                region_handler::list_spawn_points(&self.region_crud, &world_id).await
            }
            RequestPayload::ListRegionNpcs { region_id } => {
                region_handler::list_region_npcs(&self.region_service, &region_id).await
            }

            // =================================================================
            // Scene Operations
            // =================================================================
            RequestPayload::ListScenes { act_id } => {
                scene_handler::list_scenes(&self.scene_service, &act_id).await
            }
            RequestPayload::GetScene { scene_id } => {
                scene_handler::get_scene(&self.scene_service, &scene_id).await
            }
            RequestPayload::DeleteScene { scene_id } => {
                scene_handler::delete_scene(&self.scene_service, &ctx, &scene_id).await
            }
            RequestPayload::CreateScene { act_id, data } => {
                scene_handler::create_scene(&self.scene_service, &ctx, &act_id, data).await
            }
            RequestPayload::UpdateScene { scene_id, data } => {
                scene_handler::update_scene(&self.scene_service, &ctx, &scene_id, data).await
            }

            // =================================================================
            // Interaction Operations
            // =================================================================
            RequestPayload::ListInteractions { scene_id } => {
                scene_handler::list_interactions(&self.interaction_service, &scene_id).await
            }
            RequestPayload::GetInteraction { interaction_id } => {
                scene_handler::get_interaction(&self.interaction_service, &interaction_id).await
            }
            RequestPayload::DeleteInteraction { interaction_id } => {
                scene_handler::delete_interaction(&self.interaction_service, &ctx, &interaction_id)
                    .await
            }
            RequestPayload::SetInteractionAvailability {
                interaction_id,
                available,
            } => {
                scene_handler::set_interaction_availability(
                    &self.interaction_service,
                    &ctx,
                    &interaction_id,
                    available,
                )
                .await
            }
            RequestPayload::CreateInteraction { scene_id, data } => {
                scene_handler::create_interaction(&self.interaction_service, &ctx, &scene_id, data)
                    .await
            }
            RequestPayload::UpdateInteraction {
                interaction_id,
                data,
            } => {
                scene_handler::update_interaction(
                    &self.interaction_service,
                    &ctx,
                    &interaction_id,
                    data,
                )
                .await
            }

            // =================================================================
            // Challenge Operations
            // =================================================================
            RequestPayload::ListChallenges { world_id } => {
                challenge_handler::list_challenges(&self.challenge_service, &world_id).await
            }
            RequestPayload::GetChallenge { challenge_id } => {
                challenge_handler::get_challenge(&self.challenge_service, &challenge_id).await
            }
            RequestPayload::DeleteChallenge { challenge_id } => {
                challenge_handler::delete_challenge(&self.challenge_service, &ctx, &challenge_id)
                    .await
            }
            RequestPayload::SetChallengeActive {
                challenge_id,
                active,
            } => {
                challenge_handler::set_challenge_active(
                    &self.challenge_service,
                    &ctx,
                    &challenge_id,
                    active,
                )
                .await
            }
            RequestPayload::SetChallengeFavorite {
                challenge_id,
                favorite,
            } => {
                challenge_handler::set_challenge_favorite(
                    &self.challenge_service,
                    &ctx,
                    &challenge_id,
                    favorite,
                )
                .await
            }
            RequestPayload::CreateChallenge { world_id, data } => {
                challenge_handler::create_challenge(&self.challenge_service, &ctx, &world_id, data)
                    .await
            }
            RequestPayload::UpdateChallenge { challenge_id, data } => {
                challenge_handler::update_challenge(
                    &self.challenge_service,
                    &ctx,
                    &challenge_id,
                    data,
                )
                .await
            }

            // =================================================================
            // Narrative Event Operations
            // =================================================================
            RequestPayload::ListNarrativeEvents { world_id } => {
                narrative_handler::list_narrative_events(&self.narrative_event_service, &world_id)
                    .await
            }
            RequestPayload::GetNarrativeEvent { event_id } => {
                narrative_handler::get_narrative_event(&self.narrative_event_service, &event_id)
                    .await
            }
            RequestPayload::DeleteNarrativeEvent { event_id } => {
                narrative_handler::delete_narrative_event(
                    &self.narrative_event_service,
                    &ctx,
                    &event_id,
                )
                .await
            }
            RequestPayload::SetNarrativeEventActive { event_id, active } => {
                narrative_handler::set_narrative_event_active(
                    &self.narrative_event_service,
                    &ctx,
                    &event_id,
                    active,
                )
                .await
            }
            RequestPayload::SetNarrativeEventFavorite { event_id, favorite } => {
                narrative_handler::set_narrative_event_favorite(
                    &self.narrative_event_service,
                    &ctx,
                    &event_id,
                    favorite,
                )
                .await
            }
            RequestPayload::TriggerNarrativeEvent { event_id } => {
                narrative_handler::trigger_narrative_event(
                    &self.narrative_event_service,
                    &ctx,
                    &event_id,
                )
                .await
            }
            RequestPayload::ResetNarrativeEvent { event_id } => {
                narrative_handler::reset_narrative_event(
                    &self.narrative_event_service,
                    &ctx,
                    &event_id,
                )
                .await
            }
            RequestPayload::CreateNarrativeEvent { world_id, data } => {
                narrative_handler::create_narrative_event(
                    &self.narrative_event_service,
                    &self.clock,
                    &ctx,
                    &world_id,
                    data,
                )
                .await
            }
            RequestPayload::UpdateNarrativeEvent { event_id, data } => {
                narrative_handler::update_narrative_event(
                    &self.narrative_event_service,
                    &ctx,
                    &event_id,
                    data,
                )
                .await
            }

            // =================================================================
            // Event Chain Operations
            // =================================================================
            RequestPayload::ListEventChains { world_id } => {
                narrative_handler::list_event_chains(&self.event_chain_service, &world_id).await
            }
            RequestPayload::GetEventChain { chain_id } => {
                narrative_handler::get_event_chain(&self.event_chain_service, &chain_id).await
            }
            RequestPayload::DeleteEventChain { chain_id } => {
                narrative_handler::delete_event_chain(&self.event_chain_service, &ctx, &chain_id)
                    .await
            }
            RequestPayload::CreateEventChain { world_id, data } => {
                narrative_handler::create_event_chain(
                    &self.event_chain_service,
                    &self.clock,
                    &ctx,
                    &world_id,
                    data,
                )
                .await
            }
            RequestPayload::UpdateEventChain { chain_id, data } => {
                narrative_handler::update_event_chain(
                    &self.event_chain_service,
                    &ctx,
                    &chain_id,
                    data,
                )
                .await
            }
            RequestPayload::SetEventChainActive { chain_id, active } => {
                narrative_handler::set_event_chain_active(
                    &self.event_chain_service,
                    &ctx,
                    &chain_id,
                    active,
                )
                .await
            }
            RequestPayload::SetEventChainFavorite { chain_id, favorite } => {
                narrative_handler::set_event_chain_favorite(
                    &self.event_chain_service,
                    &ctx,
                    &chain_id,
                    favorite,
                )
                .await
            }
            RequestPayload::AddEventToChain {
                chain_id,
                event_id,
                position,
            } => {
                narrative_handler::add_event_to_chain(
                    &self.event_chain_service,
                    &ctx,
                    &chain_id,
                    &event_id,
                    position.map(|p| p as i32),
                )
                .await
            }
            RequestPayload::RemoveEventFromChain { chain_id, event_id } => {
                narrative_handler::remove_event_from_chain(
                    &self.event_chain_service,
                    &ctx,
                    &chain_id,
                    &event_id,
                )
                .await
            }
            RequestPayload::CompleteChainEvent { chain_id, event_id } => {
                narrative_handler::complete_chain_event(
                    &self.event_chain_service,
                    &ctx,
                    &chain_id,
                    &event_id,
                )
                .await
            }
            RequestPayload::ResetEventChain { chain_id } => {
                narrative_handler::reset_event_chain(&self.event_chain_service, &ctx, &chain_id)
                    .await
            }
            RequestPayload::GetEventChainStatus { chain_id } => {
                narrative_handler::get_event_chain_status(&self.event_chain_service, &chain_id)
                    .await
            }

            // =================================================================
            // Player Character Operations
            // =================================================================
            RequestPayload::ListPlayerCharacters { world_id } => {
                player_handler::list_player_characters(&self.player_character_service, &world_id)
                    .await
            }
            RequestPayload::GetPlayerCharacter { pc_id } => {
                player_handler::get_player_character(&self.player_character_service, &pc_id).await
            }
            RequestPayload::DeletePlayerCharacter { pc_id } => {
                player_handler::delete_player_character(
                    &self.player_character_service,
                    &ctx,
                    &pc_id,
                )
                .await
            }
            RequestPayload::CreatePlayerCharacter { world_id, data } => {
                player_handler::create_player_character(
                    &self.player_character_service,
                    &self.region_crud,
                    &ctx,
                    &world_id,
                    data,
                )
                .await
            }
            RequestPayload::UpdatePlayerCharacter { pc_id, data } => {
                player_handler::update_player_character(
                    &self.player_character_service,
                    &pc_id,
                    data,
                )
                .await
            }
            RequestPayload::UpdatePlayerCharacterLocation { pc_id, region_id } => {
                player_handler::update_player_character_location(
                    &self.player_character_service,
                    &pc_id,
                    &region_id,
                )
                .await
            }
            RequestPayload::GetMyPlayerCharacter { world_id, user_id } => {
                player_handler::get_my_player_character(
                    &self.player_character_service,
                    &world_id,
                    &user_id,
                )
                .await
            }

            // =================================================================
            // Observation Operations
            // =================================================================
            RequestPayload::ListObservations { pc_id } => {
                player_handler::list_observations(&self.observation_repo, &pc_id).await
            }
            RequestPayload::CreateObservation { pc_id, data } => {
                player_handler::create_observation(
                    &self.observation_repo,
                    &self.clock,
                    &ctx,
                    &pc_id,
                    data,
                )
                .await
            }
            RequestPayload::DeleteObservation { pc_id, npc_id } => {
                player_handler::delete_observation(&self.observation_repo, &ctx, &pc_id, &npc_id)
                    .await
            }

            // =================================================================
            // Character-Region Relationship Operations
            // =================================================================
            RequestPayload::ListCharacterRegionRelationships { character_id } => {
                player_handler::list_character_region_relationships(
                    &self.character_location,
                    &character_id,
                )
                .await
            }
            RequestPayload::SetCharacterHomeRegion {
                character_id,
                region_id,
            } => {
                player_handler::set_character_home_region(
                    &self.character_location,
                    &ctx,
                    &character_id,
                    &region_id,
                )
                .await
            }
            RequestPayload::SetCharacterWorkRegion {
                character_id,
                region_id,
            } => {
                player_handler::set_character_work_region(
                    &self.character_location,
                    &ctx,
                    &character_id,
                    &region_id,
                )
                .await
            }
            RequestPayload::RemoveCharacterRegionRelationship {
                character_id,
                region_id,
                relationship_type,
            } => {
                player_handler::remove_character_region_relationship(
                    &self.character_location,
                    &ctx,
                    &character_id,
                    &region_id,
                    relationship_type,
                )
                .await
            }

            // =================================================================
            // Story Event Operations
            // =================================================================
            RequestPayload::ListStoryEvents {
                world_id,
                page,
                page_size,
            } => {
                story_handler::list_story_events(
                    &self.story_event_service,
                    &world_id,
                    page,
                    page_size,
                )
                .await
            }
            RequestPayload::GetStoryEvent { event_id } => {
                story_handler::get_story_event(&self.story_event_service, &event_id).await
            }
            RequestPayload::UpdateStoryEvent { event_id, data } => {
                story_handler::update_story_event(&self.story_event_service, &ctx, &event_id, data)
                    .await
            }
            RequestPayload::SetStoryEventVisibility { event_id, visible } => {
                story_handler::set_story_event_visibility(
                    &self.story_event_service,
                    &ctx,
                    &event_id,
                    visible,
                )
                .await
            }
            RequestPayload::CreateDmMarker { world_id, data } => {
                story_handler::create_dm_marker(&self.story_event_service, &ctx, &world_id, data)
                    .await
            }

            // =================================================================
            // Skill Operations
            // =================================================================
            RequestPayload::ListSkills { world_id } => {
                misc_handler::list_skills(&self.skill_service, &world_id).await
            }
            RequestPayload::GetSkill { skill_id } => {
                misc_handler::get_skill(&self.skill_service, &skill_id).await
            }
            RequestPayload::DeleteSkill { skill_id } => {
                misc_handler::delete_skill(&self.skill_service, &ctx, &skill_id).await
            }
            RequestPayload::CreateSkill { world_id, data } => {
                misc_handler::create_skill(&self.skill_service, &ctx, &world_id, data).await
            }
            RequestPayload::UpdateSkill { skill_id, data } => {
                misc_handler::update_skill(&self.skill_service, &ctx, &skill_id, data).await
            }

            // =================================================================
            // Relationship Operations
            // =================================================================
            RequestPayload::GetSocialNetwork { world_id } => {
                misc_handler::get_social_network(&self.relationship_service, &world_id).await
            }
            RequestPayload::DeleteRelationship { relationship_id } => {
                misc_handler::delete_relationship(
                    &self.relationship_service,
                    &ctx,
                    &relationship_id,
                )
                .await
            }
            RequestPayload::CreateRelationship { data } => {
                misc_handler::create_relationship(&self.relationship_service, &ctx, data).await
            }

            // =================================================================
            // Actantial Context Operations
            // =================================================================
            RequestPayload::GetActantialContext { character_id } => {
                misc_handler::get_actantial_context(&self.actantial_service, &character_id).await
            }
            RequestPayload::AddActantialView {
                character_id,
                want_id,
                target_id,
                target_type,
                role,
                reason,
            } => {
                misc_handler::add_actantial_view(
                    &self.actantial_service,
                    &ctx,
                    character_id,
                    want_id,
                    target_id,
                    target_type,
                    role,
                    reason,
                )
                .await
            }
            RequestPayload::RemoveActantialView {
                character_id,
                want_id,
                target_id,
                target_type,
                role,
            } => {
                misc_handler::remove_actantial_view(
                    &self.actantial_service,
                    &ctx,
                    character_id,
                    want_id,
                    target_id,
                    target_type,
                    role,
                )
                .await
            }

            // =================================================================
            // NPC Disposition Operations
            // =================================================================
            RequestPayload::GetNpcDispositions { pc_id } => {
                misc_handler::get_npc_dispositions(&self.disposition_service, &pc_id).await
            }
            RequestPayload::SetNpcDisposition {
                npc_id,
                pc_id,
                disposition,
                reason,
            } => {
                misc_handler::set_npc_disposition(
                    &self.disposition_service,
                    &ctx,
                    npc_id,
                    pc_id,
                    disposition,
                    reason,
                )
                .await
            }
            RequestPayload::SetNpcRelationship {
                npc_id,
                pc_id,
                relationship,
            } => {
                misc_handler::set_npc_relationship(
                    &self.disposition_service,
                    &ctx,
                    npc_id,
                    pc_id,
                    relationship,
                )
                .await
            }

            // =================================================================
            // Goal Operations
            // =================================================================
            RequestPayload::ListGoals { world_id } => {
                misc_handler::list_goals(&self.actantial_service, &world_id).await
            }
            RequestPayload::GetGoal { goal_id } => {
                misc_handler::get_goal(&self.actantial_service, &goal_id).await
            }
            RequestPayload::CreateGoal { world_id, data } => {
                misc_handler::create_goal(&self.actantial_service, &ctx, &world_id, data).await
            }
            RequestPayload::UpdateGoal { goal_id, data } => {
                misc_handler::update_goal(&self.actantial_service, &ctx, &goal_id, data).await
            }
            RequestPayload::DeleteGoal { goal_id } => {
                misc_handler::delete_goal(&self.actantial_service, &ctx, &goal_id).await
            }

            // =================================================================
            // Want Operations
            // =================================================================
            RequestPayload::ListWants { character_id } => {
                misc_handler::list_wants(&self.actantial_service, &character_id).await
            }
            RequestPayload::GetWant { want_id } => {
                misc_handler::get_want(&self.actantial_service, &want_id).await
            }
            RequestPayload::CreateWant { character_id, data } => {
                misc_handler::create_want(&self.actantial_service, &ctx, &character_id, data).await
            }
            RequestPayload::UpdateWant { want_id, data } => {
                misc_handler::update_want(&self.actantial_service, &ctx, &want_id, data).await
            }
            RequestPayload::DeleteWant { want_id } => {
                misc_handler::delete_want(&self.actantial_service, &ctx, &want_id).await
            }
            RequestPayload::SetWantTarget {
                want_id,
                target_id,
                target_type,
            } => {
                misc_handler::set_want_target(
                    &self.actantial_service,
                    &ctx,
                    want_id,
                    target_id,
                    target_type,
                )
                .await
            }
            RequestPayload::RemoveWantTarget { want_id } => {
                misc_handler::remove_want_target(&self.actantial_service, &ctx, &want_id).await
            }

            // =================================================================
            // AI Suggestion Operations
            // =================================================================
            RequestPayload::SuggestDeflectionBehavior {
                npc_id,
                want_id,
                want_description,
            } => {
                generation_handler::suggest_deflection_behavior(
                    &self.character_service,
                    &self.suggestion_enqueue,
                    &ctx,
                    &npc_id,
                    &want_id,
                    want_description,
                )
                .await
            }
            RequestPayload::SuggestBehavioralTells {
                npc_id,
                want_id,
                want_description,
            } => {
                generation_handler::suggest_behavioral_tells(
                    &self.character_service,
                    &self.suggestion_enqueue,
                    &ctx,
                    &npc_id,
                    &want_id,
                    want_description,
                )
                .await
            }
            RequestPayload::SuggestWantDescription { npc_id, context } => {
                generation_handler::suggest_want_description(
                    &self.character_service,
                    &self.suggestion_enqueue,
                    &ctx,
                    &npc_id,
                    context,
                )
                .await
            }
            RequestPayload::SuggestActantialReason {
                npc_id,
                want_id,
                target_id,
                role,
            } => {
                generation_handler::suggest_actantial_reason(
                    &self.character_service,
                    &self.suggestion_enqueue,
                    &ctx,
                    &npc_id,
                    &want_id,
                    &target_id,
                    role,
                )
                .await
            }

            // =================================================================
            // Generation Queue Operations
            // =================================================================
            RequestPayload::GetGenerationQueue { world_id, user_id } => {
                generation_handler::get_generation_queue(
                    &self.generation_queue_projection,
                    &ctx,
                    &world_id,
                    user_id,
                )
                .await
            }
            RequestPayload::SyncGenerationReadState {
                world_id,
                read_batches,
                read_suggestions,
            } => {
                generation_handler::sync_generation_read_state(
                    &self.generation_read_state,
                    &ctx,
                    &world_id,
                    &read_batches,
                    &read_suggestions,
                )
                .await
            }

            // =================================================================
            // Content Suggestion Operations
            // =================================================================
            RequestPayload::EnqueueContentSuggestion {
                world_id,
                suggestion_type,
                context,
            } => {
                generation_handler::enqueue_content_suggestion(
                    &self.suggestion_enqueue,
                    &ctx,
                    &world_id,
                    suggestion_type,
                    context,
                )
                .await
            }
            RequestPayload::CancelContentSuggestion { request_id } => {
                generation_handler::cancel_content_suggestion(
                    &self.suggestion_enqueue,
                    &ctx,
                    &request_id,
                )
                .await
            }

            // =================================================================
            // Item Placement Operations
            // =================================================================
            RequestPayload::PlaceItemInRegion { region_id, item_id } => {
                generation_handler::place_item_in_region(
                    &self.item_service,
                    &ctx,
                    &region_id,
                    &item_id,
                )
                .await
            }
            RequestPayload::CreateAndPlaceItem {
                world_id,
                region_id,
                data,
            } => {
                generation_handler::create_and_place_item(
                    &self.item_service,
                    &ctx,
                    &world_id,
                    &region_id,
                    data,
                )
                .await
            }

            // =================================================================
            // Catch-all for unhandled operations
            // =================================================================
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
