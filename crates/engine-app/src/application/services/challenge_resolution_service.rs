//! Challenge resolution service - encapsulates challenge roll handling, DM-triggered
//! challenges, and challenge suggestion approvals.
//!
//! This moves challenge-related business logic out of the websocket handler into a
//! dedicated application service, keeping the transport layer thin.
//!
//! ## Architecture Note
//!
//! This service returns typed results instead of constructing protocol messages.
//! Broadcasting is handled by the use case layer via `BroadcastPort`.
//!
//! ## Refactored for Hexagonal Architecture (Phase 2A.3)
//!
//! This service now uses port traits (`ChallengeServicePort`, `SkillServicePort`,
//! `PlayerCharacterServicePort`) instead of app-layer trait generics, eliminating
//! duplicate service instantiations in the composition root.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{debug, info};

use wrldbldr_domain::entities::{Difficulty, OutcomeType};
use wrldbldr_domain::value_objects::{AdHocOutcomes, DiceRollInput, ProposedTool};
use wrldbldr_domain::value_objects::{EffectLevel, NarrativeResolutionConfig, Position};
use wrldbldr_domain::{ChallengeId, CharacterId, PlayerCharacterId, SkillId, WorldId};
use crate::application::services::internal::{
    AdHocResult as PortAdHocResult, ChallengeOutcomeApprovalServicePort,
    ChallengeResolutionServicePort, ChallengeServicePort, DiceRoll as PortDiceRoll,
    NarrativeRollContext as PortNarrativeRollContext,
    OutcomeTriggerInfo as PortOutcomeTriggerInfo, PendingResolution as PortPendingResolution,
    PlayerCharacterServicePort, RollResult as PortRollResult,
    RollResultData as PortRollResultData, SkillServicePort, TriggerResult as PortTriggerResult,
};
use wrldbldr_engine_ports::outbound::{
    ApprovalRequestLookupPort, ChallengeOutcomeData, ClockPort, DiceInputType, RandomPort,
};

// ============================================================================
// Error Types
// ============================================================================

/// Error type for challenge resolution operations
#[derive(Debug, Error)]
pub enum ChallengeResolutionError {
    #[error("Invalid challenge ID: {0}")]
    InvalidChallengeId(String),

    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),

    #[error("Failed to load challenge: {0}")]
    ChallengeLoadFailed(String),

    #[error("Player character not found")]
    PlayerCharacterNotFound,

    #[error("Failed to load player character: {0}")]
    PlayerCharacterLoadFailed(String),

    #[error("Invalid dice formula: {0}")]
    InvalidDiceFormula(String),

    #[error("Failed to queue for approval: {0}")]
    ApprovalQueueFailed(String),

    #[error("Approval lookup error: {0}")]
    ApprovalLookupError(String),

    #[error("Challenge suggestion not found in approval: {0}")]
    ChallengeSuggestionNotFound(String),
}

// ============================================================================
// Result Types
// ============================================================================

/// Result of submitting a roll for challenge resolution
#[derive(Debug, Clone)]
pub struct RollSubmissionResult {
    /// Resolution ID for tracking this pending approval
    pub resolution_id: String,
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Challenge description
    pub challenge_description: Option<String>,
    /// Skill name (if applicable)
    pub skill_name: Option<String>,
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
    /// Outcome type (success, failure, critical, etc.)
    pub outcome_type: String,
    /// Outcome description text
    pub outcome_description: String,
    /// Roll breakdown string (e.g., "1d20+5 = 15 + 5 = 20")
    pub roll_breakdown: Option<String>,
    /// Individual dice results
    pub individual_rolls: Option<Vec<i32>>,
    /// Triggers to execute on approval
    pub outcome_triggers: Vec<OutcomeTriggerInfo>,
}

/// Trigger information for roll results
#[derive(Debug, Clone)]
pub struct OutcomeTriggerInfo {
    pub trigger_type: String,
    pub description: String,
}

