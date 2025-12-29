//! Use Case Ports
//!
//! Port trait definitions for use cases in the application layer.
//! These ports define the interfaces that adapters must implement
//! to support various game engine operations.
//!
//! Moved from engine-app/use_cases as part of hexagonal architecture cleanup.
//!
//! Note: The types used by these traits (DTOs, result types, etc.) are defined
//! in `outbound::use_case_types` and re-exported here for convenience.

use async_trait::async_trait;
use uuid::Uuid;

use wrldbldr_domain::entities::StagedNpc;
use wrldbldr_domain::{ActionId, GameTime, LocationId, PlayerCharacterId, RegionId, SceneId, WorldId, CharacterId};

// Re-export types from outbound for use in trait definitions
pub use crate::outbound::{
    // Challenge types
    AdHocOutcomes, AdHocResult, ApprovalItem, DiceInputType, OutcomeDecision, RollResultData,
    TriggerInfo, TriggerResult,
    // Connection types
    ConnectedUser, ConnectionInfo, PcData, UserJoinedEvent, UserLeftEvent, WorldRole,
    // Scene types
    CharacterEntity, DirectorialContextData, InteractionEntity,
    InteractionTarget, LocationEntity, NpcMotivation, SceneApprovalDecision, SceneEntity, TimeContext,
    // Note: Use SceneDmAction for the use-case version (enum, not service struct)
    SceneDmAction as DmAction,
    // Note: Use UseCaseSceneWithRelations for the use-case version with SceneEntity
    UseCaseSceneWithRelations as SceneWithRelations,
    // Staging types
    ApprovedNpcData, PendingStagingData, PendingStagingInfo, ProposedNpc, RegeneratedNpc,
    StagedNpcData, StagingProposalData, WaitingPcData, WaitingPcInfo,
    // Observation types
    ApproachEventData, LocationEventData,
};

// =============================================================================
// Challenge Ports (from challenge.rs)
// =============================================================================

