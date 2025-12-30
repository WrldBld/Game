//! Request Handlers - WebSocket request/response pattern handlers
//!
//! This module contains the `AppRequestHandler` implementation that routes
//! WebSocket requests to the appropriate services.
//!
//! ## Architecture
//!
//! The handlers are organized by domain:
//! - `world_handler` - World, Act, SheetTemplate, GameTime operations
//! - `character_handler` - Character CRUD, Inventory, Archetype changes
//! - `location_handler` - Location CRUD, Location Connections
//! - `region_handler` - Region CRUD, RegionConnections, RegionExits, SpawnPoints
//! - `scene_handler` - Scene and Interaction operations
//! - `challenge_handler` - Challenge CRUD and management
//! - `narrative_handler` - NarrativeEvent and EventChain operations
//! - `player_handler` - PlayerCharacter, Observation, Character-Region relationships
//! - `story_handler` - StoryEvent operations
//! - `misc_handler` - Skill, Goal, Want, Relationship, Disposition, Actantial context
//! - `generation_handler` - AI suggestions and generation queue operations
//!
//! The main `AppRequestHandler` dispatches incoming `RequestPayload` messages
//! to these domain-specific handlers.

mod challenge_handler;
mod character_handler;
mod common;
mod generation_handler;
mod location_handler;
mod misc_handler;
mod narrative_handler;
mod player_handler;
mod region_handler;
mod request_handler;
mod scene_handler;
mod story_handler;
mod world_handler;

pub use request_handler::AppRequestHandler;

// Re-export common helpers for use by domain-specific handlers
pub use common::{
    convert_actantial_role,
    convert_actor_type,
    convert_want_target_type,
    convert_want_visibility,
    npc_disposition_to_dto,
    parse_act_id,
    parse_challenge_id,
    parse_character_id,
    parse_difficulty,
    parse_disposition_level,
    parse_event_chain_id,
    parse_goal_id,
    parse_interaction_id,
    parse_item_id,
    parse_location_id,
    parse_narrative_event_id,
    parse_player_character_id,
    parse_region_id,
    parse_relationship_id,
    parse_relationship_level,
    parse_scene_id,
    parse_skill_id,
    parse_story_event_id,
    parse_uuid,
    parse_want_id,
    parse_world_id,
    to_protocol_game_time,
};
