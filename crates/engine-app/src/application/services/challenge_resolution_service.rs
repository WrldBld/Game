//! Challenge resolution service - encapsulates challenge roll handling, DM-triggered
//! challenges, and challenge suggestion approvals.
//!
//! This moves challenge-related business logic out of the websocket handler into a
//! dedicated application service, keeping the transport layer thin.
//!
//! Uses `WorldConnectionPort` for world-scoped connection management, maintaining hexagonal architecture.

use std::sync::Arc;

use crate::application::dto::AdHocOutcomesDto;
use wrldbldr_protocol::AppEvent;
use wrldbldr_engine_ports::outbound::WorldConnectionPort;
use wrldbldr_engine_ports::outbound::EventBusPort;
use wrldbldr_engine_ports::outbound::ApprovalQueuePort;
use crate::application::dto::{OutcomeTriggerRequestDto, PendingChallengeResolutionDto};
use crate::application::services::{
    ChallengeOutcomeApprovalService, ChallengeService, DMApprovalQueueService, ItemService, OutcomeTriggerService,
    PlayerCharacterService, SkillService,
};
use wrldbldr_domain::entities::OutcomeType;
use wrldbldr_domain::value_objects::DiceRollInput;
use wrldbldr_domain::{ChallengeId, PlayerCharacterId, WorldId, SkillId};
use tracing::{debug, info};

/// Dice input type for challenge rolls
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum DiceInputType {
    #[serde(rename = "formula")]
    Formula(String),
    #[serde(rename = "manual")]
    Manual(i32),
}

/// Challenge resolved message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct ChallengeResolvedMessage {
    r#type: &'static str,
    challenge_id: String,
    challenge_name: String,
    character_name: String,
    roll: i32,
    modifier: i32,
    total: i32,
    outcome: String,
    outcome_description: String,
    roll_breakdown: Option<String>,
    individual_rolls: Option<Vec<i32>>,
}

/// Challenge prompt message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct ChallengePromptMessage {
    r#type: &'static str,
    challenge_id: String,
    challenge_name: String,
    skill_name: String,
    difficulty_display: String,
    description: String,
    character_modifier: i32,
    suggested_dice: Option<String>,
    rule_system_hint: Option<String>,
}

/// Error message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct ErrorMessage {
    r#type: &'static str,
    code: String,
    message: String,
}

/// Ad-hoc challenge created message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct AdHocChallengeCreatedMessage {
    r#type: &'static str,
    challenge_id: String,
    challenge_name: String,
    target_pc_id: String,
    outcomes: AdHocOutcomesDto,
}

