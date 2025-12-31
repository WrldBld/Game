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

// Note: scene-related dependency ports moved to outbound:
// - SceneWithRelationsQueryPort
// - SceneInteractionsQueryPort
// - WorldStateUpdatePort
// - SceneDmActionQueuePort
//
// Note: DirectorialContextRepositoryPort (use-case DTO persistence) moved to
// outbound as DirectorialContextDtoRepositoryPort.

// =============================================================================
// Player Action Ports (from player_action.rs)
// =============================================================================

// Note: PlayerActionQueuePort and DmNotificationPort are outbound ports.

// =============================================================================
// Staging Ports (from staging.rs and movement.rs)
// =============================================================================

// Note: staging-related dependency ports moved to outbound:
// - StagingStatePort, StagingStateExtPort
// - StagingUseCaseServicePort, StagingUseCaseServiceExtPort
