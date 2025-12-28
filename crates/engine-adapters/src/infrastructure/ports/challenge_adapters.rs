//! Challenge Use Case Adapters
//!
//! Implements challenge-related ports by wrapping existing services.

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use wrldbldr_engine_app::application::services::challenge_resolution_service::ChallengeResolutionService;
use wrldbldr_engine_app::application::services::ChallengeOutcomeApprovalService;
use wrldbldr_engine_app::application::services::DMApprovalQueueService;
use wrldbldr_engine_app::application::use_cases::{
    AdHocOutcomes, AdHocResult, ApprovalItem, ChallengeOutcomeApprovalPort,
    ChallengeResolutionPort, DiceInputType, DmApprovalQueuePort as ChallengeDmApprovalQueuePort,
    OutcomeDecision, RollResult, TriggerResult,
};

/// Marker for generic service types
pub struct ChallengeResolutionAdapter<C, S, P, E, Q, O>
where
    C: wrldbldr_engine_app::application::services::ChallengeService + Send + Sync,
    S: wrldbldr_engine_app::application::services::SkillService + Send + Sync,
    P: wrldbldr_engine_app::application::services::PlayerCharacterService + Send + Sync,
    E: wrldbldr_engine_ports::outbound::EventBusPort<wrldbldr_protocol::AppEvent> + Send + Sync,
    Q: wrldbldr_engine_ports::outbound::ApprovalQueuePort + Send + Sync,
    O: Send + Sync,
{
    service: Arc<ChallengeResolutionService<C, S, P, E, Q, O>>,
}

