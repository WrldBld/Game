//! Request payload types for WebSocket request/response pattern
//!
//! This module defines all operations that can be requested via WebSocket.
//! Each variant maps to a specific CRUD or action operation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::messages::{
    ActantialRoleData, ActorTypeData, CreateGoalData, CreateWantData, UpdateGoalData,
    UpdateWantData, WantTargetTypeData,
};

// =============================================================================
// Request Payload Enum
// =============================================================================

/// All operations that can be requested via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestPayload {
    // =========================================================================
    // World Operations
    // =========================================================================
    /// List all worlds
    ListWorlds,

    /// Get a specific world
    GetWorld { world_id: String },

    /// Create a new world
    CreateWorld { data: CreateWorldData },

    /// Update a world
    UpdateWorld {
        world_id: String,
        data: UpdateWorldData,
    },

    /// Delete a world
    DeleteWorld { world_id: String },

    /// Export a world as JSON
    ExportWorld { world_id: String },

    /// Get character sheet template for a world
    GetSheetTemplate { world_id: String },

    // =========================================================================
    // Character Operations
    // =========================================================================
    /// List all characters in a world
    ListCharacters { world_id: String },

    /// Get a specific character
    GetCharacter { character_id: String },

    /// Create a new character
    CreateCharacter {
        world_id: String,
        data: CreateCharacterData,
    },

    /// Update a character
    UpdateCharacter {
        character_id: String,
        data: UpdateCharacterData,
    },

    /// Delete a character
    DeleteCharacter { character_id: String },

    /// Change a character's archetype
    ChangeArchetype {
        character_id: String,
        data: ChangeArchetypeData,
    },

    /// Get a character's inventory
    GetCharacterInventory { character_id: String },

    // =========================================================================
    // Location Operations
    // =========================================================================
    /// List all locations in a world
    ListLocations { world_id: String },

    /// Get a specific location
    GetLocation { location_id: String },

    /// Create a new location
    CreateLocation {
        world_id: String,
        data: CreateLocationData,
    },

    /// Update a location
    UpdateLocation {
        location_id: String,
        data: UpdateLocationData,
    },

    /// Delete a location
    DeleteLocation { location_id: String },

    /// Get connections for a location
    GetLocationConnections { location_id: String },

    /// Create a connection between locations
    CreateLocationConnection { data: CreateLocationConnectionData },

    /// Delete a connection between locations
    DeleteLocationConnection { from_id: String, to_id: String },

    // =========================================================================
    // Region Operations
    // =========================================================================
    /// List all regions in a location
    ListRegions { location_id: String },

    /// Get a specific region
    GetRegion { region_id: String },

    /// Create a new region
    CreateRegion {
        location_id: String,
        data: CreateRegionData,
    },

    /// Update a region
    UpdateRegion {
        region_id: String,
        data: UpdateRegionData,
    },

    /// Delete a region
    DeleteRegion { region_id: String },

    /// Get connections for a region
    GetRegionConnections { region_id: String },

    /// Create a connection between regions
    CreateRegionConnection {
        from_id: String,
        to_id: String,
        data: CreateRegionConnectionData,
    },

    /// Delete a connection between regions
    DeleteRegionConnection { from_id: String, to_id: String },

    /// Unlock a region connection
    UnlockRegionConnection { from_id: String, to_id: String },

    /// Get exits from a region to other locations
    GetRegionExits { region_id: String },

    /// Create an exit from a region to a location
    CreateRegionExit {
        region_id: String,
        location_id: String,
        arrival_region_id: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        bidirectional: Option<bool>,
    },

    /// Delete an exit from a region
    DeleteRegionExit {
        region_id: String,
        location_id: String,
    },

    /// List spawn points in a world
    ListSpawnPoints { world_id: String },

    // =========================================================================
    // Scene Operations
    // =========================================================================
    /// List all scenes in an act
    ListScenes { act_id: String },

    /// Get a specific scene
    GetScene { scene_id: String },

    /// Create a new scene
    CreateScene {
        act_id: String,
        data: CreateSceneData,
    },

    /// Update a scene
    UpdateScene {
        scene_id: String,
        data: UpdateSceneData,
    },

    /// Delete a scene
    DeleteScene { scene_id: String },

    // =========================================================================
    // Act Operations
    // =========================================================================
    /// List all acts in a world
    ListActs { world_id: String },

    /// Create a new act
    CreateAct {
        world_id: String,
        data: CreateActData,
    },

    // =========================================================================
    // Interaction Operations
    // =========================================================================
    /// List all interactions in a scene
    ListInteractions { scene_id: String },

    /// Get a specific interaction
    GetInteraction { interaction_id: String },

    /// Create a new interaction
    CreateInteraction {
        scene_id: String,
        data: CreateInteractionData,
    },

    /// Update an interaction
    UpdateInteraction {
        interaction_id: String,
        data: UpdateInteractionData,
    },

    /// Delete an interaction
    DeleteInteraction { interaction_id: String },

    /// Set interaction availability
    SetInteractionAvailability {
        interaction_id: String,
        available: bool,
    },

    // =========================================================================
    // Skill Operations
    // =========================================================================
    /// List all skills in a world
    ListSkills { world_id: String },

    /// Get a specific skill
    GetSkill { skill_id: String },

    /// Create a new skill
    CreateSkill {
        world_id: String,
        data: CreateSkillData,
    },

    /// Update a skill
    UpdateSkill {
        skill_id: String,
        data: UpdateSkillData,
    },

    /// Delete a skill
    DeleteSkill { skill_id: String },

    // =========================================================================
    // Challenge Operations
    // =========================================================================
    /// List all challenges in a world
    ListChallenges { world_id: String },

    /// Get a specific challenge
    GetChallenge { challenge_id: String },

    /// Create a new challenge
    CreateChallenge {
        world_id: String,
        data: CreateChallengeData,
    },

    /// Update a challenge
    UpdateChallenge {
        challenge_id: String,
        data: UpdateChallengeData,
    },

    /// Delete a challenge
    DeleteChallenge { challenge_id: String },

    /// Set challenge active state
    SetChallengeActive { challenge_id: String, active: bool },

    /// Set challenge favorite state
    SetChallengeFavorite {
        challenge_id: String,
        favorite: bool,
    },

    // =========================================================================
    // Narrative Event Operations
    // =========================================================================
    /// List all narrative events in a world
    ListNarrativeEvents { world_id: String },

    /// Get a specific narrative event
    GetNarrativeEvent { event_id: String },

    /// Create a new narrative event
    CreateNarrativeEvent {
        world_id: String,
        data: CreateNarrativeEventData,
    },

    /// Update a narrative event
    UpdateNarrativeEvent {
        event_id: String,
        data: UpdateNarrativeEventData,
    },

    /// Delete a narrative event
    DeleteNarrativeEvent { event_id: String },

    /// Set narrative event active state
    SetNarrativeEventActive { event_id: String, active: bool },

    /// Set narrative event favorite state
    SetNarrativeEventFavorite { event_id: String, favorite: bool },

    /// Trigger a narrative event
    TriggerNarrativeEvent { event_id: String },

    /// Reset a narrative event
    ResetNarrativeEvent { event_id: String },

    // =========================================================================
    // Event Chain Operations
    // =========================================================================
    /// List all event chains in a world
    ListEventChains { world_id: String },

    /// Get a specific event chain
    GetEventChain { chain_id: String },

    /// Create a new event chain
    CreateEventChain {
        world_id: String,
        data: CreateEventChainData,
    },

    /// Update an event chain
    UpdateEventChain {
        chain_id: String,
        data: UpdateEventChainData,
    },

    /// Delete an event chain
    DeleteEventChain { chain_id: String },

    /// Set event chain active state
    SetEventChainActive { chain_id: String, active: bool },

    /// Set event chain favorite state
    SetEventChainFavorite { chain_id: String, favorite: bool },

    /// Add an event to a chain
    AddEventToChain {
        chain_id: String,
        event_id: String,
        #[serde(default)]
        position: Option<u32>,
    },

    /// Remove an event from a chain
    RemoveEventFromChain { chain_id: String, event_id: String },

    /// Complete an event in a chain
    CompleteChainEvent { chain_id: String, event_id: String },

    /// Reset an event chain
    ResetEventChain { chain_id: String },

    /// Get event chain status
    GetEventChainStatus { chain_id: String },

    // =========================================================================
    // Story Event Operations
    // =========================================================================
    /// List story events in a world (paginated)
    ListStoryEvents {
        world_id: String,
        #[serde(default)]
        page: Option<u32>,
        #[serde(default)]
        page_size: Option<u32>,
    },

    /// Get a specific story event
    GetStoryEvent { event_id: String },

    /// Create a DM marker story event
    CreateDmMarker {
        world_id: String,
        data: CreateDmMarkerData,
    },

    /// Update a story event
    UpdateStoryEvent {
        event_id: String,
        data: UpdateStoryEventData,
    },

    /// Set story event visibility
    SetStoryEventVisibility { event_id: String, visible: bool },

    // =========================================================================
    // Player Character Operations
    // =========================================================================
    /// List all player characters in a world
    ListPlayerCharacters { world_id: String },

    /// Get a specific player character
    GetPlayerCharacter { pc_id: String },

    /// Create a new player character
    CreatePlayerCharacter {
        world_id: String,
        data: CreatePlayerCharacterData,
    },

    /// Update a player character
    UpdatePlayerCharacter {
        pc_id: String,
        data: UpdatePlayerCharacterData,
    },

    /// Delete a player character
    DeletePlayerCharacter { pc_id: String },

    /// Update player character location
    UpdatePlayerCharacterLocation { pc_id: String, region_id: String },

    /// Get current user's player character in a world
    GetMyPlayerCharacter { world_id: String, user_id: String },

    // =========================================================================
    // Relationship Operations
    // =========================================================================
    /// Get the social network for a world
    GetSocialNetwork { world_id: String },

    /// Create a relationship
    CreateRelationship { data: CreateRelationshipData },

    /// Delete a relationship
    DeleteRelationship { relationship_id: String },

    // =========================================================================
    // Observation Operations
    // =========================================================================
    /// List observations for a player character
    ListObservations { pc_id: String },

    /// Create an observation
    CreateObservation {
        pc_id: String,
        data: CreateObservationData,
    },

    /// Delete an observation
    DeleteObservation { pc_id: String, npc_id: String },

    // =========================================================================
    // Goal Operations (Actantial)
    // =========================================================================
    /// List all goals in a world
    ListGoals { world_id: String },

    /// Get a specific goal
    GetGoal { goal_id: String },

    /// Create a new goal
    CreateGoal {
        world_id: String,
        data: CreateGoalData,
    },

    /// Update a goal
    UpdateGoal {
        goal_id: String,
        data: UpdateGoalData,
    },

    /// Delete a goal
    DeleteGoal { goal_id: String },

    // =========================================================================
    // Want Operations (Actantial)
    // =========================================================================
    /// List all wants for a character
    ListWants { character_id: String },

    /// Get a specific want
    GetWant { want_id: String },

    /// Create a new want
    CreateWant {
        character_id: String,
        data: CreateWantData,
    },

    /// Update a want
    UpdateWant {
        want_id: String,
        data: UpdateWantData,
    },

    /// Delete a want
    DeleteWant { want_id: String },

    /// Set the target for a want
    SetWantTarget {
        want_id: String,
        target_id: String,
        target_type: WantTargetTypeData,
    },

    /// Remove the target from a want
    RemoveWantTarget { want_id: String },

    // =========================================================================
    // Actantial View Operations
    // =========================================================================
    /// Get full actantial context for a character
    GetActantialContext { character_id: String },

    /// Add an actantial view
    AddActantialView {
        character_id: String,
        want_id: String,
        target_id: String,
        target_type: ActorTypeData,
        role: ActantialRoleData,
        reason: String,
    },

    /// Remove an actantial view
    RemoveActantialView {
        character_id: String,
        want_id: String,
        target_id: String,
        target_type: ActorTypeData,
        role: ActantialRoleData,
    },

    // =========================================================================
    // Game Time Operations
    // =========================================================================
    /// Get the current game time for a world
    GetGameTime { world_id: String },

    /// Advance the game time
    AdvanceGameTime { world_id: String, hours: u32 },

    // =========================================================================
    // Character-Region Relationship Operations
    // =========================================================================
    /// List character-region relationships
    ListCharacterRegionRelationships { character_id: String },

    /// Set character's home region
    SetCharacterHomeRegion {
        character_id: String,
        region_id: String,
    },

    /// Set character's work region
    SetCharacterWorkRegion {
        character_id: String,
        region_id: String,
    },

    /// Remove character-region relationship
    RemoveCharacterRegionRelationship {
        character_id: String,
        region_id: String,
        relationship_type: String,
    },

    /// List NPCs in a region
    ListRegionNpcs { region_id: String },

    // =========================================================================
    // NPC Disposition Operations
    // =========================================================================
    /// Set an NPC's disposition toward a specific PC
    SetNpcDisposition {
        npc_id: String,
        pc_id: String,
        disposition: String,
        #[serde(default)]
        reason: Option<String>,
    },

    /// Set an NPC's relationship level toward a specific PC
    SetNpcRelationship {
        npc_id: String,
        pc_id: String,
        relationship: String,
    },

    /// Get all NPCs' dispositions toward a specific PC
    GetNpcDispositions { pc_id: String },

    // =========================================================================
    // LLM Suggestion Operations
    // =========================================================================
    /// Request LLM suggestions for deflection behavior
    SuggestDeflectionBehavior {
        npc_id: String,
        want_id: String,
        want_description: String,
    },

    /// Request LLM suggestions for behavioral tells
    SuggestBehavioralTells {
        npc_id: String,
        want_id: String,
        want_description: String,
    },

    /// Request LLM suggestions for want description
    SuggestWantDescription {
        npc_id: String,
        #[serde(default)]
        context: Option<String>,
    },

    /// Request LLM suggestions for actantial view reason
    SuggestActantialReason {
        npc_id: String,
        want_id: String,
        target_id: String,
        role: ActantialRoleData,
    },

    // =========================================================================
    // Generation Queue Operations
    // =========================================================================
    /// Get generation queue snapshot for hydration
    GetGenerationQueue {
        world_id: String,
        #[serde(default)]
        user_id: Option<String>,
    },

    /// Sync read state markers to backend
    SyncGenerationReadState {
        world_id: String,
        read_batches: Vec<String>,
        read_suggestions: Vec<String>,
    },

    // =========================================================================
    // Content Suggestion Operations (General LLM Suggestions)
    // =========================================================================
    /// Enqueue a content suggestion request (async, queued)
    ///
    /// Used for character names, descriptions, location details, etc.
    /// Returns a request_id immediately; results are delivered via
    /// SuggestionCompleted/SuggestionFailed WebSocket events.
    EnqueueContentSuggestion {
        world_id: String,
        /// Suggestion type: "character_name", "character_description",
        /// "character_wants", "character_fears", "character_backstory",
        /// "location_name", "location_description", "location_atmosphere",
        /// "location_features", "location_secrets"
        suggestion_type: String,
        context: SuggestionContextData,
    },

    /// Cancel a pending content suggestion request
    CancelContentSuggestion { request_id: String },

    // =========================================================================
    // Item Placement Operations (DM only)
    // =========================================================================
    /// Place an existing item into a region (DM only)
    PlaceItemInRegion { region_id: String, item_id: String },

    /// Create a new item and place it in a region (DM only)
    CreateAndPlaceItem {
        world_id: String,
        region_id: String,
        data: CreateItemData,
    },

    /// Unknown request type for forward compatibility
    ///
    /// When deserializing an unknown variant, this variant is used instead of
    /// failing. Allows older servers to gracefully handle new request types.
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
}

/// Data for updating an event chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEventChainData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
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
