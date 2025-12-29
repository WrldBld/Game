//! Use Case Types
//!
//! Types that are part of the port contract between use cases (application layer)
//! and adapters. These types are used as inputs and outputs for use case operations.
//!
//! # Design Rationale
//!
//! These types live in the ports layer because:
//! 1. Adapters need to import them to call use cases and handle results
//! 2. They define the contract between layers
//! 3. Moving them here avoids adapters depending on engine-app internals
//!
//! # Categories
//!
//! - **Movement types**: Results of movement operations
//! - **Connection types**: User connection and session data
//! - **Scene types**: Scene context and directorial data
//! - **Challenge types**: Challenge trigger and outcome data

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use wrldbldr_domain::{
    CharacterId, GameTime, ItemId, LocationId, PlayerCharacterId, RegionId, SceneId, WorldId,
};

use super::{SceneChangedEvent, StagedNpcData, WaitingPcData};

// =============================================================================
// Movement Types
// =============================================================================

/// Result of a movement operation
#[derive(Debug, Clone)]
pub enum MovementResult {
    /// Movement succeeded, scene changed
    SceneChanged(SceneChangedEvent),

    /// Movement is pending staging approval
    StagingPending {
        region_id: RegionId,
        region_name: String,
    },

    /// Movement was blocked (locked door, etc.)
    Blocked { reason: String },
}

/// Errors that can occur during movement operations
#[derive(Debug, Error)]
pub enum MovementError {
    /// Player character not found in database
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    /// Target region not found
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    /// Target location not found
    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),

    /// Region connection is locked
    #[error("Connection is locked: {0}")]
    ConnectionLocked(String),

    /// Location has no arrival region (no default, no spawn points)
    #[error("No arrival region available for location")]
    NoArrivalRegion,

    /// Specified arrival region doesn't belong to target location
    #[error("Region does not belong to target location")]
    RegionLocationMismatch,

    /// PC not connected to a world
    #[error("Not connected to a world")]
    NotConnected,

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),

    /// Staging system error
    #[error("Staging error: {0}")]
    Staging(String),
}

/// Input for selecting a player character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectCharacterInput {
    pub pc_id: PlayerCharacterId,
}

/// Result of selecting a player character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectCharacterResult {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub location_id: LocationId,
    pub region_id: Option<RegionId>,
}

/// Input for moving to a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveToRegionInput {
    pub pc_id: PlayerCharacterId,
    pub target_region_id: RegionId,
}

/// Input for exiting to a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitToLocationInput {
    pub pc_id: PlayerCharacterId,
    pub target_location_id: LocationId,
    pub arrival_region_id: Option<RegionId>,
}

/// Data for a pending staging approval
#[derive(Debug, Clone)]
pub struct PendingStagingData {
    pub request_id: String,
    pub world_id: WorldId,
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub region_name: String,
    pub location_name: String,
    pub game_time: GameTime,
    pub rule_based_npcs: Vec<StagedNpcData>,
    pub llm_based_npcs: Vec<StagedNpcData>,
    pub waiting_pcs: Vec<WaitingPcData>,
    pub default_ttl_hours: i32,
}

/// Staging proposal data returned by the staging service
#[derive(Debug, Clone)]
pub struct StagingProposalData {
    pub request_id: String,
    pub rule_based_npcs: Vec<StagedNpcData>,
    pub llm_based_npcs: Vec<StagedNpcData>,
}

// =============================================================================
// Connection Types
// =============================================================================

/// World role for connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldRole {
    DM,
    Player,
    Spectator,
}

/// Connected user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    pub user_id: String,
    pub role: WorldRole,
    pub pc_id: Option<PlayerCharacterId>,
    pub pc_name: Option<String>,
}

/// Connection info
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_id: Uuid,
    pub client_id: String,
    pub user_id: String,
    pub world_id: Option<Uuid>,
    pub role: Option<WorldRole>,
    pub pc_id: Option<Uuid>,
    pub spectate_pc_id: Option<Uuid>,
}

impl ConnectionInfo {
    pub fn is_spectator(&self) -> bool {
        matches!(self.role, Some(WorldRole::Spectator))
    }
}

/// PC data for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcData {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub world_id: String,
    pub current_location_id: String,
    pub current_region_id: Option<String>,
    pub description: Option<String>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// User joined event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJoinedEvent {
    pub user_id: String,
    pub role: WorldRole,
    pub pc: Option<PcData>,
}

/// User left event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLeftEvent {
    pub user_id: String,
}

/// Input for joining a world
#[derive(Debug, Clone)]
pub struct JoinWorldInput {
    /// World to join
    pub world_id: WorldId,
    /// Role to join as
    pub role: WorldRole,
    /// PC to use (for Player role)
    pub pc_id: Option<PlayerCharacterId>,
    /// PC to spectate (for Spectator role)
    pub spectate_pc_id: Option<PlayerCharacterId>,
}