/// Result of triggering a challenge for a player
#[derive(Debug, Clone)]
pub struct ChallengeTriggerResult {
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
#[derive(Debug, Clone)]
pub struct AdHocChallengeResult {
    /// Generated challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Target player character ID
    pub target_pc_id: String,
    /// Challenge outcomes
    pub outcomes: AdHocOutcomes,
}

// DiceInputType is imported from engine-ports

/// Narrative context for roll evaluation (Blades in the Dark style)
#[derive(Debug, Clone, Default)]
pub struct NarrativeRollContext {
    /// Position for Blades-style resolution (Controlled, Risky, Desperate)
    pub position: Option<Position>,
    /// Effect level for Blades-style resolution (Limited, Standard, Great, etc.)
    pub effect: Option<EffectLevel>,
    /// Individual dice results (for critical detection in d6 pools)
    pub dice_results: Option<Vec<i32>>,
}

/// Preamble data gathered for challenge resolution.
/// This struct holds the common data needed by both `handle_roll` and `handle_roll_input`.
struct ChallengePreamble {
    challenge: wrldbldr_domain::entities::Challenge,
    /// Skill ID fetched from REQUIRES_SKILL edge (may be None if no skill is set)
    skill_id: Option<wrldbldr_domain::SkillId>,
    world_id: WorldId,
    player_name: String,
    character_modifier: i32,
    character_id: String,
}

/// Service responsible for challenge-related flows.
///
/// This service returns typed results and delegates all broadcasting to the use case layer.
/// It orchestrates domain logic for challenge resolution, including:
/// - Roll evaluation against challenge difficulty
/// - Queuing outcomes for DM approval
/// - Triggering challenges for players
/// - Handling DM suggestion decisions
///
/// ## Architecture
///
/// - Returns `Result<T, ChallengeResolutionError>` instead of `Option<serde_json::Value>`
/// - Does NOT construct `ServerMessage` (hexagonal architecture compliance)
/// - All challenge outcomes go through DM approval (no `has_dm()` bypass)
/// - Uses port traits for dependencies, enabling single instantiation in composition root
pub struct ChallengeResolutionService {
    challenge_service: Arc<dyn ChallengeServicePort>,
    skill_service: Arc<dyn SkillServicePort>,
    player_character_service: Arc<dyn PlayerCharacterServicePort>,
    approval_request_lookup: Arc<dyn ApprovalRequestLookupPort>,
    challenge_outcome_approval_service: Arc<dyn ChallengeOutcomeApprovalServicePort>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
    /// Random number generator for dice rolls (required for testability)
    rng: Arc<dyn RandomPort>,
}

impl ChallengeResolutionService {
    /// Create a new challenge resolution service
    ///
    /// All challenges are routed through the approval service for DM review.
    ///
    /// # Arguments
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    /// * `rng` - Random number generator. Use `ThreadRngAdapter` in production,
    ///           `FixedRandomPort` in tests for deterministic behavior.
    pub fn new(
        challenge_service: Arc<dyn ChallengeServicePort>,
        skill_service: Arc<dyn SkillServicePort>,
        player_character_service: Arc<dyn PlayerCharacterServicePort>,
        approval_request_lookup: Arc<dyn ApprovalRequestLookupPort>,
        challenge_outcome_approval_service: Arc<dyn ChallengeOutcomeApprovalServicePort>,
        clock: Arc<dyn ClockPort>,
        rng: Arc<dyn RandomPort>,
    ) -> Self {
        Self {
            challenge_service,
            skill_service,
            player_character_service,
            approval_request_lookup,
            challenge_outcome_approval_service,
            clock,
            rng,
        }
    }

    /// Get the current time
    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Gather the common preamble data needed for challenge resolution.
    ///
    /// This extracts the duplicated setup logic from `handle_roll` and `handle_roll_input`:
    /// - Challenge ID parsing and loading
    /// - Player name lookup
    /// - Character modifier lookup
    /// - Character ID resolution
    async fn gather_challenge_preamble(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: &str,
        log_prefix: &str,
    ) -> Result<ChallengePreamble, ChallengeResolutionError> {
        // Parse challenge_id
        let challenge_uuid = uuid::Uuid::parse_str(challenge_id_str)
            .map(ChallengeId::from_uuid)
            .map_err(|_| {
                ChallengeResolutionError::InvalidChallengeId(challenge_id_str.to_string())
            })?;

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return Err(ChallengeResolutionError::ChallengeNotFound(
                    challenge_id_str.to_string(),
                ));
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Err(ChallengeResolutionError::ChallengeLoadFailed(e.to_string()));
            }
        };