/// Port for challenge resolution operations
///
/// This abstracts the ChallengeResolutionService for use case consumption.
/// Methods include `world_id` to support world-scoped challenge resolution.
#[async_trait]
pub trait ChallengeResolutionPort: Send + Sync {
    /// Handle a dice roll submission
    async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
    ) -> Result<RollResultData, String>;

    /// Handle dice input (formula or manual)
    async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
    ) -> Result<RollResultData, String>;

    /// Trigger a challenge against a target
    async fn trigger_challenge(
        &self,
        world_id: &WorldId,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> Result<TriggerResult, String>;

    /// Handle DM's decision on a suggestion
    async fn handle_suggestion_decision(
        &self,
        world_id: &WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Result<(), String>;

    /// Create an ad-hoc challenge
    async fn create_adhoc_challenge(
        &self,
        world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String>;
}

/// Port for challenge outcome approval operations
#[async_trait]
pub trait ChallengeOutcomeApprovalPort: Send + Sync {
    /// Process DM's decision on an outcome
    async fn process_decision(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        decision: OutcomeDecision,
    ) -> Result<(), String>;

    /// Request outcome branches
    async fn request_branches(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<(), String>;

    /// Select a specific branch
    async fn select_branch(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<(), String>;
}

/// Port for DM approval queue operations
#[async_trait]
pub trait ChallengeDmApprovalQueuePort: Send + Sync {
    /// Get an approval item by ID
    async fn get_by_id(&self, request_id: &str) -> Result<Option<ApprovalItem>, String>;

    /// Discard a challenge from the queue
    async fn discard_challenge(&self, dm_id: &str, request_id: &str);
}

// =============================================================================
// Connection Ports (from connection.rs)
// =============================================================================

/// Port for connection management
#[async_trait]
pub trait ConnectionManagerPort: Send + Sync {
    /// Register a new connection
    async fn register_connection(&self, connection_id: Uuid, client_id: String, user_id: String);

    /// Join a world
    async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: Uuid,
        role: WorldRole,
        pc_id: Option<Uuid>,
        spectate_pc_id: Option<Uuid>,
    ) -> Result<Vec<ConnectedUser>, String>;

    /// Leave a world
    async fn leave_world(&self, connection_id: Uuid) -> Option<(Uuid, WorldRole)>;

    /// Get connection info
    async fn get_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo>;

    /// Set spectate target
    async fn set_spectate_target(&self, connection_id: Uuid, pc_id: Option<Uuid>);

    /// Get world connections
    async fn get_world_connections(&self, world_id: Uuid) -> Vec<Uuid>;

    /// Send to connection
    async fn send_to_connection(&self, connection_id: Uuid, user_joined: UserJoinedEvent);

    /// Broadcast to world
    async fn broadcast_to_world(&self, world_id: Uuid, event: UserLeftEvent);
}

/// Port for world service operations
#[async_trait]
pub trait WorldServicePort: Send + Sync {
    /// Export world snapshot
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<serde_json::Value, String>;
}

/// Port for player character service
#[async_trait]
pub trait PlayerCharacterServicePort: Send + Sync {
    /// Get PC by ID
    async fn get_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<PcData>, String>;
}

/// Port for directorial context
#[async_trait]
pub trait DirectorialContextPort: Send + Sync {
    /// Get directorial context
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialContextData>, String>;
}

// =============================================================================
// Scene Ports (from scene.rs)
// =============================================================================

/// Port for scene service operations
#[async_trait]
pub trait SceneServicePort: Send + Sync {
    /// Get scene with all relations
    async fn get_scene_with_relations(
        &self,
        scene_id: SceneId,
    ) -> Result<Option<SceneWithRelations>, String>;
}

/// Port for interaction service
#[async_trait]
pub trait InteractionServicePort: Send + Sync {
    /// List interactions for a scene
    async fn list_interactions(&self, scene_id: SceneId) -> Result<Vec<InteractionEntity>, String>;
}

/// Port for world state management
pub trait WorldStatePort: Send + Sync {
    /// Set the current scene for a world
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);

    /// Set directorial context for a world
    fn set_directorial_context(&self, world_id: &WorldId, context: DirectorialContextData);
}

/// Port for directorial context persistence
#[async_trait]
pub trait DirectorialContextRepositoryPort: Send + Sync {
    /// Save directorial context
    async fn save(
        &self,
        world_id: &WorldId,
        context: &DirectorialContextData,
    ) -> Result<(), String>;
}

/// Port for DM action queue
#[async_trait]
pub trait SceneDmActionQueuePort: Send + Sync {
    /// Enqueue a DM action
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        dm_id: String,
        action: DmAction,
    ) -> Result<(), String>;
}

// =============================================================================
// Player Action Ports (from player_action.rs)
// =============================================================================

/// Port for player action queue operations
#[async_trait]
pub trait PlayerActionQueuePort: Send + Sync {
    /// Enqueue an action
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        player_id: String,
        pc_id: Option<PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<ActionId, String>;

    /// Get current queue depth
    async fn depth(&self) -> Result<usize, String>;
}

/// Port for sending messages to DM
#[async_trait]
pub trait DmNotificationPort: Send + Sync {
    /// Send action queued notification to DM
    async fn notify_action_queued(
        &self,
        world_id: &WorldId,
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    );
}

// =============================================================================
// Staging Ports (from staging.rs and movement.rs)
// =============================================================================

/// Port for managing pending staging state
#[async_trait]
pub trait StagingStatePort: Send + Sync {
    /// Get current game time for the world
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;

    /// Check if there's a pending staging for a region
    fn has_pending_staging(&self, world_id: &WorldId, region_id: &RegionId) -> bool;

    /// Add a PC to the waiting list for a pending staging
    fn add_waiting_pc(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
        pc_id: Uuid,
        pc_name: String,
        user_id: String,
        client_id: String,
    );

    /// Store a new pending staging approval
    fn store_pending_staging(&self, pending: PendingStagingData);
}

/// Port for staging service operations
#[async_trait]
pub trait StagingServicePort: Send + Sync {
    /// Get current valid staging for a region
    async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Option<Vec<StagedNpc>>, String>;

    /// Generate a staging proposal for a region
    async fn generate_proposal(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_id: LocationId,
        location_name: &str,
        game_time: &GameTime,
        ttl_hours: i32,
        dm_guidance: Option<&str>,
    ) -> Result<StagingProposalData, String>;
}

/// Extended port for staging state management
#[async_trait]
pub trait StagingStateExtPort: StagingStatePort {
    /// Get a pending staging by request ID
    fn get_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<PendingStagingInfo>;

    /// Remove a pending staging
    fn remove_pending_staging(&self, world_id: &WorldId, request_id: &str);

    /// Update the LLM suggestions for a pending staging
    fn update_llm_suggestions(
        &self,
        world_id: &WorldId,
        request_id: &str,
        npcs: Vec<RegeneratedNpc>,
    );
}

/// Extended staging service port with additional operations
#[async_trait]
pub trait StagingServiceExtPort: StagingServicePort {
    /// Approve staging and persist it
    async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        approved_npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        source: wrldbldr_domain::entities::StagingSource,
        approved_by: &str,
    ) -> Result<Vec<StagedNpc>, String>;

    /// Regenerate LLM suggestions with guidance
    async fn regenerate_suggestions(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_name: &str,
        game_time: &GameTime,
        guidance: &str,
    ) -> Result<Vec<RegeneratedNpc>, String>;

    /// Pre-stage a region
    async fn pre_stage_region(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        dm_user_id: &str,
    ) -> Result<Vec<StagedNpc>, String>;
}

// =============================================================================
// Observation Ports (from observation.rs)
// =============================================================================

/// Port for sending messages to specific users
#[async_trait]
pub trait WorldMessagePort: Send + Sync {
    /// Send a message to a specific user in a world
    async fn send_to_user(&self, user_id: &str, world_id: Uuid, event: ApproachEventData);

    /// Broadcast to all in a world
    async fn broadcast_to_world(&self, world_id: Uuid, event: LocationEventData);
}