/// Result of joining a world
#[derive(Debug, Clone)]
pub struct JoinWorldResult {
    /// World ID joined
    pub world_id: WorldId,
    /// World snapshot (JSON value for now)
    pub snapshot: serde_json::Value,
    /// List of connected users
    pub connected_users: Vec<ConnectedUser>,
    /// Your role in the world
    pub your_role: WorldRole,
    /// Your PC data (if Player role)
    pub your_pc: Option<PcData>,
}

/// Result of leaving a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveWorldResult {
    /// Successfully left
    pub left: bool,
}

/// Input for setting spectate target
#[derive(Debug, Clone)]
pub struct SetSpectateTargetInput {
    /// PC to spectate
    pub pc_id: PlayerCharacterId,
}

/// Result of setting spectate target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectateTargetResult {
    /// Target PC ID
    pub pc_id: PlayerCharacterId,
    /// Target PC name
    pub pc_name: String,
}

// =============================================================================
// Scene Types
// =============================================================================

/// Time context for scenes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeContext {
    Unspecified,
    TimeOfDay(String),
    During(String),
    Custom(String),
}

impl Default for TimeContext {
    fn default() -> Self {
        Self::Unspecified
    }
}

/// Directorial context data for scene management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorialContextData {
    pub npc_motivations: Vec<NpcMotivation>,
    pub scene_mood: Option<String>,
    pub pacing: Option<String>,
    pub dm_notes: Option<String>,
}

impl Default for DirectorialContextData {
    fn default() -> Self {
        Self {
            npc_motivations: Vec::new(),
            scene_mood: None,
            pacing: None,
            dm_notes: None,
        }
    }
}

/// NPC motivation data for directorial context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMotivation {
    /// NPC character ID
    pub character_id: String,
    /// Current motivation
    pub motivation: String,
    /// Emotional state
    pub emotional_state: Option<String>,
}

/// Scene entity for scene operations
#[derive(Debug, Clone)]
pub struct SceneEntity {
    pub id: SceneId,
    pub name: String,
    pub location_id: LocationId,
    pub backdrop_override: Option<String>,
    pub time_context: TimeContext,
    pub directorial_notes: Option<String>,
}

/// Location entity (simplified for scene context)
#[derive(Debug, Clone)]
pub struct LocationEntity {
    pub name: String,
    pub backdrop_asset: Option<String>,
}

/// Character entity (simplified for scene context)
#[derive(Debug, Clone)]
pub struct CharacterEntity {
    pub id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Interaction entity for scene context
#[derive(Debug, Clone)]
pub struct InteractionEntity {
    pub id: wrldbldr_domain::InteractionId,
    pub name: String,
    pub interaction_type: String,
    pub target: InteractionTarget,
    pub is_available: bool,
}

/// Interaction target types
#[derive(Debug, Clone)]
pub enum InteractionTarget {
    Character(CharacterId),
    Item(ItemId),
    Environment(String),
    None,
}

/// Approval decision types for scene operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// Approve as-is
    Approve,
    /// Reject the request
    Reject { reason: String },
    /// Approve with modifications
    ApproveWithEdits { modified_text: String },
}

/// DM action types for scene operations
#[derive(Debug, Clone)]
pub enum DmAction {
    ApprovalDecision {
        request_id: String,
        decision: ApprovalDecision,
    },
}

// =============================================================================
// Challenge Types
// =============================================================================

/// Information about a trigger for challenge outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub trigger_type: String,
    pub description: String,
}

/// Result of triggering a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerResult {
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Skill name required
    pub skill_name: String,
    /// Difficulty display string
    pub difficulty_display: String,
    /// Challenge description
    pub description: String,
    /// Target character's modifier for this skill
    pub character_modifier: i32,
    /// Suggested dice formula
    pub suggested_dice: String,
    /// Rule system hint
    pub rule_system_hint: String,
}

/// Result of creating an ad-hoc challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdHocResult {
    /// Created challenge ID
    pub challenge_id: String,
}

/// Custom outcomes for ad-hoc challenges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    pub critical_success: Option<String>,
    pub success: Option<String>,
    pub failure: Option<String>,
    pub critical_failure: Option<String>,
}

/// Type of dice input for challenge resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiceInputType {
    /// A dice formula like "1d20+5"
    Formula(String),
    /// A manually entered roll value
    Manual(i32),
}

/// Result of submitting a roll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResultData {
    /// Resolution ID for tracking this pending approval
    pub resolution_id: String,
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Character ID who rolled
    pub character_id: String,
    /// Character name who rolled
    pub character_name: String,
    /// The raw roll value
    pub roll: i32,
    /// Skill modifier applied
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Outcome type (success, failure, etc.)
    pub outcome_type: String,
    /// Outcome description text
    pub outcome_description: String,
    /// Roll breakdown string (e.g., "1d20+5 = 15 + 5 = 20")
    pub roll_breakdown: Option<String>,
    /// Individual dice results
    pub individual_rolls: Option<Vec<i32>>,
    /// Triggers to execute on approval
    pub triggers: Vec<TriggerInfo>,
    /// Whether outcome requires DM approval
    pub pending_approval: bool,
}