        // Fetch skill_id from REQUIRES_SKILL edge
        let skill_id = match self
            .challenge_service
            .get_required_skill(challenge_uuid)
            .await
        {
            Ok(skill_id) => skill_id,
            Err(e) => {
                tracing::warn!(
                    "Failed to get required skill for challenge {}: {}",
                    challenge_uuid,
                    e
                );
                None
            }
        };

        // Get player character to lookup name
        let pc = match self.player_character_service.get_pc(*pc_id).await {
            Ok(Some(pc)) => pc,
            Ok(None) => {
                return Err(ChallengeResolutionError::PlayerCharacterNotFound);
            }
            Err(e) => {
                tracing::error!("Failed to load player character: {}", e);
                return Err(ChallengeResolutionError::PlayerCharacterLoadFailed(
                    e.to_string(),
                ));
            }
        };

        let player_name = pc.name.clone();
        let character_id = pc_id.to_string();

        // Look up character's skill modifier from PlayerCharacterServicePort
        let character_modifier = if let Some(ref sid) = skill_id {
            match self
                .player_character_service
                .get_skill_modifier(*pc_id, *sid)
                .await
            {
                Ok(modifier) => {
                    debug!(
                        pc_id = %pc_id,
                        skill_id = %sid,
                        modifier = modifier,
                        "Found skill modifier for player character ({})", log_prefix
                    );
                    modifier
                }
                Err(e) => {
                    debug!(
                        pc_id = %pc_id,
                        skill_id = %sid,
                        error = %e,
                        "Failed to get skill modifier, defaulting to 0 ({})", log_prefix
                    );
                    0
                }
            }
        } else {
            debug!(
                pc_id = %pc_id,
                "No skill assigned to challenge, defaulting modifier to 0 ({})", log_prefix
            );
            0
        };

