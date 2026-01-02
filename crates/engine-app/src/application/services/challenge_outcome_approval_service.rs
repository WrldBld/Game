//! Challenge Outcome Approval Service (P3.3)
//!
//! Manages the DM approval workflow for challenge resolutions.
//! After a player rolls, the outcome goes to this service before
//! being broadcast to all players.
//!
//! # Architecture
//!
//! This service uses an event channel pattern for hexagonal architecture compliance:
//! - Service emits `ChallengeApprovalEvent` through an mpsc channel
//! - `ChallengeApprovalEventPublisher` receives events and broadcasts via `BroadcastPort`
//! - No direct protocol message construction in the service layer

use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use crate::application::dto::{OutcomeSuggestionRequest, PendingChallengeResolutionDto};
use crate::application::services::challenge_approval_events::{
    ChallengeApprovalEvent, OutcomeBranchData, OutcomeTriggerData,
};
use crate::application::services::OutcomeSuggestionService;
use async_trait::async_trait;
use wrldbldr_domain::value_objects::{ChallengeOutcomeData, ProposedTool};
use wrldbldr_domain::{CharacterId, WorldId};
use crate::application::services::internal::{
    ChallengeOutcomeApprovalServicePort, OutcomeDecision, OutcomeTriggerServicePort,
    PromptTemplateServicePort, SettingsUseCasePort, StateChange,
};
use wrldbldr_engine_ports::outbound::{
    ChallengeOutcomePendingPort, ClockPort, ItemRepositoryPort, LlmPort,
    PlayerCharacterCrudPort, PlayerCharacterInventoryPort, QueuePort,
};

/// Result of challenge approval operations
///
/// This enum represents the outcomes of various approval operations,
/// allowing the use case layer to handle broadcasting appropriately.
#[derive(Debug, Clone)]
pub enum ChallengeApprovalResult {
    /// Item queued for DM approval
    Queued { resolution_id: String },
    /// Challenge resolved (approved by DM)
    Resolved {
        challenge_id: String,
        outcome: ResolvedOutcome,
        state_changes: Vec<StateChange>,
    },
    /// LLM suggestions ready
    SuggestionsReady {
        resolution_id: String,
        suggestions: Vec<String>,
    },
    /// Outcome branches ready
    BranchesReady {
        resolution_id: String,
        branches: Vec<OutcomeBranchInfo>,
    },
}

/// Resolved outcome details
#[derive(Debug, Clone)]
pub struct ResolvedOutcome {
    pub outcome_type: String,
    pub outcome_description: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub roll_breakdown: Option<String>,
    pub individual_rolls: Option<Vec<i32>>,
    pub challenge_name: String,
    pub character_name: String,
}

