//! Challenge Outcome Approval Service (P3.3)
//!
//! Manages the DM approval workflow for challenge resolutions.
//! After a player rolls, the outcome goes to this service before
//! being broadcast to all players.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::application::dto::{
    ChallengeOutcomeApprovalItem, ChallengeOutcomeDecision, OutcomeSuggestionRequest,
    PendingChallengeResolutionDto,
};
use wrldbldr_engine_ports::outbound::{WorldConnectionPort, LlmPort, PlayerCharacterRepositoryPort, ItemRepositoryPort};
use crate::application::services::{OutcomeSuggestionService, OutcomeTriggerService, PromptTemplateService, SettingsService};
use crate::application::services::tool_execution_service::StateChange;
use wrldbldr_domain::WorldId;

/// Error type for challenge outcome approval operations
#[derive(Debug, thiserror::Error)]
pub enum ChallengeOutcomeError {
    #[error("Resolution not found: {0}")]
    NotFound(String),
    #[error("Session error: {0}")]
    SessionError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Service for managing challenge outcome approvals
///
/// This service holds pending challenge resolutions in memory until the DM
/// approves, edits, or requests suggestions for them.
///
/// Generic over `L: LlmPort` for LLM suggestion generation.
pub struct ChallengeOutcomeApprovalService<L: LlmPort> {
    /// Pending resolutions indexed by resolution_id
    pending: Arc<RwLock<HashMap<String, ChallengeOutcomeApprovalItem>>>,
    /// World connection port for broadcasting messages
    world_connection: Arc<dyn WorldConnectionPort>,
    /// Outcome trigger service for executing triggers
    outcome_trigger_service: Arc<OutcomeTriggerService>,
    /// Player Character repository for inventory management
    pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
    /// Item repository for creating items
    item_repository: Arc<dyn ItemRepositoryPort>,
    /// LLM port for generating outcome suggestions
    llm_port: Option<Arc<L>>,
    /// Settings service for configurable values
    settings_service: Option<Arc<SettingsService>>,
    /// Prompt template service for resolving prompt templates
    prompt_template_service: Arc<PromptTemplateService>,
}

impl<L: LlmPort + 'static> ChallengeOutcomeApprovalService<L> {
    /// Create a new challenge outcome approval service
    pub fn new(
        world_connection: Arc<dyn WorldConnectionPort>,
        outcome_trigger_service: Arc<OutcomeTriggerService>,
        pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
        item_repository: Arc<dyn ItemRepositoryPort>,
        prompt_template_service: Arc<PromptTemplateService>,
    ) -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            world_connection,
            outcome_trigger_service,
            pc_repository,
            item_repository,
            llm_port: None,
            settings_service: None,
            prompt_template_service,
        }
    }

    /// Set the LLM port for generating outcome suggestions
    pub fn with_llm_port(mut self, llm_port: Arc<L>) -> Self {
        self.llm_port = Some(llm_port);
        self
    }

    /// Set the settings service for configurable values
    pub fn with_settings_service(mut self, settings_service: Arc<SettingsService>) -> Self {
        self.settings_service = Some(settings_service);
        self
    }

    /// Queue a challenge resolution for DM approval
    ///
    /// Returns the resolution_id for tracking.
    pub async fn queue_for_approval(
        &self,
        world_id: &WorldId,
        resolution: PendingChallengeResolutionDto,
    ) -> Result<String, ChallengeOutcomeError> {
        let resolution_id = resolution.resolution_id.clone();

        // Convert DTO to approval item
        let item = ChallengeOutcomeApprovalItem {
            resolution_id: resolution.resolution_id.clone(),
            world_id: (*world_id).into(),
            challenge_id: resolution.challenge_id,
            challenge_name: resolution.challenge_name.clone(),
            challenge_description: resolution.challenge_description,
            skill_name: resolution.skill_name,
            character_id: resolution.character_id,
            character_name: resolution.character_name.clone(),
            roll: resolution.roll,
            modifier: resolution.modifier,
            total: resolution.total,
            outcome_type: resolution.outcome_type.clone(),
            outcome_description: resolution.outcome_description.clone(),
            outcome_triggers: resolution
                .outcome_triggers
                .iter()
                .map(|t| wrldbldr_protocol::ProposedToolInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{:?}", t),
                    description: String::new(),
                    arguments: serde_json::json!({}),
                })
                .collect(),
            original_triggers: resolution.outcome_triggers,
            roll_breakdown: resolution.roll_breakdown,
            timestamp: Utc::now(),
            suggestions: None,
            is_generating_suggestions: false,
        };

        // Store in pending map
        {
            let mut pending = self.pending.write().await;
            pending.insert(resolution_id.clone(), item.clone());
        }

        // Notify DM of pending outcome
        self.notify_dm_pending_outcome(world_id, &item).await?;

        // Notify player that roll is awaiting approval
        self.notify_player_awaiting_approval(world_id, &item).await?;

        tracing::info!(
            "Challenge resolution {} queued for DM approval",
            resolution_id
        );

        Ok(resolution_id)
    }

    /// Process DM's decision on a challenge outcome
    pub async fn process_decision(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        decision: ChallengeOutcomeDecision,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            let pending = self.pending.read().await;
            pending
                .get(resolution_id)
                .cloned()
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify world matches
        let world_uuid: uuid::Uuid = (*world_id).into();
        if item.world_id != world_uuid {
            return Err(ChallengeOutcomeError::InvalidState(
                "World mismatch".to_string(),
            ));
        }

        match decision {
            ChallengeOutcomeDecision::Accept => {
                // Broadcast resolution with original description
                self.broadcast_resolution(world_id, &item, None).await?;
                // Remove from pending
                self.remove_pending(resolution_id).await;
            }
            ChallengeOutcomeDecision::Edit {
                modified_description,
            } => {
                // Broadcast resolution with modified description
                self.broadcast_resolution(world_id, &item, Some(modified_description))
                    .await?;
                // Remove from pending
                self.remove_pending(resolution_id).await;
            }
            ChallengeOutcomeDecision::Suggest { guidance } => {
                // Mark as generating suggestions
                self.set_generating_suggestions(resolution_id, true).await;

                // Check if LLM port is configured
                if let Some(ref llm_port) = self.llm_port {
                    tracing::info!(
                        "Generating LLM suggestions for {}: {:?}",
                        resolution_id,
                        guidance
                    );

                    // Build suggestion request
                    let request = OutcomeSuggestionRequest {
                        challenge_id: item.challenge_id.clone(),
                        challenge_name: item.challenge_name.clone(),
                        challenge_description: item.challenge_description.clone(),
                        skill_name: item.skill_name.clone().unwrap_or_default(),
                        outcome_type: item.outcome_type.clone(),
                        roll_context: format!(
                            "rolled {} + {} = {} ({})",
                            item.roll, item.modifier, item.total, item.outcome_type
                        ),
                        guidance,
                        narrative_context: None,
                        world_id: Some(world_id.to_string()),
                    };

                    // Spawn async task to generate suggestions
                    let llm = llm_port.clone();
                    let pending = self.pending.clone();
                    let world_connection = self.world_connection.clone();
                    let resolution_id_owned = resolution_id.to_string();
                    let prompt_template_service = self.prompt_template_service.clone();
                    let world_id_owned = world_id.clone();

                    tokio::spawn(async move {
                        let suggestion_service = OutcomeSuggestionService::new(llm, prompt_template_service);
                        match suggestion_service.generate_suggestions(&request).await {
                            Ok(suggestions) => {
                                // Update suggestions in pending map
                                let mut pending_guard = pending.write().await;
                                if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                                    pending_item.suggestions = Some(suggestions.clone());
                                    pending_item.is_generating_suggestions = false;
                                    drop(pending_guard);

                                    // Notify DM
                                    let message = wrldbldr_protocol::ServerMessage::OutcomeSuggestionReady {
                                        resolution_id: resolution_id_owned.clone(),
                                        suggestions,
                                    };
                                    if let Err(e) = world_connection.send_to_dm(&world_id_owned, message).await {
                                        tracing::error!("Failed to send suggestions to DM: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to generate outcome suggestions for {}: {}",
                                    resolution_id_owned,
                                    e
                                );
                                // Mark as no longer generating
                                let mut pending_guard = pending.write().await;
                                if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                                    pending_item.is_generating_suggestions = false;
                                }
                            }
                        }
                    });
                } else {
                    tracing::warn!(
                        "LLM port not configured, cannot generate suggestions for {}",
                        resolution_id
                    );
                    // Mark as no longer generating
                    self.set_generating_suggestions(resolution_id, false).await;
                }
            }
        }

        Ok(())
    }

    /// Update suggestions for a pending resolution
    pub async fn update_suggestions(
        &self,
        resolution_id: &str,
        suggestions: Vec<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        let mut pending = self.pending.write().await;
        if let Some(item) = pending.get_mut(resolution_id) {
            item.suggestions = Some(suggestions.clone());
            item.is_generating_suggestions = false;

            // Notify DM that suggestions are ready
            let world_id = WorldId::from_uuid(item.world_id);
            drop(pending);

            let message = wrldbldr_protocol::ServerMessage::OutcomeSuggestionReady {
                resolution_id: resolution_id.to_string(),
                suggestions,
            };
            self.world_connection
                .send_to_dm(&world_id, message)
                .await
                .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

            Ok(())
        } else {
            Err(ChallengeOutcomeError::NotFound(resolution_id.to_string()))
        }
    }

    /// Get all pending resolutions for a world
    pub async fn get_pending_for_world(
        &self,
        world_id: &WorldId,
    ) -> Vec<ChallengeOutcomeApprovalItem> {
        let pending = self.pending.read().await;
        pending
            .values()
            .filter(|item| item.world_id == uuid::Uuid::from(*world_id))
            .cloned()
            .collect()
    }

    /// Broadcast the final resolution to all players
    async fn broadcast_resolution(
        &self,
        world_id: &WorldId,
        item: &ChallengeOutcomeApprovalItem,
        modified_description: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        let description = modified_description.unwrap_or_else(|| item.outcome_description.clone());

        // Build ChallengeResolved message
        let message = wrldbldr_protocol::ServerMessage::ChallengeResolved {
            challenge_id: item.challenge_id.clone(),
            challenge_name: item.challenge_name.clone(),
            character_name: item.character_name.clone(),
            roll: item.roll,
            modifier: item.modifier,
            total: item.total,
            outcome: item.outcome_type.clone(),
            outcome_description: description.clone(),
            roll_breakdown: item.roll_breakdown.clone(),
            individual_rolls: None,
        };

        // Broadcast to all world participants
        self.world_connection
            .broadcast_to_world(world_id, message)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

        // Execute outcome triggers if any
        if !item.original_triggers.is_empty() {
            use wrldbldr_domain::entities::OutcomeTrigger;
            
            // Convert DTOs to domain triggers
            let domain_triggers: Vec<OutcomeTrigger> = item
                .original_triggers
                .iter()
                .cloned()
                .map(OutcomeTrigger::from)
                .collect();
            
            let result = self
                .outcome_trigger_service
                .execute_triggers(&domain_triggers, *world_id)
                .await;
            
            // Process state changes from trigger execution
            if !result.state_changes.is_empty() {
                if let Err(e) = self.process_state_changes(&result.state_changes, &item, world_id).await {
                    tracing::warn!(
                        error = %e,
                        "Failed to process some state changes for challenge {}",
                        item.challenge_id
                    );
                }
            }
            
            tracing::info!(
                trigger_count = result.trigger_count,
                state_changes = result.state_changes.len(),
                warnings = ?result.warnings,
                "Executed {} outcome triggers for challenge {}",
                result.trigger_count,
                item.challenge_id
            );
        } else {
            tracing::info!(
                "Challenge {} resolved with outcome: {} (no triggers)",
                item.challenge_id,
                item.outcome_type
            );
        }

        Ok(())
    }

    /// Notify DM of a pending outcome approval
    async fn notify_dm_pending_outcome(
        &self,
        world_id: &WorldId,
        item: &ChallengeOutcomeApprovalItem,
    ) -> Result<(), ChallengeOutcomeError> {
        let message = wrldbldr_protocol::ServerMessage::ChallengeOutcomePending {
            resolution_id: item.resolution_id.clone(),
            challenge_id: item.challenge_id.clone(),
            challenge_name: item.challenge_name.clone(),
            character_id: item.character_id.clone(),
            character_name: item.character_name.clone(),
            roll: item.roll,
            modifier: item.modifier,
            total: item.total,
            outcome_type: item.outcome_type.clone(),
            outcome_description: item.outcome_description.clone(),
            outcome_triggers: item.outcome_triggers.clone(),
            roll_breakdown: item.roll_breakdown.clone(),
        };

        self.world_connection
            .send_to_dm(world_id, message)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

        Ok(())
    }

    /// Notify player that their roll is awaiting DM approval
    async fn notify_player_awaiting_approval(
        &self,
        world_id: &WorldId,
        item: &ChallengeOutcomeApprovalItem,
    ) -> Result<(), ChallengeOutcomeError> {
        let message = wrldbldr_protocol::ServerMessage::ChallengeRollSubmitted {
            challenge_id: item.challenge_id.clone(),
            challenge_name: item.challenge_name.clone(),
            roll: item.roll,
            modifier: item.modifier,
            total: item.total,
            outcome_type: item.outcome_type.clone(),
            status: "pending_approval".to_string(),
        };

        // Broadcast to all world participants (they'll see the roll is pending)
        self.world_connection
            .broadcast_to_world(world_id, message)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

        Ok(())
    }

    /// Remove a resolution from pending
    async fn remove_pending(&self, resolution_id: &str) {
        let mut pending = self.pending.write().await;
        pending.remove(resolution_id);
    }

    /// Mark a resolution as generating suggestions
    async fn set_generating_suggestions(&self, resolution_id: &str, generating: bool) {
        let mut pending = self.pending.write().await;
        if let Some(item) = pending.get_mut(resolution_id) {
            item.is_generating_suggestions = generating;
        }
    }

    /// Request LLM to generate outcome branches for DM selection
    ///
    /// This method is similar to `process_decision` with Suggest, but generates
    /// structured branches instead of simple text suggestions.
    pub async fn request_branches(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            let pending = self.pending.read().await;
            pending
                .get(resolution_id)
                .cloned()
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify world matches
        let world_uuid: uuid::Uuid = (*world_id).into();
        if item.world_id != world_uuid {
            return Err(ChallengeOutcomeError::InvalidState(
                "World mismatch".to_string(),
            ));
        }

        // Mark as generating
        self.set_generating_suggestions(resolution_id, true).await;

        // Check if LLM port is configured
        if let Some(ref llm_port) = self.llm_port {
            tracing::info!(
                "Generating LLM outcome branches for {}: {:?}",
                resolution_id,
                guidance
            );

            // Build suggestion request (same format)
            let request = OutcomeSuggestionRequest {
                challenge_id: item.challenge_id.clone(),
                challenge_name: item.challenge_name.clone(),
                challenge_description: item.challenge_description.clone(),
                skill_name: item.skill_name.clone().unwrap_or_default(),
                outcome_type: item.outcome_type.clone(),
                roll_context: format!(
                    "rolled {} + {} = {} ({})",
                    item.roll, item.modifier, item.total, item.outcome_type
                ),
                guidance,
                narrative_context: None,
                world_id: Some(world_id.to_string()),
            };

            // Get settings for branch count and tokens per branch
            let (branch_count, tokens_per_branch) = if let Some(ref settings_service) = self.settings_service {
                let settings = settings_service.get_for_world(*world_id).await;
                (settings.outcome_branch_count as usize, settings.suggestion_tokens_per_branch)
            } else {
                (2, 200) // Defaults if no settings service configured
            };

            // Spawn async task to generate branches
            let llm = llm_port.clone();
            let pending = self.pending.clone();
            let world_connection = self.world_connection.clone();
            let resolution_id_owned = resolution_id.to_string();
            let outcome_type = item.outcome_type.clone();
            let prompt_template_service = self.prompt_template_service.clone();
            let world_id_owned = world_id.clone();

            tokio::spawn(async move {
                let suggestion_service = OutcomeSuggestionService::new(llm, prompt_template_service);
                match suggestion_service.generate_branches(&request, branch_count, tokens_per_branch).await {
                    Ok(branches) => {
                        // Update pending item
                        let mut pending_guard = pending.write().await;
                        if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                            // Store branches as suggestions (converted to strings for backward compat)
                            pending_item.suggestions = Some(
                                branches.iter().map(|b| b.description.clone()).collect(),
                            );
                            pending_item.is_generating_suggestions = false;
                            drop(pending_guard);

                            // Notify DM with structured branches
                            // Convert OutcomeBranchDto to OutcomeBranchData
                            let branches_data: Vec<wrldbldr_protocol::OutcomeBranchData> = branches
                                .into_iter()
                                .map(|b| wrldbldr_protocol::OutcomeBranchData {
                                    id: b.id,
                                    title: b.title,
                                    description: b.description,
                                    effects: b.effects,
                                })
                                .collect();
                            
                            let message = wrldbldr_protocol::ServerMessage::OutcomeBranchesReady {
                                resolution_id: resolution_id_owned.clone(),
                                outcome_type,
                                branches: branches_data,
                            };
                            if let Err(e) = world_connection.send_to_dm(&world_id_owned, message).await {
                                tracing::error!("Failed to send branches to DM: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to generate outcome branches for {}: {}",
                            resolution_id_owned,
                            e
                        );
                        // Mark as no longer generating
                        let mut pending_guard = pending.write().await;
                        if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                            pending_item.is_generating_suggestions = false;
                        }
                    }
                }
            });
        } else {
            tracing::warn!(
                "LLM port not configured, cannot generate branches for {}",
                resolution_id
            );
            self.set_generating_suggestions(resolution_id, false).await;
        }

        Ok(())
    }

    /// Select an outcome branch and resolve the challenge
    ///
    /// The DM picks a branch by ID, optionally modifying the description.
    pub async fn select_branch(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        _branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            let pending = self.pending.read().await;
            pending
                .get(resolution_id)
                .cloned()
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify world matches
        let world_uuid: uuid::Uuid = (*world_id).into();
        if item.world_id != world_uuid {
            return Err(ChallengeOutcomeError::InvalidState(
                "World mismatch".to_string(),
            ));
        }

        // The branch_id would be used to look up the selected branch's description
        // For now, we use the modified_description if provided, or fall back to the original
        // TODO: Store branches in the approval item and look up by branch_id
        let final_description = modified_description.unwrap_or_else(|| item.outcome_description.clone());

        // Broadcast the resolution with the selected branch description
        self.broadcast_resolution(world_id, &item, Some(final_description))
            .await?;

        // Remove from pending
        self.remove_pending(resolution_id).await;

        Ok(())
    }

    /// Process state changes from trigger execution
    ///
    /// This method handles the actual application of state changes to the game world.
    /// For example, ItemAdded changes will create the item and add it to the PC's inventory.
    async fn process_state_changes(
        &self,
        state_changes: &[StateChange],
        item: &ChallengeOutcomeApprovalItem,
        world_id: &WorldId,
    ) -> anyhow::Result<()> {
        use wrldbldr_domain::entities::Item;
        use anyhow::Context;

        for change in state_changes {
            match change {
                StateChange::ItemAdded { character, item: item_name } => {
                    // Only handle "active_pc" for now - this refers to the character who rolled
                    if character == "active_pc" {
                        tracing::info!(
                            character_id = %item.character_id,
                            item_name = %item_name,
                            "Processing ItemAdded state change"
                        );

                        // Create a new item
                        let new_item = Item::new(*world_id, item_name.clone())
                            .with_description("Generated from challenge outcome trigger")
                            .with_type("Quest Item");

                        // Save the item to the repository
                        self.item_repository
                            .create(&new_item)
                            .await
                            .with_context(|| format!("Failed to create item '{}'", item_name))?;

                        // Add to the PC's inventory
                        let character_id = uuid::Uuid::parse_str(&item.character_id)
                            .with_context(|| format!("Invalid character ID: {}", item.character_id))?
                            .into();
                        
                        self.pc_repository
                            .add_inventory_item(
                                character_id,
                                new_item.id,
                                1, // Default quantity
                                false, // Not equipped by default
                                Some(wrldbldr_domain::entities::AcquisitionMethod::Gifted), // Challenge reward
                            )
                            .await
                            .with_context(|| {
                                format!("Failed to add item '{}' to character inventory", item_name)
                            })?;

                        tracing::info!(
                            character_id = %item.character_id,
                            item_id = %new_item.id,
                            item_name = %item_name,
                            "Successfully added item to PC inventory"
                        );
                    } else {
                        tracing::warn!(
                            character = %character,
                            item_name = %item_name,
                            "Unhandled character reference for ItemAdded - only 'active_pc' is supported"
                        );
                    }
                }
                StateChange::InfoRevealed { .. } => {
                    // Information revealing is already handled by adding to conversation history
                    // No additional processing needed
                    tracing::debug!("InfoRevealed state change - already handled in conversation history");
                }
                StateChange::CharacterStatUpdated { character_id, stat_name, delta } => {
                    // Resolve "active_pc" to the actual character ID from the approval item
                    let resolved_character_id = if character_id == "active_pc" {
                        item.character_id.clone()
                    } else {
                        character_id.clone()
                    };

                    // Handle stat updates for player characters
                    tracing::info!(
                        character_id = %resolved_character_id,
                        stat_name = %stat_name,
                        delta = %delta,
                        "Processing CharacterStatUpdated state change"
                    );

                    // Parse the character ID
                    let pc_id: wrldbldr_domain::PlayerCharacterId = match uuid::Uuid::parse_str(&resolved_character_id) {
                        Ok(uuid) => uuid.into(),
                        Err(e) => {
                            tracing::error!(
                                character_id = %resolved_character_id,
                                error = %e,
                                "Invalid character ID for stat update"
                            );
                            continue;
                        }
                    };

                    // Get the player character
                    let mut pc = match self.pc_repository.get(pc_id).await {
                        Ok(Some(pc)) => pc,
                        Ok(None) => {
                            tracing::warn!(
                                character_id = %resolved_character_id,
                                "Player character not found for stat update"
                            );
                            continue;
                        }
                        Err(e) => {
                            tracing::error!(
                                character_id = %resolved_character_id,
                                error = %e,
                                "Failed to get player character for stat update"
                            );
                            continue;
                        }
                    };

                    // Get or create sheet data
                    let sheet_data = pc.sheet_data.get_or_insert_with(|| {
                        wrldbldr_domain::entities::CharacterSheetData::new()
                    });

                    // Get current value (default to 0 if not set)
                    let current_value = sheet_data.get_number(stat_name).unwrap_or(0);
                    let new_value = current_value + delta;

                    // Update the stat
                    sheet_data.set(stat_name.clone(), wrldbldr_domain::entities::FieldValue::Number(new_value));

                    // Save the updated PC
                    if let Err(e) = self.pc_repository.update(&pc).await {
                        tracing::error!(
                            character_id = %resolved_character_id,
                            stat_name = %stat_name,
                            error = %e,
                            "Failed to save character stat update"
                        );
                        continue;
                    }

                    tracing::info!(
                        character_id = %resolved_character_id,
                        stat_name = %stat_name,
                        old_value = %current_value,
                        delta = %delta,
                        new_value = %new_value,
                        "Successfully updated character stat"
                    );
                }
                StateChange::EventTriggered { .. } => {
                    // Event triggering is informational - no state change needed
                    tracing::debug!("EventTriggered state change - informational only");
                }
                _ => {
                    tracing::warn!(
                        state_change = ?change,
                        "Unhandled state change type"
                    );
                }
            }
        }

        Ok(())
    }
}
