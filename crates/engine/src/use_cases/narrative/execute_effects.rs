//! Execute event effects use case.
//!
//! When a narrative event triggers and the DM approves, this executes the effects
//! from the selected outcome (give items, modify relationships, enable/disable challenges, etc.)

use std::sync::Arc;

use wrldbldr_domain::{
    CharacterId, EventEffect, NarrativeEventId, PlayerCharacterId, RelationshipEvent,
    RelationshipType, SceneId, WorldId,
};

use crate::entities::{Challenge, Character, Inventory, Narrative, Observation, PlayerCharacter, Scene};
use crate::infrastructure::ports::ClockPort;

/// Result of executing a single effect.
#[derive(Debug, Clone)]
pub struct EffectExecutionResult {
    /// Description of what was done
    pub description: String,
    /// Whether the effect was successfully executed
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Whether this effect requires DM action (for Custom effects)
    pub requires_dm_action: bool,
}

/// Summary of executing all effects for an event outcome.
#[derive(Debug)]
pub struct EffectExecutionSummary {
    /// The event that was triggered
    pub event_id: NarrativeEventId,
    /// The outcome that was selected
    pub outcome_name: String,
    /// Results for each effect
    pub results: Vec<EffectExecutionResult>,
    /// Count of successfully executed effects
    pub success_count: usize,
    /// Count of failed effects
    pub failure_count: usize,
    /// Effects that require DM action
    pub pending_dm_actions: Vec<String>,
}

/// Context for effect execution - provides the "who" and "where" for effects.
#[derive(Debug, Clone)]
pub struct EffectExecutionContext {
    /// The player character receiving effects (for GiveItem, ModifyStat, etc.)
    pub pc_id: PlayerCharacterId,
    /// The world where effects are applied
    pub world_id: WorldId,
    /// Optional scene context (for TriggerScene)
    pub current_scene_id: Option<SceneId>,
}

/// Executes event effects when narrative events trigger.
///
/// Orchestrates multiple entity modules to apply effects like:
/// - GiveItem / TakeItem via Inventory
/// - EnableChallenge / DisableChallenge via Challenge
/// - EnableEvent / DisableEvent via Narrative
/// - ModifyRelationship via Character
/// - RevealInformation via Observation
/// - ModifyStat via PlayerCharacter
/// - TriggerScene via Scene
pub struct ExecuteEffects {
    inventory: Arc<Inventory>,
    challenge: Arc<Challenge>,
    narrative: Arc<Narrative>,
    character: Arc<Character>,
    observation: Arc<Observation>,
    player_character: Arc<PlayerCharacter>,
    scene: Arc<Scene>,
    clock: Arc<dyn ClockPort>,
}

