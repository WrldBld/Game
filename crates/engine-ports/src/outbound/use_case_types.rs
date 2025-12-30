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

use super::{NpcPresenceData, SceneChangedEvent, StagedNpcData, WaitingPcData};

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
#[derive(Default)]
pub enum TimeContext {
    #[default]
    Unspecified,
    TimeOfDay(String),
    During(String),
    Custom(String),
}


/// Directorial context data for scene management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct DirectorialContextData {
    pub npc_motivations: Vec<NpcMotivation>,
    pub scene_mood: Option<String>,
    pub pacing: Option<String>,
    pub dm_notes: Option<String>,
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
pub enum SceneApprovalDecision {
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
        decision: SceneApprovalDecision,
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

// Re-export AdHocOutcomes from domain layer
// The domain type has required success/failure fields, which is the correct semantic.
pub use wrldbldr_domain::value_objects::AdHocOutcomes;

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
// Challenge Input Types
// =============================================================================

/// Input for submitting a dice roll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitRollInput {
    pub challenge_id: String,
    pub roll: i32,
}

/// Input for submitting dice input (formula or manual)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitDiceInputInput {
    pub challenge_id: String,
    pub input_type: DiceInputType,
}

/// Input for triggering a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerChallengeInput {
    pub challenge_id: String,
    pub target_character_id: CharacterId,
}

/// Input for a challenge suggestion decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestionDecisionInput {
    pub request_id: String,
    pub approved: bool,
    pub modified_difficulty: Option<String>,
}

/// Input for regenerating outcome text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateOutcomeInput {
    pub request_id: String,
    pub outcome_type: Option<String>,
    pub guidance: Option<String>,
}

/// Input for discarding a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscardChallengeInput {
    pub request_id: String,
    pub feedback: Option<String>,
}

/// Input for creating an ad-hoc challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAdHocInput {
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty: String,
    pub target_pc_id: PlayerCharacterId,
    pub outcomes: AdHocOutcomes,
}

/// Input for challenge outcome decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDecisionInput {
    pub resolution_id: String,
    pub decision: OutcomeDecision,
}

/// Input for requesting outcome suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSuggestionInput {
    pub resolution_id: String,
    pub guidance: Option<String>,
}

/// Input for requesting outcome branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBranchesInput {
    pub resolution_id: String,
    pub guidance: Option<String>,
}

/// Input for selecting an outcome branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectBranchInput {
    pub resolution_id: String,
    pub branch_id: String,
    pub modified_description: Option<String>,
}

// =============================================================================
// Staging Types
// =============================================================================

/// Input for approving a staging proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveInput {
    /// Request ID of the pending staging
    pub request_id: String,
    /// Approved NPCs with presence decisions
    pub approved_npcs: Vec<ApprovedNpcInput>,
    /// TTL in hours for the staging
    pub ttl_hours: i32,
    /// How this staging was finalized: rule, llm, or custom
    pub source: StagingApprovalSource,
}

/// An approved NPC with presence decision (input type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedNpcInput {
    pub character_id: CharacterId,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: Option<String>,
}

/// Source of staging decision
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StagingApprovalSource {
    RuleBased,
    LlmBased,
    DmCustomized,
}

impl From<StagingApprovalSource> for wrldbldr_domain::entities::StagingSource {
    fn from(source: StagingApprovalSource) -> Self {
        match source {
            StagingApprovalSource::RuleBased => wrldbldr_domain::entities::StagingSource::RuleBased,
            StagingApprovalSource::LlmBased => wrldbldr_domain::entities::StagingSource::LlmBased,
            StagingApprovalSource::DmCustomized => {
                wrldbldr_domain::entities::StagingSource::DmCustomized
            }
        }
    }
}

/// Result of approving staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveResult {
    /// NPCs now present in the region
    pub npcs_present: Vec<NpcPresenceData>,
    /// Number of waiting PCs that were notified
    pub notified_pc_count: usize,
}

/// Input for regenerating staging suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateInput {
    /// Request ID of the pending staging
    pub request_id: String,
    /// DM guidance for the LLM
    pub guidance: String,
}

/// Regenerated NPC suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratedNpc {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Result of regenerating suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingRegenerateResult {
    /// New LLM-based suggestions
    pub llm_based_npcs: Vec<RegeneratedNpc>,
}

/// Input for pre-staging a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreStageInput {
    /// Region to pre-stage
    pub region_id: RegionId,
    /// NPCs to stage
    pub npcs: Vec<ApprovedNpcInput>,
    /// TTL in hours
    pub ttl_hours: i32,
}

