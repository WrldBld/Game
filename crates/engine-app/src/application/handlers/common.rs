//! Common helper functions for request handlers
//!
//! This module contains parsing and conversion utilities shared across
//! domain-specific request handlers.

use uuid::Uuid;

use wrldbldr_domain::entities::{ActantialRole, Difficulty, WantVisibility};
use wrldbldr_domain::value_objects::{DispositionLevel, RelationshipLevel};
use wrldbldr_domain::{
    ActId, ChallengeId, CharacterId, EventChainId, GoalId, InteractionId, ItemId, LocationId,
    NarrativeEventId, PlayerCharacterId, RegionId, RelationshipId, SceneId, SkillId, StoryEventId,
    WantId, WorldId,
};
use wrldbldr_protocol::{
    ActantialRoleData, ActorTypeData, ErrorCode, ResponseResult, WantTargetTypeData,
    WantVisibilityData,
};

use crate::application::services::ActorTargetType;

// =============================================================================
// UUID Parsing Helpers
// =============================================================================

/// Parse a string as a UUID, returning a ResponseResult error if invalid
pub fn parse_uuid(id: &str, entity_name: &str) -> Result<Uuid, ResponseResult> {
    Uuid::parse_str(id).map_err(|_| {
        ResponseResult::error(
            ErrorCode::BadRequest,
            format!("Invalid {} ID: {}", entity_name, id),
        )
    })
}

/// Parse a world ID string into a WorldId
pub fn parse_world_id(id: &str) -> Result<WorldId, ResponseResult> {
    let uuid = parse_uuid(id, "world")?;
    Ok(WorldId::from_uuid(uuid))
}

/// Parse a character ID string into a CharacterId
pub fn parse_character_id(id: &str) -> Result<CharacterId, ResponseResult> {
    let uuid = parse_uuid(id, "character")?;
    Ok(CharacterId::from_uuid(uuid))
}

/// Parse a location ID string into a LocationId
pub fn parse_location_id(id: &str) -> Result<LocationId, ResponseResult> {
    let uuid = parse_uuid(id, "location")?;
    Ok(LocationId::from_uuid(uuid))
}

/// Parse a skill ID string into a SkillId
pub fn parse_skill_id(id: &str) -> Result<SkillId, ResponseResult> {
    let uuid = parse_uuid(id, "skill")?;
    Ok(SkillId::from_uuid(uuid))
}

/// Parse a scene ID string into a SceneId
pub fn parse_scene_id(id: &str) -> Result<SceneId, ResponseResult> {
    let uuid = parse_uuid(id, "scene")?;
    Ok(SceneId::from_uuid(uuid))
}

/// Parse an act ID string into an ActId
pub fn parse_act_id(id: &str) -> Result<ActId, ResponseResult> {
    let uuid = parse_uuid(id, "act")?;
    Ok(ActId::from_uuid(uuid))
}

/// Parse a challenge ID string into a ChallengeId
pub fn parse_challenge_id(id: &str) -> Result<ChallengeId, ResponseResult> {
    let uuid = parse_uuid(id, "challenge")?;
    Ok(ChallengeId::from_uuid(uuid))
}

/// Parse a narrative event ID string into a NarrativeEventId
pub fn parse_narrative_event_id(id: &str) -> Result<NarrativeEventId, ResponseResult> {
    let uuid = parse_uuid(id, "narrative_event")?;
    Ok(NarrativeEventId::from_uuid(uuid))
}

/// Parse an event chain ID string into an EventChainId
pub fn parse_event_chain_id(id: &str) -> Result<EventChainId, ResponseResult> {
    let uuid = parse_uuid(id, "event_chain")?;
    Ok(EventChainId::from_uuid(uuid))
}

/// Parse a player character ID string into a PlayerCharacterId
pub fn parse_player_character_id(id: &str) -> Result<PlayerCharacterId, ResponseResult> {
    let uuid = parse_uuid(id, "player_character")?;
    Ok(PlayerCharacterId::from_uuid(uuid))
}

/// Parse an interaction ID string into an InteractionId
pub fn parse_interaction_id(id: &str) -> Result<InteractionId, ResponseResult> {
    let uuid = parse_uuid(id, "interaction")?;
    Ok(InteractionId::from_uuid(uuid))
}

/// Parse a goal ID string into a GoalId
pub fn parse_goal_id(id: &str) -> Result<GoalId, ResponseResult> {
    let uuid = parse_uuid(id, "goal")?;
    Ok(GoalId::from_uuid(uuid))
}

/// Parse a want ID string into a WantId
pub fn parse_want_id(id: &str) -> Result<WantId, ResponseResult> {
    let uuid = parse_uuid(id, "want")?;
    Ok(WantId::from_uuid(uuid))
}

/// Parse a region ID string into a RegionId
pub fn parse_region_id(id: &str) -> Result<RegionId, ResponseResult> {
    let uuid = parse_uuid(id, "region")?;
    Ok(RegionId::from_uuid(uuid))
}

