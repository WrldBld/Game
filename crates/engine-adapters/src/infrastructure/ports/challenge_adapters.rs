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
//! - **Protocol Types** (from `wrldbldr_protocol`): `AdHocOutcomes` (wire format)
//!
//! Note: Protocol types are converted in the WebSocket layer before reaching these adapters.

use std::sync::Arc;

use wrldbldr_domain::value_objects::NarrativeResolutionConfig;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::inbound::{
    AdHocOutcomes, AdHocResult, ApprovalItem as UseCaseApprovalItem, ChallengeDmApprovalQueuePort,
    DiceInputType, RollResultData as RollResult, TriggerResult,
};
use wrldbldr_engine_ports::outbound::{
    ChallengeOutcomeApprovalServicePort, ChallengeResolutionServicePort, DiceRoll,
    DmApprovalQueueServicePort, OutcomeDecision,
    ChallengeOutcomeApprovalPort, ChallengeResolutionPort, NarrativeRollContext,
};

// =============================================================================
// ChallengeOutcomeApprovalAdapter
// =============================================================================

/// Adapter that wraps ChallengeOutcomeApprovalServicePort to implement ChallengeOutcomeApprovalPort.
pub struct ChallengeOutcomeApprovalAdapter {
    service: Arc<dyn ChallengeOutcomeApprovalServicePort>,
}