/// Outcome branch information
#[derive(Debug, Clone)]
pub struct OutcomeBranchInfo {
    pub branch_id: String,
    pub title: String,
    pub description: String,
    pub effects: Vec<String>,
}

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
///
/// # Queue Support
///
/// The service can optionally use a persistent queue (via `with_queue()`) instead
/// of the in-memory HashMap. When a queue is configured:
/// - Items are stored in the queue for persistence across restarts
/// - `list_by_world()` returns items from the queue instead of memory
/// - The in-memory HashMap is still used as a cache for active operations
///
/// # Event Channel
///
/// The service sends `ChallengeApprovalEvent` through a channel rather than
/// directly constructing protocol messages. This maintains hexagonal architecture
/// by keeping the service layer protocol-agnostic.
pub struct ChallengeOutcomeApprovalService<L: LlmPort> {
    /// Pending resolutions indexed by resolution_id (in-memory cache, adapter-managed)
    pending: Arc<dyn ChallengeOutcomePendingPort>,
    /// Persistent queue for challenge outcomes
    queue: Arc<dyn QueuePort<ChallengeOutcomeData> + Send + Sync>,
    /// Event channel sender for broadcasting events (bounded channel)
    event_sender: mpsc::Sender<ChallengeApprovalEvent>,
    /// Outcome trigger service for executing triggers
    outcome_trigger_service: Arc<dyn OutcomeTriggerServicePort>,
    /// Player Character CRUD operations (get, update)
    pc_crud: Arc<dyn PlayerCharacterCrudPort>,
    /// Player Character inventory operations
    pc_inventory: Arc<dyn PlayerCharacterInventoryPort>,
    /// Item repository for creating items
    item_repository: Arc<dyn ItemRepositoryPort>,
    /// LLM port for generating outcome suggestions
    llm_port: Arc<L>,
    /// Settings service for configurable values
    settings_service: Arc<dyn SettingsUseCasePort>,
    /// Prompt template service for resolving prompt templates
    prompt_template_service: Arc<dyn PromptTemplateServicePort>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl<L: LlmPort + 'static> ChallengeOutcomeApprovalService<L> {
    /// Create a new challenge outcome approval service
    ///
    /// All dependencies are required - there are no optional features.
    pub fn new(
        event_sender: mpsc::Sender<ChallengeApprovalEvent>,
        outcome_trigger_service: Arc<dyn OutcomeTriggerServicePort>,
        pc_crud: Arc<dyn PlayerCharacterCrudPort>,
        pc_inventory: Arc<dyn PlayerCharacterInventoryPort>,
        item_repository: Arc<dyn ItemRepositoryPort>,
        prompt_template_service: Arc<dyn PromptTemplateServicePort>,
        pending: Arc<dyn ChallengeOutcomePendingPort>,
        queue: Arc<dyn QueuePort<ChallengeOutcomeData> + Send + Sync>,
        llm_port: Arc<L>,
        settings_service: Arc<dyn SettingsUseCasePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            pending,
            queue,
            event_sender,
            outcome_trigger_service,
            pc_crud,
            pc_inventory,
            item_repository,
            llm_port,
            settings_service,
            prompt_template_service,
            clock,
        }
    }

    /// Get the current time
    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
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

        // Parse character_id to domain type
        let character_id = uuid::Uuid::parse_str(&resolution.character_id)
            .map(CharacterId::from)
            .unwrap_or_else(|_| CharacterId::new());

        // Convert DTO to domain type
        let item = ChallengeOutcomeData {
            resolution_id: resolution.resolution_id.clone(),
            world_id: *world_id,
            challenge_id: resolution.challenge_id,
            challenge_name: resolution.challenge_name.clone(),
            challenge_description: resolution.challenge_description,
            skill_name: resolution.skill_name,
            character_id,
            character_name: resolution.character_name.clone(),
            roll: resolution.roll,
            modifier: resolution.modifier,
            total: resolution.total,
            outcome_type: resolution.outcome_type.clone(),
            outcome_description: resolution.outcome_description.clone(),
            outcome_triggers: resolution
                .outcome_triggers
                .iter()
                .map(|t| ProposedTool {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{:?}", t),
                    description: String::new(),
                    arguments: serde_json::json!({}),
                })
                .collect(),
            roll_breakdown: resolution.roll_breakdown,
            timestamp: self.now(),
            suggestions: None,
            is_generating_suggestions: false,
        };

        // Enqueue for persistence
        self.queue
            .enqueue(item.clone(), 0)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;
        tracing::debug!(
            resolution_id = %resolution_id,
            "Challenge outcome enqueued to persistent storage"
        );

        // Store in pending map (in-memory cache for active operations)
        self.pending.insert(item.clone()).await;

        // Emit roll submitted event (notifies both DM and players via publisher)
        self.emit_roll_submitted(world_id, &item);

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
        decision: OutcomeDecision,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            self.pending
                .get(resolution_id)
                .await
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify world matches
        if item.world_id != *world_id {
            return Err(ChallengeOutcomeError::InvalidState(
                "World mismatch".to_string(),
            ));
        }

        match decision {
            OutcomeDecision::Accept => {
                // Broadcast resolution with original description
                self.broadcast_resolution(world_id, &item, None).await?;
                // Remove from pending
                self.remove_pending(resolution_id).await;
            }
            OutcomeDecision::Edit { modified_text } => {
                // Broadcast resolution with modified description
                self.broadcast_resolution(world_id, &item, Some(modified_text))
                    .await?;
                // Remove from pending
                self.remove_pending(resolution_id).await;
            }
            OutcomeDecision::Suggest { guidance } => {
                // Mark as generating suggestions
                self.set_generating_suggestions(resolution_id, true).await;

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
                let llm = self.llm_port.clone();
                let pending = self.pending.clone();
                let event_sender = self.event_sender.clone();
                let resolution_id_owned = resolution_id.to_string();
                let prompt_template_service = self.prompt_template_service.clone();
                let world_id_owned = *world_id;

                tokio::spawn(async move {
                    match OutcomeSuggestionService::<L>::generate_suggestions_with(
                        llm,
                        prompt_template_service,
                        &request,
                    )
                    .await
                    {
                        Ok(suggestions) => {
                            pending
                                .set_suggestions(&resolution_id_owned, Some(suggestions.clone()))
                                .await;
                            pending
                                .set_generating_suggestions(&resolution_id_owned, false)
                                .await;

                            // Emit suggestions ready event
                            let event = ChallengeApprovalEvent::SuggestionsReady {
                                world_id: world_id_owned,
                                resolution_id: resolution_id_owned.clone(),
                                suggestions,
                            };
                            if let Err(e) = event_sender.try_send(event) {
                                tracing::error!("Failed to emit SuggestionsReady event: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to generate outcome suggestions for {}: {}",
                                resolution_id_owned,
                                e
                            );
                            // Mark as no longer generating
                            pending
                                .set_generating_suggestions(&resolution_id_owned, false)
                                .await;
                        }
                    }
                });
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
        let Some(item) = self.pending.get(resolution_id).await else {
            return Err(ChallengeOutcomeError::NotFound(resolution_id.to_string()));
        };

        self.pending
            .set_suggestions(resolution_id, Some(suggestions.clone()))
            .await;
        self.pending
            .set_generating_suggestions(resolution_id, false)
            .await;

        // Emit suggestions ready event
        let event = ChallengeApprovalEvent::SuggestionsReady {
            world_id: item.world_id,
            resolution_id: resolution_id.to_string(),
            suggestions,
        };

        if let Err(e) = self.event_sender.try_send(event) {
            tracing::error!("Failed to emit SuggestionsReady event: {}", e);
        }

        Ok(())
    }

    /// Get all pending resolutions for a world
    pub async fn get_pending_for_world(&self, world_id: &WorldId) -> Vec<ChallengeOutcomeData> {
        self.pending.list_for_world(*world_id).await
    }

    /// Broadcast the final resolution to all players
    async fn broadcast_resolution(
        &self,
        world_id: &WorldId,
        item: &ChallengeOutcomeData,
        modified_description: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        let description = modified_description.unwrap_or_else(|| item.outcome_description.clone());

        // Emit resolved event
        let event = ChallengeApprovalEvent::Resolved {
            world_id: *world_id,
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

        if let Err(e) = self.event_sender.try_send(event) {
            tracing::error!("Failed to emit Resolved event: {}", e);
        }

        // Execute outcome triggers if any
        if !item.outcome_triggers.is_empty() {
            use wrldbldr_domain::entities::OutcomeTrigger;

            // Convert ProposedTool to domain OutcomeTrigger
            // Note: ProposedTool stores tool info for display; we parse the name to determine trigger type
            let domain_triggers: Vec<OutcomeTrigger> = item
                .outcome_triggers
                .iter()
                .filter_map(|t| Self::proposed_tool_to_outcome_trigger(t))
                .collect();

            let result = self
                .outcome_trigger_service
                .execute_triggers(domain_triggers, *world_id)
                .await;

            // Process state changes from trigger execution
            if !result.state_changes.is_empty() {
                if let Err(e) = self
                    .process_state_changes(&result.state_changes, item, world_id)
                    .await
                {
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

    /// Convert a ProposedTool to an OutcomeTrigger if possible
    ///
    /// This parses the tool name and arguments to reconstruct the domain trigger type.
    fn proposed_tool_to_outcome_trigger(
        tool: &ProposedTool,
    ) -> Option<wrldbldr_domain::entities::OutcomeTrigger> {
        use wrldbldr_domain::entities::OutcomeTrigger;
        use wrldbldr_domain::{ChallengeId, SceneId};

        // Parse the tool name to determine type
        let name_lower = tool.name.to_lowercase();

        if name_lower.contains("revealinformation") || name_lower.contains("reveal_information") {
            let info = tool
                .arguments
                .get("info")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let persist = tool
                .arguments
                .get("persist")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(OutcomeTrigger::RevealInformation { info, persist })
        } else if name_lower.contains("enablechallenge") || name_lower.contains("enable_challenge")
        {
            let challenge_id_str = tool
                .arguments
                .get("challenge_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let challenge_id = uuid::Uuid::parse_str(challenge_id_str)
                .map(ChallengeId::from_uuid)
                .unwrap_or_else(|_| ChallengeId::new());
            Some(OutcomeTrigger::EnableChallenge { challenge_id })
        } else if name_lower.contains("disablechallenge")
            || name_lower.contains("disable_challenge")
        {
            let challenge_id_str = tool
                .arguments
                .get("challenge_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let challenge_id = uuid::Uuid::parse_str(challenge_id_str)
                .map(ChallengeId::from_uuid)
                .unwrap_or_else(|_| ChallengeId::new());
            Some(OutcomeTrigger::DisableChallenge { challenge_id })
        } else if name_lower.contains("modifycharacterstat")
            || name_lower.contains("modify_character_stat")
        {
            let stat = tool
                .arguments
                .get("stat")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let modifier = tool
                .arguments
                .get("modifier")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            Some(OutcomeTrigger::ModifyCharacterStat { stat, modifier })
        } else if name_lower.contains("triggerscene") || name_lower.contains("trigger_scene") {
            let scene_id_str = tool
                .arguments
                .get("scene_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let scene_id = uuid::Uuid::parse_str(scene_id_str)
                .map(SceneId::from_uuid)
                .unwrap_or_else(|_| SceneId::new());
            Some(OutcomeTrigger::TriggerScene { scene_id })
        } else if name_lower.contains("giveitem") || name_lower.contains("give_item") {
            let item_name = tool
                .arguments
                .get("item_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let item_description = tool
                .arguments
                .get("item_description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some(OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            })
        } else if name_lower.contains("custom") {
            let description = tool
                .arguments
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&tool.description)
                .to_string();
            Some(OutcomeTrigger::Custom { description })
        } else {
            // Unknown trigger type - log and skip
            tracing::warn!(
                tool_name = %tool.name,
                "Unknown trigger type in ProposedTool, skipping"
            );
            None
        }
    }

    /// Notify of a roll submission (triggers both DM pending and player status events)
    ///
    /// The `ChallengeApprovalEvent::RollSubmitted` event will be processed by the
    /// publisher to send appropriate messages to DM and players.
    fn emit_roll_submitted(&self, world_id: &WorldId, item: &ChallengeOutcomeData) {
        let event = ChallengeApprovalEvent::RollSubmitted {
            world_id: *world_id,
            resolution_id: item.resolution_id.clone(),
            challenge_id: item.challenge_id.clone(),
            challenge_name: item.challenge_name.clone(),
            character_id: item.character_id.to_string(),
            character_name: item.character_name.clone(),
            roll: item.roll,
            modifier: item.modifier,
            total: item.total,
            outcome_type: item.outcome_type.clone(),
            outcome_description: item.outcome_description.clone(),
            roll_breakdown: item.roll_breakdown.clone(),
            outcome_triggers: item
                .outcome_triggers
                .iter()
                .map(|t| OutcomeTriggerData {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    description: t.description.clone(),
                    arguments: t.arguments.clone(),
                })
                .collect(),
        };

        if let Err(e) = self.event_sender.try_send(event) {
            tracing::error!("Failed to emit RollSubmitted event: {}", e);
        }
    }

    /// Remove a resolution from pending (both in-memory and queue)
    async fn remove_pending(&self, resolution_id: &str) {
        // Remove from in-memory cache
        self.pending.remove(resolution_id).await;

        // Note: We need the queue item ID, but we only have resolution_id
        // For now, we just remove from in-memory. The queue worker will handle
        // completing items when processing them.
        // TODO: Track queue item ID -> resolution_id mapping for proper completion
        tracing::debug!(
            resolution_id = %resolution_id,
            "Removed from in-memory cache (queue completion handled by worker)"
        );
    }

    /// Mark a resolution as generating suggestions
    async fn set_generating_suggestions(&self, resolution_id: &str, generating: bool) {
        self.pending
            .set_generating_suggestions(resolution_id, generating)
            .await;
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
            self.pending
                .get(resolution_id)
                .await
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify world matches
        if item.world_id != *world_id {
            return Err(ChallengeOutcomeError::InvalidState(
                "World mismatch".to_string(),
            ));
        }

        // Mark as generating
        self.set_generating_suggestions(resolution_id, true).await;

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
        let settings = self.settings_service.get_for_world(*world_id).await;
        let branch_count = settings.outcome_branch_count as usize;
        let tokens_per_branch = settings.suggestion_tokens_per_branch;

        // Spawn async task to generate branches
        let llm = self.llm_port.clone();
        let pending = self.pending.clone();
        let event_sender = self.event_sender.clone();
        let resolution_id_owned = resolution_id.to_string();
        let outcome_type = item.outcome_type.clone();
        let prompt_template_service = self.prompt_template_service.clone();
        let world_id_owned = *world_id;

        tokio::spawn(async move {
            match OutcomeSuggestionService::<L>::generate_branches_with(
                llm,
                prompt_template_service,
                &request,
                branch_count,
                tokens_per_branch,
            )
            .await
            {
                Ok(branches) => {
                    // Store branches as suggestions (converted to strings for backward compat)
                    pending
                        .set_suggestions(
                            &resolution_id_owned,
                            Some(branches.iter().map(|b| b.description.clone()).collect()),
                        )
                        .await;
                    pending
                        .set_generating_suggestions(&resolution_id_owned, false)
                        .await;

                    // Emit branches ready event
                    let event = ChallengeApprovalEvent::BranchesReady {
                        world_id: world_id_owned,
                        resolution_id: resolution_id_owned.clone(),
                        outcome_type,
                        branches: branches
                            .into_iter()
                            .map(|b| OutcomeBranchData {
                                id: b.id,
                                title: b.title,
                                description: b.description,
                                effects: b.effects,
                            })
                            .collect(),
                    };
                    if let Err(e) = event_sender.try_send(event) {
                        tracing::error!("Failed to emit BranchesReady event: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to generate outcome branches for {}: {}",
                        resolution_id_owned,
                        e
                    );
                    // Mark as no longer generating
                    pending
                        .set_generating_suggestions(&resolution_id_owned, false)
                        .await;
                }
            }
        });

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
            self.pending
                .get(resolution_id)
                .await
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify world matches
        if item.world_id != *world_id {
            return Err(ChallengeOutcomeError::InvalidState(
                "World mismatch".to_string(),
            ));
        }

        // The branch_id would be used to look up the selected branch's description
        // For now, we use the modified_description if provided, or fall back to the original
        // TODO: Store branches in the approval item and look up by branch_id
        let final_description =
            modified_description.unwrap_or_else(|| item.outcome_description.clone());

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
        item: &ChallengeOutcomeData,
        world_id: &WorldId,
    ) -> anyhow::Result<()> {
        use anyhow::Context;
        use wrldbldr_domain::entities::Item;

        for change in state_changes {
            match change {
                StateChange::ItemAdded {
                    character,
                    item: item_name,
                } => {
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
                        let character_id: wrldbldr_domain::PlayerCharacterId =
                            uuid::Uuid::from(item.character_id).into();

                        self.pc_inventory
                            .add_inventory_item(
                                character_id,
                                new_item.id,
                                1,     // Default quantity
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
                    tracing::debug!(
                        "InfoRevealed state change - already handled in conversation history"
                    );
                }
                StateChange::CharacterStatUpdated {
                    character_id,
                    stat_name,
                    delta,
                } => {
                    // Resolve "active_pc" to the actual character ID from the approval item
                    let resolved_character_id = if character_id == "active_pc" {
                        item.character_id.to_string()
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
                    let pc_id: wrldbldr_domain::PlayerCharacterId =
                        match uuid::Uuid::parse_str(&resolved_character_id) {
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
                    let mut pc = match self.pc_crud.get(pc_id).await {
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
                    let sheet_data = pc
                        .sheet_data
                        .get_or_insert_with(wrldbldr_domain::entities::CharacterSheetData::new);

                    // Get current value (default to 0 if not set)
                    let current_value = sheet_data.get_number(stat_name).unwrap_or(0);
                    let new_value = current_value + delta;

                    // Update the stat
                    sheet_data.set(
                        stat_name.clone(),
                        wrldbldr_domain::entities::FieldValue::Number(new_value),
                    );

                    // Save the updated PC
                    if let Err(e) = self.pc_crud.update(&pc).await {
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

                    // Emit stat update event
                    let event = ChallengeApprovalEvent::StatUpdated {
                        world_id: item.world_id,
                        character_id: resolved_character_id.clone(),
                        character_name: pc.name.clone(),
                        stat_name: stat_name.clone(),
                        old_value: current_value,
                        new_value,
                        delta: *delta,
                    };

                    if let Err(e) = self.event_sender.try_send(event) {
                        tracing::warn!(
                            character_id = %resolved_character_id,
                            stat_name = %stat_name,
                            error = %e,
                            "Failed to emit stat update event"
                        );
                    }
                }
                StateChange::EventTriggered { .. } => {
                    // Event triggering is informational - no state change needed
                    tracing::debug!("EventTriggered state change - informational only");
                }
                _ => {
                    tracing::warn!(state_change = ?change, "Unhandled state change type");
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

/// Implementation of the `ChallengeOutcomeApprovalServicePort` for `ChallengeOutcomeApprovalService`.
///
/// This exposes challenge outcome approval methods to infrastructure adapters.
#[async_trait]
impl<L: LlmPort + 'static> ChallengeOutcomeApprovalServicePort
    for ChallengeOutcomeApprovalService<L>
{
    async fn queue_for_approval(
        &self,
        world_id: WorldId,
        resolution: ChallengeOutcomeData,
    ) -> anyhow::Result<String> {
        let resolution_id = resolution.resolution_id.clone();

        // Create DTO from domain type
        let dto = PendingChallengeResolutionDto {
            resolution_id: resolution.resolution_id,
            challenge_id: resolution.challenge_id,
            challenge_name: resolution.challenge_name,
            challenge_description: resolution.challenge_description,
            skill_name: resolution.skill_name,
            character_id: resolution.character_id.to_string(),
            character_name: resolution.character_name,
            roll: resolution.roll,
            modifier: resolution.modifier,
            total: resolution.total,
            outcome_type: resolution.outcome_type,
            outcome_description: resolution.outcome_description,
            outcome_triggers: resolution
                .outcome_triggers
                .iter()
                .filter_map(|t| {
                    // Convert ProposedTool to OutcomeTriggerRequestDto based on tool name
                    let name_lower = t.name.to_lowercase();
                    if name_lower.contains("reveal_information")
                        || name_lower.contains("revealinformation")
                    {
                        let info = t
                            .arguments
                            .get("info")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let persist = t
                            .arguments
                            .get("persist")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        Some(
                            crate::application::dto::OutcomeTriggerRequestDto::RevealInformation {
                                info,
                                persist,
                            },
                        )
                    } else if name_lower.contains("enable_challenge")
                        || name_lower.contains("enablechallenge")
                    {
                        let challenge_id = t
                            .arguments
                            .get("challenge_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some(
                            crate::application::dto::OutcomeTriggerRequestDto::EnableChallenge {
                                challenge_id,
                            },
                        )
                    } else if name_lower.contains("disable_challenge")
                        || name_lower.contains("disablechallenge")
                    {
                        let challenge_id = t
                            .arguments
                            .get("challenge_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some(
                            crate::application::dto::OutcomeTriggerRequestDto::DisableChallenge {
                                challenge_id,
                            },
                        )
                    } else if name_lower.contains("custom") {
                        let description = t
                            .arguments
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&t.description)
                            .to_string();
                        Some(crate::application::dto::OutcomeTriggerRequestDto::Custom {
                            description,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            roll_breakdown: resolution.roll_breakdown,
            individual_rolls: None,
            timestamp: resolution.timestamp.to_rfc3339(),
        };

        self.queue_for_approval(&world_id, dto)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(resolution_id)
    }

    async fn process_decision(
        &self,
        world_id: WorldId,
        resolution_id: &str,
        decision: OutcomeDecision,
    ) -> anyhow::Result<()> {
        ChallengeOutcomeApprovalService::process_decision(self, &world_id, resolution_id, decision)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn update_suggestions(
        &self,
        resolution_id: &str,
        suggestions: Vec<String>,
    ) -> anyhow::Result<()> {
        ChallengeOutcomeApprovalService::update_suggestions(self, resolution_id, suggestions)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get_pending_for_world(&self, world_id: WorldId) -> Vec<ChallengeOutcomeData> {
        ChallengeOutcomeApprovalService::get_pending_for_world(self, &world_id).await
    }

    async fn request_branches(
        &self,
        world_id: WorldId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> anyhow::Result<()> {
        ChallengeOutcomeApprovalService::request_branches(self, &world_id, resolution_id, guidance)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn select_branch(
        &self,
        world_id: WorldId,
        resolution_id: &str,
        branch_id: &str,
        modified_description: Option<String>,
    ) -> anyhow::Result<()> {
        ChallengeOutcomeApprovalService::select_branch(
            self,
            &world_id,
            resolution_id,
            branch_id,
            modified_description,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