/// Parse a relationship ID string into a RelationshipId
pub fn parse_relationship_id(id: &str) -> Result<RelationshipId, ResponseResult> {
    let uuid = parse_uuid(id, "relationship")?;
    Ok(RelationshipId::from_uuid(uuid))
}

/// Parse a story event ID string into a StoryEventId
pub fn parse_story_event_id(id: &str) -> Result<StoryEventId, ResponseResult> {
    let uuid = parse_uuid(id, "story_event")?;
    Ok(StoryEventId::from_uuid(uuid))
}

/// Parse an item ID string into an ItemId
pub fn parse_item_id(id: &str) -> Result<ItemId, ResponseResult> {
    let uuid = parse_uuid(id, "item")?;
    Ok(ItemId::from_uuid(uuid))
}

// =============================================================================
// Value Parsing Helpers
// =============================================================================

/// Parse a difficulty string into a Difficulty enum
pub fn parse_difficulty(s: &str) -> Difficulty {
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

/// Parse a disposition level string into a DispositionLevel enum
pub fn parse_disposition_level(s: &str) -> DispositionLevel {
    match s.to_lowercase().as_str() {
        "hostile" => DispositionLevel::Hostile,
        "suspicious" => DispositionLevel::Suspicious,
        "dismissive" => DispositionLevel::Dismissive,
        "neutral" => DispositionLevel::Neutral,
        "respectful" => DispositionLevel::Respectful,
        "friendly" => DispositionLevel::Friendly,
        "grateful" => DispositionLevel::Grateful,
        _ => DispositionLevel::Neutral, // Default to neutral
    }
}

/// Parse a relationship level string into a RelationshipLevel enum
pub fn parse_relationship_level(s: &str) -> RelationshipLevel {
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

// =============================================================================
// Protocol Conversion Helpers
// =============================================================================

/// Convert ActorTypeData to ActorTargetType
pub fn convert_actor_type(data: ActorTypeData) -> ActorTargetType {
    match data {
        ActorTypeData::Npc => ActorTargetType::Npc,
        ActorTypeData::Pc | ActorTypeData::Unknown => ActorTargetType::Pc, // Default unknown to PC
    }
}

/// Convert ActantialRoleData to ActantialRole
pub fn convert_actantial_role(data: ActantialRoleData) -> ActantialRole {
    match data {
        ActantialRoleData::Helper | ActantialRoleData::Unknown => ActantialRole::Helper, // Default unknown to Helper
        ActantialRoleData::Opponent => ActantialRole::Opponent,
        ActantialRoleData::Sender => ActantialRole::Sender,
        ActantialRoleData::Receiver => ActantialRole::Receiver,
    }
}

/// Convert WantTargetTypeData to target type string
pub fn convert_want_target_type(data: WantTargetTypeData) -> &'static str {
    match data {
        WantTargetTypeData::Character | WantTargetTypeData::Unknown => "Character", // Default unknown to Character
        WantTargetTypeData::Item => "Item",
        WantTargetTypeData::Goal => "Goal",
    }
}

/// Convert WantVisibilityData to domain WantVisibility
pub fn convert_want_visibility(data: WantVisibilityData) -> WantVisibility {
    match data {
        WantVisibilityData::Known => WantVisibility::Known,
        WantVisibilityData::Suspected => WantVisibility::Suspected,
        WantVisibilityData::Hidden | WantVisibilityData::Unknown => WantVisibility::Hidden, // Default unknown to Hidden
    }
}

/// Convert domain GameTime to protocol GameTime for wire transfer.
///
/// This conversion lives in the application layer (not protocol) to maintain
/// the separation between domain and wire-format types.
pub fn to_protocol_game_time(
    game_time: &wrldbldr_domain::GameTime,
) -> wrldbldr_protocol::GameTime {
    use chrono::Timelike;
    let current = game_time.current();
    wrldbldr_protocol::GameTime::new(
        game_time.day_ordinal(),
        current.hour() as u8,
        current.minute() as u8,
        game_time.is_paused(),
    )
}

/// Convert NpcDispositionState to NpcDispositionStateDto for wire transfer.
///
/// This conversion lives in the application layer (not protocol) to maintain
/// the separation between domain and wire-format types.
pub fn npc_disposition_to_dto(
    state: &wrldbldr_domain::value_objects::NpcDispositionState,
) -> wrldbldr_protocol::NpcDispositionStateDto {
    wrldbldr_protocol::NpcDispositionStateDto {
        npc_id: state.npc_id.to_uuid(),
        pc_id: state.pc_id.to_uuid(),
        disposition: state.disposition,
        relationship: state.relationship,
        sentiment: state.sentiment,
        updated_at: state.updated_at.to_rfc3339(),
        disposition_reason: state.disposition_reason.clone(),
        relationship_points: state.relationship_points,
    }
}
