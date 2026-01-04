//! Port traits for infrastructure boundaries.
//!
//! These are the ONLY abstractions in the engine. Everything else is concrete types.
//! Ports exist for:
//! - Database access (could swap Neo4j -> Postgres)
//! - LLM calls (could swap Ollama -> Claude/OpenAI)
//! - Image generation (could swap ComfyUI -> other)
//! - Queues (could swap SQLite -> Redis)
//! - Clock/Random (for testing)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use wrldbldr_domain::*;

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("Not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ImageGenError {
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Service unavailable")]
    Unavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue error: {0}")]
    Error(String),
}

// =============================================================================
// Infrastructure Types
// =============================================================================

/// NPC-Region relationship for staging suggestions
#[derive(Debug, Clone)]
pub struct NpcRegionRelationship {
    pub region_id: RegionId,
    pub relationship_type: NpcRegionRelationType,
    pub shift: Option<String>,          // For WORKS_AT: "day", "night", "always"
    pub frequency: Option<String>,      // For FREQUENTS: "always", "often", "sometimes", "rarely"
    pub time_of_day: Option<String>,    // For FREQUENTS: "morning", "afternoon", "evening", "night"
    pub reason: Option<String>,         // For AVOIDS: why they avoid it
}

/// Type of NPC-Region relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcRegionRelationType {
    HomeRegion,
    WorksAt,
    Frequents,
    Avoids,
}

impl std::fmt::Display for NpcRegionRelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HomeRegion => write!(f, "HOME_REGION"),
            Self::WorksAt => write!(f, "WORKS_AT_REGION"),
            Self::Frequents => write!(f, "FREQUENTS_REGION"),
            Self::Avoids => write!(f, "AVOIDS_REGION"),
        }
    }
}

/// NPC with their region relationship info (for staging suggestions)
#[derive(Debug, Clone)]
pub struct NpcWithRegionInfo {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub relationship_type: NpcRegionRelationType,
    pub shift: Option<String>,
    pub frequency: Option<String>,
    pub time_of_day: Option<String>,
    pub reason: Option<String>,
}

// =============================================================================
// Database Ports (one per entity type)
// =============================================================================

#[async_trait]
pub trait CharacterRepo: Send + Sync {
    // CRUD
    async fn get(&self, id: CharacterId) -> Result<Option<Character>, RepoError>;
    async fn save(&self, character: &Character) -> Result<(), RepoError>;
    async fn delete(&self, id: CharacterId) -> Result<(), RepoError>;
    
    // Queries
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Character>, RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Character>, RepoError>;
    async fn list_npcs_in_world(&self, world_id: WorldId) -> Result<Vec<Character>, RepoError>;
    
    // Position
    async fn update_position(&self, id: CharacterId, region_id: RegionId) -> Result<(), RepoError>;
    
    // Relationships
    async fn get_relationships(&self, id: CharacterId) -> Result<Vec<Relationship>, RepoError>;
    async fn save_relationship(&self, relationship: &Relationship) -> Result<(), RepoError>;
    
    // Inventory
    async fn get_inventory(&self, id: CharacterId) -> Result<Vec<Item>, RepoError>;
    async fn add_to_inventory(&self, character_id: CharacterId, item_id: ItemId) -> Result<(), RepoError>;
    async fn remove_from_inventory(&self, character_id: CharacterId, item_id: ItemId) -> Result<(), RepoError>;
    
    // Wants/Goals
    async fn get_wants(&self, id: CharacterId) -> Result<Vec<Want>, RepoError>;
    async fn save_want(&self, character_id: CharacterId, want: &Want) -> Result<(), RepoError>;
    
    // Disposition (NPC's view of a specific PC)
    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>, RepoError>;
    async fn save_disposition(&self, disposition: &NpcDispositionState) -> Result<(), RepoError>;
    
    // Actantial
    async fn get_actantial_context(&self, id: CharacterId) -> Result<Option<ActantialContext>, RepoError>;
    async fn save_actantial_context(&self, id: CharacterId, context: &ActantialContext) -> Result<(), RepoError>;
    
