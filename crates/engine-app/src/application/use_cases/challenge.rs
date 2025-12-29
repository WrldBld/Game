//! Challenge Use Case
//!
//! Handles challenge resolution, including dice rolls, triggering challenges,
//! and DM approval of outcomes.
//!
//! # Responsibilities
//!
//! - Submit dice rolls for active challenges
//! - Trigger challenges against target characters (DM)
//! - Create ad-hoc challenges (DM)
//! - Handle challenge outcome decisions (DM)
//! - Request outcome suggestions and branches (DM)
//! - Regenerate outcome text (DM)
//! - Discard pending challenges (DM)
//!
//! # Architecture Note
//!
//! This use case primarily delegates to `ChallengeResolutionService` which
//! contains the core challenge logic. The use case layer adds:
//! - Authorization context (DM vs player)
//! - Result type conversion (no ServerMessage)
//! - Broadcasting via BroadcastPort

use std::sync::Arc;
use tracing::{debug, info};

use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, GameEvent, OutcomeTriggerInfo as PortTriggerInfo,
};

use super::errors::ChallengeError;

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{
    AdHocOutcomes, AdHocResult, ApprovalItem,
    ChallengeSuggestionDecisionInput as SuggestionDecisionInput, CreateAdHocInput, DiceInputType,
    DiscardChallengeInput, DiscardResult, OutcomeDecision, OutcomeDecisionInput,
    OutcomeDecisionResult, OutcomeDetail, RegenerateOutcomeInput, RegenerateResult,
    RequestBranchesInput, RequestSuggestionInput, SelectBranchInput, SubmitDiceInputInput,
    SubmitRollInput, TriggerChallengeInput, TriggerInfo, TriggerResult,
};

/// Result of submitting a roll
#[derive(Debug, Clone)]
pub struct RollResult {
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

// =============================================================================
// Challenge Resolution Service Port
// =============================================================================

/// Port for challenge resolution operations
///
/// This abstracts the ChallengeResolutionService for use case consumption.
/// Methods include `world_id` to support world-scoped challenge resolution.
#[async_trait::async_trait]
pub trait ChallengeResolutionPort: Send + Sync {
    /// Handle a dice roll submission
    async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
    ) -> Result<RollResult, String>;

    /// Handle dice input (formula or manual)
    async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
    ) -> Result<RollResult, String>;

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
#[async_trait::async_trait]
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
#[async_trait::async_trait]
pub trait DmApprovalQueuePort: Send + Sync {
    /// Get an approval item by ID
    async fn get_by_id(&self, request_id: &str) -> Result<Option<ApprovalItem>, String>;

    /// Discard a challenge from the queue
    async fn discard_challenge(&self, dm_id: &str, request_id: &str);
}

// =============================================================================
// Challenge Use Case
// =============================================================================

/// Use case for challenge operations
///
/// Coordinates challenge resolution with proper authorization
/// and result type conversion.
pub struct ChallengeUseCase {
    resolution_service: Arc<dyn ChallengeResolutionPort>,
    outcome_approval: Arc<dyn ChallengeOutcomeApprovalPort>,
    approval_queue: Arc<dyn DmApprovalQueuePort>,
    broadcast: Arc<dyn BroadcastPort>,
}

