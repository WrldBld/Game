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

use std::sync::Arc;

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::application::dto::AdHocOutcomesDto;
use wrldbldr_engine_ports::outbound::{ApprovalQueuePort, ClockPort};
use crate::application::dto::{OutcomeTriggerRequestDto, PendingChallengeResolutionDto};
use crate::application::services::{
    ChallengeOutcomeApprovalService, ChallengeService, DMApprovalQueueService, ItemService,
    PlayerCharacterService, SkillService,
};
use wrldbldr_domain::entities::OutcomeType;
use wrldbldr_domain::value_objects::DiceRollInput;
use wrldbldr_domain::{ChallengeId, PlayerCharacterId, WorldId, SkillId};
use tracing::{debug, info};

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
    pub outcomes: AdHocOutcomesDto,
}

/// Dice input type for challenge rolls
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum DiceInputType {
    #[serde(rename = "formula")]
    Formula(String),
    #[serde(rename = "manual")]
    Manual(i32),
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

use wrldbldr_engine_ports::outbound::LlmPort;

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
///
/// Generic over `L: LlmPort` for LLM-powered suggestion generation via the approval service.
/// Generic over `I: ItemService` for item operations in the DM approval queue.
pub struct ChallengeResolutionService<S: ChallengeService, K: SkillService, Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>, P: PlayerCharacterService, L: LlmPort, I: ItemService> {
    challenge_service: Arc<S>,
    skill_service: Arc<K>,
    player_character_service: Arc<P>,
    dm_approval_queue_service: Arc<DMApprovalQueueService<Q, I>>,
    challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L>>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl<S, K, Q, P, L, I> ChallengeResolutionService<S, K, Q, P, L, I>
where
    S: ChallengeService,
    K: SkillService,
    Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>,
    P: PlayerCharacterService,
    L: LlmPort + 'static,
    I: ItemService,
{
    /// Create a new challenge resolution service
    ///
    /// All challenges are routed through the approval service for DM review.
    ///
    /// # Arguments
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    pub fn new(
        challenge_service: Arc<S>,
        skill_service: Arc<K>,
        player_character_service: Arc<P>,
        dm_approval_queue_service: Arc<DMApprovalQueueService<Q, I>>,
        challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L>>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            challenge_service,
            skill_service,
            player_character_service,
            dm_approval_queue_service,
            challenge_outcome_approval_service,
            clock,
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
            .map_err(|_| ChallengeResolutionError::InvalidChallengeId(challenge_id_str.to_string()))?;

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return Err(ChallengeResolutionError::ChallengeNotFound(challenge_id_str.to_string()));
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Err(ChallengeResolutionError::ChallengeLoadFailed(e.to_string()));
            }
        };

        // Fetch skill_id from REQUIRES_SKILL edge
        let skill_id = match self.challenge_service.get_required_skill(challenge_uuid).await {
            Ok(skill_id) => skill_id,
            Err(e) => {
                tracing::warn!("Failed to get required skill for challenge {}: {}", challenge_uuid, e);
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
                return Err(ChallengeResolutionError::PlayerCharacterLoadFailed(e.to_string()));
            }
        };

        let player_name = pc.name.clone();
        let character_id = pc_id.to_string();

        // Look up character's skill modifier from PlayerCharacterService
        let character_modifier = if let Some(ref sid) = skill_id {
            match self
                .player_character_service
                .get_skill_modifier(*pc_id, sid.clone())
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
            match self.skill_service.get_skill(sid.clone()).await {
                Ok(Some(skill)) => Some(skill.name),
                Ok(None) => {
                    tracing::warn!("Skill {} not found for challenge {}", sid, challenge_id_str);
                    None
                }
                Err(e) => {
                    tracing::warn!("Failed to look up skill {} for challenge {}: {}", sid, challenge_id_str, e);
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

        // Build PendingChallengeResolutionDto for approval queue
        let resolution = PendingChallengeResolutionDto {
            resolution_id: resolution_id.clone(),
            challenge_id: challenge_id_str.to_string(),
            challenge_name: challenge.name.clone(),
            challenge_description: challenge.description.clone(),
            skill_name: skill_name.clone(),
            character_id: character_id.clone(),
            character_name: player_name.clone(),
            roll,
            modifier,
            total,
            outcome_type: outcome_type.display_name().to_string(),
            outcome_description: outcome.description.clone(),
            outcome_triggers: outcome
                .triggers
                .iter()
                .cloned()
                .map(OutcomeTriggerRequestDto::from)
                .collect(),
            roll_breakdown: roll_breakdown.clone(),
            individual_rolls: individual_rolls.clone(),
            timestamp: self.now().to_rfc3339(),
        };

        // Queue for DM approval
        self.challenge_outcome_approval_service
            .queue_for_approval(&world_id, resolution)
            .await
            .map_err(|e| ChallengeResolutionError::ApprovalQueueFailed(e.to_string()))?;

        info!(
            resolution_id = %resolution_id,
            challenge_id = %challenge_id_str,
            "Challenge outcome queued for DM approval"
        );

        // Return result for broadcasting by use case layer
        Ok(RollSubmissionResult {
            resolution_id,
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
    pub async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: String,
        roll: i32,
    ) -> Result<RollSubmissionResult, ChallengeResolutionError> {
        // Gather common preamble data
        let preamble = self
            .gather_challenge_preamble(world_id, pc_id, &challenge_id_str, "legacy roll")
            .await?;

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&preamble.challenge, roll, preamble.character_modifier);

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
    pub async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: String,
        dice_input: DiceInputType,
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

        // Resolve the dice roll with character modifier
        let roll_result = roll_input
            .resolve_with_modifier(preamble.character_modifier)
            .map_err(|e| ChallengeResolutionError::InvalidDiceFormula(e.to_string()))?;

        // For d20 systems, check natural 1/20 using the raw die roll (before modifier)
        let raw_roll = if roll_result.is_manual() {
            roll_result.total // For manual, we use the total as the "roll"
        } else {
            roll_result.dice_total // For formula, use just the dice total
        };

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&preamble.challenge, raw_roll, preamble.character_modifier);

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
                return Err(ChallengeResolutionError::ChallengeNotFound(challenge_id_str));
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Err(ChallengeResolutionError::ChallengeLoadFailed(e.to_string()));
            }
        };

        // Fetch skill_id from REQUIRES_SKILL edge
        let skill_id = match self.challenge_service.get_required_skill(challenge_uuid).await {
            Ok(skill_id) => skill_id,
            Err(e) => {
                tracing::warn!("Failed to get required skill for challenge {}: {}", challenge_uuid, e);
                None
            }
        };

        // Look up skill name from skill service
        let skill_name = if let Some(ref sid) = skill_id {
            match self.skill_service.get_skill(sid.clone()).await {
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
            if let Ok(pc_id) = uuid::Uuid::parse_str(&target_character_id)
                .map(PlayerCharacterId::from_uuid)
            {
                match self.player_character_service
                    .get_skill_modifier(pc_id, sid.clone())
                    .await
                {
                    Ok(modifier) => modifier,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get skill modifier for PC {}: {}, using 0",
                            target_character_id, e
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

        // Get suggested dice based on difficulty type
        let (suggested_dice, rule_system_hint) = get_dice_suggestion_for_challenge(&challenge);

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

        // Look up the approval item
        let approval_item = self.dm_approval_queue_service
            .get_by_id(&request_id)
            .await
            .map_err(|e| ChallengeResolutionError::ApprovalLookupError(e.to_string()))?
            .ok_or_else(|| ChallengeResolutionError::ApprovalLookupError(format!("Approval request {} not found", request_id)))?;

        // Get challenge suggestion from the approval item
        let challenge_suggestion = approval_item.payload.challenge_suggestion
            .as_ref()
            .ok_or_else(|| ChallengeResolutionError::ChallengeSuggestionNotFound(request_id.clone()))?;

        // Parse challenge ID
        let challenge_uuid = uuid::Uuid::parse_str(&challenge_suggestion.challenge_id)
            .map(ChallengeId::from_uuid)
            .map_err(|_| ChallengeResolutionError::InvalidChallengeId(challenge_suggestion.challenge_id.clone()))?;

        // Load challenge
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Err(ChallengeResolutionError::ChallengeNotFound(challenge_suggestion.challenge_id.clone()));
            }
            Err(e) => {
                return Err(ChallengeResolutionError::ChallengeLoadFailed(e.to_string()));
            }
        };

        // Fetch skill_id from REQUIRES_SKILL edge
        let skill_id = match self.challenge_service.get_required_skill(challenge_uuid).await {
            Ok(skill_id) => skill_id,
            Err(e) => {
                tracing::warn!("Failed to get required skill for challenge {}: {}", challenge_uuid, e);
                None
            }
        };

        let difficulty_display = modified_difficulty
            .unwrap_or_else(|| challenge.difficulty.display());

        // Look up skill modifier for target character if available
        let character_modifier = if let Some(ref sid) = skill_id {
            if let Some(ref pc_id_str) = challenge_suggestion.target_pc_id {
                if let Ok(pc_id) = uuid::Uuid::parse_str(pc_id_str)
                    .map(PlayerCharacterId::from_uuid)
                {
                    match self.player_character_service
                        .get_skill_modifier(pc_id, sid.clone())
                        .await
                    {
                        Ok(modifier) => modifier,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get skill modifier for PC {}: {}, using 0",
                                pc_id_str, e
                            );
                            0
                        }
                    }
                } else {
                    tracing::warn!(
                        "Invalid target_pc_id format: {}, using modifier 0",
                        pc_id_str
                    );
                    0
                }
            } else {
                tracing::debug!("No target_pc_id in challenge suggestion, using modifier 0");
                0
            }
        } else {
            0
        };

        // Get suggested dice based on difficulty type
        let (suggested_dice, rule_system_hint) = get_dice_suggestion_for_challenge(&challenge);

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
        outcomes: AdHocOutcomesDto,
    ) -> Result<(AdHocChallengeResult, ChallengeTriggerResult), ChallengeResolutionError> {
        // Generate a temporary challenge ID for this ad-hoc challenge
        let adhoc_challenge_id = uuid::Uuid::new_v4().to_string();

        info!(
            challenge_name = %challenge_name,
            target_pc_id = %target_pc_id,
            difficulty = %difficulty,
            "DM created ad-hoc challenge"
        );

        // Determine suggested dice from difficulty string
        let (suggested_dice, rule_system_hint) = if difficulty.to_uppercase().starts_with("DC") {
            ("1d20".to_string(), "Roll 1d20 and add your modifier".to_string())
        } else if difficulty.ends_with('%') {
            ("1d100".to_string(), "Roll percentile dice".to_string())
        } else {
            ("2d6".to_string(), "Roll 2d6 and add your modifier".to_string())
        };

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

/// Get suggested dice and rule system hint based on challenge difficulty type.
fn get_dice_suggestion_for_challenge(
    challenge: &wrldbldr_domain::entities::Challenge,
) -> (String, String) {
    match &challenge.difficulty {
        wrldbldr_domain::entities::Difficulty::DC(_) => {
            // D20 systems (D&D, Pathfinder, etc.)
            (
                "1d20".to_string(),
                "Roll 1d20 and add your skill modifier".to_string(),
            )
        }
        wrldbldr_domain::entities::Difficulty::Percentage(_) => {
            // Percentile systems (Call of Cthulhu, etc.)
            (
                "1d100".to_string(),
                "Roll percentile dice (1d100), lower is better".to_string(),
            )
        }
        wrldbldr_domain::entities::Difficulty::Descriptor(desc) => {
            // Narrative systems - suggest 2d6 for PbtA-style games
            (
                "2d6".to_string(),
                format!("Roll 2d6 for {} difficulty", desc.display_name()),
            )
        }
        wrldbldr_domain::entities::Difficulty::Opposed => {
            // Opposed rolls - both parties roll
            (
                "1d20".to_string(),
                "Opposed roll - both parties roll and compare".to_string(),
            )
        }
        wrldbldr_domain::entities::Difficulty::Custom(desc) => {
            // Custom difficulty - let the hint explain
            (
                "1d20".to_string(),
                format!("Custom difficulty: {}", desc),
            )
        }
    }
}

/// Evaluate a challenge roll result (moved from websocket.rs)
fn evaluate_challenge_result(
    challenge: &wrldbldr_domain::entities::Challenge,
    roll: i32,
    modifier: i32,
) -> (OutcomeType, &wrldbldr_domain::entities::Outcome) {
    let total = roll + modifier;

    match &challenge.difficulty {
        wrldbldr_domain::entities::Difficulty::DC(dc) => {
            if roll == 20 {
                if let Some(ref critical_success) = challenge.outcomes.critical_success {
                    return (OutcomeType::CriticalSuccess, critical_success);
                }
            }
            if roll == 1 {
                if let Some(ref critical_failure) = challenge.outcomes.critical_failure {
                    return (OutcomeType::CriticalFailure, critical_failure);
                }
            }

            if total >= *dc as i32 {
                (OutcomeType::Success, &challenge.outcomes.success)
            } else {
                (OutcomeType::Failure, &challenge.outcomes.failure)
            }
        }
        wrldbldr_domain::entities::Difficulty::Percentage(target) => {
            if roll == 1 {
                if let Some(ref critical_success) = challenge.outcomes.critical_success {
                    return (OutcomeType::CriticalSuccess, critical_success);
                }
            }
            if roll == 100 {
                if let Some(ref critical_failure) = challenge.outcomes.critical_failure {
                    return (OutcomeType::CriticalFailure, critical_failure);
                }
            }

            if roll <= *target as i32 {
                (OutcomeType::Success, &challenge.outcomes.success)
            } else {
                (OutcomeType::Failure, &challenge.outcomes.failure)
            }
        }
        wrldbldr_domain::entities::Difficulty::Descriptor(_) => {
            if roll >= 11 {
                (OutcomeType::Success, &challenge.outcomes.success)
            } else {
                (OutcomeType::Failure, &challenge.outcomes.failure)
            }
        }
        wrldbldr_domain::entities::Difficulty::Opposed => {
            (OutcomeType::Success, &challenge.outcomes.success)
        }
        wrldbldr_domain::entities::Difficulty::Custom(_) => {
            (OutcomeType::Success, &challenge.outcomes.success)
        }
    }
}