impl ChallengeOutcomeApprovalAdapter {
    pub fn new(service: Arc<dyn ChallengeOutcomeApprovalServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl ChallengeOutcomeApprovalPort for ChallengeOutcomeApprovalAdapter {
    async fn process_decision(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        decision: OutcomeDecision,
    ) -> Result<(), String> {
        // OutcomeDecision is now used directly - no conversion needed
        self.service
            .process_decision(*world_id, resolution_id, decision)
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
            .request_branches(*world_id, resolution_id, guidance)
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
            .select_branch(*world_id, resolution_id, branch_id, modified_description)
            .await
            .map_err(|e| e.to_string())
    }
}

// =============================================================================
// DmApprovalQueueAdapter
// =============================================================================

/// Adapter that wraps DmApprovalQueueServicePort to implement ChallengeDmApprovalQueuePort.
pub struct ChallengeDmApprovalQueueAdapter {
    service: Arc<dyn DmApprovalQueueServicePort>,
}

impl ChallengeDmApprovalQueueAdapter {
    pub fn new(service: Arc<dyn DmApprovalQueueServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl ChallengeDmApprovalQueuePort for ChallengeDmApprovalQueueAdapter {
    async fn get_by_id(&self, request_id: &str) -> Result<Option<UseCaseApprovalItem>, String> {
        let uuid = uuid::Uuid::parse_str(request_id).map_err(|e| e.to_string())?;
        self.service
            .get(uuid)
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
        // Use the discard_challenge method on the service port
        if let Err(e) = self.service.discard_challenge(request_id).await {
            tracing::warn!(
                dm_id = dm_id,
                request_id = request_id,
                error = %e,
                "ChallengeDmApprovalQueueAdapter::discard_challenge failed"
            );
        }
    }
}

// =============================================================================
// ChallengeResolutionAdapter
// =============================================================================

/// Adapter that wraps ChallengeResolutionServicePort to implement ChallengeResolutionPort.
///
/// This adapter converts between the service port's result types and the use case's
/// simplified result types.
///
/// # Note
///
/// The ChallengeResolutionServicePort has a simpler API (start_resolution, submit_roll)
/// than what the inbound ChallengeResolutionPort requires. This adapter uses the
/// available port methods and converts types appropriately.
pub struct ChallengeResolutionAdapter {
    service: Arc<dyn ChallengeResolutionServicePort>,
}

impl ChallengeResolutionAdapter {
    pub fn new(service: Arc<dyn ChallengeResolutionServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl ChallengeResolutionPort for ChallengeResolutionAdapter {
    async fn handle_roll(
        &self,
        _world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
        _narrative_config: &NarrativeResolutionConfig,
        _narrative_context: Option<&NarrativeRollContext>,
    ) -> Result<RollResult, String> {
        let uuid = uuid::Uuid::parse_str(&challenge_id).map_err(|e| e.to_string())?;
        let challenge_id_parsed = wrldbldr_domain::ChallengeId::from_uuid(uuid);

        // Start a resolution and immediately submit the roll
        // TODO: Pass narrative_config and narrative_context to the service port
        // once ChallengeResolutionServicePort is extended to support them
        let resolution_id = self
            .service
            .start_resolution(challenge_id_parsed, pc_id)
            .await
            .map_err(|e| e.to_string())?;

        let dice_roll = DiceRoll::simple(roll);

        let result = self
            .service
            .submit_roll(resolution_id, dice_roll)
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
            triggers: vec![], // Port doesn't expose triggers - would need port extension
            pending_approval: true,
        })
    }

    async fn handle_roll_input(
        &self,
        _world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
        _narrative_config: &NarrativeResolutionConfig,
        _narrative_context: Option<&NarrativeRollContext>,
    ) -> Result<RollResult, String> {
        let uuid = uuid::Uuid::parse_str(&challenge_id).map_err(|e| e.to_string())?;
        let challenge_id_parsed = wrldbldr_domain::ChallengeId::from_uuid(uuid);

        // Start a resolution and immediately submit the roll
        // TODO: Pass narrative_config and narrative_context to the service port
        // once ChallengeResolutionServicePort is extended to support them
        let resolution_id = self
            .service
            .start_resolution(challenge_id_parsed, pc_id)
            .await
            .map_err(|e| e.to_string())?;

        // Convert DiceInputType to DiceRoll
        let dice_roll = match input_type {
            DiceInputType::Manual(value) => DiceRoll::simple(value),
            DiceInputType::Formula(_formula) => {
                // For formula input, we'd need to evaluate it - for now, return an error
                // since the port doesn't support formula evaluation
                return Err("Formula-based dice input requires port extension".to_string());
            }
        };

        let result = self
            .service
            .submit_roll(resolution_id, dice_roll)
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
            triggers: vec![],
            pending_approval: true,
        })
    }

    async fn trigger_challenge(
        &self,
        _world_id: &WorldId,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> Result<TriggerResult, String> {
        // The ChallengeResolutionServicePort doesn't expose trigger functionality
        // This would need a port extension or use of ChallengeServicePort
        Err(format!(
            "trigger_challenge not supported by ChallengeResolutionServicePort - \
             challenge_id: {}, target: {}",
            challenge_id, target_character_id
        ))
    }

    async fn handle_suggestion_decision(
        &self,
        _world_id: &WorldId,
        request_id: String,
        approved: bool,
        _modified_difficulty: Option<String>,
    ) -> Result<(), String> {
        // The ChallengeResolutionServicePort doesn't expose suggestion decision functionality
        // This would need a port extension
        Err(format!(
            "handle_suggestion_decision not supported by ChallengeResolutionServicePort - \
             request_id: {}, approved: {}",
            request_id, approved
        ))
    }

    async fn create_adhoc_challenge(
        &self,
        _world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        _outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String> {
        // The ChallengeResolutionServicePort doesn't expose ad-hoc challenge creation
        // This would need a port extension or use of ChallengeServicePort
        Err(format!(
            "create_adhoc_challenge not supported by ChallengeResolutionServicePort - \
             name: {}, skill: {}, difficulty: {}, target: {}",
            challenge_name, skill_name, difficulty, target_pc_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_decision_variants() {
        // Test that OutcomeDecision variants can be created
        let accept = OutcomeDecision::Accept;
        let edit = OutcomeDecision::Edit {
            modified_text: "new text".to_string(),
        };
        let suggest = OutcomeDecision::Suggest {
            guidance: Some("be dramatic".to_string()),
        };

        assert!(matches!(accept, OutcomeDecision::Accept));
        assert!(matches!(edit, OutcomeDecision::Edit { .. }));
        assert!(matches!(suggest, OutcomeDecision::Suggest { .. }));
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
