//! Execute event effects use case.
//!
//! When a narrative event triggers and the DM approves, this executes the effects
//! from the selected outcome (give items, modify relationships, enable/disable challenges, etc.)

use std::sync::Arc;

use serde_json::json;
use wrldbldr_domain::{
    CharacterId, EventEffect, NarrativeEventId, PlayerCharacterId, RelationshipEvent,
    RelationshipType, SceneId, WorldId,
};

use crate::repositories::{
    Challenge, Flag, Inventory, Observation, PlayerCharacter, World,
};
use crate::repositories::character::Character;
use crate::use_cases::narrative_operations::Narrative;
use crate::repositories::scene::Scene;
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
/// - SetFlag via Flag
pub struct ExecuteEffects {
    inventory: Arc<Inventory>,
    challenge: Arc<Challenge>,
    narrative: Arc<Narrative>,
    character: Arc<Character>,
    observation: Arc<Observation>,
    player_character: Arc<PlayerCharacter>,
    scene: Arc<Scene>,
    flag: Arc<Flag>,
    world: Arc<World>,
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
        flag: Arc<Flag>,
        world: Arc<World>,
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
            flag,
            world,
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
        let failure_count = results
            .iter()
            .filter(|r| !r.success && !r.requires_dm_action)
            .count();

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

            EventEffect::TakeItem {
                item_name,
                quantity,
            } => {
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
                self.execute_set_flag(context.world_id, flag_name, *value)
                    .await
            }

