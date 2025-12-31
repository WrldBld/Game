//! Outcome Trigger Service - Executes triggers after challenge resolution
//!
//! This service handles the execution of outcome triggers from challenge outcomes.
//! It works alongside the ToolExecutionService but handles the static triggers
//! defined in challenge definitions rather than LLM-proposed tools.

use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::entities::OutcomeTrigger;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{
    ChallengeCrudPort, OutcomeTriggerExecutionResult, OutcomeTriggerServicePort, StateChange,
};

use async_trait::async_trait;

/// Service for executing outcome triggers
pub struct OutcomeTriggerService {
    challenge_crud: Arc<dyn ChallengeCrudPort>,
}

impl OutcomeTriggerService {
    /// Create a new OutcomeTriggerService
    pub fn new(challenge_crud: Arc<dyn ChallengeCrudPort>) -> Self {
        Self { challenge_crud }
    }

    /// Execute a list of outcome triggers
    ///
    /// This method processes each trigger and generates appropriate state changes.
    /// Some triggers may require async operations (like enabling/disabling challenges).
    ///
    /// Note: Conversation history recording has been removed as it's now handled
    /// by the caller via WorldStateManager.
    #[instrument(skip(self))]
    pub async fn execute_triggers(
        &self,
        triggers: Vec<OutcomeTrigger>,
        _world_id: WorldId,
    ) -> OutcomeTriggerExecutionResult {
        let mut state_changes = Vec::new();
        let mut warnings = Vec::new();

        for trigger in &triggers {
            match self.execute_single_trigger(trigger).await {
                Ok(changes) => state_changes.extend(changes),
                Err(e) => {
                    warnings.push(format!("Trigger execution warning: {}", e));
                }
            }
        }

        info!(
            trigger_count = triggers.len(),
            state_changes = state_changes.len(),
            warnings = warnings.len(),
            "Executed outcome triggers"
        );

        OutcomeTriggerExecutionResult {
            trigger_count: triggers.len(),
            state_changes,
            warnings,
        }
    }

    /// Execute a single trigger and return state changes
    async fn execute_single_trigger(
        &self,
        trigger: &OutcomeTrigger,
    ) -> Result<Vec<StateChange>, String> {
        match trigger {
            OutcomeTrigger::RevealInformation { info, persist } => {
                debug!(info = %info, persist = %persist, "Revealing information");

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::InfoRevealed {
                    info: if *persist {
                        format!("[JOURNAL] {}", info)
                    } else {
                        info.clone()
                    },
                }])
            }

            OutcomeTrigger::EnableChallenge { challenge_id } => {
                debug!(challenge_id = %challenge_id, "Enabling challenge");

                // Update the challenge in the repository
                if let Err(e) = self.challenge_crud.set_active(*challenge_id, true).await {
                    return Err(format!("Failed to enable challenge: {}", e));
                }

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::EventTriggered {
                    name: format!("Challenge {} enabled", challenge_id),
                }])
            }

            OutcomeTrigger::DisableChallenge { challenge_id } => {
                debug!(challenge_id = %challenge_id, "Disabling challenge");

                // Update the challenge in the repository
                if let Err(e) = self.challenge_crud.set_active(*challenge_id, false).await {
                    return Err(format!("Failed to disable challenge: {}", e));
                }

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::EventTriggered {
                    name: format!("Challenge {} disabled", challenge_id),
                }])
            }

            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                debug!(stat = %stat, modifier = %modifier, "Modifying character stat");

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::CharacterStatUpdated {
                    character_id: "active_pc".to_string(), // Will be resolved by caller
                    stat_name: stat.clone(),
                    delta: *modifier,
                }])
            }

            OutcomeTrigger::TriggerScene { scene_id } => {
                debug!(scene_id = %scene_id, "Triggering scene transition");

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::EventTriggered {
                    name: format!("Scene transition to {}", scene_id),
                }])
            }

            OutcomeTrigger::GiveItem {
                item_name,
                item_description: _,
            } => {
                debug!(item_name = %item_name, "Giving item");

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::ItemAdded {
                    character: "active_pc".to_string(),
                    item: item_name.clone(),
                }])
            }

            OutcomeTrigger::Custom { description } => {
                debug!(description = %description, "Custom trigger");

                // Conversation history is now managed by caller via WorldStateManager
                Ok(vec![StateChange::EventTriggered {
                    name: format!("Custom: {}", description),
                }])
            }
        }
    }
}

// =============================================================================
// Port implementation
// =============================================================================

#[async_trait]
impl OutcomeTriggerServicePort for OutcomeTriggerService {
    async fn execute_triggers(
        &self,
        triggers: Vec<OutcomeTrigger>,
        world_id: WorldId,
    ) -> OutcomeTriggerExecutionResult {
        OutcomeTriggerService::execute_triggers(self, triggers, world_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::WorldId;
    use wrldbldr_engine_ports::outbound::MockChallengeRepository;

    #[tokio::test]
    async fn test_reveal_information() {
        let mock = MockChallengeRepository::new();
        let service = OutcomeTriggerService::new(Arc::new(mock));
        let world_id = WorldId::new();

        let triggers = vec![OutcomeTrigger::reveal("A secret passage is revealed!")];

        let result = service.execute_triggers(triggers, world_id).await;

        assert_eq!(result.trigger_count, 1);
        assert_eq!(result.state_changes.len(), 1);
        assert!(result.warnings.is_empty());
        assert!(matches!(
            &result.state_changes[0],
            StateChange::InfoRevealed { info } if info.contains("secret passage")
        ));
    }

    #[tokio::test]
    async fn test_give_item() {
        let mock = MockChallengeRepository::new();
        let service = OutcomeTriggerService::new(Arc::new(mock));
        let world_id = WorldId::new();

        let triggers = vec![OutcomeTrigger::GiveItem {
            item_name: "Magic Sword".to_string(),
            item_description: Some("A blade that glows blue".to_string()),
        }];

        let result = service.execute_triggers(triggers, world_id).await;

        assert_eq!(result.trigger_count, 1);
        assert!(matches!(
            &result.state_changes[0],
            StateChange::ItemAdded { item, .. } if item == "Magic Sword"
        ));
    }

    #[tokio::test]
    async fn test_multiple_triggers() {
        let mock = MockChallengeRepository::new();
        let service = OutcomeTriggerService::new(Arc::new(mock));
        let world_id = WorldId::new();

        let triggers = vec![
            OutcomeTrigger::reveal("You learn the password"),
            OutcomeTrigger::GiveItem {
                item_name: "Key".to_string(),
                item_description: None,
            },
            OutcomeTrigger::modify_stat("reputation", 10),
        ];

        let result = service.execute_triggers(triggers, world_id).await;

        assert_eq!(result.trigger_count, 3);
        assert_eq!(result.state_changes.len(), 3);
        assert!(result.warnings.is_empty());
    }
}
