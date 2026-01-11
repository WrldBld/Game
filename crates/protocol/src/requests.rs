//! Request payload types for WebSocket request/response pattern
//!
//! This module defines all operations that can be requested via WebSocket.
//! Each variant maps to a specific CRUD or action operation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_true() -> bool {
    true
}

pub mod act;
pub mod actantial;
pub mod ai;
pub mod challenge;
pub mod character;
pub mod event_chain;
pub mod expression;
pub mod generation;
pub mod goal;
pub mod interaction;
pub mod items;
pub mod location;
pub mod lore;
pub mod narrative_event;
pub mod npc;
pub mod observation;
pub mod player_character;
pub mod region;
pub mod relationship;
pub mod scene;
pub mod skill;
pub mod stat;
pub mod story_event;
pub mod time;
pub mod want;
pub mod world;

// =============================================================================
// Request Payload Enum
// =============================================================================

/// All operations that can be requested via WebSocket
///
/// This is the top-level grouping enum. Actual request variants live in
/// per-group enums in `requests/*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "group", content = "payload", rename_all = "snake_case")]
pub enum RequestPayload {
    World(world::WorldRequest),
    Character(character::CharacterRequest),
    Location(location::LocationRequest),
    Region(region::RegionRequest),
    Scene(scene::SceneRequest),
    Act(act::ActRequest),
    Interaction(interaction::InteractionRequest),
    Skill(skill::SkillRequest),
    Challenge(challenge::ChallengeRequest),
    NarrativeEvent(narrative_event::NarrativeEventRequest),
    EventChain(event_chain::EventChainRequest),
    StoryEvent(story_event::StoryEventRequest),
    PlayerCharacter(player_character::PlayerCharacterRequest),
    Relationship(relationship::RelationshipRequest),
    Observation(observation::ObservationRequest),
    Goal(goal::GoalRequest),
    Want(want::WantRequest),
    Actantial(actantial::ActantialRequest),
    Time(time::TimeRequest),
    Npc(npc::NpcRequest),
    Generation(generation::GenerationRequest),
    Expression(expression::ExpressionRequest),
    Ai(ai::AiRequest),
    Items(items::ItemsRequest),
    Lore(lore::LoreRequest),
    Stat(stat::StatRequest),

    #[serde(other)]
    Unknown,
}

// =============================================================================
// Data Types for Create/Update Operations
// =============================================================================

/// Data for creating a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorldData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub setting: Option<String>,
}

/// Data for updating a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorldData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub setting: Option<String>,
}

/// Data for creating a character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCharacterData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub archetype: Option<String>,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
}

/// Data for updating a character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCharacterData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
    #[serde(default)]
    pub is_alive: Option<bool>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Data for changing an archetype
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeArchetypeData {
    pub new_archetype: String,
    pub reason: String,
}

/// Data for creating a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocationData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub setting: Option<String>,
}

/// Data for updating a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocationData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub setting: Option<String>,
}

/// Data for creating a location connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocationConnectionData {
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub bidirectional: Option<bool>,
}

/// Data for creating a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegionData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_spawn_point: Option<bool>,
}

/// Data for updating a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRegionData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_spawn_point: Option<bool>,
}

/// Data for creating a region connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegionConnectionData {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub locked: Option<bool>,
    #[serde(default)]
    pub bidirectional: Option<bool>,
}

/// Data for creating a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSceneData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
}

/// Data for updating a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSceneData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location_id: Option<String>,
}

/// Data for creating an act
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub order: Option<u32>,
}

/// Data for creating an interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInteractionData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
}

/// Data for updating an interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInteractionData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
}

/// Data for creating a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
}

/// Data for updating a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkillData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub is_hidden: Option<bool>,
}

/// Data for creating a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChallengeData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub skill_id: String,
    pub difficulty: String,
    #[serde(default)]
    pub success_outcome: Option<String>,
    #[serde(default)]
    pub failure_outcome: Option<String>,
}

/// Data for updating a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChallengeData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub skill_id: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub success_outcome: Option<String>,
    #[serde(default)]
    pub failure_outcome: Option<String>,
}

/// Data for creating a narrative event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNarrativeEventData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub trigger_conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub outcomes: Option<serde_json::Value>,
}

/// Data for updating a narrative event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNarrativeEventData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub trigger_conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub outcomes: Option<serde_json::Value>,
}

/// Data for creating an event chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventChainData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Initial events to add to the chain
    #[serde(default)]
    pub events: Option<Vec<String>>,
    /// Optional act association
    #[serde(default)]
    pub act_id: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Display color (hex or named)
    #[serde(default)]
    pub color: Option<String>,
    /// Whether the chain is active (defaults to true if not specified)
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Data for updating an event chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEventChainData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Events to set (replaces existing)
    #[serde(default)]
    pub events: Option<Vec<String>>,
    /// Optional act association
    #[serde(default)]
    pub act_id: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Display color (hex or named)
    #[serde(default)]
    pub color: Option<String>,
    /// Whether the chain is active
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Data for creating a DM marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDmMarkerData {
    pub title: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// Data for updating a story event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStoryEventData {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Data for creating a player character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlayerCharacterData {
    pub name: String,
    #[serde(default)]
    pub user_id: Option<String>,
    /// Starting region ID - if provided, PC will spawn at this region
    #[serde(default)]
    pub starting_region_id: Option<String>,
    #[serde(default)]
    pub sheet_data: Option<serde_json::Value>,
}

/// Data for updating a player character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlayerCharacterData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sheet_data: Option<serde_json::Value>,
}

/// Data for creating a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationshipData {
    pub from_character_id: String,
    pub to_character_id: String,
    pub relationship_type: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Data for creating an observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateObservationData {
    pub npc_id: String,
    pub observation_type: String,
    #[serde(default)]
    pub location_id: Option<String>,
    #[serde(default)]
    pub region_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

// =============================================================================
// Suggestion Types
// =============================================================================

/// Context data for content suggestions
///
/// This context is passed to the LLM to help generate relevant suggestions.
/// Fields can be populated by the client with whatever information is available.
/// The engine may auto-enrich this context with world data when world_id is available.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionContextData {
    /// Type of entity being created (e.g., "character", "location", "tavern", "forest")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,

    /// Name of the entity (if already set)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,

    /// World/setting name or type (e.g., "Dark Fantasy", "Sci-Fi Western")
    /// If not provided, the engine may auto-populate this from the world record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_setting: Option<String>,

    /// Hints or keywords to guide generation (e.g., archetype, theme)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hints: Option<String>,

    /// Additional context from other fields (e.g., description, backstory)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,

    /// World ID for context enrichment (matches domain SuggestionContext.world_id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_id: Option<Uuid>,
}

/// Data for creating a new item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
}

// =============================================================================
// Lore Data Types
// =============================================================================

/// Data for creating a lore entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLoreData {
    pub title: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_common_knowledge: Option<bool>,
    /// Initial chunks to create with the lore
    #[serde(default)]
    pub chunks: Option<Vec<CreateLoreChunkData>>,
}

/// Data for updating a lore entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLoreData {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_common_knowledge: Option<bool>,
}

/// Data for creating a lore chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLoreChunkData {
    /// Optional title for this chunk (may be omitted)
    #[serde(default)]
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub discovery_hint: Option<String>,
}

/// Data for updating a lore chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLoreChunkData {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub discovery_hint: Option<String>,
}