            EventEffect::StartCombat {
                participants,
                participant_names,
                combat_description,
            } => {
                // Validate parameters before reporting unimplemented
                if participants.is_empty() && participant_names.is_empty() {
                    return EffectExecutionResult {
                        description: "Start combat effect has no participants".to_string(),
                        success: false,
                        error: Some("Combat effect requires at least one participant".to_string()),
                        requires_dm_action: false,
                    };
                }
                if combat_description.trim().is_empty() {
                    return EffectExecutionResult {
                        description: "Start combat effect has empty description".to_string(),
                        success: false,
                        error: Some("Combat effect requires a description".to_string()),
                        requires_dm_action: false,
                    };
                }

                // Combat system not implemented - DM should handle manually
                let participants_display = if !participant_names.is_empty() {
                    participant_names.join(", ")
                } else {
                    // Fall back to showing IDs when names aren't provided
                    participants
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                EffectExecutionResult {
                    description: format!(
                        "Start combat: {} with {} (NOT IMPLEMENTED - combat system needed)",
                        combat_description, participants_display
                    ),
                    success: false,
                    error: Some("Combat system not implemented".to_string()),
                    requires_dm_action: true,
                }
            }

            EventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => {
                // Validate parameters
                if reward_type.trim().is_empty() {
                    return EffectExecutionResult {
                        description: "Add reward effect has empty reward type".to_string(),
                        success: false,
                        error: Some("Reward effect requires a reward type (e.g., 'gold', 'xp', 'item')".to_string()),
                        requires_dm_action: false,
                    };
                }
                if *amount == 0 {
                    return EffectExecutionResult {
                        description: format!("Add {} reward with zero amount", reward_type),
                        success: false,
                        error: Some("Reward amount must be greater than zero".to_string()),
                        requires_dm_action: false,
                    };
                }
                if description.trim().is_empty() {
                    return EffectExecutionResult {
                        description: format!("Add {} x{} reward with empty description", reward_type, amount),
                        success: false,
                        error: Some("Reward effect requires a description".to_string()),
                        requires_dm_action: false,
                    };
                }

                // Handle XP rewards - use system-aware field mapping
                let reward_type_lower = reward_type.to_lowercase();
                if reward_type_lower == "xp"
                    || reward_type_lower == "experience"
                    || reward_type_lower == "exp"
                {
                    self.execute_add_xp_reward_system_aware(
                        context.pc_id,
                        context.world_id,
                        *amount,
                        description,
                    )
                    .await
                } else if reward_type_lower == "gold"
                    || reward_type_lower == "gp"
                    || reward_type_lower == "coins"
                {
                    // Gold rewards - use system-aware gold field
                    self.execute_add_gold_reward_system_aware(
                        context.pc_id,
                        context.world_id,
                        *amount,
                        description,
                    )
                    .await
                } else if reward_type_lower == "fate"
                    || reward_type_lower == "fate_points"
                    || reward_type_lower == "fp"
                {
                    // FATE points - for FATE Core system
                    self.execute_add_stat_reward(
                        context.pc_id,
                        "CURRENT_FATE_POINTS",
                        *amount,
                        description,
                    )
                    .await
                } else {
                    // Other reward types - mark for DM handling
                    EffectExecutionResult {
                        description: format!(
                            "Add {} {} reward: {} (requires DM to apply)",
                            amount, reward_type, description
                        ),
                        success: true,
                        error: None,
                        requires_dm_action: true,
                    }
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
                // Find existing relationship to target
                let existing_relationship = relationships
                    .into_iter()
                    .find(|r| r.to_character == to_character);

                // If no existing relationship, create a new one with logging
                let (mut relationship, is_new_relationship) = match existing_relationship {
                    Some(rel) => (rel, false),
                    None => {
                        tracing::info!(
                            from_character = %from_character,
                            from_name = %from_name,
                            to_character = %to_character,
                            to_name = %to_name,
                            reason = %reason,
                            "Creating new relationship (none existed) with default type 'Acquaintance'"
                        );
                        (
                            wrldbldr_domain::Relationship::new(
                                from_character,
                                to_character,
                                RelationshipType::Custom("Acquaintance".to_string()),
                            ),
                            true,
                        )
                    }
                };

                // Validate sentiment change is a finite number
                if !sentiment_change.is_finite() {
                    tracing::warn!(
                        from_character = %from_character,
                        to_character = %to_character,
                        sentiment_change = %sentiment_change,
                        "Invalid sentiment change value (NaN or Infinity), skipping"
                    );
                    return EffectExecutionResult {
                        description: format!(
                            "Invalid sentiment change value: {}",
                            sentiment_change
                        ),
                        success: false,
                        error: Some("Sentiment change is NaN or Infinity".to_string()),
                        requires_dm_action: false,
                    };
                }

                // Apply sentiment change
                let old_sentiment = relationship.sentiment;
                relationship.sentiment =
                    (relationship.sentiment + sentiment_change).clamp(-1.0, 1.0);

                // Add event to history
                relationship.add_event(RelationshipEvent {
                    description: reason.to_string(),
                    sentiment_change,
                    timestamp: self.clock.now(),
                });

                // Save updated relationship
                match self.character.save_relationship(&relationship).await {
                    Ok(()) => {
                        let action = if is_new_relationship {
                            "Created new relationship"
                        } else {
                            "Modified relationship"
                        };
                        EffectExecutionResult {
                            description: format!(
                                "{}: {} -> {} (sentiment {:.2} -> {:.2}, reason: {})",
                                action, from_name, to_name, old_sentiment, relationship.sentiment, reason
                            ),
                            success: true,
                            error: None,
                            requires_dm_action: false,
                        }
                    }
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
            match self
                .observation
                .record_deduced_info(pc_id, info_entry)
                .await
            {
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

    async fn execute_set_flag(
        &self,
        world_id: WorldId,
        flag_name: &str,
        value: bool,
    ) -> EffectExecutionResult {
        let result = if value {
            self.flag.set_world_flag(world_id, flag_name).await
        } else {
            self.flag.unset_world_flag(world_id, flag_name).await
        };

        match result {
            Ok(()) => EffectExecutionResult {
                description: format!(
                    "Set flag '{}' to {}",
                    flag_name,
                    if value { "true" } else { "false" }
                ),
                success: true,
                error: None,
                requires_dm_action: false,
            },
            Err(e) => EffectExecutionResult {
                description: format!("Failed to set flag '{}'", flag_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    /// Add XP reward to a player character.
    /// Updates XP_CURRENT in the character's sheet_data.
    #[allow(dead_code)]
    async fn execute_add_xp_reward(
        &self,
        pc_id: PlayerCharacterId,
        amount: i32,
        description: &str,
    ) -> EffectExecutionResult {
        self.execute_add_stat_reward(pc_id, "XP_CURRENT", amount, description)
            .await
    }

    /// Add a numeric stat reward to a player character.
    /// Updates the specified stat in the character's sheet_data.
    async fn execute_add_stat_reward(
        &self,
        pc_id: PlayerCharacterId,
        stat_name: &str,
        amount: i32,
        description: &str,
    ) -> EffectExecutionResult {
        // Get the PC
        let pc = match self.player_character.get(pc_id).await {
            Ok(Some(pc)) => pc,
            Ok(None) => {
                return EffectExecutionResult {
                    description: format!("Failed to add {} {}: PC not found", amount, stat_name),
                    success: false,
                    error: Some("Player character not found".to_string()),
                    requires_dm_action: false,
                };
            }
            Err(e) => {
                return EffectExecutionResult {
                    description: format!("Failed to add {} {}: error loading PC", amount, stat_name),
                    success: false,
                    error: Some(e.to_string()),
                    requires_dm_action: false,
                };
            }
        };

        // Get current value from sheet_data (default to 0 if not present)
        let current_value = pc
            .sheet_data
            .as_ref()
            .and_then(|sd| sd.get_number(stat_name))
            .unwrap_or(0);

        let new_value = current_value.saturating_add(amount as i64);

        // Create updated sheet_data
        let mut sheet_data = pc.sheet_data.clone().unwrap_or_default();
        sheet_data.set(stat_name, json!(new_value));

        // Create updated PC
        let updated_pc = wrldbldr_domain::PlayerCharacter {
            sheet_data: Some(sheet_data),
            ..pc
        };

        // Save the updated PC
        match self.player_character.save(&updated_pc).await {
            Ok(()) => {
                tracing::info!(
                    pc_id = %pc_id,
                    stat = %stat_name,
                    old_value = current_value,
                    added = amount,
                    new_value = new_value,
                    description = %description,
                    "Added stat reward to PC"
                );
                EffectExecutionResult {
                    description: format!(
                        "Added {} {} (now {}): {}",
                        amount, stat_name, new_value, description
                    ),
                    success: true,
                    error: None,
                    requires_dm_action: false,
                }
            }
            Err(e) => EffectExecutionResult {
                description: format!("Failed to save {} reward", stat_name),
                success: false,
                error: Some(e.to_string()),
                requires_dm_action: false,
            },
        }
    }

    /// Add XP reward using system-appropriate field name.
    /// Different game systems track XP differently:
    /// - D&D 5e, Pathfinder 2e: XP_CURRENT
    /// - Powered by the Apocalypse: XP
    /// - Blades in the Dark: PLAYBOOK_XP
    /// - FATE Core, Call of Cthulhu: No XP (milestone/skill-based advancement)
    async fn execute_add_xp_reward_system_aware(
        &self,
        pc_id: PlayerCharacterId,
        world_id: WorldId,
        amount: i32,
        description: &str,
    ) -> EffectExecutionResult {
        use wrldbldr_domain::RuleSystemVariant;

        // Get the world to determine the rule system
        let world = match self.world.get(world_id).await {
            Ok(Some(w)) => w,
            Ok(None) => {
                // Fall back to generic XP_CURRENT if world not found
                tracing::warn!(world_id = %world_id, "World not found, using default XP_CURRENT field");
                return self
                    .execute_add_stat_reward(pc_id, "XP_CURRENT", amount, description)
                    .await;
            }
            Err(e) => {
                return EffectExecutionResult {
                    description: format!("Failed to get world for XP reward: {}", e),
                    success: false,
                    error: Some(e.to_string()),
                    requires_dm_action: false,
                };
            }
        };

        // Map system variant to XP field name
        let xp_field = match world.rule_system.variant {
            RuleSystemVariant::Dnd5e
            | RuleSystemVariant::Pathfinder2e
            | RuleSystemVariant::GenericD20 => "XP_CURRENT",

            RuleSystemVariant::PoweredByApocalypse => "XP",

            RuleSystemVariant::BladesInTheDark => "PLAYBOOK_XP",

            // Systems without XP-based advancement
            RuleSystemVariant::FateCore => {
                return EffectExecutionResult {
                    description: format!(
                        "FATE Core uses milestone advancement, not XP. {} XP reward noted for DM.",
                        amount
                    ),
                    success: true,
                    error: None,
                    requires_dm_action: true,
                };
            }
            RuleSystemVariant::CallOfCthulhu7e => {
                return EffectExecutionResult {
                    description: format!(
                        "Call of Cthulhu uses skill improvement checks, not XP. {} XP reward noted for DM.",
                        amount
                    ),
                    success: true,
                    error: None,
                    requires_dm_action: true,
                };
            }

            // Other systems - use generic XP field
            RuleSystemVariant::KidsOnBikes
            | RuleSystemVariant::RuneQuest
            | RuleSystemVariant::GenericD100
            | RuleSystemVariant::Custom(_)
            | RuleSystemVariant::Unknown => "XP_CURRENT",
        };

        self.execute_add_stat_reward(pc_id, xp_field, amount, description)
            .await
    }

    /// Add gold/currency reward using system-appropriate field name.
    /// Different game systems track currency differently:
    /// - D&D 5e, Pathfinder 2e: GP (gold pieces)
    /// - Blades in the Dark: COIN
    /// - Call of Cthulhu: SPENDING_LEVEL or specific currency
    /// - FATE Core, PbtA: Usually narrative-based (no currency tracking)
    async fn execute_add_gold_reward_system_aware(
        &self,
        pc_id: PlayerCharacterId,
        world_id: WorldId,
        amount: i32,
        description: &str,
    ) -> EffectExecutionResult {
        use wrldbldr_domain::RuleSystemVariant;

        // Get the world to determine the rule system
        let world = match self.world.get(world_id).await {
            Ok(Some(w)) => w,
            Ok(None) => {
                // Fall back to generic GP if world not found
                tracing::warn!(world_id = %world_id, "World not found, using default GP field");
                return self
                    .execute_add_stat_reward(pc_id, "GP", amount, description)
                    .await;
            }
            Err(e) => {
                return EffectExecutionResult {
                    description: format!("Failed to get world for gold reward: {}", e),
                    success: false,
                    error: Some(e.to_string()),
                    requires_dm_action: false,
                };
            }
        };

        // Map system variant to currency field name
        match world.rule_system.variant {
            RuleSystemVariant::Dnd5e
            | RuleSystemVariant::Pathfinder2e
            | RuleSystemVariant::GenericD20 => {
                self.execute_add_stat_reward(pc_id, "GP", amount, description)
                    .await
            }

            RuleSystemVariant::BladesInTheDark => {
                self.execute_add_stat_reward(pc_id, "COIN", amount, description)
                    .await
            }

            RuleSystemVariant::CallOfCthulhu7e => {
                // CoC uses Credit Rating and spending, not direct gold
                EffectExecutionResult {
                    description: format!(
                        "Call of Cthulhu uses Credit Rating for wealth. {} coins noted for DM.",
                        amount
                    ),
                    success: true,
                    error: None,
                    requires_dm_action: true,
                }
            }

            // Narrative systems typically don't track currency
            RuleSystemVariant::FateCore | RuleSystemVariant::PoweredByApocalypse => {
                EffectExecutionResult {
                    description: format!(
                        "Narrative system typically doesn't track currency. {} gold/coins noted for DM.",
                        amount
                    ),
                    success: true,
                    error: None,
                    requires_dm_action: true,
                }
            }

            // Other systems - use generic GP field
            RuleSystemVariant::KidsOnBikes
            | RuleSystemVariant::RuneQuest
            | RuleSystemVariant::GenericD100
            | RuleSystemVariant::Custom(_)
            | RuleSystemVariant::Unknown => {
                self.execute_add_stat_reward(pc_id, "GP", amount, description)
                    .await
            }
        }
    }
}