/// Result of pre-staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreStageResult {
    /// NPCs now present in the region
    pub npcs_present: Vec<NpcPresenceData>,
}

/// Information about a pending staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingStagingInfo {
    pub request_id: String,
    pub world_id: WorldId,
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub region_name: String,
    pub location_name: String,
    pub waiting_pcs: Vec<WaitingPcInfo>,
    pub rule_based_npcs: Vec<ProposedNpc>,
    pub llm_based_npcs: Vec<ProposedNpc>,
}

/// A waiting PC info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitingPcInfo {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub user_id: String,
}

/// A proposed NPC from the staging proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedNpc {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Approved NPC data for the service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedNpcData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

// =============================================================================
// Scene Input/Output Types
// =============================================================================

/// Input for requesting a scene change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSceneChangeInput {
    /// Scene ID to change to
    pub scene_id: SceneId,
}

/// Input for updating directorial context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDirectorialInput {
    /// NPC motivations for the scene
    pub npc_motivations: Vec<NpcMotivation>,
    /// Overall scene mood
    pub scene_mood: Option<String>,
    /// Pacing hints
    pub pacing: Option<String>,
    /// Additional DM notes
    pub dm_notes: Option<String>,
}

/// Input for scene approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneApprovalDecisionInput {
    /// Request ID being decided
    pub request_id: String,
    /// The decision
    pub decision: SceneApprovalDecision,
}

/// Scene data for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub time_context: String,
    pub directorial_notes: Option<String>,
}

/// Character data for scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: String,
    pub is_speaking: bool,
    pub emotion: Option<String>,
}

/// Interaction data for scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInteractionData {
    pub id: String,
    pub name: String,
    pub interaction_type: String,
    pub target_name: Option<String>,
    pub is_available: bool,
}

/// Result of requesting a scene change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneChangeResult {
    /// Scene was changed and broadcast
    pub scene_changed: bool,
    /// Scene data for the new scene
    pub scene: Option<SceneData>,
    /// Characters in the scene
    pub characters: Vec<SceneCharacterData>,
    /// Interactions available
    pub interactions: Vec<SceneInteractionData>,
}

/// Result of updating directorial context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorialUpdateResult {
    /// Context was updated
    pub updated: bool,
}

/// Result of scene approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneApprovalDecisionResult {
    /// Decision was processed
    pub processed: bool,
}

/// Scene with all related entities
#[derive(Debug, Clone)]
pub struct SceneWithRelations {
    pub scene: SceneEntity,
    pub location: LocationEntity,
    pub featured_characters: Vec<CharacterEntity>,
}

// =============================================================================
// Player Action Types
// =============================================================================

/// Input for a player action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionInput {
    /// Type of action (e.g., "travel", "interact", "speak")
    pub action_type: String,
    /// Target of the action (e.g., location ID, character ID)
    pub target: Option<String>,
    /// Dialogue for speech actions
    pub dialogue: Option<String>,
}

/// Result of a player action
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// Travel completed, scene changed (not queued)
    TravelCompleted {
        action_id: String,
        scene: SceneChangedEvent,
    },
    /// Travel pending staging approval
    TravelPending {
        action_id: String,
        region_id: RegionId,
        region_name: String,
    },
    /// Action was queued for processing
    Queued {
        action_id: String,
        queue_depth: usize,
    },
}

// =============================================================================
// Observation Types
// =============================================================================

/// Input for sharing NPC location with a PC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareNpcLocationInput {
    /// PC to share the information with
    pub pc_id: PlayerCharacterId,
    /// NPC whose location is being shared
    pub npc_id: CharacterId,
    /// Location where NPC was observed
    pub location_id: LocationId,
    /// Region within the location
    pub region_id: RegionId,
    /// Optional notes about how PC learned this
    pub notes: Option<String>,
}

/// Input for triggering an approach event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerApproachInput {
    /// NPC who is approaching
    pub npc_id: CharacterId,
    /// PC being approached
    pub target_pc_id: PlayerCharacterId,
    /// Description of the approach
    pub description: String,
    /// Whether to reveal the NPC's identity
    pub reveal: bool,
}

/// Input for triggering a location event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerLocationEventInput {
    /// Region where the event occurs
    pub region_id: RegionId,
    /// Description of the event
    pub description: String,
}

/// Result of sharing NPC location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareNpcLocationResult {
    /// Observation was created
    pub observation_created: bool,
}

/// Result of triggering an approach event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerApproachResult {
    /// NPC who approached
    pub npc_name: String,
    /// PC who was approached
    pub target_pc_name: String,
}