/// Helper function to create error messages
fn error_message(code: &str, message: &str) -> Option<serde_json::Value> {
    let error_msg = ErrorMessage {
        r#type: "Error",
        code: code.to_string(),
        message: message.to_string(),
    };
    serde_json::to_value(&error_msg).ok()
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
/// This service uses `AsyncSessionPort` for all session operations, maintaining
/// hexagonal architecture compliance. Session lookups and broadcasts go through
/// the port trait rather than concrete infrastructure types.
///
/// Generic over `L: LlmPort` for LLM-powered suggestion generation via the approval service.
/// Generic over `I: ItemService` for item operations in the DM approval queue.
pub struct ChallengeResolutionService<S: ChallengeService, K: SkillService, Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>, P: PlayerCharacterService, L: LlmPort, I: ItemService> {
    world_connection: Arc<dyn WorldConnectionPort>,
    challenge_service: Arc<S>,
    skill_service: Arc<K>,
    player_character_service: Arc<P>,
    event_bus: Arc<dyn EventBusPort<AppEvent>>,
    dm_approval_queue_service: Arc<DMApprovalQueueService<Q, I>>,
    outcome_trigger_service: Arc<OutcomeTriggerService>,
    challenge_outcome_approval_service: Option<Arc<ChallengeOutcomeApprovalService<L>>>,
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
    pub fn new(
        world_connection: Arc<dyn WorldConnectionPort>,
        challenge_service: Arc<S>,
        skill_service: Arc<K>,
        player_character_service: Arc<P>,
        event_bus: Arc<dyn EventBusPort<AppEvent>>,
        dm_approval_queue_service: Arc<DMApprovalQueueService<Q, I>>,
        outcome_trigger_service: Arc<OutcomeTriggerService>,
    ) -> Self {
        Self {
            world_connection,
            challenge_service,
            skill_service,
            player_character_service,
            event_bus,
            dm_approval_queue_service,
            outcome_trigger_service,
            challenge_outcome_approval_service: None,
        }
    }

    /// Set the challenge outcome approval service for P3.3 DM approval workflow
    pub fn with_outcome_approval_service(
        mut self,
        service: Arc<ChallengeOutcomeApprovalService<L>>,
    ) -> Self {
        self.challenge_outcome_approval_service = Some(service);
        self
    }

    /// Gather the common preamble data needed for challenge resolution.
    ///
    /// This extracts the duplicated setup logic from `handle_roll` and `handle_roll_input`:
    /// - Challenge ID parsing and loading
    /// - Player name lookup
    /// - Character modifier lookup
    /// - Character ID resolution
    ///
    /// Returns `Ok(preamble)` on success, or `Err(error_message)` on failure.
    async fn gather_challenge_preamble(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: &str,
        log_prefix: &str,
    ) -> Result<ChallengePreamble, Option<serde_json::Value>> {
        // Parse challenge_id
        let challenge_uuid = match uuid::Uuid::parse_str(challenge_id_str) {
            Ok(uuid) => ChallengeId::from_uuid(uuid),
            Err(_) => {
                return Err(error_message("INVALID_CHALLENGE_ID", "Invalid challenge ID format"));
            }
        };

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return Err(error_message(
                    "CHALLENGE_NOT_FOUND",
                    &format!("Challenge {} not found", challenge_id_str),
                ));
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Err(error_message("CHALLENGE_LOAD_ERROR", "Failed to load challenge"));
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
                return Err(error_message(
                    "PLAYER_CHARACTER_NOT_FOUND",
                    "Player character not found",
                ));
            }
            Err(e) => {
                tracing::error!("Failed to load player character: {}", e);
                return Err(error_message("PLAYER_CHARACTER_LOAD_ERROR", "Failed to load player character"));
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

    /// Internal helper to resolve challenge outcome and broadcast results.
    ///
    /// This handles the common logic shared between `handle_roll()` and `handle_roll_input()`:
    /// 1. If world has DM and approval service is configured, queue for DM approval (P3.3)
    /// 2. Otherwise: Publishes AppEvent, executes triggers, broadcasts ChallengeResolved
    async fn resolve_challenge_internal(
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
    ) {
        // P3.3: If world has DM and approval service is configured, queue for approval
        if self.world_connection.has_dm(&world_id) {
            if let Some(ref approval_service) = self.challenge_outcome_approval_service {
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

                // Build PendingChallengeResolutionDto for approval queue
                let resolution = PendingChallengeResolutionDto {
                    resolution_id: uuid::Uuid::new_v4().to_string(),
                    challenge_id: challenge_id_str.to_string(),
                    challenge_name: challenge.name.clone(),
                    challenge_description: challenge.description.clone(),
                    skill_name,
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
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                match approval_service.queue_for_approval(&world_id, resolution).await {
                    Ok(resolution_id) => {
                        info!(
                            resolution_id = %resolution_id,
                            challenge_id = %challenge_id_str,
                            "Challenge outcome queued for DM approval"
                        );
                        // Return early - don't broadcast yet, DM will approve
                        return;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to queue challenge for DM approval: {}, falling back to immediate broadcast",
                            e
                        );
                        // Fall through to immediate broadcast
                    }
                }
            }
        }

        // No DM or approval service not configured - immediate resolution
        let success =
            outcome_type == OutcomeType::Success || outcome_type == OutcomeType::CriticalSuccess;

        // Publish AppEvent for challenge resolution
        let app_event = AppEvent::ChallengeResolved {
            challenge_id: Some(challenge_id_str.to_string()),
            challenge_name: challenge.name.clone(),
            world_id: world_id.to_string(),
            character_id: character_id.clone(),
            success,
            roll: Some(roll),
            total: Some(total),
            session_id: None, // No longer session-based
        };
        if let Err(e) = self.event_bus.publish(app_event).await {
            tracing::error!("Failed to publish ChallengeResolved event: {}", e);
        }

        // Execute outcome triggers
        let trigger_result = self
            .outcome_trigger_service
            .execute_triggers(&outcome.triggers, world_id)
            .await;

        if !trigger_result.warnings.is_empty() {
            info!(
                trigger_count = trigger_result.trigger_count,
                warnings = ?trigger_result.warnings,
                "Outcome triggers executed with warnings"
            );
        }

        // Broadcast ChallengeResolved to all participants
        let message = wrldbldr_protocol::ServerMessage::ChallengeResolved {
            challenge_id: challenge_id_str.to_string(),
            challenge_name: challenge.name.clone(),
            character_name: player_name,
            roll,
            modifier,
            total,
            outcome: outcome_type.display_name().to_string(),
            outcome_description: outcome.description.clone(),
            roll_breakdown,
            individual_rolls,
        };
        if let Err(e) = self
            .world_connection
            .broadcast_to_world(&world_id, message)
            .await
        {
            tracing::error!("Failed to broadcast ChallengeResolved: {}", e);
        }
    }

    /// Handle a player submitting a challenge roll (legacy method with simple integer roll).
    pub async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: String,
        roll: i32,
    ) -> Option<serde_json::Value> {
        // Gather common preamble data
        let preamble = match self
            .gather_challenge_preamble(world_id, pc_id, &challenge_id_str, "legacy roll")
            .await
        {
            Ok(p) => p,
            Err(err_msg) => return err_msg,
        };

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&preamble.challenge, roll, preamble.character_modifier);

        // Use common helper to publish events, execute triggers, and broadcast
        self.resolve_challenge_internal(
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
        .await;

        None
    }

    /// Handle a player submitting a challenge roll with dice input (formula or manual).
    /// This is the enhanced version that supports dice formulas like "1d20+5".
    pub async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: &PlayerCharacterId,
        challenge_id_str: String,
        dice_input: DiceInputType,
    ) -> Option<serde_json::Value> {
        // Gather common preamble data
        let preamble = match self
            .gather_challenge_preamble(world_id, pc_id, &challenge_id_str, "dice input roll")
            .await
        {
            Ok(p) => p,
            Err(err_msg) => return err_msg,
        };

        // Convert DiceInputType to DiceRollInput
        let roll_input = match dice_input {
            DiceInputType::Formula(formula) => DiceRollInput::Formula(formula),
            DiceInputType::Manual(value) => DiceRollInput::ManualResult(value),
        };

        // Resolve the dice roll with character modifier
        let roll_result = match roll_input.resolve_with_modifier(preamble.character_modifier) {
            Ok(result) => result,
            Err(e) => {
                return error_message("INVALID_DICE_FORMULA", &format!("Invalid dice formula: {}", e));
            }
        };

        // For d20 systems, check natural 1/20 using the raw die roll (before modifier)
        let raw_roll = if roll_result.is_manual() {
            roll_result.total // For manual, we use the total as the "roll"
        } else {
            roll_result.dice_total // For formula, use just the dice total
        };

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&preamble.challenge, raw_roll, preamble.character_modifier);

        // Use common helper to publish events, execute triggers, and broadcast
        self.resolve_challenge_internal(
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
        .await;

        None
    }

    /// Handle DM-triggered challenges.
    pub async fn handle_trigger(
        &self,
        world_id: &WorldId,
        challenge_id_str: String,
        target_character_id: String,
    ) -> Option<serde_json::Value> {

        // Parse challenge_id
        let challenge_uuid = match uuid::Uuid::parse_str(&challenge_id_str) {
            Ok(uuid) => ChallengeId::from_uuid(uuid),
            Err(_) => {
                return error_message("INVALID_CHALLENGE_ID", "Invalid challenge ID format");
            }
        };

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return error_message("CHALLENGE_NOT_FOUND", &format!("Challenge {} not found", challenge_id_str));
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return error_message("CHALLENGE_LOAD_ERROR", "Failed to load challenge");
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

        let message = wrldbldr_protocol::ServerMessage::ChallengePrompt {
            challenge_id: challenge_id_str.clone(),
            challenge_name: challenge.name.clone(),
            skill_name: skill_name.clone(),
            difficulty_display: challenge.difficulty.display(),
            description: challenge.description.clone(),
            character_modifier,
            suggested_dice: Some(suggested_dice),
            rule_system_hint: Some(rule_system_hint),
        };

        if let Err(e) = self.world_connection.broadcast_to_world(world_id, message).await {
            tracing::error!("Failed to broadcast challenge prompt: {}", e);
        }

        tracing::info!(
            "DM triggered challenge {} for character {} in world {}",
            challenge_id_str,
            target_character_id,
            world_id
        );

        None
    }

    /// Handle DM approval/rejection of a challenge suggestion.
    pub async fn handle_suggestion_decision(
        &self,
        world_id: &WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Option<serde_json::Value> {

        if approved {
            let approval_item = self.dm_approval_queue_service.get_by_id(&request_id).await;

            match approval_item {
                Ok(Some(item)) => {
                    if let Some(challenge_suggestion) = &item.payload.challenge_suggestion {
                        let challenge_uuid =
                            match uuid::Uuid::parse_str(&challenge_suggestion.challenge_id) {
                                Ok(uuid) => ChallengeId::from_uuid(uuid),
                                Err(_) => {
                                    tracing::error!(
                                        "Invalid challenge_id in suggestion: {}",
                                        challenge_suggestion.challenge_id
                                    );
                                    return error_message("INVALID_CHALLENGE_ID", "Invalid challenge ID format");
                                }
                            };

                        let challenge =
                            match self.challenge_service.get_challenge(challenge_uuid).await {
                                Ok(Some(c)) => c,
                                Ok(None) => {
                                    tracing::error!(
                                        "Challenge {} not found",
                                        challenge_suggestion.challenge_id
                                    );
                                    return error_message("CHALLENGE_NOT_FOUND", &format!("Challenge {} not found", challenge_suggestion.challenge_id));
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load challenge: {}", e);
                                    return error_message("CHALLENGE_LOAD_ERROR", &format!("Failed to load challenge: {}", e));
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
                        let (suggested_dice, rule_system_hint) =
                            get_dice_suggestion_for_challenge(&challenge);

                        let message = wrldbldr_protocol::ServerMessage::ChallengePrompt {
                            challenge_id: challenge_suggestion.challenge_id.clone(),
                            challenge_name: challenge.name.clone(),
                            skill_name: challenge_suggestion.skill_name.clone(),
                            difficulty_display,
                            description: challenge.description.clone(),
                            character_modifier,
                            suggested_dice: Some(suggested_dice),
                            rule_system_hint: Some(rule_system_hint),
                        };

                        if let Err(e) = self.world_connection.broadcast_to_world(world_id, message).await {
                            tracing::error!("Failed to broadcast challenge prompt: {}", e);
                        }

                        tracing::info!(
                            "Triggered challenge '{}' for world via suggestion approval",
                            challenge.name
                        );
                    } else {
                        tracing::warn!(
                            "No challenge suggestion found in approval item {}",
                            request_id
                        );
                        return error_message("NO_CHALLENGE_SUGGESTION", "No challenge suggestion found in approval request");
                    }
                }
                Ok(None) => {
                    tracing::error!("Approval item {} not found", request_id);
                    return error_message("APPROVAL_NOT_FOUND", &format!("Approval request {} not found", request_id));
                }
                Err(e) => {
                    tracing::error!("Failed to get approval item: {}", e);
                    return error_message("APPROVAL_LOOKUP_ERROR", &format!("Failed to look up approval: {}", e));
                }
            }
        } else {
            tracing::info!("DM rejected challenge suggestion for request {}", request_id);
        }

        None
    }

    /// Handle DM creating an ad-hoc challenge (no LLM involved)
    pub async fn handle_adhoc_challenge(
        &self,
        world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: String,
        outcomes: AdHocOutcomesDto,
    ) -> Option<serde_json::Value> {

        // Generate a temporary challenge ID for this ad-hoc challenge
        let adhoc_challenge_id = uuid::Uuid::new_v4().to_string();

        // Store the ad-hoc outcomes in the session for later resolution
        // For now, we just broadcast the challenge prompt to the target player
        tracing::info!(
            "DM created ad-hoc challenge '{}' for PC {}: difficulty {}",
            challenge_name,
            target_pc_id,
            difficulty
        );

        // Determine suggested dice from difficulty string
        let (suggested_dice, rule_system_hint) = if difficulty.to_uppercase().starts_with("DC") {
            ("1d20".to_string(), "Roll 1d20 and add your modifier".to_string())
        } else if difficulty.ends_with('%') {
            ("1d100".to_string(), "Roll percentile dice".to_string())
        } else {
            ("2d6".to_string(), "Roll 2d6 and add your modifier".to_string())
        };

        let message = wrldbldr_protocol::ServerMessage::ChallengePrompt {
            challenge_id: adhoc_challenge_id.clone(),
            challenge_name: challenge_name.clone(),
            skill_name,
            difficulty_display: difficulty,
            description: format!("Ad-hoc challenge created by DM"),
            character_modifier: 0, // DM would need to specify this
            suggested_dice: Some(suggested_dice),
            rule_system_hint: Some(rule_system_hint),
        };

        // Broadcast to world (the target player will see it)
        if let Err(e) = self.world_connection.broadcast_to_world(world_id, message).await {
            tracing::error!("Failed to broadcast ad-hoc challenge prompt: {}", e);
        }

        // Notify DM that challenge was created (includes outcomes for confirmation)
        let msg = AdHocChallengeCreatedMessage {
            r#type: "AdHocChallengeCreated",
            challenge_id: adhoc_challenge_id,
            challenge_name,
            target_pc_id,
            outcomes,
        };
        serde_json::to_value(&msg).ok()
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