        Ok(ChallengePreamble {
            challenge,
            skill_id,
            world_id: *world_id,
            player_name,
            character_modifier,
            character_id,
        })
    }

    /// Internal helper to queue challenge outcome for DM approval.
    ///
    /// All challenge outcomes go through DM approval. This method:
    /// 1. Looks up skill name for display
    /// 2. Builds the pending resolution DTO
    /// 3. Queues for DM approval
    /// 4. Returns the result for broadcasting by the use case layer
    async fn queue_for_approval(
        &self,
        challenge_id_str: &str,
        challenge: &wrldbldr_domain::entities::Challenge,
        skill_id: Option<SkillId>,
        outcome_type: OutcomeType,
        outcome: &wrldbldr_domain::entities::Outcome,
        world_id: WorldId,
        character_id: String,
        player_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        roll_breakdown: Option<String>,
        individual_rolls: Option<Vec<i32>>,
    ) -> Result<RollSubmissionResult, ChallengeResolutionError> {
        // Look up skill name if we have a skill_id
        let skill_name = if let Some(ref sid) = skill_id {
            match self.skill_service.get_skill(*sid).await {
                Ok(Some(skill)) => Some(skill.name),
                Ok(None) => {
                    tracing::warn!("Skill {} not found for challenge {}", sid, challenge_id_str);
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to look up skill {} for challenge {}: {}",
                        sid,
                        challenge_id_str,
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        // Generate resolution ID
        let resolution_id = uuid::Uuid::new_v4().to_string();

        // Convert outcome triggers to info structs
        let outcome_triggers: Vec<OutcomeTriggerInfo> = outcome
            .triggers
            .iter()
            .map(|t| OutcomeTriggerInfo {
                trigger_type: format!("{:?}", t),
                description: String::new(), // Triggers don't have description field
            })
            .collect();

        // Parse character_id into domain type used by ChallengeOutcomeData
        let character_uuid = uuid::Uuid::parse_str(&character_id)
            .ok()
            .map(CharacterId::from)
            .unwrap_or_else(CharacterId::new);

        // Build ChallengeOutcomeData for approval queue (domain type)
        let resolution = ChallengeOutcomeData {
            resolution_id: resolution_id.clone(),
            world_id,
            challenge_id: challenge_id_str.to_string(),
            challenge_name: challenge.name.clone(),
            challenge_description: challenge.description.clone(),
            skill_name: skill_name.clone(),
            character_id: character_uuid,
            character_name: player_name.clone(),
            roll,
            modifier,
            total,
            outcome_type: outcome_type.display_name().to_string(),
            outcome_description: outcome.description.clone(),
            outcome_triggers: outcome
                .triggers
                .iter()
                .map(|t| ProposedTool {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{:?}", t),
                    description: String::new(),
                    arguments: serde_json::json!({}),
                })
                .collect(),
            roll_breakdown: roll_breakdown.clone(),
            timestamp: self.now(),
            suggestions: None,
            is_generating_suggestions: false,
        };

        // Queue for DM approval
        let queued_resolution_id = self
            .challenge_outcome_approval_service
            .queue_for_approval(world_id, resolution)
            .await
            .map_err(|e| ChallengeResolutionError::ApprovalQueueFailed(e.to_string()))?;

        info!(
            resolution_id = %resolution_id,
            challenge_id = %challenge_id_str,
            "Challenge outcome queued for DM approval"
        );

        // Return result for broadcasting by use case layer
        Ok(RollSubmissionResult {
            resolution_id: queued_resolution_id,
            challenge_id: challenge_id_str.to_string(),
            challenge_name: challenge.name.clone(),
            challenge_description: Some(challenge.description.clone()),
            skill_name,
            character_id,
            character_name: player_name,
            roll,
            modifier,
            total,
            outcome_type: outcome_type.display_name().to_string(),
            outcome_description: outcome.description.clone(),
            roll_breakdown,
            individual_rolls,
            outcome_triggers,
        })
    }

    /// Handle a player submitting a challenge roll (legacy method with simple integer roll).
    ///
    /// Returns the roll submission result for broadcasting by the use case layer.
    ///
    /// # Arguments
    /// * `world_id` - The world this challenge belongs to
    /// * `pc_id` - The player character making the roll
    /// * `challenge_id_str` - The challenge ID as a string
    /// * `roll` - The raw dice roll value
    /// * `narrative_config` - Narrative resolution config from the world's rule system.
    /// * `narrative_context` - Optional context for Blades-style resolution (position/effect)
    pub async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: String,
        roll: i32,
        narrative_config: &NarrativeResolutionConfig,
        narrative_context: Option<&NarrativeRollContext>,
    ) -> Result<RollSubmissionResult, ChallengeResolutionError> {
        // Gather common preamble data
        let preamble = self
            .gather_challenge_preamble(world_id, pc_id, &challenge_id_str, "legacy roll")
            .await?;

        // Evaluate challenge result using domain method with narrative support
        let (outcome_type, outcome) = preamble.challenge.evaluate_roll_narrative(
            roll,
            preamble.character_modifier,
            Some(narrative_config),
            narrative_context.and_then(|ctx| ctx.position),
            narrative_context.and_then(|ctx| ctx.effect),
            narrative_context.and_then(|ctx| ctx.dice_results.as_deref()),
        );

        // Queue for DM approval and return result
        self.queue_for_approval(
            &challenge_id_str,
            &preamble.challenge,
            preamble.skill_id,
            outcome_type,
            outcome,
            preamble.world_id,
            preamble.character_id,
            preamble.player_name,
            roll,
            preamble.character_modifier,
            roll + preamble.character_modifier,
            None, // Legacy method doesn't have formula info
            None,
        )
        .await
    }

    /// Handle a player submitting a challenge roll with dice input (formula or manual).
    /// This is the enhanced version that supports dice formulas like "1d20+5".
    ///
    /// Returns the roll submission result for broadcasting by the use case layer.
    ///
    /// # Arguments
    /// * `world_id` - The world this challenge belongs to
    /// * `pc_id` - The player character making the roll
    /// * `challenge_id_str` - The challenge ID as a string
    /// * `dice_input` - The dice input (formula like "1d20+5" or manual value)
    /// * `narrative_config` - Narrative resolution config from the world's rule system.
    /// * `narrative_context` - Optional context for Blades-style resolution (position/effect)
    pub async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: String,
        dice_input: DiceInputType,
        narrative_config: &NarrativeResolutionConfig,
        narrative_context: Option<&NarrativeRollContext>,
    ) -> Result<RollSubmissionResult, ChallengeResolutionError> {
        // Gather common preamble data
        let preamble = self
            .gather_challenge_preamble(world_id, pc_id, &challenge_id_str, "dice input roll")
            .await?;

        // Convert DiceInputType to DiceRollInput
        let roll_input = match dice_input {
            DiceInputType::Formula(formula) => DiceRollInput::Formula(formula),
            DiceInputType::Manual(value) => DiceRollInput::ManualResult(value),
        };

        // Resolve the dice roll with character modifier using injected RNG
        let rng = self.rng.clone();
        let roll_result = roll_input
            .resolve_with_modifier(preamble.character_modifier, |min, max| {
                rng.random_range(min, max)
            })
            .map_err(|e| ChallengeResolutionError::InvalidDiceFormula(e.to_string()))?;

        // For d20 systems, check natural 1/20 using the raw die roll (before modifier)
        let raw_roll = if roll_result.is_manual() {
            roll_result.total // For manual, we use the total as the "roll"
        } else {
            roll_result.dice_total // For formula, use just the dice total
        };

        // Build dice results for Blades-style critical detection
        // Merge individual rolls from the dice formula with any provided in narrative context
        let dice_results = if !roll_result.is_manual() {
            Some(roll_result.individual_rolls.clone())
        } else {
            narrative_context.and_then(|ctx| ctx.dice_results.clone())
        };

        // Evaluate challenge result using domain method with narrative support
        let (outcome_type, outcome) = preamble.challenge.evaluate_roll_narrative(
            raw_roll,
            preamble.character_modifier,
            Some(narrative_config),
            narrative_context.and_then(|ctx| ctx.position),
            narrative_context.and_then(|ctx| ctx.effect),
            dice_results.as_deref(),
        );

        // Queue for DM approval and return result
        self.queue_for_approval(
            &challenge_id_str,
            &preamble.challenge,
            preamble.skill_id,
            outcome_type,
            outcome,
            preamble.world_id,
            preamble.character_id,
            preamble.player_name,
            raw_roll,
            roll_result.modifier_applied,
            roll_result.total,
            Some(roll_result.breakdown()),
            if roll_result.is_manual() {
                None
            } else {
                Some(roll_result.individual_rolls.clone())
            },
        )
        .await
    }

    /// Handle DM-triggered challenges.
    ///
    /// Returns the challenge trigger result for broadcasting by the use case layer.
    pub async fn handle_trigger(
        &self,
        _world_id: &WorldId,
        challenge_id_str: String,
        target_character_id: String,
    ) -> Result<ChallengeTriggerResult, ChallengeResolutionError> {
        // Parse challenge_id
        let challenge_uuid = uuid::Uuid::parse_str(&challenge_id_str)
            .map(ChallengeId::from_uuid)
            .map_err(|_| ChallengeResolutionError::InvalidChallengeId(challenge_id_str.clone()))?;

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return Err(ChallengeResolutionError::ChallengeNotFound(
                    challenge_id_str,
                ));
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Err(ChallengeResolutionError::ChallengeLoadFailed(e.to_string()));
            }
        };

        // Fetch skill_id from REQUIRES_SKILL edge
        let skill_id = match self
            .challenge_service
            .get_required_skill(challenge_uuid)
            .await
        {
            Ok(skill_id) => skill_id,
            Err(e) => {
                tracing::warn!(
                    "Failed to get required skill for challenge {}: {}",
                    challenge_uuid,
                    e
                );
                None
            }
        };

        // Look up skill name from skill service
        let skill_name = if let Some(ref sid) = skill_id {
            match self.skill_service.get_skill(*sid).await {
                Ok(Some(skill)) => skill.name,
                Ok(None) => {
                    tracing::warn!("Skill {} not found for challenge", sid);
                    sid.to_string()
                }
                Err(e) => {
                    tracing::error!("Failed to look up skill {}: {}", sid, e);
                    sid.to_string()
                }
            }
        } else {
            "Unknown Skill".to_string()
        };

        // Look up skill modifier for target character
        let character_modifier = if let Some(ref sid) = skill_id {
            if let Ok(pc_id) =
                uuid::Uuid::parse_str(&target_character_id).map(PlayerCharacterId::from_uuid)
            {
                match self
                    .player_character_service
                    .get_skill_modifier(pc_id, *sid)
                    .await
                {
                    Ok(modifier) => modifier,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get skill modifier for PC {}: {}, using 0",
                            target_character_id,
                            e
                        );
                        0
                    }
                }
            } else {
                tracing::warn!(
                    "Invalid target_character_id format: {}, using modifier 0",
                    target_character_id
                );
                0
            }
        } else {
            0
        };

        // Get suggested dice based on difficulty type using domain method
        let (dice, hint) = challenge.difficulty.dice_suggestion();
        let (suggested_dice, rule_system_hint) = (dice.to_string(), hint.to_string());

        info!(
            challenge_id = %challenge_id_str,
            target_character_id = %target_character_id,
            "Challenge triggered for player"
        );

        Ok(ChallengeTriggerResult {
            challenge_id: challenge_id_str,
            challenge_name: challenge.name.clone(),
            skill_name,
            difficulty_display: challenge.difficulty.display(),
            description: challenge.description.clone(),
            character_modifier,
            suggested_dice,
            rule_system_hint,
        })
    }

    /// Handle DM approval/rejection of a challenge suggestion.
    ///
    /// Returns the trigger result if approved, for broadcasting by the use case layer.
    pub async fn handle_suggestion_decision(
        &self,
        _world_id: &WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Result<Option<ChallengeTriggerResult>, ChallengeResolutionError> {
        if !approved {
            info!(request_id = %request_id, "DM rejected challenge suggestion");
            return Ok(None);
        }

        // Look up the approval request payload
        let approval_request = self
            .approval_request_lookup
            .get_by_id(&request_id)
            .await
            .map_err(|e| ChallengeResolutionError::ApprovalLookupError(e.to_string()))?
            .ok_or_else(|| {
                ChallengeResolutionError::ApprovalLookupError(format!(
                    "Approval request {} not found",
                    request_id
                ))
            })?;

        // Get challenge suggestion from the approval request
        let challenge_suggestion =
            approval_request
                .challenge_suggestion
                .as_ref()
                .ok_or_else(|| {
                    ChallengeResolutionError::ChallengeSuggestionNotFound(request_id.clone())
                })?;

        // Parse challenge ID
        let challenge_uuid = uuid::Uuid::parse_str(&challenge_suggestion.challenge_id)
            .map(ChallengeId::from_uuid)
            .map_err(|_| {
                ChallengeResolutionError::InvalidChallengeId(
                    challenge_suggestion.challenge_id.clone(),
                )
            })?;

        // Load challenge
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Err(ChallengeResolutionError::ChallengeNotFound(
                    challenge_suggestion.challenge_id.clone(),
                ));
            }
            Err(e) => {
                return Err(ChallengeResolutionError::ChallengeLoadFailed(e.to_string()));
            }
        };

        // Fetch skill_id from REQUIRES_SKILL edge
        let skill_id = match self
            .challenge_service
            .get_required_skill(challenge_uuid)
            .await
        {
            Ok(skill_id) => skill_id,
            Err(e) => {
                tracing::warn!(
                    "Failed to get required skill for challenge {}: {}",
                    challenge_uuid,
                    e
                );
                None
            }
        };

        let difficulty_display =
            modified_difficulty.unwrap_or_else(|| challenge.difficulty.display());

        // Look up skill modifier for target character if available
        let character_modifier = if let Some(ref sid) = skill_id {
            // target_pc_id is already a PlayerCharacterId in domain type
            if let Some(pc_id) = challenge_suggestion.target_pc_id {
                match self
                    .player_character_service
                    .get_skill_modifier(pc_id, *sid)
                    .await
                {
                    Ok(modifier) => modifier,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get skill modifier for PC {}: {}, using 0",
                            pc_id,
                            e
                        );
                        0
                    }
                }
            } else {
                tracing::debug!("No valid target_pc_id in challenge suggestion, using modifier 0");
                0
            }
        } else {
            0
        };

        // Get suggested dice based on difficulty type using domain method
        let (dice, hint) = challenge.difficulty.dice_suggestion();
        let (suggested_dice, rule_system_hint) = (dice.to_string(), hint.to_string());

        info!(
            challenge_name = %challenge.name,
            request_id = %request_id,
            "Challenge suggestion approved, triggering challenge"
        );

        Ok(Some(ChallengeTriggerResult {
            challenge_id: challenge_suggestion.challenge_id.clone(),
            challenge_name: challenge.name.clone(),
            skill_name: challenge_suggestion.skill_name.clone(),
            difficulty_display,
            description: challenge.description.clone(),
            character_modifier,
            suggested_dice,
            rule_system_hint,
        }))
    }

    /// Handle DM creating an ad-hoc challenge (no LLM involved)
    ///
    /// Returns both the created challenge result and the trigger result for broadcasting.
    pub async fn handle_adhoc_challenge(
        &self,
        _world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: String,
        outcomes: AdHocOutcomes,
    ) -> Result<(AdHocChallengeResult, ChallengeTriggerResult), ChallengeResolutionError> {
        // Generate a temporary challenge ID for this ad-hoc challenge
        let adhoc_challenge_id = uuid::Uuid::new_v4().to_string();

        info!(
            challenge_name = %challenge_name,
            target_pc_id = %target_pc_id,
            difficulty = %difficulty,
            "DM created ad-hoc challenge"
        );

        // Parse difficulty string using domain logic
        let parsed_difficulty = Difficulty::parse(&difficulty);
        let (dice, hint) = parsed_difficulty.dice_suggestion();
        let (suggested_dice, rule_system_hint) = (dice.to_string(), hint.to_string());

        // Build the ad-hoc challenge result for the DM
        let adhoc_result = AdHocChallengeResult {
            challenge_id: adhoc_challenge_id.clone(),
            challenge_name: challenge_name.clone(),
            target_pc_id: target_pc_id.clone(),
            outcomes,
        };

        // Build the trigger result for broadcasting to the player
        let trigger_result = ChallengeTriggerResult {
            challenge_id: adhoc_challenge_id,
            challenge_name,
            skill_name,
            difficulty_display: difficulty,
            description: "Ad-hoc challenge created by DM".to_string(),
            character_modifier: 0, // DM would need to specify this
            suggested_dice,
            rule_system_hint,
        };

        Ok((adhoc_result, trigger_result))
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

/// Implementation of the `ChallengeResolutionServicePort` for `ChallengeResolutionService`.
///
/// This exposes challenge resolution methods to infrastructure adapters.
#[async_trait]
impl ChallengeResolutionServicePort for ChallengeResolutionService {
    async fn start_resolution(
        &self,
        challenge_id: ChallengeId,
        pc_id: PlayerCharacterId,
    ) -> anyhow::Result<String> {
        // Generate a resolution ID for tracking
        let resolution_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            resolution_id = %resolution_id,
            challenge_id = %challenge_id,
            pc_id = %pc_id,
            "Started challenge resolution"
        );

        Ok(resolution_id)
    }

    async fn submit_roll(
        &self,
        resolution_id: String,
        roll: PortDiceRoll,
    ) -> anyhow::Result<PortRollResult> {
        // Parse resolution_id to extract challenge info
        // In a full implementation, we'd store pending resolutions
        // For now, return a placeholder indicating the roll was received
        Ok(PortRollResult {
            resolution_id,
            challenge_id: String::new(),
            challenge_name: String::new(),
            challenge_description: None,
            skill_name: None,
            character_id: String::new(),
            character_name: String::new(),
            roll: roll.roll,
            modifier: 0,
            total: roll.roll,
            outcome_type: "pending".to_string(),
            outcome_description: "Awaiting resolution".to_string(),
            roll_breakdown: roll.breakdown,
            individual_rolls: roll.individual_rolls,
        })
    }

    async fn get_pending_resolution(
        &self,
        _pc_id: PlayerCharacterId,
    ) -> anyhow::Result<Option<PortPendingResolution>> {
        // In a full implementation, this would query pending resolutions for the PC
        Ok(None)
    }

    async fn handle_roll(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
        narrative_config: NarrativeResolutionConfig,
        narrative_context: Option<PortNarrativeRollContext>,
    ) -> anyhow::Result<PortRollResultData> {
        // Convert port context to internal context
        let internal_context = narrative_context.map(|ctx| NarrativeRollContext {
            position: ctx.position,
            effect: ctx.effect,
            dice_results: ctx.dice_results.clone(),
        });

        let result = ChallengeResolutionService::handle_roll(
            self,
            &world_id,
            &pc_id,
            challenge_id,
            roll,
            &narrative_config,
            internal_context.as_ref(),
        )
        .await?;

        Ok(PortRollResultData {
            resolution_id: result.resolution_id,
            challenge_id: result.challenge_id,
            challenge_name: result.challenge_name,
            character_id: result.character_id,
            character_name: result.character_name,
            roll: result.roll,
            modifier: result.modifier,
            total: result.total,
            outcome_type: result.outcome_type,
            outcome_description: result.outcome_description,
            roll_breakdown: result.roll_breakdown,
            individual_rolls: result.individual_rolls,
            triggers: result
                .outcome_triggers
                .into_iter()
                .map(|t| PortOutcomeTriggerInfo {
                    trigger_type: t.trigger_type,
                    description: t.description,
                })
                .collect(),
            pending_approval: true,
        })
    }

    async fn handle_roll_input(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
        narrative_config: NarrativeResolutionConfig,
        narrative_context: Option<PortNarrativeRollContext>,
    ) -> anyhow::Result<PortRollResultData> {
        // Convert port context to internal context
        let internal_context = narrative_context.map(|ctx| NarrativeRollContext {
            position: ctx.position,
            effect: ctx.effect,
            dice_results: ctx.dice_results.clone(),
        });

        let result = ChallengeResolutionService::handle_roll_input(
            self,
            &world_id,
            &pc_id,
            challenge_id,
            input_type,
            &narrative_config,
            internal_context.as_ref(),
        )
        .await?;

        Ok(PortRollResultData {
            resolution_id: result.resolution_id,
            challenge_id: result.challenge_id,
            challenge_name: result.challenge_name,
            character_id: result.character_id,
            character_name: result.character_name,
            roll: result.roll,
            modifier: result.modifier,
            total: result.total,
            outcome_type: result.outcome_type,
            outcome_description: result.outcome_description,
            roll_breakdown: result.roll_breakdown,
            individual_rolls: result.individual_rolls,
            triggers: result
                .outcome_triggers
                .into_iter()
                .map(|t| PortOutcomeTriggerInfo {
                    trigger_type: t.trigger_type,
                    description: t.description,
                })
                .collect(),
            pending_approval: true,
        })
    }

    async fn trigger_challenge(
        &self,
        world_id: WorldId,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> anyhow::Result<PortTriggerResult> {
        let result = ChallengeResolutionService::handle_trigger(
            self,
            &world_id,
            challenge_id,
            target_character_id.to_string(),
        )
        .await?;

        Ok(PortTriggerResult {
            challenge_id: result.challenge_id,
            challenge_name: result.challenge_name,
            skill_name: result.skill_name,
            difficulty_display: result.difficulty_display,
            description: result.description,
            character_modifier: result.character_modifier,
            suggested_dice: result.suggested_dice,
            rule_system_hint: result.rule_system_hint,
        })
    }

    async fn handle_suggestion_decision(
        &self,
        world_id: WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> anyhow::Result<()> {
        ChallengeResolutionService::handle_suggestion_decision(
            self,
            &world_id,
            request_id,
            approved,
            modified_difficulty,
        )
        .await?;
        Ok(())
    }

    async fn create_adhoc_challenge(
        &self,
        world_id: WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        outcomes: AdHocOutcomes,
    ) -> anyhow::Result<PortAdHocResult> {
        let (adhoc_result, _trigger_result) = ChallengeResolutionService::handle_adhoc_challenge(
            self,
            &world_id,
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id.to_string(),
            outcomes.clone(),
        )
        .await?;

        Ok(PortAdHocResult {
            challenge_id: adhoc_result.challenge_id,
        })
    }
}