/// Result of triggering a location event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerLocationEventResult {
    /// Event was broadcast
    pub event_broadcast: bool,
}

/// Data for approach event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproachEventData {
    pub npc_id: String,
    pub npc_name: String,
    pub npc_sprite: Option<String>,
    pub description: String,
    pub reveal: bool,
}

/// Data for location event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationEventData {
    pub region_id: String,
    pub description: String,
}

// =============================================================================
// Inventory Types
// =============================================================================

/// Input for equipping an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
}

/// Input for unequipping an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnequipInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
}

/// Input for dropping an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
    pub quantity: u32,
}

/// Input for picking up an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
}

/// Result of equipping an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipResult {
    pub item_name: String,
}

/// Result of unequipping an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnequipResult {
    pub item_name: String,
}

/// Result of dropping an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropResult {
    pub item_name: String,
    pub quantity: u32,
    pub region_id: RegionId,
}

/// Result of picking up an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupResult {
    pub item_name: String,
}

// =============================================================================
// Narrative Event Types
// =============================================================================

/// Input for narrative event suggestion decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestionDecisionInput {
    pub request_id: String,
    pub event_id: String,
    pub approved: bool,
    pub selected_outcome: Option<String>,
}

/// Result of a narrative event decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventDecisionResult {
    /// Whether the event was triggered
    pub triggered: bool,
}

// =============================================================================
// Error Types
// =============================================================================

/// Trait for extracting error codes from use case errors
///
/// Implemented by all use case error types to provide standardized
/// error code strings. The adapters layer uses this to convert
/// errors to protocol messages.
pub trait ErrorCode: std::fmt::Display {
    /// Get the error code string (e.g., "PC_NOT_FOUND")
    fn code(&self) -> &'static str;
}

/// Blanket implementation for String error types (used by some use case error aliases)
impl ErrorCode for String {
    fn code(&self) -> &'static str {
        "USE_CASE_ERROR"
    }
}

/// Errors that can occur during connection operations
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// World not found
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),

    /// Player character not found
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    /// Already connected to a world
    #[error("Already connected to a world")]
    AlreadyConnected,

    /// Not connected to any world
    #[error("Not connected to any world")]
    NotConnected,

    /// Character already claimed by another player
    #[error("Character already claimed by another player")]
    CharacterClaimed,

    /// Invalid spectate target
    #[error("Invalid spectate target: {0}")]
    InvalidSpectateTarget(String),

    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),

    /// Player role requires a PC ID
    #[error("Player role requires a PC ID")]
    PlayerRequiresPc,

    /// Spectator role requires a spectate target PC ID
    #[error("Spectator role requires a spectate target PC ID")]
    SpectatorRequiresTarget,

    /// DM already connected (another user is DM in this world)
    #[error("DM already connected: {existing_user_id}")]
    DmAlreadyConnected { existing_user_id: String },
}

impl ErrorCode for ConnectionError {
    fn code(&self) -> &'static str {
        match self {
            Self::WorldNotFound(_) => "WORLD_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::AlreadyConnected => "ALREADY_CONNECTED",
            Self::NotConnected => "NOT_CONNECTED",
            Self::CharacterClaimed => "CHARACTER_CLAIMED",
            Self::InvalidSpectateTarget(_) => "INVALID_SPECTATE_TARGET",
            Self::ConnectionFailed(_) => "CONNECTION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
            Self::PlayerRequiresPc => "PLAYER_REQUIRES_PC",
            Self::SpectatorRequiresTarget => "SPECTATOR_REQUIRES_TARGET",
            Self::DmAlreadyConnected { .. } => "DM_ALREADY_CONNECTED",
        }
    }
}

impl ErrorCode for MovementError {
    fn code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::LocationNotFound(_) => "LOCATION_NOT_FOUND",
            Self::ConnectionLocked(_) => "CONNECTION_LOCKED",
            Self::NoArrivalRegion => "NO_ARRIVAL_REGION",
            Self::RegionLocationMismatch => "REGION_MISMATCH",
            Self::NotConnected => "NOT_CONNECTED",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Staging(_) => "STAGING_ERROR",
        }
    }
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
        let approve = SceneApprovalDecision::Approve;
        let reject = SceneApprovalDecision::Reject {
            reason: "Not appropriate".to_string(),
        };
        let edit = SceneApprovalDecision::ApproveWithEdits {
            modified_text: "New text".to_string(),
        };

        assert!(matches!(approve, SceneApprovalDecision::Approve));
        assert!(matches!(reject, SceneApprovalDecision::Reject { .. }));
        assert!(matches!(
            edit,
            SceneApprovalDecision::ApproveWithEdits { .. }
        ));
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
