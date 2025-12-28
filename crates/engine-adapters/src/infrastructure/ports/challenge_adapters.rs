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
//! # Implementation Note
//!
//! The ChallengeResolutionService has complex generic bounds and its methods
//! return `Option<serde_json::Value>` rather than proper Result types. This
//! makes direct adaptation challenging.
//!
//! Until the service signatures are refactored, handlers should continue to
//! call the services directly rather than going through the use case layer.
//!
//! These adapters are provided as scaffolding for future implementation.

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::dto::ApprovalItem;
use wrldbldr_engine_app::application::services::{
    ChallengeOutcomeApprovalService, DMApprovalQueueService, ItemService,
};
use wrldbldr_engine_app::application::use_cases::{
    AdHocOutcomes, AdHocResult, ApprovalItem as UseCaseApprovalItem, ChallengeOutcomeApprovalPort,
    ChallengeOutcomeDecision, ChallengeDmApprovalQueuePort, ChallengeResolutionPort, DiceInputType,
    RollResult, TriggerResult,
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
    Q: ApprovalQueuePort<ApprovalItem> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    service: Arc<DMApprovalQueueService<Q, I>>,
}

impl<Q, I> ChallengeDmApprovalQueueAdapter<Q, I>
where
    Q: ApprovalQueuePort<ApprovalItem> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    pub fn new(service: Arc<DMApprovalQueueService<Q, I>>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl<Q, I> ChallengeDmApprovalQueuePort for ChallengeDmApprovalQueueAdapter<Q, I>
where
    Q: ApprovalQueuePort<ApprovalItem> + Send + Sync + 'static,
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
// ChallengeResolutionPort Placeholder Implementation
// =============================================================================

/// Placeholder adapter for ChallengeResolutionPort.
///
/// The actual ChallengeResolutionService has complex generic bounds and
/// returns `Option<serde_json::Value>` rather than proper Result types.
/// This placeholder returns errors indicating handlers should call services directly.
pub struct ChallengeResolutionPlaceholder;

#[async_trait::async_trait]
impl ChallengeResolutionPort for ChallengeResolutionPlaceholder {
    async fn handle_roll(
        &self,
        _world_id: &WorldId,
        _pc_id: PlayerCharacterId,
        _challenge_id: String,
        _roll: i32,
    ) -> Result<RollResult, String> {
        Err("ChallengeResolutionPort: Use handler directly until service refactoring".to_string())
    }

    async fn handle_roll_input(
        &self,
        _world_id: &WorldId,
        _pc_id: PlayerCharacterId,
        _challenge_id: String,
        _input_type: DiceInputType,
    ) -> Result<RollResult, String> {
        Err("ChallengeResolutionPort: Use handler directly until service refactoring".to_string())
    }

    async fn trigger_challenge(
        &self,
        _world_id: &WorldId,
        _challenge_id: String,
        _target_character_id: CharacterId,
    ) -> Result<TriggerResult, String> {
        Err("ChallengeResolutionPort: Use handler directly until service refactoring".to_string())
    }

    async fn handle_suggestion_decision(
        &self,
        _world_id: &WorldId,
        _request_id: String,
        _approved: bool,
        _modified_difficulty: Option<String>,
    ) -> Result<(), String> {
        Err("ChallengeResolutionPort: Use handler directly until service refactoring".to_string())
    }

    async fn create_adhoc_challenge(
        &self,
        _world_id: &WorldId,
        _challenge_name: String,
        _skill_name: String,
        _difficulty: String,
        _target_pc_id: PlayerCharacterId,
        _outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String> {
        Err("ChallengeResolutionPort: Use handler directly until service refactoring".to_string())
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
