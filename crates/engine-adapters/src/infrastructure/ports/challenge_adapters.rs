//! Challenge Use Case Adapters
//!
//! Implements challenge-related ports by wrapping existing services.
//!
//! # Architecture
//!
//! The challenge services are heavily generic due to dependency injection.
//! These adapters wrap the concrete service types used in AppState and
//! implement the port traits defined in the ChallengeUseCase.
//!
//! # Type Boundaries
//!
//! This module bridges between:
//! - **Use Case Types** (from `wrldbldr_engine_ports`): `OutcomeDecision`, `AdHocOutcomes`
//! - **DTO Types** (from `wrldbldr_engine_app`): `ChallengeOutcomeDecision`, `AdHocOutcomesDto`
//!
//! Note: Protocol types (`wrldbldr_protocol`) are converted in the WebSocket layer
//! before reaching these adapters.

use std::sync::Arc;

use wrldbldr_domain::value_objects::ApprovalRequestData;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::dto::AdHocOutcomesDto;
use wrldbldr_engine_app::application::services::{
    ChallengeOutcomeApprovalService, ChallengeResolutionService, ChallengeService,
    DMApprovalQueueService, ItemService, PlayerCharacterService, SkillService,
};
use wrldbldr_engine_app::application::use_cases::{
    AdHocOutcomes, AdHocResult, ApprovalItem as UseCaseApprovalItem, ChallengeDmApprovalQueuePort,
    ChallengeOutcomeApprovalPort, ChallengeOutcomeDecision, ChallengeResolutionPort, DiceInputType,
    RollResult, TriggerInfo, TriggerResult,
};
use wrldbldr_engine_ports::outbound::{ApprovalQueuePort, LlmPort};

// =============================================================================
// ChallengeOutcomeApprovalAdapter
// =============================================================================

/// Adapter that wraps ChallengeOutcomeApprovalService to implement ChallengeOutcomeApprovalPort.
pub struct ChallengeOutcomeApprovalAdapter<L: LlmPort + Send + Sync + 'static> {
    service: Arc<ChallengeOutcomeApprovalService<L>>,
}