impl<C, S, P, E, Q, O> ChallengeResolutionAdapter<C, S, P, E, Q, O>
where
    C: wrldbldr_engine_app::application::services::ChallengeService + Send + Sync,
    S: wrldbldr_engine_app::application::services::SkillService + Send + Sync,
    P: wrldbldr_engine_app::application::services::PlayerCharacterService + Send + Sync,
    E: wrldbldr_engine_ports::outbound::EventBusPort<wrldbldr_protocol::AppEvent> + Send + Sync,
    Q: wrldbldr_engine_ports::outbound::ApprovalQueuePort + Send + Sync,
    O: Send + Sync,
{
    pub fn new(service: Arc<ChallengeResolutionService<C, S, P, E, Q, O>>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl<C, S, P, E, Q, O> ChallengeResolutionPort for ChallengeResolutionAdapter<C, S, P, E, Q, O>
where
    C: wrldbldr_engine_app::application::services::ChallengeService + Send + Sync,
    S: wrldbldr_engine_app::application::services::SkillService + Send + Sync,
    P: wrldbldr_engine_app::application::services::PlayerCharacterService + Send + Sync,
    E: wrldbldr_engine_ports::outbound::EventBusPort<wrldbldr_protocol::AppEvent> + Send + Sync,
    Q: wrldbldr_engine_ports::outbound::ApprovalQueuePort + Send + Sync,
    O: Send + Sync,
{
    async fn handle_roll(
        &self,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
    ) -> Result<RollResult, String> {
        match self.service.handle_roll(&pc_id, &challenge_id, roll).await {
            Ok(result) => Ok(RollResult {
                roll,
                outcome: result.outcome_type.to_string(),
                pending_approval: result.pending_approval,
            }),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn handle_roll_input(
        &self,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
    ) -> Result<RollResult, String> {
        // Convert to underlying service's expected format
        let roll = match input_type {
            DiceInputType::Formula(formula) => {
                // Parse dice formula - simple implementation for now
                parse_dice_formula(&formula).map_err(|e| e.to_string())?
            }
            DiceInputType::Manual(value) => value,
        };

        self.handle_roll(pc_id, challenge_id, roll).await
    }

    async fn trigger_challenge(
        &self,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> Result<TriggerResult, String> {
        self.service
            .trigger_challenge(&challenge_id, &target_character_id)
            .await
            .map(|_| TriggerResult {
                challenge_id,
                target_name: target_character_id.to_string(), // Would need to fetch name
            })
            .map_err(|e| e.to_string())
    }

    async fn handle_suggestion_decision(
        &self,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Result<(), String> {
        self.service
            .handle_suggestion_decision(&request_id, approved, modified_difficulty.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    async fn create_adhoc_challenge(
        &self,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String> {
        self.service
            .create_adhoc_challenge(
                &challenge_name,
                &skill_name,
                &difficulty,
                &target_pc_id,
                outcomes.critical_success.as_deref(),
                outcomes.success.as_deref(),
                outcomes.failure.as_deref(),
                outcomes.critical_failure.as_deref(),
            )
            .await
            .map(|id| AdHocResult { challenge_id: id })
            .map_err(|e| e.to_string())
    }
}

/// Adapter for ChallengeOutcomeApprovalService
pub struct ChallengeOutcomeApprovalAdapter {
    service: Arc<ChallengeOutcomeApprovalService>,
}

impl ChallengeOutcomeApprovalAdapter {
    pub fn new(service: Arc<ChallengeOutcomeApprovalService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl ChallengeOutcomeApprovalPort for ChallengeOutcomeApprovalAdapter {
    async fn process_decision(
        &self,
        resolution_id: &str,
        decision: OutcomeDecision,
    ) -> Result<(), String> {
        match decision {
            OutcomeDecision::Accept => {
                self.service.accept_outcome(resolution_id).await
            }
            OutcomeDecision::Edit { modified_text } => {
                self.service
                    .edit_outcome(resolution_id, &modified_text)
                    .await
            }
            OutcomeDecision::Suggest { guidance } => {
                self.service
                    .request_suggestions(resolution_id, guidance.as_deref())
                    .await
            }
        }
        .map_err(|e| e.to_string())
    }

    async fn request_branches(
        &self,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<(), String> {
        self.service
            .request_branches(resolution_id, guidance.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    async fn select_branch(
        &self,
        resolution_id: &str,
        branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<(), String> {
        self.service
            .select_branch(resolution_id, branch_id, modified_description.as_deref())
            .await
            .map_err(|e| e.to_string())
    }
}

/// Adapter for DMApprovalQueueService
pub struct DmApprovalQueueAdapter {
    service: Arc<DMApprovalQueueService>,
}

impl DmApprovalQueueAdapter {
    pub fn new(service: Arc<DMApprovalQueueService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl ChallengeDmApprovalQueuePort for DmApprovalQueueAdapter {
    async fn get_by_id(&self, request_id: &str) -> Result<Option<ApprovalItem>, String> {
        self.service
            .get_by_id(request_id)
            .await
            .map(|opt| {
                opt.map(|item| ApprovalItem {
                    request_id: item.request_id,
                    proposed_dialogue: item.proposed_text.unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())
    }

    async fn discard_challenge(&self, dm_id: &str, request_id: &str) {
        let _ = self.service.discard(dm_id, request_id).await;
    }
}

/// Parse a simple dice formula like "1d20+5"
fn parse_dice_formula(formula: &str) -> Result<i32, &'static str> {
    use rand::Rng;

    let formula = formula.trim().to_lowercase();

    // Simple parser for NdM+B format
    if let Some(d_pos) = formula.find('d') {
        let count: i32 = formula[..d_pos].parse().unwrap_or(1);
        let rest = &formula[d_pos + 1..];

        let (sides, bonus) = if let Some(plus_pos) = rest.find('+') {
            (
                rest[..plus_pos].parse().map_err(|_| "Invalid sides")?,
                rest[plus_pos + 1..].parse().unwrap_or(0),
            )
        } else if let Some(minus_pos) = rest.find('-') {
            (
                rest[..minus_pos].parse().map_err(|_| "Invalid sides")?,
                -rest[minus_pos + 1..].parse::<i32>().unwrap_or(0),
            )
        } else {
            (rest.parse().map_err(|_| "Invalid sides")?, 0)
        };

        let mut rng = rand::thread_rng();
        let mut total = bonus;
        for _ in 0..count {
            total += rng.gen_range(1..=sides);
        }

        Ok(total)
    } else {
        // Just a number
        formula.parse().map_err(|_| "Invalid formula")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dice_formula() {
        // Test simple number
        assert_eq!(parse_dice_formula("10").unwrap(), 10);

        // Dice roll results are random, just verify they parse
        assert!(parse_dice_formula("1d20").is_ok());
        assert!(parse_dice_formula("2d6+5").is_ok());
        assert!(parse_dice_formula("1d20-2").is_ok());

        // Invalid should error
        assert!(parse_dice_formula("invalid").is_err());
    }
}