    // NPC-Region relationships (for staging suggestions)
    /// Get all region relationships for a character (home, work, frequents, avoids)
    async fn get_region_relationships(&self, id: CharacterId) -> Result<Vec<NpcRegionRelationship>, RepoError>;
    /// Set an NPC's home region
    async fn set_home_region(&self, id: CharacterId, region_id: RegionId) -> Result<(), RepoError>;
    /// Set an NPC's work region with optional shift (day/night/always)
    async fn set_work_region(&self, id: CharacterId, region_id: RegionId, shift: Option<String>) -> Result<(), RepoError>;
    /// Add a region the NPC frequents with frequency (always/often/sometimes/rarely)
    async fn add_frequents_region(&self, id: CharacterId, region_id: RegionId, frequency: String, time_of_day: Option<String>) -> Result<(), RepoError>;
    /// Add a region the NPC avoids
    async fn add_avoids_region(&self, id: CharacterId, region_id: RegionId, reason: Option<String>) -> Result<(), RepoError>;
    /// Remove a region relationship
    async fn remove_region_relationship(&self, id: CharacterId, region_id: RegionId, relationship_type: &str) -> Result<(), RepoError>;
    /// Get NPCs that have any relationship to a region (for staging suggestions)
    async fn get_npcs_for_region(&self, region_id: RegionId) -> Result<Vec<NpcWithRegionInfo>, RepoError>;
}

#[async_trait]
pub trait PlayerCharacterRepo: Send + Sync {
    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>, RepoError>;
    async fn save(&self, pc: &PlayerCharacter) -> Result<(), RepoError>;
    async fn delete(&self, id: PlayerCharacterId) -> Result<(), RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<PlayerCharacter>, RepoError>;
    async fn get_by_user(&self, world_id: WorldId, user_id: &str) -> Result<Option<PlayerCharacter>, RepoError>;
    async fn update_position(&self, id: PlayerCharacterId, location_id: LocationId, region_id: RegionId) -> Result<(), RepoError>;
    async fn get_inventory(&self, id: PlayerCharacterId) -> Result<Vec<Item>, RepoError>;
    