impl ChallengeUseCase {
    /// Create a new ChallengeUseCase with all dependencies
    pub fn new(
        resolution_service: Arc<dyn ChallengeResolutionPort>,
        outcome_approval: Arc<dyn ChallengeOutcomeApprovalPort>,
        approval_queue: Arc<dyn DmApprovalQueuePort>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            resolution_service,
            outcome_approval,
            approval_queue,
            broadcast,
        }
    }

    /// Submit a dice roll for an active challenge
    ///
    /// Player-facing operation: submits a roll result for challenge resolution.
    pub async fn submit_roll(
        &self,
        ctx: UseCaseContext,
        input: SubmitRollInput,
    ) -> Result<RollResult, ChallengeError> {
        let pc_id = ctx
            .pc_id
            .ok_or(ChallengeError::PcNotFound(PlayerCharacterId::from_uuid(
                uuid::Uuid::nil(),
            )))?;

        debug!(
            challenge_id = %input.challenge_id,
            roll = input.roll,
            "Submitting challenge roll"
        );

        let result = self
            .resolution_service
            .handle_roll(&ctx.world_id, pc_id, input.challenge_id.clone(), input.roll)
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))?;

        // Broadcast roll submission event
        // Adapter routes to DM (full details) and players (status only)
        self.broadcast
            .broadcast(
                ctx.world_id.clone(),
                GameEvent::ChallengeRollSubmitted {
                    world_id: ctx.world_id,
                    resolution_id: result.resolution_id.clone(),
                    challenge_id: result.challenge_id.clone(),
                    challenge_name: result.challenge_name.clone(),
                    character_id: result.character_id.clone(),
                    character_name: result.character_name.clone(),
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome_type: result.outcome_type.clone(),
                    outcome_description: result.outcome_description.clone(),
                    roll_breakdown: result.roll_breakdown.clone(),
                    individual_rolls: result.individual_rolls.clone(),
                    outcome_triggers: result
                        .triggers
                        .iter()
                        .map(|t| PortTriggerInfo {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: t.trigger_type.clone(),
                            description: t.description.clone(),
                            arguments: serde_json::Value::Null,
                        })
                        .collect(),
                },
            )
            .await;

        Ok(result)
    }

    /// Submit dice input (formula or manual) for a challenge
    pub async fn submit_dice_input(
        &self,
        ctx: UseCaseContext,
        input: SubmitDiceInputInput,
    ) -> Result<RollResult, ChallengeError> {
        let pc_id = ctx
            .pc_id
            .ok_or(ChallengeError::PcNotFound(PlayerCharacterId::from_uuid(
                uuid::Uuid::nil(),
            )))?;

        debug!(
            challenge_id = %input.challenge_id,
            input_type = ?input.input_type,
            "Submitting dice input"
        );

        let result = self
            .resolution_service
            .handle_roll_input(
                &ctx.world_id,
                pc_id,
                input.challenge_id.clone(),
                input.input_type,
            )
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))?;

        // Broadcast roll submission event
        // Adapter routes to DM (full details) and players (status only)
        self.broadcast
            .broadcast(
                ctx.world_id.clone(),
                GameEvent::ChallengeRollSubmitted {
                    world_id: ctx.world_id,
                    resolution_id: result.resolution_id.clone(),
                    challenge_id: result.challenge_id.clone(),
                    challenge_name: result.challenge_name.clone(),
                    character_id: result.character_id.clone(),
                    character_name: result.character_name.clone(),
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome_type: result.outcome_type.clone(),
                    outcome_description: result.outcome_description.clone(),
                    roll_breakdown: result.roll_breakdown.clone(),
                    individual_rolls: result.individual_rolls.clone(),
                    outcome_triggers: result
                        .triggers
                        .iter()
                        .map(|t| PortTriggerInfo {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: t.trigger_type.clone(),
                            description: t.description.clone(),
                            arguments: serde_json::Value::Null,
                        })
                        .collect(),
                },
            )
            .await;

        Ok(result)
    }

    /// Trigger a challenge against a target character
    ///
    /// DM-only operation.
    pub async fn trigger_challenge(
        &self,
        ctx: UseCaseContext,
        input: TriggerChallengeInput,
    ) -> Result<TriggerResult, ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            challenge_id = %input.challenge_id,
            target = %input.target_character_id,
            "DM triggering challenge"
        );

        let result = self
            .resolution_service
            .trigger_challenge(&ctx.world_id, input.challenge_id, input.target_character_id)
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))?;

        // Broadcast challenge prompt to world
        self.broadcast
            .broadcast(
                ctx.world_id.clone(),
                GameEvent::ChallengePromptSent {
                    world_id: ctx.world_id,
                    challenge_id: result.challenge_id.clone(),
                    challenge_name: result.challenge_name.clone(),
                    skill_name: result.skill_name.clone(),
                    difficulty_display: result.difficulty_display.clone(),
                    description: result.description.clone(),
                    character_modifier: result.character_modifier,
                    suggested_dice: result.suggested_dice.clone(),
                    rule_system_hint: result.rule_system_hint.clone(),
                },
            )
            .await;

        Ok(result)
    }

    /// Handle DM's decision on a challenge suggestion
    pub async fn suggestion_decision(
        &self,
        ctx: UseCaseContext,
        input: SuggestionDecisionInput,
    ) -> Result<(), ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        self.resolution_service
            .handle_suggestion_decision(
                &ctx.world_id,
                input.request_id,
                input.approved,
                input.modified_difficulty,
            )
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))
    }

    /// Regenerate challenge outcome text
    pub async fn regenerate_outcome(
        &self,
        ctx: UseCaseContext,
        input: RegenerateOutcomeInput,
    ) -> Result<RegenerateResult, ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        debug!(
            request_id = %input.request_id,
            outcome_type = ?input.outcome_type,
            "DM requesting outcome regeneration"
        );

        // Best-effort: look up the approval item for context
        let maybe_approval = self
            .approval_queue
            .get_by_id(&input.request_id)
            .await
            .ok()
            .flatten();

        let base_flavor = if let Some(item) = maybe_approval {
            format!("{} (regenerated)", item.proposed_dialogue.trim())
        } else {
            "Regenerated outcome (no approval context found)".to_string()
        };

        let flavor_text = if let Some(g) = input.guidance {
            if g.trim().is_empty() {
                base_flavor
            } else {
                format!("{} — Guidance: {}", base_flavor, g.trim())
            }
        } else {
            base_flavor
        };

        let outcome_type = input.outcome_type.unwrap_or_else(|| "all".to_string());

        Ok(RegenerateResult {
            outcome_type,
            new_outcome: OutcomeDetail {
                flavor_text,
                scene_direction: "DM: narrate this regenerated outcome to the table.".to_string(),
                proposed_tools: Vec::new(),
            },
        })
    }

    /// Discard a challenge from the approval queue
    pub async fn discard_challenge(
        &self,
        ctx: UseCaseContext,
        input: DiscardChallengeInput,
    ) -> Result<DiscardResult, ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            request_id = %input.request_id,
            feedback = ?input.feedback,
            "DM discarding challenge"
        );

        self.approval_queue
            .discard_challenge(&ctx.user_id, &input.request_id)
            .await;

        Ok(DiscardResult {
            request_id: input.request_id,
        })
    }

    /// Create an ad-hoc challenge
    pub async fn create_adhoc(
        &self,
        ctx: UseCaseContext,
        input: CreateAdHocInput,
    ) -> Result<AdHocResult, ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            challenge_name = %input.challenge_name,
            target_pc = %input.target_pc_id,
            "DM creating ad-hoc challenge"
        );

        self.resolution_service
            .create_adhoc_challenge(
                &ctx.world_id,
                input.challenge_name,
                input.skill_name,
                input.difficulty,
                input.target_pc_id,
                input.outcomes,
            )
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))
    }

    /// Handle DM's decision on a challenge outcome
    pub async fn outcome_decision(
        &self,
        ctx: UseCaseContext,
        input: OutcomeDecisionInput,
    ) -> Result<OutcomeDecisionResult, ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            resolution_id = %input.resolution_id,
            decision = ?input.decision,
            "DM making outcome decision"
        );

        self.outcome_approval
            .process_decision(&ctx.world_id, &input.resolution_id, input.decision)
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))?;

        Ok(OutcomeDecisionResult {
            outcome_text: None, // Resolution broadcast handled by service
            suggestions_pending: false,
        })
    }

    /// Request AI-generated outcome suggestions
    pub async fn request_suggestion(
        &self,
        ctx: UseCaseContext,
        input: RequestSuggestionInput,
    ) -> Result<(), ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            resolution_id = %input.resolution_id,
            guidance = ?input.guidance,
            "DM requesting outcome suggestion"
        );

        let decision = OutcomeDecision::Suggest {
            guidance: input.guidance,
        };

        self.outcome_approval
            .process_decision(&ctx.world_id, &input.resolution_id, decision)
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))
    }

    /// Request branching outcome options
    pub async fn request_branches(
        &self,
        ctx: UseCaseContext,
        input: RequestBranchesInput,
    ) -> Result<(), ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            resolution_id = %input.resolution_id,
            guidance = ?input.guidance,
            "DM requesting outcome branches"
        );

        self.outcome_approval
            .request_branches(&ctx.world_id, &input.resolution_id, input.guidance)
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))
    }

    /// Select a specific outcome branch
    pub async fn select_branch(
        &self,
        ctx: UseCaseContext,
        input: SelectBranchInput,
    ) -> Result<(), ChallengeError> {
        if !ctx.is_dm {
            return Err(ChallengeError::NotAuthorized);
        }

        info!(
            resolution_id = %input.resolution_id,
            branch_id = %input.branch_id,
            "DM selecting outcome branch"
        );

        self.outcome_approval
            .select_branch(
                &ctx.world_id,
                &input.resolution_id,
                &input.branch_id,
                input.modified_description,
            )
            .await
            .map_err(|e| ChallengeError::ResolutionFailed(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