/// DM's decision on a challenge outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeDecision {
    /// Accept the outcome as-is
    Accept,
    /// Edit the outcome description
    Edit { modified_text: String },
    /// Request AI suggestions
    Suggest { guidance: Option<String> },
}

/// Result of an outcome decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDecisionResult {
    /// The finalized outcome text
    pub outcome_text: Option<String>,
    /// Whether suggestions are pending
    pub suggestions_pending: bool,
}

/// Result of discarding a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscardResult {
    /// Discarded request ID
    pub request_id: String,
}

/// Result of regenerating outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateResult {
    /// The outcome type that was regenerated
    pub outcome_type: String,
    /// New outcome text
    pub new_outcome: OutcomeDetail,
}

/// Outcome detail for regeneration results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDetail {
    pub flavor_text: String,
    pub scene_direction: String,
    pub proposed_tools: Vec<String>,
}

/// An item from the approval queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalItem {
    pub request_id: String,
    pub proposed_dialogue: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_role_equality() {
        assert_eq!(WorldRole::DM, WorldRole::DM);
        assert_eq!(WorldRole::Player, WorldRole::Player);
        assert_eq!(WorldRole::Spectator, WorldRole::Spectator);
        assert_ne!(WorldRole::DM, WorldRole::Player);
    }

    #[test]
    fn test_connection_info_is_spectator() {
        let spectator = ConnectionInfo {
            connection_id: Uuid::new_v4(),
            client_id: "test".to_string(),
            user_id: "user".to_string(),
            world_id: None,
            role: Some(WorldRole::Spectator),
            pc_id: None,
            spectate_pc_id: None,
        };

        let player = ConnectionInfo {
            connection_id: Uuid::new_v4(),
            client_id: "test".to_string(),
            user_id: "user".to_string(),
            world_id: None,
            role: Some(WorldRole::Player),
            pc_id: None,
            spectate_pc_id: None,
        };

        assert!(spectator.is_spectator());
        assert!(!player.is_spectator());
    }

    #[test]
    fn test_time_context_default() {
        let ctx = TimeContext::default();
        assert!(matches!(ctx, TimeContext::Unspecified));
    }

    #[test]
    fn test_directorial_context_default() {
        let ctx = DirectorialContextData::default();
        assert!(ctx.npc_motivations.is_empty());
        assert!(ctx.scene_mood.is_none());
        assert!(ctx.pacing.is_none());
        assert!(ctx.dm_notes.is_none());
    }

    #[test]
    fn test_dice_input_types() {
        let formula = DiceInputType::Formula("1d20+5".to_string());
        let manual = DiceInputType::Manual(17);

        match formula {
            DiceInputType::Formula(f) => assert_eq!(f, "1d20+5"),
            _ => panic!("Expected formula"),
        }

        match manual {
            DiceInputType::Manual(v) => assert_eq!(v, 17),
            _ => panic!("Expected manual"),
        }
    }

    #[test]
    fn test_outcome_decision_variants() {
        let accept = OutcomeDecision::Accept;
        let edit = OutcomeDecision::Edit {
            modified_text: "New text".to_string(),
        };
        let suggest = OutcomeDecision::Suggest {
            guidance: Some("Be dramatic".to_string()),
        };

        assert!(matches!(accept, OutcomeDecision::Accept));
        assert!(matches!(edit, OutcomeDecision::Edit { .. }));
        assert!(matches!(suggest, OutcomeDecision::Suggest { .. }));
    }

    #[test]
    fn test_approval_decision_variants() {
        let approve = ApprovalDecision::Approve;
        let reject = ApprovalDecision::Reject {
            reason: "Not appropriate".to_string(),
        };
        let edit = ApprovalDecision::ApproveWithEdits {
            modified_text: "New text".to_string(),
        };

        assert!(matches!(approve, ApprovalDecision::Approve));
        assert!(matches!(reject, ApprovalDecision::Reject { .. }));
        assert!(matches!(edit, ApprovalDecision::ApproveWithEdits { .. }));
    }

    #[test]
    fn test_movement_result_variants() {
        let blocked = MovementResult::Blocked {
            reason: "Door is locked".to_string(),
        };
        assert!(matches!(blocked, MovementResult::Blocked { .. }));

        let pending = MovementResult::StagingPending {
            region_id: RegionId::from_uuid(Uuid::new_v4()),
            region_name: "Test Region".to_string(),
        };
        assert!(matches!(pending, MovementResult::StagingPending { .. }));
    }
}