    // Inventory management
    async fn add_to_inventory(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<(), RepoError>;
    async fn remove_from_inventory(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<(), RepoError>;
    
    /// Modify a stat on a player character (for ModifyCharacterStat trigger)
    async fn modify_stat(&self, id: PlayerCharacterId, stat: &str, modifier: i32) -> Result<(), RepoError>;
}

#[async_trait]
pub trait LocationRepo: Send + Sync {
    // Location CRUD
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>, RepoError>;
    async fn save_location(&self, location: &Location) -> Result<(), RepoError>;
    async fn list_locations_in_world(&self, world_id: WorldId) -> Result<Vec<Location>, RepoError>;
    
    // Region CRUD
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>, RepoError>;
    async fn save_region(&self, region: &Region) -> Result<(), RepoError>;
    async fn list_regions_in_location(&self, location_id: LocationId) -> Result<Vec<Region>, RepoError>;
    
    // Connections
    async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>, RepoError>;
    async fn save_connection(&self, connection: &RegionConnection) -> Result<(), RepoError>;
    
    // Location connections (exits)
    async fn get_location_exits(&self, location_id: LocationId) -> Result<Vec<LocationConnection>, RepoError>;
}

#[async_trait]
pub trait SceneRepo: Send + Sync {
    async fn get(&self, id: SceneId) -> Result<Option<Scene>, RepoError>;
    async fn save(&self, scene: &Scene) -> Result<(), RepoError>;
    async fn get_current(&self, world_id: WorldId) -> Result<Option<Scene>, RepoError>;
    async fn set_current(&self, world_id: WorldId, scene_id: SceneId) -> Result<(), RepoError>;
    async fn list_for_region(&self, region_id: RegionId) -> Result<Vec<Scene>, RepoError>;
    async fn get_featured_characters(&self, scene_id: SceneId) -> Result<Vec<CharacterId>, RepoError>;
    async fn set_featured_characters(&self, scene_id: SceneId, characters: &[CharacterId]) -> Result<(), RepoError>;
    
    // Completed scene tracking for scene resolution
    /// Check if a PC has completed a specific scene.
    async fn has_completed_scene(&self, pc_id: PlayerCharacterId, scene_id: SceneId) -> Result<bool, RepoError>;
    /// Mark a scene as completed for a PC.
    async fn mark_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId) -> Result<(), RepoError>;
    /// Get all completed scene IDs for a PC.
    async fn get_completed_scenes(&self, pc_id: PlayerCharacterId) -> Result<Vec<SceneId>, RepoError>;
}

#[async_trait]
pub trait ChallengeRepo: Send + Sync {
    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>, RepoError>;
    async fn save(&self, challenge: &Challenge) -> Result<(), RepoError>;
    async fn list_for_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>, RepoError>;
    async fn list_pending_for_world(&self, world_id: WorldId) -> Result<Vec<Challenge>, RepoError>;
    async fn mark_resolved(&self, id: ChallengeId) -> Result<(), RepoError>;
    /// Enable or disable a challenge (for EnableChallenge/DisableChallenge triggers)
    async fn set_enabled(&self, id: ChallengeId, enabled: bool) -> Result<(), RepoError>;
}

#[async_trait]
pub trait NarrativeRepo: Send + Sync {
    // Events
    async fn get_event(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>, RepoError>;
    async fn save_event(&self, event: &NarrativeEvent) -> Result<(), RepoError>;
    async fn list_events_for_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>, RepoError>;
    
    // Event chains
    async fn get_chain(&self, id: EventChainId) -> Result<Option<EventChain>, RepoError>;
    async fn save_chain(&self, chain: &EventChain) -> Result<(), RepoError>;
    
    // Story events
    async fn get_story_event(&self, id: StoryEventId) -> Result<Option<StoryEvent>, RepoError>;
    async fn save_story_event(&self, event: &StoryEvent) -> Result<(), RepoError>;
    async fn list_story_events(&self, world_id: WorldId, limit: usize) -> Result<Vec<StoryEvent>, RepoError>;
    
    // Dialogue history
    /// Get dialogue exchanges between a PC and NPC (reverse chronological order).
    async fn get_dialogues_with_npc(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<StoryEvent>, RepoError>;
    
    /// Update or create SPOKE_TO relationship between PC and NPC.
    /// Tracks last dialogue timestamp, topic, and increments conversation count.
    async fn update_spoke_to(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        timestamp: chrono::DateTime<chrono::Utc>,
        last_topic: Option<String>,
    ) -> Result<(), RepoError>;
    
    // Triggers
    async fn get_triggers_for_region(&self, region_id: RegionId) -> Result<Vec<NarrativeEvent>, RepoError>;
    
    // Event management for effect execution
    /// Set a narrative event's active status (for EnableEvent/DisableEvent effects)
    async fn set_event_active(&self, id: NarrativeEventId, active: bool) -> Result<(), RepoError>;
}

#[async_trait]
pub trait StagingRepo: Send + Sync {
    async fn get_staged_npcs(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError>;
    async fn stage_npc(&self, region_id: RegionId, character_id: CharacterId) -> Result<(), RepoError>;
    async fn unstage_npc(&self, region_id: RegionId, character_id: CharacterId) -> Result<(), RepoError>;
    async fn get_pending_staging(&self, world_id: WorldId) -> Result<Vec<Staging>, RepoError>;
    async fn save_pending_staging(&self, staging: &Staging) -> Result<(), RepoError>;
    async fn delete_pending_staging(&self, id: StagingId) -> Result<(), RepoError>;
    
    /// Get active staging for a region, checking TTL expiry.
    /// Returns None if no staging exists or if the current staging is expired.
    async fn get_active_staging(&self, region_id: RegionId, current_game_time: DateTime<Utc>) -> Result<Option<Staging>, RepoError>;
    
    /// Activate a staging (after DM approval), replacing any existing current staging.
    async fn activate_staging(&self, staging_id: StagingId, region_id: RegionId) -> Result<(), RepoError>;
    
    /// Get staging history for a region (most recent first, limited).
    /// Returns past stagings that are no longer active.
    async fn get_staging_history(&self, region_id: RegionId, limit: usize) -> Result<Vec<Staging>, RepoError>;
}

#[async_trait]
pub trait ObservationRepo: Send + Sync {
    async fn get_observations(&self, pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>, RepoError>;
    async fn save_observation(&self, observation: &NpcObservation) -> Result<(), RepoError>;
    async fn has_observed(&self, pc_id: PlayerCharacterId, target_id: CharacterId) -> Result<bool, RepoError>;
    /// Save deduced information from a challenge (for RevealInformation trigger)
    async fn save_deduced_info(&self, pc_id: PlayerCharacterId, info: String) -> Result<(), RepoError>;
}

#[async_trait]
pub trait ItemRepo: Send + Sync {
    async fn get(&self, id: ItemId) -> Result<Option<Item>, RepoError>;
    async fn save(&self, item: &Item) -> Result<(), RepoError>;
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Item>, RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Item>, RepoError>;
    
    // Equipment management (EQUIPPED_BY edge)
    async fn set_equipped(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<(), RepoError>;
    async fn set_unequipped(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<(), RepoError>;
    
    // Region placement (IN_REGION edge for dropped items)
    async fn place_in_region(&self, item_id: ItemId, region_id: RegionId) -> Result<(), RepoError>;
    async fn remove_from_region(&self, item_id: ItemId) -> Result<(), RepoError>;
}

#[async_trait]
pub trait WorldRepo: Send + Sync {
    async fn get(&self, id: WorldId) -> Result<Option<World>, RepoError>;
    async fn save(&self, world: &World) -> Result<(), RepoError>;
    async fn list_all(&self) -> Result<Vec<World>, RepoError>;
    async fn delete(&self, id: WorldId) -> Result<(), RepoError>;
}

#[async_trait]
pub trait AssetRepo: Send + Sync {
    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>, RepoError>;
    async fn save(&self, asset: &GalleryAsset) -> Result<(), RepoError>;
    async fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<Vec<GalleryAsset>, RepoError>;
    async fn set_active(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        asset_id: AssetId,
    ) -> Result<(), RepoError>;
}

// =============================================================================
// External Service Ports
// =============================================================================

/// LLM request/response types
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// The conversation history
    pub messages: Vec<ChatMessage>,
    /// System prompt / context
    pub system_prompt: Option<String>,
    /// Temperature for response generation (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Optional images for multimodal models
    pub images: Vec<ImageData>,
}

impl LlmRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            images: Vec::new(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// A message in the conversation
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Unknown,
}

/// Image data for multimodal requests
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Base64-encoded image data
    pub data: String,
    /// MIME type (e.g., "image/png")
    pub media_type: String,
}

/// Response from the LLM
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The generated text content
    pub content: String,
    /// Tool calls proposed by the model
    pub tool_calls: Vec<ToolCall>,
    /// Finish reason
    pub finish_reason: FinishReason,
    /// Token usage
    pub usage: Option<TokenUsage>,
}

/// Definition of a tool the LLM can call
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool call proposed by the LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Reason the generation finished
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Unknown,
}

/// Token usage information
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError>;
}

/// Image generation request/response types
#[derive(Debug, Clone)]
pub struct ImageRequest {
    pub prompt: String,
    pub workflow: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct ImageResult {
    pub image_data: Vec<u8>,
    pub format: String,
}

#[async_trait]
pub trait ImageGenPort: Send + Sync {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult, ImageGenError>;
    async fn check_health(&self) -> Result<bool, ImageGenError>;
}

// =============================================================================
// Queue Port
// =============================================================================

/// Queue item wrapper with metadata.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub id: Uuid,
    pub data: QueueItemData,
    pub created_at: DateTime<Utc>,
    pub status: QueueItemStatus,
}

/// Concrete queue item data - avoids generics for dyn compatibility.
#[derive(Debug, Clone)]
pub enum QueueItemData {
    PlayerAction(PlayerActionData),
    LlmRequest(LlmRequestData),
    DmApproval(ApprovalRequestData),
    AssetGeneration(AssetGenerationData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[async_trait]
pub trait QueuePort: Send + Sync {
    // Player action queue
    async fn enqueue_player_action(&self, data: &PlayerActionData) -> Result<Uuid, QueueError>;
    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError>;

    // LLM request queue
    async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<Uuid, QueueError>;
    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError>;

    // DM approval queue
    async fn enqueue_dm_approval(&self, data: &ApprovalRequestData) -> Result<Uuid, QueueError>;
    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError>;

    // Asset generation queue
    async fn enqueue_asset_generation(&self, data: &AssetGenerationData) -> Result<Uuid, QueueError>;
    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError>;

    // Common operations
    async fn mark_complete(&self, id: Uuid) -> Result<(), QueueError>;
    async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), QueueError>;
    async fn get_pending_count(&self, queue_type: &str) -> Result<usize, QueueError>;
    
    /// Get an approval request by ID (for extracting NPC info when processing decision)
    async fn get_approval_request(&self, id: Uuid) -> Result<Option<ApprovalRequestData>, QueueError>;
}

// =============================================================================
// Flag Storage Port
// =============================================================================

/// Repository for game flags (used in scene conditions and narrative triggers).
#[async_trait]
pub trait FlagRepo: Send + Sync {
    /// Get all set flags for a world (world-scoped flags).
    async fn get_world_flags(&self, world_id: WorldId) -> Result<Vec<String>, RepoError>;
    
    /// Get all set flags for a player character (PC-scoped flags).
    async fn get_pc_flags(&self, pc_id: PlayerCharacterId) -> Result<Vec<String>, RepoError>;
    
    /// Set a world-scoped flag.
    async fn set_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError>;
    
    /// Unset a world-scoped flag.
    async fn unset_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError>;
    
    /// Set a PC-scoped flag.
    async fn set_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<(), RepoError>;
    
    /// Unset a PC-scoped flag.
    async fn unset_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<(), RepoError>;
    
    /// Check if a world-scoped flag is set.
    async fn is_world_flag_set(&self, world_id: WorldId, flag_name: &str) -> Result<bool, RepoError>;
    
    /// Check if a PC-scoped flag is set.
    async fn is_pc_flag_set(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<bool, RepoError>;
}

// =============================================================================
// Testability Ports
// =============================================================================

pub trait ClockPort: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub trait RandomPort: Send + Sync {
    fn gen_range(&self, min: i32, max: i32) -> i32;
    fn gen_uuid(&self) -> Uuid;
}


