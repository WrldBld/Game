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
}

#[async_trait]
pub trait ChallengeRepo: Send + Sync {
    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>, RepoError>;
    async fn save(&self, challenge: &Challenge) -> Result<(), RepoError>;
    async fn list_for_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>, RepoError>;
    async fn list_pending_for_world(&self, world_id: WorldId) -> Result<Vec<Challenge>, RepoError>;
    async fn mark_resolved(&self, id: ChallengeId) -> Result<(), RepoError>;
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
    
    // Triggers
    async fn get_triggers_for_region(&self, region_id: RegionId) -> Result<Vec<NarrativeEvent>, RepoError>;
}

#[async_trait]
pub trait StagingRepo: Send + Sync {
    async fn get_staged_npcs(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError>;
    async fn stage_npc(&self, region_id: RegionId, character_id: CharacterId) -> Result<(), RepoError>;
    async fn unstage_npc(&self, region_id: RegionId, character_id: CharacterId) -> Result<(), RepoError>;
    async fn get_pending_staging(&self, world_id: WorldId) -> Result<Vec<Staging>, RepoError>;
    async fn save_pending_staging(&self, staging: &Staging) -> Result<(), RepoError>;
    async fn delete_pending_staging(&self, id: StagingId) -> Result<(), RepoError>;
}

#[async_trait]
pub trait ObservationRepo: Send + Sync {
    async fn get_observations(&self, pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>, RepoError>;
    async fn save_observation(&self, observation: &NpcObservation) -> Result<(), RepoError>;
    async fn has_observed(&self, pc_id: PlayerCharacterId, target_id: CharacterId) -> Result<bool, RepoError>;
}

#[async_trait]
pub trait ItemRepo: Send + Sync {
    async fn get(&self, id: ItemId) -> Result<Option<Item>, RepoError>;
    async fn save(&self, item: &Item) -> Result<(), RepoError>;
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Item>, RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Item>, RepoError>;
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
    pub system_prompt: String,
    pub messages: Vec<LlmMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
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

// =============================================================================
// WebSocket/Connection Port
// =============================================================================

#[async_trait]
pub trait ConnectionManager: Send + Sync {
    async fn register(&self, client_id: Uuid, world_id: WorldId, user_id: String, is_dm: bool);
    async fn unregister(&self, client_id: Uuid);
    async fn get_world_id(&self, client_id: Uuid) -> Option<WorldId>;
    async fn get_user_id(&self, client_id: Uuid) -> Option<String>;
    async fn is_dm(&self, client_id: Uuid) -> bool;
    async fn broadcast_to_world(&self, world_id: WorldId, message: &str);
    async fn send_to_client(&self, client_id: Uuid, message: &str);
    async fn get_dm_client(&self, world_id: WorldId) -> Option<Uuid>;
}
