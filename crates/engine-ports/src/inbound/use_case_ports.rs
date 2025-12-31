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
use wrldbldr_domain::{
    GameTime, LocationId, RegionId, SceneId, WorldId,
};

// Re-export types from outbound for use in trait definitions
pub use crate::outbound::{
    // Challenge types
    AdHocOutcomes,
    AdHocResult,
    // Observation types
    ApproachEventData,
    ApprovalItem,
    // Staging types
    ApprovedNpcData,
    // Scene types
    CharacterEntity,
    // Connection types
    ConnectedUser,
    ConnectionInfo,
    DiceInputType,
    DirectorialContextData,
    InteractionEntity,
    InteractionTarget,
    LocationEntity,
    LocationEventData,
    NpcMotivation,
    OutcomeDecision,
    PcData,
    PendingStagingData,
    PendingStagingInfo,
    ProposedNpc,
    RegeneratedNpc,
    RollResultData,
    SceneApprovalDecision,
    // Note: Use SceneDmAction for the use-case version (enum, not service struct)
    SceneDmAction as DmAction,
    SceneEntity,
    StagedNpcData,
    StagingProposalData,
    TimeContext,
    TriggerInfo,
    TriggerResult,
    // Note: Use UseCaseSceneWithRelations for the use-case version with SceneEntity
    UseCaseSceneWithRelations as SceneWithRelations,
    UserJoinedEvent,
    UserLeftEvent,
    WaitingPcData,
    WaitingPcInfo,
    WorldRole,
};

// =============================================================================
// Challenge Ports (from challenge.rs)
// =============================================================================

// Note: ChallengeResolutionPort, ChallengeOutcomeApprovalPort, and NarrativeRollContext
// are outbound ports/types.

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

// Note: connection-related DTO ports moved to outbound:
// - WorldSnapshotJsonPort
// - PlayerCharacterDtoPort
// - DirectorialContextQueryPort

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

// Note: DirectorialContextRepositoryPort (use-case DTO persistence) moved to
// outbound as DirectorialContextDtoRepositoryPort.

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

// Note: PlayerActionQueuePort and DmNotificationPort are outbound ports.

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