impl<L: LlmPort + Send + Sync + 'static> ChallengeOutcomeApprovalAdapter<L> {
    pub fn new(service: Arc<ChallengeOutcomeApprovalService<L>>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl<L: LlmPort + Send + Sync + 'static> ChallengeOutcomeApprovalPort
    for ChallengeOutcomeApprovalAdapter<L>
{
    async fn process_decision(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        decision: ChallengeOutcomeDecision,
    ) -> Result<(), String> {
        // Convert use case decision type to DTO decision type
        use wrldbldr_engine_app::application::dto::ChallengeOutcomeDecision as DtoDecision;

        let service_decision = match decision {
            ChallengeOutcomeDecision::Accept => DtoDecision::Accept,
            ChallengeOutcomeDecision::Edit { modified_text } => DtoDecision::Edit {
                modified_description: modified_text,
            },
            ChallengeOutcomeDecision::Suggest { guidance } => DtoDecision::Suggest { guidance },
        };

        self.service
            .process_decision(world_id, resolution_id, service_decision)
            .await
            .map_err(|e| e.to_string())
    }

    async fn request_branches(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<(), String> {
        self.service
            .request_branches(world_id, resolution_id, guidance)
            .await
            .map_err(|e| e.to_string())
    }

    async fn select_branch(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<(), String> {
        self.service
            .select_branch(world_id, resolution_id, branch_id, modified_description)
            .await
            .map_err(|e| e.to_string())
    }
}

// =============================================================================
// DmApprovalQueueAdapter
// =============================================================================

/// Adapter that wraps DMApprovalQueueService to implement DmApprovalQueuePort.
pub struct ChallengeDmApprovalQueueAdapter<Q, I>
where
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    service: Arc<DMApprovalQueueService<Q, I>>,
}

impl<Q, I> ChallengeDmApprovalQueueAdapter<Q, I>
where
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    pub fn new(service: Arc<DMApprovalQueueService<Q, I>>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl<Q, I> ChallengeDmApprovalQueuePort for ChallengeDmApprovalQueueAdapter<Q, I>
where
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    async fn get_by_id(&self, request_id: &str) -> Result<Option<UseCaseApprovalItem>, String> {
        self.service
            .get_by_id(request_id)
            .await
            .map(|opt| {
                opt.map(|item| UseCaseApprovalItem {
                    request_id: item.id.to_string(),
                    proposed_dialogue: item.payload.proposed_dialogue.clone(),
                })
            })
            .map_err(|e| e.to_string())
    }

    async fn discard_challenge(&self, dm_id: &str, request_id: &str) {
        // The service doesn't have a direct discard method with these parameters.
        // Log and do nothing for now - this is a partial implementation.
        tracing::warn!(
            dm_id = dm_id,
            request_id = request_id,
            "ChallengeDmApprovalQueueAdapter::discard_challenge not fully implemented"
        );
    }
}

// =============================================================================
// ChallengeResolutionAdapter
// =============================================================================

/// Adapter that wraps ChallengeResolutionService to implement ChallengeResolutionPort.
///
/// This adapter converts between the service's typed results and the use case's
/// simplified result types.
pub struct ChallengeResolutionAdapter<S, K, Q, P, L, I>
where
    S: ChallengeService + Send + Sync + 'static,
    K: SkillService + Send + Sync + 'static,
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    P: PlayerCharacterService + Send + Sync + 'static,
    L: LlmPort + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    service: Arc<ChallengeResolutionService<S, K, Q, P, L, I>>,
}

impl<S, K, Q, P, L, I> ChallengeResolutionAdapter<S, K, Q, P, L, I>
where
    S: ChallengeService + Send + Sync + 'static,
    K: SkillService + Send + Sync + 'static,
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    P: PlayerCharacterService + Send + Sync + 'static,
    L: LlmPort + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    pub fn new(service: Arc<ChallengeResolutionService<S, K, Q, P, L, I>>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl<S, K, Q, P, L, I> ChallengeResolutionPort for ChallengeResolutionAdapter<S, K, Q, P, L, I>
where
    S: ChallengeService + Send + Sync + 'static,
    K: SkillService + Send + Sync + 'static,
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    P: PlayerCharacterService + Send + Sync + 'static,
    L: LlmPort + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
    ) -> Result<RollResult, String> {
        let result = self
            .service
            .handle_roll(world_id, &pc_id, challenge_id, roll)
            .await
            .map_err(|e| e.to_string())?;

        // Convert RollSubmissionResult to use case RollResult
        Ok(RollResult {
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
                .map(|t| TriggerInfo {
                    trigger_type: t.trigger_type,
                    description: t.description,
                })
                .collect(),
            pending_approval: true, // All challenges now go through approval
        })
    }

    async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
    ) -> Result<RollResult, String> {
        // Convert use case DiceInputType to service DiceInputType
        use wrldbldr_engine_app::application::services::challenge_resolution_service::DiceInputType as ServiceDiceInput;

        let service_input = match input_type {
            DiceInputType::Formula(formula) => ServiceDiceInput::Formula(formula),
            DiceInputType::Manual(value) => ServiceDiceInput::Manual(value),
        };

        let result = self
            .service
            .handle_roll_input(world_id, &pc_id, challenge_id, service_input)
            .await
            .map_err(|e| e.to_string())?;

        Ok(RollResult {
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
                .map(|t| TriggerInfo {
                    trigger_type: t.trigger_type,
                    description: t.description,
                })
                .collect(),
            pending_approval: true,
        })
    }

    async fn trigger_challenge(
        &self,
        world_id: &WorldId,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> Result<TriggerResult, String> {
        let result = self
            .service
            .handle_trigger(world_id, challenge_id, target_character_id.to_string())
            .await
            .map_err(|e| e.to_string())?;

        Ok(TriggerResult {
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
        world_id: &WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Result<(), String> {
        self.service
            .handle_suggestion_decision(world_id, request_id, approved, modified_difficulty)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn create_adhoc_challenge(
        &self,
        world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String> {
        // Convert use case AdHocOutcomes to DTO
        // DTO requires success/failure as non-optional, use empty string as fallback
        let dto_outcomes = AdHocOutcomesDto {
            success: outcomes.success.unwrap_or_default(),
            failure: outcomes.failure.unwrap_or_default(),
            critical_success: outcomes.critical_success,
            critical_failure: outcomes.critical_failure,
        };

        let (adhoc_result, _trigger_result) = self
            .service
            .handle_adhoc_challenge(
                world_id,
                challenge_name,
                skill_name,
                difficulty,
                target_pc_id.to_string(),
                dto_outcomes,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(AdHocResult {
            challenge_id: adhoc_result.challenge_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_decision_variants() {
        // Test that OutcomeDecision variants can be created
        let accept = ChallengeOutcomeDecision::Accept;
        let edit = ChallengeOutcomeDecision::Edit {
            modified_text: "new text".to_string(),
        };
        let suggest = ChallengeOutcomeDecision::Suggest {
            guidance: Some("be dramatic".to_string()),
        };

        assert!(matches!(accept, ChallengeOutcomeDecision::Accept));
        assert!(matches!(edit, ChallengeOutcomeDecision::Edit { .. }));
        assert!(matches!(suggest, ChallengeOutcomeDecision::Suggest { .. }));
    }

    #[test]
    fn test_approval_item_mapping() {
        let use_case_item = UseCaseApprovalItem {
            request_id: "test-123".to_string(),
            proposed_dialogue: "Hello, adventurer!".to_string(),
        };

        assert_eq!(use_case_item.request_id, "test-123");
        assert_eq!(use_case_item.proposed_dialogue, "Hello, adventurer!");
    }
}