impl ExecuteEffects {
    pub fn new(
        inventory: Arc<Inventory>,
        challenge: Arc<Challenge>,
        narrative: Arc<Narrative>,
        character: Arc<Character>,
        observation: Arc<Observation>,
        player_character: Arc<PlayerCharacter>,
        scene: Arc<Scene>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            inventory,
            challenge,
            narrative,
            character,
            observation,
            player_character,
            scene,
            clock,
        }
    }

    /// Execute all effects from an event outcome.
    ///
    /// # Arguments
    /// * `event_id` - The narrative event being triggered
    /// * `outcome_name` - Name of the selected outcome
    /// * `effects` - List of effects to execute
    /// * `context` - Execution context (PC, world, etc.)
    ///
    /// # Returns
    /// Summary of all effect executions
    pub async fn execute(
        &self,
        event_id: NarrativeEventId,
        outcome_name: String,
        effects: &[EventEffect],
        context: &EffectExecutionContext,
    ) -> EffectExecutionSummary {
        let mut results = Vec::new();
        let mut pending_dm_actions = Vec::new();

        for effect in effects {
            let result = self.execute_single_effect(effect, context).await;
            
            if result.requires_dm_action {
                pending_dm_actions.push(result.description.clone());
            }
            
            results.push(result);
        }

        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.iter().filter(|r| !r.success && !r.requires_dm_action).count();

        tracing::info!(
            event_id = %event_id,
            outcome = %outcome_name,
            total_effects = results.len(),
            success_count = success_count,
            failure_count = failure_count,
            pending_dm = pending_dm_actions.len(),
            "Executed event effects"
        );

        EffectExecutionSummary {
            event_id,
            outcome_name,
            results,
            success_count,
            failure_count,
            pending_dm_actions,
        }
    }

    /// Execute a single effect.
    async fn execute_single_effect(
        &self,
        effect: &EventEffect,
        context: &EffectExecutionContext,
    ) -> EffectExecutionResult {
        match effect {
            EventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            } => {
                self.execute_give_item(
                    context.pc_id,
                    item_name.clone(),
                    item_description.clone(),
                    *quantity,
                )
                .await
            }

            EventEffect::TakeItem { item_name, quantity } => {
                self.execute_take_item(context.pc_id, item_name, *quantity)
                    .await
            }

            EventEffect::EnableChallenge {
                challenge_id,
                challenge_name,
            } => {
                self.execute_enable_challenge(*challenge_id, challenge_name)
                    .await
            }

            EventEffect::DisableChallenge {
                challenge_id,
                challenge_name,
            } => {
                self.execute_disable_challenge(*challenge_id, challenge_name)
                    .await
            }

            EventEffect::EnableEvent {
                event_id,
                event_name,
            } => self.execute_enable_event(*event_id, event_name).await,

            EventEffect::DisableEvent {
                event_id,
                event_name,
            } => self.execute_disable_event(*event_id, event_name).await,

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
                )
                .await
            }

            EventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            } => {
                self.execute_reveal_information(
                    context.pc_id,
                    info_type,
                    title,
                    content,
                    *persist_to_journal,
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
                    context.pc_id, // Note: Uses PC from context, not character_id (which may be NPC)
                    *character_id,
                    character_name,
                    stat_name,
                    *modifier,
                )
                .await
            }

            EventEffect::TriggerScene {
                scene_id,
                scene_name,
            } => {
                self.execute_trigger_scene(context.world_id, *scene_id, scene_name)
                    .await
            }

            EventEffect::SetFlag { flag_name, value } => {
                // GAP: Flag storage system not implemented
                EffectExecutionResult {
                    description: format!(
                        "Set flag '{}' to {} (NOT IMPLEMENTED - flag storage needed)",
                        flag_name, value
                    ),
                    success: false,
                    error: Some("Flag storage system not implemented".to_string()),
                    requires_dm_action: false,
                }
            }

            EventEffect::StartCombat {
                participants: _,
                participant_names,
                combat_description,
            } => {
                // GAP: Combat system not implemented
                EffectExecutionResult {
                    description: format!(
                        "Start combat: {} with {} (NOT IMPLEMENTED - combat system needed)",
                        combat_description,
                        participant_names.join(", ")
                    ),
                    success: false,
                    error: Some("Combat system not implemented".to_string()),
                    requires_dm_action: true, // DM should handle manually
                }
            }

            EventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => {
                // GAP: Reward/XP system not implemented
                EffectExecutionResult {
                    description: format!(
                        "Add {} {} reward: {} (NOT IMPLEMENTED - reward system needed)",
                        amount, reward_type, description
                    ),
                    success: false,
                    error: Some("Reward system not implemented".to_string()),
                    requires_dm_action: true, // DM should handle manually
                }
            }

            EventEffect::Custom {
                description,
                requires_dm_action,
            } => EffectExecutionResult {
                description: format!("Custom effect: {}", description),
                success: true, // Custom effects are "successful" in that they're noted
                error: None,
                requires_dm_action: *requires_dm_action,
            },
        }
    }

    // =========================================================================
    // Individual effect implementations
    // =========================================================================

    async fn execute_give_item(
        &self,
        pc_id: PlayerCharacterId,
        item_name: String,
        item_description: Option<String>,
        quantity: u32,
    ) -> EffectExecutionResult {
        match self
            .inventory
            .give_item_to_pc(pc_id, item_name.clone(), item_description)
            .await
        {
            Ok(result) => EffectExecutionResult {
                description: format!("Gave {} x{} to player", result.item_name, quantity),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to give {} to player", item_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_take_item(
        &self,
        pc_id: PlayerCharacterId,
        item_name: &str,
        quantity: u32,
    ) -> EffectExecutionResult {
        // Find the item in PC's inventory by name
        match self.inventory.get_pc_inventory(pc_id).await {
            Ok(inventory) => {
                if let Some(item) = inventory.iter().find(|i| i.name == item_name) {
                    let item_id = item.id;
                    // For now, just drop it (removes from inventory)
                    match self.inventory.drop_item(pc_id, item_id, quantity).await {
                        Ok(_) => EffectExecutionResult {
                            description: format!("Took {} x{} from player", item_name, quantity),
                            success: true,
                            error: None,
                            requires_dm_action: false,
                        },
                        Err(e) => EffectExecutionResult {
                            description: format!("Failed to take {} from player", item_name),
                            success: false,
                            error: Some(e.to_string()),
                            requires_dm_action: false,
                        },
                    }
                } else {
                    EffectExecutionResult {
                        description: format!("Player doesn't have {}", item_name),
                        success: false,
                        error: Some("Item not in inventory".to_string()),
                        requires_dm_action: false,
                    }
                }
            }
            Err(e) => EffectExecutionResult {
                description: format!("Failed to check player inventory for {}", item_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_enable_challenge(
        &self,
        challenge_id: wrldbldr_domain::ChallengeId,
        challenge_name: &str,
    ) -> EffectExecutionResult {
        match self.challenge.set_enabled(challenge_id, true).await {
            Ok(()) => EffectExecutionResult {
                description: format!("Enabled challenge: {}", challenge_name),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to enable challenge: {}", challenge_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_disable_challenge(
        &self,
        challenge_id: wrldbldr_domain::ChallengeId,
        challenge_name: &str,
    ) -> EffectExecutionResult {
        match self.challenge.set_enabled(challenge_id, false).await {
            Ok(()) => EffectExecutionResult {
                description: format!("Disabled challenge: {}", challenge_name),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to disable challenge: {}", challenge_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_enable_event(
        &self,
        event_id: NarrativeEventId,
        event_name: &str,
    ) -> EffectExecutionResult {
        match self.narrative.set_event_active(event_id, true).await {
            Ok(()) => EffectExecutionResult {
                description: format!("Enabled narrative event: {}", event_name),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to enable event: {}", event_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_disable_event(
        &self,
        event_id: NarrativeEventId,
        event_name: &str,
    ) -> EffectExecutionResult {
        match self.narrative.set_event_active(event_id, false).await {
            Ok(()) => EffectExecutionResult {
                description: format!("Disabled narrative event: {}", event_name),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to disable event: {}", event_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_modify_relationship(
        &self,
        from_character: CharacterId,
        from_name: &str,
        to_character: CharacterId,
        to_name: &str,
        sentiment_change: f32,
        reason: &str,
    ) -> EffectExecutionResult {
        // Get existing relationships from the source character
        match self.character.get_relationships(from_character).await {
            Ok(relationships) => {
                // Find existing relationship to target, or create new one
                let mut relationship = relationships
                    .into_iter()
                    .find(|r| r.to_character == to_character)
                    .unwrap_or_else(|| {
                        wrldbldr_domain::Relationship::new(
                            from_character,
                            to_character,
                            RelationshipType::Custom("Acquaintance".to_string()),
                        )
                    });

                // Apply sentiment change
                let old_sentiment = relationship.sentiment;
                relationship.sentiment = (relationship.sentiment + sentiment_change).clamp(-1.0, 1.0);

                // Add event to history
                relationship.add_event(RelationshipEvent {
                    description: reason.to_string(),
                    sentiment_change,
                    timestamp: self.clock.now(),
                });

                // Save updated relationship
                match self.character.save_relationship(&relationship).await {
                    Ok(()) => EffectExecutionResult {
                        description: format!(
                            "Modified relationship: {} -> {} (sentiment {:.2} -> {:.2}, reason: {})",
                            from_name, to_name, old_sentiment, relationship.sentiment, reason
                        ),
                        success: true,
                        error: None,
                        requires_dm_action: false,
                    },
                    Err(e) => EffectExecutionResult {
                        description: format!(
                            "Failed to save relationship change: {} -> {}",
                            from_name, to_name
                        ),
                        success: false,
                        error: Some(e.to_string()),
                        requires_dm_action: false,
                    },
                }
            }
            Err(e) => EffectExecutionResult {
                description: format!("Failed to get relationships for {}", from_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_reveal_information(
        &self,
        pc_id: PlayerCharacterId,
        info_type: &str,
        title: &str,
        content: &str,
        persist_to_journal: bool,
    ) -> EffectExecutionResult {
        if persist_to_journal {
            // Save as deduced info in the observation system
            let info_entry = format!("[{}] {}: {}", info_type, title, content);
            match self.observation.record_deduced_info(pc_id, info_entry).await {
                Ok(()) => EffectExecutionResult {
                    description: format!("Revealed {} '{}' (saved to journal)", info_type, title),
                    success: true,
                    error: None,
                    requires_dm_action: false,
                },
                Err(e) => EffectExecutionResult {
                    description: format!("Failed to save revealed info: {}", title),
                    success: false,
                    error: Some(e.to_string()),
                    requires_dm_action: false,
                },
            }
        } else {
            // Just log it - the info will be shown to player but not persisted
            EffectExecutionResult {
                description: format!(
                    "Revealed {} '{}': {} (not persisted)",
                    info_type, title, content
                ),
                success: true,
                error: None,
                requires_dm_action: false,
            }
        }
    }

    async fn execute_modify_stat(
        &self,
        pc_id: PlayerCharacterId,
        _character_id: CharacterId,
        character_name: &str,
        stat_name: &str,
        modifier: i32,
    ) -> EffectExecutionResult {
        // Use the PC from context for stat modification
        match self
            .player_character
            .modify_stat(pc_id, stat_name, modifier)
            .await
        {
            Ok(()) => EffectExecutionResult {
                description: format!(
                    "Modified stat {} by {:+} for {}",
                    stat_name, modifier, character_name
                ),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to modify stat {} for {}", stat_name, character_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    async fn execute_trigger_scene(
        &self,
        world_id: WorldId,
        scene_id: SceneId,
        scene_name: &str,
    ) -> EffectExecutionResult {
        match self.scene.set_current(world_id, scene_id).await {
            Ok(()) => EffectExecutionResult {
                description: format!("Triggered scene: {}", scene_name),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to trigger scene: {}", scene_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }
}
