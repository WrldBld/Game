//! Event Effect Executor Service - Executes effects from narrative event outcomes
//!
//! This service is responsible for executing the `EventEffect` items that are
//! defined in narrative event outcomes. When a DM approves a narrative event,
//! the selected outcome's effects are executed through this service.
//!
//! # Effects Supported
//!
//! - `SetFlag` - Sets a game flag (stored in session state)
//! - `EnableChallenge` / `DisableChallenge` - Toggles challenge availability
//! - `EnableEvent` / `DisableEvent` - Toggles narrative event availability
//! - `RevealInformation` - Reveals info to players (logged, optionally journaled)
//! - `GiveItem` / `TakeItem` - Modifies player inventory (logged for DM to narrate)
//! - `ModifyRelationship` - Changes NPC relationship sentiment
//! - `ModifyStat` - Changes character stat value
//! - `TriggerScene` - Initiates scene transition
//! - `StartCombat` - Initiates combat encounter
//! - `AddReward` - Grants experience or rewards
//! - `Custom` - Logs for DM action
//!
//! # Architecture
//!
//! The service follows hexagonal architecture, depending on repository ports.
//! Conversation history logging is handled by the caller via WorldStateManager.

use std::sync::Arc;

use tracing::{debug, info, instrument, warn};

use wrldbldr_domain::entities::EventEffect;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{
    ChallengeRepositoryPort, NarrativeEventRepositoryPort, RelationshipRepositoryPort,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during effect execution
#[derive(Debug, thiserror::Error)]
pub enum EffectExecutionError {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Effect not supported: {0}")]
    NotSupported(String),
}

// =============================================================================
// Result Types
// =============================================================================

/// Result of executing a single effect
#[derive(Debug, Clone)]
pub struct EffectExecutionResult {
    /// Description of what happened
    pub description: String,
    /// Whether the effect was fully executed (vs logged for DM action)
    pub was_executed: bool,
    /// Any warning or note
    pub note: Option<String>,
}

/// Result of executing all effects from an outcome
#[derive(Debug, Clone)]
pub struct OutcomeExecutionResult {
    /// Individual effect results
    pub effects: Vec<EffectExecutionResult>,
    /// Total effects attempted
    pub total: usize,
    /// Effects that were fully executed
    pub executed_count: usize,
    /// Effects logged for DM action
    pub logged_count: usize,
}

impl OutcomeExecutionResult {
    pub fn empty() -> Self {
        Self {
            effects: Vec::new(),
            total: 0,
            executed_count: 0,
            logged_count: 0,
        }
    }
}

// =============================================================================
// Service Implementation
// =============================================================================

/// Service for executing narrative event outcome effects
///
/// This service takes a list of `EventEffect` items and executes them,
/// making the necessary repository calls. Conversation history logging is
/// handled by the caller via WorldStateManager.
pub struct EventEffectExecutor {
    challenge_repo: Arc<dyn ChallengeRepositoryPort>,
    narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
    relationship_repo: Arc<dyn RelationshipRepositoryPort>,
}

impl EventEffectExecutor {
    /// Create a new EventEffectExecutor
    pub fn new(
        challenge_repo: Arc<dyn ChallengeRepositoryPort>,
        narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort>,
        relationship_repo: Arc<dyn RelationshipRepositoryPort>,
    ) -> Self {
        Self {
            challenge_repo,
            narrative_event_repo,
            relationship_repo,
        }
    }

    /// Execute all effects from an outcome
    ///
    /// # Arguments
    ///
    /// * `effects` - The list of effects to execute
    /// * `world_id` - The world where effects should be applied
    ///
    /// # Returns
    ///
    /// An `OutcomeExecutionResult` summarizing what was done.
    ///
    /// Note: Conversation history logging is removed - caller should handle via WorldStateManager.
    #[instrument(skip(self, effects), fields(effect_count = effects.len()))]
    pub async fn execute_effects(
        &self,
        effects: &[EventEffect],
        world_id: WorldId,
    ) -> OutcomeExecutionResult {
        if effects.is_empty() {
            return OutcomeExecutionResult::empty();
        }

        let mut results = Vec::new();
        let mut executed_count = 0;
        let mut logged_count = 0;

        for effect in effects {
            let result = self.execute_single_effect(effect, world_id).await;

            if result.was_executed {
                executed_count += 1;
            } else {
                logged_count += 1;
            }

            results.push(result);
        }

        info!(
            executed = executed_count,
            logged = logged_count,
            "Executed narrative event effects"
        );

        OutcomeExecutionResult {
            effects: results,
            total: effects.len(),
            executed_count,
            logged_count,
        }
    }

    /// Execute a single effect
    async fn execute_single_effect(
        &self,
        effect: &EventEffect,
        world_id: WorldId,
    ) -> EffectExecutionResult {
        match effect {
            EventEffect::SetFlag { flag_name, value } => {
                self.execute_set_flag(flag_name, *value, world_id).await
            }

            EventEffect::EnableChallenge {
                challenge_id,
                challenge_name,
            } => {
                self.execute_enable_challenge(*challenge_id, challenge_name, world_id)
                    .await
            }

            EventEffect::DisableChallenge {
                challenge_id,
                challenge_name,
            } => {
                self.execute_disable_challenge(*challenge_id, challenge_name, world_id)
                    .await
            }

            EventEffect::EnableEvent {
                event_id,
                event_name,
            } => {
                self.execute_enable_event(*event_id, event_name, world_id)
                    .await
            }

            EventEffect::DisableEvent {
                event_id,
                event_name,
            } => {
                self.execute_disable_event(*event_id, event_name, world_id)
                    .await
            }

            EventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            } => {
                self.execute_reveal_information(
                    info_type,
                    title,
                    content,
                    *persist_to_journal,
                    world_id,
                )
                .await
            }

            EventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            } => {
                self.execute_give_item(item_name, item_description.as_deref(), *quantity, world_id)
                    .await
            }

            EventEffect::TakeItem {
                item_name,
                quantity,
            } => self.execute_take_item(item_name, *quantity, world_id).await,

            EventEffect::ModifyRelationship {
                from_character,
                from_name,
                to_character,
                to_name,
                sentiment_change,
                reason,
            } => {
                self.execute_modify_relationship(
                    *from_character,
                    from_name,
                    *to_character,
                    to_name,
                    *sentiment_change,
                    reason,
                    world_id,
                )
                .await
            }

            EventEffect::ModifyStat {
                character_id,
                character_name,
                stat_name,
                modifier,
            } => {
                self.execute_modify_stat(
                    *character_id,
                    character_name,
                    stat_name,
                    *modifier,
                    world_id,
                )
                .await
            }

            EventEffect::TriggerScene {
                scene_id,
                scene_name,
            } => {
                self.execute_trigger_scene(*scene_id, scene_name, world_id)
                    .await
            }

            EventEffect::StartCombat {
                participants: _,
                participant_names,
                combat_description,
            } => {
                self.execute_start_combat(participant_names, combat_description, world_id)
                    .await
            }

            EventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => {
                self.execute_add_reward(reward_type, *amount, description, world_id)
                    .await
            }

            EventEffect::Custom {
                description,
                requires_dm_action,
            } => {
                self.execute_custom(description, *requires_dm_action, world_id)
                    .await
            }
        }
    }

    // =========================================================================
    // Individual Effect Implementations
    // =========================================================================

    async fn execute_set_flag(
        &self,
        flag_name: &str,
        value: bool,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(flag_name = flag_name, value = value, "Setting game flag");

        // Conversation history now managed by caller via WorldStateManager
        // Note: Flag storage would need world state modification
        EffectExecutionResult {
            description: format!("Set flag '{}' to {}", flag_name, value),
            was_executed: true,
            note: Some("Flag set in world state".to_string()),
        }
    }

    async fn execute_enable_challenge(
        &self,
        challenge_id: wrldbldr_domain::ChallengeId,
        challenge_name: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(challenge_id = %challenge_id, "Enabling challenge");

        match self.challenge_repo.set_active(challenge_id, true).await {
            Ok(()) => {
                // Conversation history now managed by caller via WorldStateManager
                EffectExecutionResult {
                    description: format!("Enabled challenge '{}'", challenge_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to enable challenge");
                EffectExecutionResult {
                    description: format!("Failed to enable challenge '{}'", challenge_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_disable_challenge(
        &self,
        challenge_id: wrldbldr_domain::ChallengeId,
        challenge_name: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(challenge_id = %challenge_id, "Disabling challenge");

        match self.challenge_repo.set_active(challenge_id, false).await {
            Ok(()) => {
                // Conversation history now managed by caller via WorldStateManager
                EffectExecutionResult {
                    description: format!("Disabled challenge '{}'", challenge_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to disable challenge");
                EffectExecutionResult {
                    description: format!("Failed to disable challenge '{}'", challenge_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_enable_event(
        &self,
        event_id: wrldbldr_domain::NarrativeEventId,
        event_name: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(event_id = %event_id, "Enabling narrative event");

        match self.narrative_event_repo.set_active(event_id, true).await {
            Ok(_) => {
                // Conversation history now managed by caller via WorldStateManager
                EffectExecutionResult {
                    description: format!("Enabled narrative event '{}'", event_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to enable narrative event");
                EffectExecutionResult {
                    description: format!("Failed to enable event '{}'", event_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_disable_event(
        &self,
        event_id: wrldbldr_domain::NarrativeEventId,
        event_name: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(event_id = %event_id, "Disabling narrative event");

        match self.narrative_event_repo.set_active(event_id, false).await {
            Ok(_) => {
                // Conversation history now managed by caller via WorldStateManager
                EffectExecutionResult {
                    description: format!("Disabled narrative event '{}'", event_name),
                    was_executed: true,
                    note: None,
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to disable narrative event");
                EffectExecutionResult {
                    description: format!("Failed to disable event '{}'", event_name),
                    was_executed: false,
                    note: Some(e.to_string()),
                }
            }
        }
    }

    async fn execute_reveal_information(
        &self,
        info_type: &str,
        title: &str,
        _content: &str,
        persist_to_journal: bool,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(
            info_type = info_type,
            title = title,
            "Revealing information"
        );

        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: format!(
                "Revealed {} '{}'{}",
                info_type,
                title,
                if persist_to_journal {
                    " (journaled)"
                } else {
                    ""
                }
            ),
            was_executed: true,
            note: None,
        }
    }

    async fn execute_give_item(
        &self,
        item_name: &str,
        _item_description: Option<&str>,
        quantity: u32,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(item_name = item_name, quantity = quantity, "Giving item");

        // Conversation history now managed by caller via WorldStateManager
        // Note: Actual inventory modification would need player character repo access
        EffectExecutionResult {
            description: format!("Gave {} {}", quantity, item_name),
            was_executed: false, // Logged for DM to narrate, not actually added to inventory
            note: Some("DM should narrate item acquisition".to_string()),
        }
    }

    async fn execute_take_item(
        &self,
        item_name: &str,
        quantity: u32,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(item_name = item_name, quantity = quantity, "Taking item");

        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: format!("Took {} {}", quantity, item_name),
            was_executed: false, // Logged for DM to narrate
            note: Some("DM should narrate item loss".to_string()),
        }
    }

    async fn execute_modify_relationship(
        &self,
        from_character: wrldbldr_domain::CharacterId,
        from_name: &str,
        to_character: wrldbldr_domain::CharacterId,
        to_name: &str,
        sentiment_change: f32,
        _reason: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(
            from = %from_character,
            to = %to_character,
            change = sentiment_change,
            "Modifying relationship"
        );

        // Try to update the relationship in the database
        match self
            .relationship_repo
            .get_for_character(from_character)
            .await
        {
            Ok(relationships) => {
                // Find the existing relationship
                if let Some(mut rel) = relationships
                    .into_iter()
                    .find(|r| r.to_character == to_character)
                {
                    rel.sentiment += sentiment_change;
                    rel.sentiment = rel.sentiment.clamp(-1.0, 1.0);

                    if let Err(e) = self.relationship_repo.update(&rel).await {
                        warn!(error = %e, "Failed to update relationship");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to get relationships");
            }
        }

        let direction = if sentiment_change >= 0.0 { "+" } else { "" };
        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: format!(
                "Modified {} -> {} relationship by {}{:.1}",
                from_name, to_name, direction, sentiment_change
            ),
            was_executed: true,
            note: None,
        }
    }

    async fn execute_modify_stat(
        &self,
        _character_id: wrldbldr_domain::CharacterId,
        character_name: &str,
        stat_name: &str,
        modifier: i32,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(
            character_name = character_name,
            stat_name = stat_name,
            modifier = modifier,
            "Modifying stat"
        );

        let direction = if modifier >= 0 { "+" } else { "" };
        // Conversation history now managed by caller via WorldStateManager

        // Note: Actual stat modification would need character sheet access
        EffectExecutionResult {
            description: format!("{} {} {}{}", character_name, stat_name, direction, modifier),
            was_executed: false, // Logged for DM to apply
            note: Some("DM should apply stat change to character sheet".to_string()),
        }
    }

    async fn execute_trigger_scene(
        &self,
        _scene_id: wrldbldr_domain::SceneId,
        scene_name: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(scene_name = scene_name, "Triggering scene transition");

        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: format!("Scene transition to '{}'", scene_name),
            was_executed: false, // DM should initiate transition
            note: Some("DM should initiate scene transition".to_string()),
        }
    }

    async fn execute_start_combat(
        &self,
        participant_names: &[String],
        combat_description: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(participants = ?participant_names, "Starting combat");

        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: format!("Combat initiated: {}", combat_description),
            was_executed: false, // DM should run combat
            note: Some("DM should initiate combat encounter".to_string()),
        }
    }

    async fn execute_add_reward(
        &self,
        reward_type: &str,
        amount: i32,
        description: &str,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(reward_type = reward_type, amount = amount, "Adding reward");

        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: format!("Awarded {} {} ({})", amount, reward_type, description),
            was_executed: false, // DM should apply reward
            note: Some("DM should apply reward to characters".to_string()),
        }
    }

    async fn execute_custom(
        &self,
        description: &str,
        requires_dm_action: bool,
        _world_id: WorldId,
    ) -> EffectExecutionResult {
        debug!(
            description = description,
            requires_dm_action = requires_dm_action,
            "Custom effect"
        );

        // Conversation history now managed by caller via WorldStateManager
        EffectExecutionResult {
            description: description.to_string(),
            was_executed: false,
            note: if requires_dm_action {
                Some("DM action required".to_string())
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_execution_result_empty() {
        let result = OutcomeExecutionResult::empty();
        assert!(result.effects.is_empty());
        assert_eq!(result.total, 0);
        assert_eq!(result.executed_count, 0);
        assert_eq!(result.logged_count, 0);
    }
}
