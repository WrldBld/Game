//! Request Handlers - WebSocket request/response pattern handlers
//!
//! This module contains the `AppRequestHandler` implementation that routes
//! WebSocket requests to the appropriate services.

mod common;
mod request_handler;

pub use request_handler::AppRequestHandler;

// Re-export common helpers for use by domain-specific handlers
pub use common::{
    convert_actantial_role,
    // Protocol conversion helpers
    convert_actor_type,
    convert_want_target_type,
    convert_want_visibility,
    parse_act_id,
    parse_challenge_id,
    parse_character_id,
    // Value parsing helpers
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
    // UUID parsing helpers
    parse_uuid,
    parse_want_id,
    parse_world_id,
};
