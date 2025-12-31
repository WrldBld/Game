//! DM Approval Queue Service - Manages DM approval workflow
//!
//! This service manages the DMApprovalQueue, which holds decisions awaiting
//! DM approval. It provides history, delay, and expiration features.
//!
//! NOTE: This service has been refactored to remove SessionManagementPort dependency.
//! Broadcasting and conversation history management should be handled by the caller
//! using WorldConnectionPort after receiving approval outcomes.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::services::dm_action_processor_service::ApprovalProcessorPort;
use crate::application::services::item_service::ItemService;
use crate::application::services::tool_execution_service::ToolExecutionService;
use std::collections::HashMap;
use wrldbldr_domain::entities::AcquisitionMethod;
use wrldbldr_domain::value_objects::{
    ApprovalRequestData, DmApprovalDecision, GameTool, ProposedTool,
};
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, SceneId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ApprovalRequestLookupPort,
    ApprovalDecisionType as PortApprovalDecisionType, ApprovalQueueItem as PortApprovalQueueItem,
    ApprovalQueuePort, ApprovalRequest, ApprovalUrgency as PortApprovalUrgency,
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, ClockPort, DialogueContextServicePort,
    DmApprovalDecision as PortDmApprovalDecision, DmApprovalQueueServicePort,
    NarrativeEventSuggestionInfo, ProposedToolInfo, QueueError, QueueItem, QueueItemId,
    QueueItemStatus,
};

/// Maximum number of times a response can be rejected before requiring TakeOver
const MAX_RETRY_COUNT: u32 = 3;

/// Service for managing the DM approval queue
pub struct DMApprovalQueueService<Q: ApprovalQueuePort<ApprovalRequestData>, I: ItemService> {
    pub(crate) queue: Arc<Q>,
    /// Dialogue context service for recording dialogue exchanges (ISP-split from StoryEventService)
    dialogue_context_service: Arc<dyn DialogueContextServicePort>,
    /// Item service for creating items and managing inventory
    item_service: Arc<I>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl<Q: ApprovalQueuePort<ApprovalRequestData>, I: ItemService> DMApprovalQueueService<Q, I> {
    pub fn queue(&self) -> &Arc<Q> {
        &self.queue
    }

    /// Create a new DM approval queue service
    ///
    /// # Arguments
    /// * `queue` - The underlying approval queue backend
    /// * `dialogue_context_service` - Service for recording dialogue exchanges (ISP-split from StoryEventService)
    /// * `item_service` - Service for item creation and inventory management
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    pub fn new(
        queue: Arc<Q>,
        dialogue_context_service: Arc<dyn DialogueContextServicePort>,
        item_service: Arc<I>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            queue,
            dialogue_context_service,
            item_service,
            clock,
        }
    }

    /// Get the current time
    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Get all pending approvals for a world (for DM UI)
    pub async fn get_pending(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<QueueItem<ApprovalRequestData>>, QueueError> {
        self.queue.list_by_world(world_id).await
    }

    /// Get an approval item by its string ID
    pub async fn get_by_id(
        &self,
        id: &str,
    ) -> Result<Option<QueueItem<ApprovalRequestData>>, QueueError> {
        let item_id = uuid::Uuid::parse_str(id)
            .map_err(|e| QueueError::Backend(format!("Invalid UUID: {}", e)))?;
        self.queue.get(item_id).await
    }

    /// Process DM approval decision
    ///
    /// This method handles the DM's decision on an approval request and
    /// routes it to the appropriate handler based on the decision type.
    ///
    /// Returns an ApprovalOutcome that the caller should use to:
    /// 1. Broadcast messages to players via WorldConnectionPort
    /// 2. Update conversation history
    /// 3. Record story events
    pub async fn process_decision(
        &self,
        world_id: WorldId,
        item_id: QueueItemId,
        decision: DmApprovalDecision,
    ) -> Result<ApprovalOutcome, QueueError> {
        let item = self
            .queue
            .get(item_id)
            .await?
            .ok_or_else(|| QueueError::NotFound(item_id.to_string()))?;

        let outcome = match decision {
            DmApprovalDecision::Accept => {
                self.handle_accept(world_id, &item.payload, HashMap::new())
                    .await?
            }
            DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
                self.handle_accept(world_id, &item.payload, item_recipients)
                    .await?
            }
            DmApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            } => {
                self.handle_accept_modified(
                    world_id,
                    &item.payload,
                    &modified_dialogue,
                    &approved_tools,
                    &rejected_tools,
                    item_recipients,
                )
                .await?
            }
            DmApprovalDecision::Reject { feedback } => {
                self.handle_reject(&item.payload, &feedback).await?
            }
            DmApprovalDecision::TakeOver { dm_response } => {
                self.handle_takeover(world_id, &item.payload, &dm_response)
                    .await?
            }
        };

        // Mark item based on outcome
        match &outcome {
            ApprovalOutcome::Broadcast { .. } => {
                self.queue.complete(item_id).await?;
            }
            ApprovalOutcome::Rejected {
                needs_reprocessing: true,
                ..
            } => {
                // Item stays in queue, will be reprocessed
                self.queue
                    .delay(item_id, self.now() + Duration::from_secs(1))
                    .await?;
            }
            ApprovalOutcome::Rejected {
                needs_reprocessing: false,
                ..
            }
            | ApprovalOutcome::MaxRetriesExceeded { .. } => {
                self.queue.fail(item_id, "Rejected by DM").await?;
            }
        }

        Ok(outcome)
    }

    /// Handle accepting an approval as-is
    ///
    /// Returns an ApprovalOutcome with the dialogue and executed tools.
    /// The caller is responsible for:
    /// - Broadcasting the dialogue to players
    /// - Adding to conversation history
    /// - Recording story events
    async fn handle_accept(
        &self,
        world_id: WorldId,
        approval: &ApprovalRequestData,
        item_recipients: HashMap<String, Vec<String>>,
    ) -> Result<ApprovalOutcome, QueueError> {
        let tool_execution_service = ToolExecutionService;

        // Record dialogue exchange as a story event
        self.record_dialogue_event(world_id, approval, &approval.proposed_dialogue)
            .await;

        // Execute approved tool calls
        let mut executed_tools = Vec::new();
        for tool in &approval.proposed_tools {
            // Check if this is a give_item with DM-specified recipients
            if tool.name == "give_item" {
                self.execute_give_item_with_recipients(
                    world_id,
                    tool,
                    item_recipients.get(&tool.id),
                )
                .await;
                executed_tools.push(tool.name.clone());
            } else {
                // Convert ProposedTool to GameTool
                // Parse the tool arguments JSON to determine tool type
                if let Ok(game_tool) = self.parse_tool_from_proposed(tool) {
                    if let Err(e) = tool_execution_service.execute_tool(&game_tool).await {
                        tracing::warn!("Failed to execute tool {}: {}", tool.name, e);
                        // Continue with other tools even if one fails
                    } else {
                        executed_tools.push(tool.name.clone());
                    }
                }
            }
        }

        Ok(ApprovalOutcome::Broadcast {
            dialogue: approval.proposed_dialogue.clone(),
            npc_name: approval.npc_name.clone(),
            executed_tools,
        })
    }

    /// Handle accepting with modifications
    ///
    /// Returns an ApprovalOutcome with the modified dialogue and executed tools.
    /// The caller is responsible for broadcasting and history updates.
    async fn handle_accept_modified(
        &self,
        world_id: WorldId,
        approval: &ApprovalRequestData,
        modified_dialogue: &str,
        approved_tools: &[String],
        _rejected_tools: &[String],
        item_recipients: HashMap<String, Vec<String>>,
    ) -> Result<ApprovalOutcome, QueueError> {
        let tool_execution_service = ToolExecutionService;

        // Record dialogue exchange as a story event (with modified dialogue)
        self.record_dialogue_event(world_id, approval, modified_dialogue)
            .await;

        // Execute approved tool calls (filter based on approved_tools list)
        // approved_tools contains tool IDs that should be executed
        let mut executed_tools = Vec::new();
        for tool in &approval.proposed_tools {
            // Check if this tool is in the approved list
            if approved_tools.contains(&tool.id) {
                // Check if this is a give_item with DM-specified recipients
                if tool.name == "give_item" {
                    self.execute_give_item_with_recipients(
                        world_id,
                        tool,
                        item_recipients.get(&tool.id),
                    )
                    .await;
                    executed_tools.push(tool.name.clone());
                } else if let Ok(game_tool) = self.parse_tool_from_proposed(tool) {
                    if let Err(e) = tool_execution_service.execute_tool(&game_tool).await {
                        tracing::warn!("Failed to execute tool {}: {}", tool.name, e);
                    } else {
                        executed_tools.push(tool.name.clone());
                    }
                }
            }
            // Tools in rejected_tools are simply skipped
        }

        Ok(ApprovalOutcome::Broadcast {
            dialogue: modified_dialogue.to_string(),
            npc_name: approval.npc_name.clone(),
            executed_tools,
        })
    }

    /// Handle rejecting an approval
    async fn handle_reject(
        &self,
        approval: &ApprovalRequestData,
        feedback: &str,
    ) -> Result<ApprovalOutcome, QueueError> {
        if approval.retry_count >= MAX_RETRY_COUNT {
            return Ok(ApprovalOutcome::MaxRetriesExceeded {
                feedback: feedback.to_string(),
            });
        }

        // Re-enqueue to LLM queue with feedback for regeneration
        // This would require access to the LLM queue service
        // For now, we mark it as needing reprocessing
        // The actual re-enqueue would happen in a worker that processes rejected approvals
        tracing::info!(
            "Approval rejected with feedback: {}. Will be reprocessed.",
            feedback
        );

        Ok(ApprovalOutcome::Rejected {
            feedback: feedback.to_string(),
            needs_reprocessing: true,
        })
    }

    /// Handle DM taking over
    ///
    /// Returns an ApprovalOutcome with the DM's response.
    /// The caller is responsible for broadcasting and history updates.
    async fn handle_takeover(
        &self,
        world_id: WorldId,
        approval: &ApprovalRequestData,
        dm_response: &str,
    ) -> Result<ApprovalOutcome, QueueError> {
        // Record dialogue exchange as a story event (with DM's response)
        self.record_dialogue_event(world_id, approval, dm_response)
            .await;

        Ok(ApprovalOutcome::Broadcast {
            dialogue: dm_response.to_string(),
            npc_name: approval.npc_name.clone(),
            executed_tools: Vec::new(),
        })
    }

    /// Record a dialogue exchange as a story event
    ///
    /// This is called after dialogue is processed to persist it to the story timeline.
    /// Errors are logged but don't fail the approval flow.
    async fn record_dialogue_event(
        &self,
        world_id: WorldId,
        approval: &ApprovalRequestData,
        npc_response: &str,
    ) {
        // Get NPC ID from approval item (now a domain type Option<CharacterId>)
        let npc_id = match &approval.npc_id {
            Some(id) => *id,
            None => {
                tracing::warn!(
                    "Cannot record dialogue event: npc_id not set in approval for '{}'",
                    approval.npc_name
                );
                return;
            }
        };

        // Record the dialogue exchange with full context from ApprovalRequestData
        // scene_id and location_id are now domain types (Option<SceneId>, Option<LocationId>)
        let scene_id = approval.scene_id;
        let location_id = approval.location_id;

        if let Err(e) = self
            .dialogue_context_service
            .record_dialogue_exchange(
                world_id,
                scene_id,
                location_id,
                npc_id,
                approval.npc_name.clone(),
                approval.player_dialogue.clone().unwrap_or_default(),
                npc_response.to_string(),
                approval.topics.clone(),
                None,         // tone
                vec![npc_id], // involved_characters
                approval.game_time.clone(),
            )
            .await
        {
            tracing::error!(
                "Failed to record dialogue event for NPC '{}': {}",
                approval.npc_name,
                e
            );
        } else {
            tracing::debug!(
                "Recorded dialogue exchange with NPC '{}' in world {}",
                approval.npc_name,
                world_id
            );
        }

        // Update SPOKE_TO edge if we have both PC and NPC IDs
        if let Some(pc_id) = approval.pc_id {
            if let Err(e) = self
                .dialogue_context_service
                .update_spoke_to_edge(pc_id, npc_id, None) // topic could be extracted in future
                .await
            {
                tracing::warn!(
                    "Failed to update SPOKE_TO edge between PC {} and NPC {}: {}",
                    pc_id,
                    npc_id,
                    e
                );
            } else {
                tracing::debug!(
                    "Updated SPOKE_TO edge: PC {} -> NPC '{}'",
                    pc_id,
                    approval.npc_name
                );
            }
        }
    }

    /// Delay a decision for later
    pub async fn delay_decision(
        &self,
        item_id: QueueItemId,
        duration: Duration,
    ) -> Result<(), QueueError> {
        self.queue.delay(item_id, self.now() + duration).await
    }

    /// Get decision history for a world
    pub async fn get_history(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<QueueItem<ApprovalRequestData>>, QueueError> {
        self.queue.get_history_by_world(world_id, limit).await
    }

    /// Clean up old completed/failed items beyond retention period
    ///
    /// Delegates to the underlying queue's cleanup method.
    pub async fn cleanup(&self, retention: Duration) -> anyhow::Result<u64> {
        self.queue
            .cleanup(retention)
            .await
            .map(|count| count as u64)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Expire approval items older than the specified timeout
    ///
    /// Delegates to the underlying queue's expire_old method.
    pub async fn expire_old(&self, timeout: Duration) -> anyhow::Result<u64> {
        self.queue
            .expire_old(timeout)
            .await
            .map(|count| count as u64)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Discard a challenge suggestion from an approval item
    ///
    /// This is called when the DM doesn't want a challenge suggested by the LLM.
    /// The approval item is marked as failed since the DM rejected the challenge.
    /// A new LLM request should be made for a non-challenge response.
    pub async fn discard_challenge(&self, _client_id: &str, request_id: &str) {
        // Parse request_id to item ID
        if let Ok(item_id) = uuid::Uuid::parse_str(request_id) {
            // Mark the item as failed since the DM rejected the challenge
            if let Err(e) = self.queue.fail(item_id, "Challenge discarded by DM").await {
                tracing::warn!("Failed to mark approval as failed: {}", e);
            }

            // NOTE: Re-enqueueing to LLM queue for a non-challenge response would require:
            // 1. Access to the LLM queue service (add as dependency)
            // 2. Retrieve the original prompt from source_action_id
            // 3. Add guidance to the prompt context (e.g., "do not suggest a challenge")
            // 4. Create a new LLMRequestItem with modified context
            // For now, the DM can manually trigger a new response if needed.
            tracing::info!(
                "Challenge discarded for approval {}. DM should trigger new response if needed.",
                request_id
            );
        } else {
            tracing::warn!("Invalid request_id format: {}", request_id);
        }
    }

    /// Parse ProposedTool into GameTool
    fn parse_tool_from_proposed(&self, tool: &ProposedTool) -> Result<GameTool, QueueError> {
        // Parse tool based on name and arguments (arguments is serde_json::Value)
        let args = &tool.arguments;

        match tool.name.as_str() {
            "give_item" => {
                let item_name = args
                    .get("item_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing item_name".to_string()))?
                    .to_string();
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing description".to_string()))?
                    .to_string();
                Ok(GameTool::GiveItem {
                    item_name,
                    description,
                })
            }
            "reveal_info" => {
                let info_type = args
                    .get("info_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing info_type".to_string()))?
                    .to_string();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing content".to_string()))?
                    .to_string();
                let importance_str = args
                    .get("importance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("minor");
                let importance = match importance_str {
                    "minor" => wrldbldr_domain::value_objects::InfoImportance::Minor,
                    "major" => wrldbldr_domain::value_objects::InfoImportance::Major,
                    "critical" => wrldbldr_domain::value_objects::InfoImportance::Critical,
                    _ => wrldbldr_domain::value_objects::InfoImportance::Minor,
                };
                Ok(GameTool::RevealInfo {
                    info_type,
                    content,
                    importance,
                })
            }
            "change_relationship" => {
                let change_str = args
                    .get("change")
                    .and_then(|v| v.as_str())
                    .unwrap_or("improve");
                let change = match change_str {
                    "improve" => wrldbldr_domain::value_objects::RelationshipChange::Improve,
                    "worsen" => wrldbldr_domain::value_objects::RelationshipChange::Worsen,
                    _ => wrldbldr_domain::value_objects::RelationshipChange::Improve,
                };
                let amount_str = args
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("slight");
                let amount = match amount_str {
                    "slight" => wrldbldr_domain::value_objects::ChangeAmount::Slight,
                    "moderate" => wrldbldr_domain::value_objects::ChangeAmount::Moderate,
                    "significant" => wrldbldr_domain::value_objects::ChangeAmount::Significant,
                    _ => wrldbldr_domain::value_objects::ChangeAmount::Slight,
                };
                let reason = args
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(GameTool::ChangeRelationship {
                    change,
                    amount,
                    reason,
                })
            }
            "trigger_event" => {
                let event_type = args
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing event_type".to_string()))?
                    .to_string();
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| QueueError::Backend("Missing description".to_string()))?
                    .to_string();
                Ok(GameTool::TriggerEvent {
                    event_type,
                    description,
                })
            }
            _ => Err(QueueError::Backend(format!("Unknown tool: {}", tool.name))),
        }
    }

    /// Execute a give_item tool with DM-specified recipients
    ///
    /// If recipients is None or empty, the item is not given (DM chose not to give it).
    /// If recipients has PC IDs, creates the item and gives it to each recipient.
    async fn execute_give_item_with_recipients(
        &self,
        world_id: WorldId,
        tool: &ProposedTool,
        recipients: Option<&Vec<String>>,
    ) {
        let args = &tool.arguments;

        // Parse item details from tool arguments
        let item_name = match args.get("item_name").and_then(|v| v.as_str()) {
            Some(name) => name.to_string(),
            None => {
                tracing::warn!("give_item tool missing item_name");
                return;
            }
        };

        let item_description = args
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Check if DM specified recipients
        let recipient_ids: Vec<PlayerCharacterId> = match recipients {
            Some(ids) if !ids.is_empty() => ids
                .iter()
                .filter_map(|id| {
                    uuid::Uuid::parse_str(id)
                        .ok()
                        .map(PlayerCharacterId::from_uuid)
                })
                .collect(),
            _ => {
                // No recipients means DM chose not to give this item
                tracing::info!(
                    item_name = %item_name,
                    "DM chose not to give item (no recipients specified)"
                );
                return;
            }
        };

        // Create and give items to recipients
        match self
            .item_service
            .give_item_to_multiple_pcs(
                world_id,
                item_name.clone(),
                item_description.clone(),
                recipient_ids.clone(),
                AcquisitionMethod::Gifted,
            )
            .await
        {
            Ok(results) => {
                for result in &results {
                    tracing::info!(
                        item_id = %result.item.id,
                        item_name = %result.item.name,
                        pc_id = %result.recipient_pc_id,
                        "Gave item to PC"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    item_name = %item_name,
                    error = %e,
                    "Failed to give item to PCs"
                );
            }
        }
    }
}

/// Outcome of processing an approval decision
///
/// The caller should use this outcome to:
/// - Broadcast dialogue to players via WorldConnectionPort
/// - Update conversation history
#[derive(Debug, Clone)]
pub enum ApprovalOutcome {
    /// Approval was accepted and should be broadcast to players
    Broadcast {
        /// The dialogue text to broadcast
        dialogue: String,
        /// The NPC name (speaker)
        npc_name: String,
        /// Names of tools that were executed
        executed_tools: Vec<String>,
    },
    /// Approval was rejected
    Rejected {
        feedback: String,
        needs_reprocessing: bool,
    },
    /// Maximum retries exceeded
    MaxRetriesExceeded { feedback: String },
}

// ============================================================================
// Port Implementation Helper Functions
// ============================================================================

/// Convert domain ApprovalDecisionType to port ApprovalDecisionType
fn convert_domain_decision_type(
    decision_type: wrldbldr_domain::value_objects::ApprovalDecisionType,
) -> PortApprovalDecisionType {
    match decision_type {
        wrldbldr_domain::value_objects::ApprovalDecisionType::NpcResponse => {
            PortApprovalDecisionType::NpcResponse
        }
        wrldbldr_domain::value_objects::ApprovalDecisionType::ToolUsage => {
            PortApprovalDecisionType::ToolUsage
        }
        wrldbldr_domain::value_objects::ApprovalDecisionType::ChallengeSuggestion => {
            PortApprovalDecisionType::ChallengeSuggestion
        }
        wrldbldr_domain::value_objects::ApprovalDecisionType::SceneTransition => {
            PortApprovalDecisionType::SceneTransition
        }
        wrldbldr_domain::value_objects::ApprovalDecisionType::ChallengeOutcome => {
            PortApprovalDecisionType::ChallengeOutcome
        }
    }
}

/// Convert domain ApprovalUrgency to port ApprovalUrgency
fn convert_domain_urgency(
    urgency: wrldbldr_domain::value_objects::ApprovalUrgency,
) -> PortApprovalUrgency {
    match urgency {
        wrldbldr_domain::value_objects::ApprovalUrgency::Normal => PortApprovalUrgency::Normal,
        wrldbldr_domain::value_objects::ApprovalUrgency::AwaitingPlayer => {
            PortApprovalUrgency::AwaitingPlayer
        }
        wrldbldr_domain::value_objects::ApprovalUrgency::SceneCritical => {
            PortApprovalUrgency::SceneCritical
        }
    }
}

/// Convert port ApprovalDecisionType to domain ApprovalDecisionType
fn convert_port_decision_type(
    decision_type: PortApprovalDecisionType,
) -> wrldbldr_domain::value_objects::ApprovalDecisionType {
    match decision_type {
        PortApprovalDecisionType::NpcResponse => {
            wrldbldr_domain::value_objects::ApprovalDecisionType::NpcResponse
        }
        PortApprovalDecisionType::ToolUsage => {
            wrldbldr_domain::value_objects::ApprovalDecisionType::ToolUsage
        }
        PortApprovalDecisionType::ChallengeSuggestion => {
            wrldbldr_domain::value_objects::ApprovalDecisionType::ChallengeSuggestion
        }
        PortApprovalDecisionType::SceneTransition => {
            wrldbldr_domain::value_objects::ApprovalDecisionType::SceneTransition
        }
        PortApprovalDecisionType::ChallengeOutcome => {
            wrldbldr_domain::value_objects::ApprovalDecisionType::ChallengeOutcome
        }
    }
}

/// Convert port ApprovalUrgency to domain ApprovalUrgency
fn convert_port_urgency(
    urgency: PortApprovalUrgency,
) -> wrldbldr_domain::value_objects::ApprovalUrgency {
    match urgency {
        PortApprovalUrgency::Normal => wrldbldr_domain::value_objects::ApprovalUrgency::Normal,
        PortApprovalUrgency::AwaitingPlayer => {
            wrldbldr_domain::value_objects::ApprovalUrgency::AwaitingPlayer
        }
        PortApprovalUrgency::SceneCritical => {
            wrldbldr_domain::value_objects::ApprovalUrgency::SceneCritical
        }
    }
}

/// Convert domain ApprovalRequestData to port ApprovalRequest
fn convert_domain_to_port_approval(
    data: &ApprovalRequestData,
    id: uuid::Uuid,
    priority: u8,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> PortApprovalQueueItem {
    PortApprovalQueueItem {
        id,
        payload: ApprovalRequest {
            world_id: data.world_id.to_uuid(),
            source_action_id: data.source_action_id,
            decision_type: convert_domain_decision_type(data.decision_type),
            urgency: convert_domain_urgency(data.urgency),
            pc_id: data.pc_id.map(|id| id.to_uuid()),
            npc_id: data.npc_id.map(|id| id.to_string()),
            npc_name: data.npc_name.clone(),
            proposed_dialogue: data.proposed_dialogue.clone(),
            internal_reasoning: data.internal_reasoning.clone(),
            proposed_tools: data
                .proposed_tools
                .iter()
                .map(|t| ProposedToolInfo {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    description: t.description.clone(),
                    arguments: t.arguments.clone(),
                })
                .collect(),
            retry_count: data.retry_count,
            challenge_suggestion: data.challenge_suggestion.as_ref().map(|cs| {
                ChallengeSuggestionInfo {
                    challenge_id: cs.challenge_id.clone(),
                    challenge_name: cs.challenge_name.clone(),
                    skill_name: cs.skill_name.clone(),
                    difficulty_display: cs.difficulty_display.clone(),
                    confidence: cs.confidence.clone(),
                    reasoning: cs.reasoning.clone(),
                    target_pc_id: cs.target_pc_id.map(|id| id.to_string()),
                    outcomes: cs.outcomes.as_ref().map(|o| ChallengeSuggestionOutcomes {
                        success: o.success.clone(),
                        failure: o.failure.clone(),
                        critical_success: o.critical_success.clone(),
                        critical_failure: o.critical_failure.clone(),
                    }),
                }
            }),
            narrative_event_suggestion: data.narrative_event_suggestion.as_ref().map(|nes| {
                NarrativeEventSuggestionInfo {
                    event_id: nes.event_id.clone(),
                    event_name: nes.event_name.clone(),
                    description: nes.description.clone(),
                    scene_direction: nes.scene_direction.clone(),
                    confidence: nes.confidence.clone(),
                    reasoning: nes.reasoning.clone(),
                    matched_triggers: nes.matched_triggers.clone(),
                    suggested_outcome: nes.suggested_outcome.clone(),
                }
            }),
            player_dialogue: data.player_dialogue.clone(),
            scene_id: data.scene_id.map(|id| id.to_string()),
            location_id: data.location_id.map(|id| id.to_string()),
            game_time: data.game_time.clone(),
            topics: data.topics.clone(),
        },
        priority,
        enqueued_at: created_at,
        updated_at,
    }
}

/// Convert port ApprovalRequest to domain ApprovalRequestData
fn convert_port_to_domain_approval(request: &ApprovalRequest) -> ApprovalRequestData {
    ApprovalRequestData {
        world_id: WorldId::from_uuid(request.world_id),
        source_action_id: request.source_action_id,
        decision_type: convert_port_decision_type(request.decision_type.clone()),
        urgency: convert_port_urgency(request.urgency),
        pc_id: request.pc_id.map(PlayerCharacterId::from_uuid),
        npc_id: request
            .npc_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok().map(CharacterId::from_uuid)),
        npc_name: request.npc_name.clone(),
        proposed_dialogue: request.proposed_dialogue.clone(),
        internal_reasoning: request.internal_reasoning.clone(),
        proposed_tools: request
            .proposed_tools
            .iter()
            .map(|t| ProposedTool {
                id: t.id.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                arguments: t.arguments.clone(),
            })
            .collect(),
        retry_count: request.retry_count,
        challenge_suggestion: request.challenge_suggestion.as_ref().map(|cs| {
            wrldbldr_domain::value_objects::ChallengeSuggestion {
                challenge_id: cs.challenge_id.clone(),
                challenge_name: cs.challenge_name.clone(),
                skill_name: cs.skill_name.clone(),
                difficulty_display: cs.difficulty_display.clone(),
                confidence: cs.confidence.clone(),
                reasoning: cs.reasoning.clone(),
                target_pc_id: cs.target_pc_id.as_ref().and_then(|s| {
                    uuid::Uuid::parse_str(s)
                        .ok()
                        .map(PlayerCharacterId::from_uuid)
                }),
                outcomes: cs.outcomes.as_ref().map(|o| {
                    wrldbldr_domain::value_objects::ChallengeSuggestionOutcomes {
                        success: o.success.clone(),
                        failure: o.failure.clone(),
                        critical_success: o.critical_success.clone(),
                        critical_failure: o.critical_failure.clone(),
                    }
                }),
            }
        }),
        narrative_event_suggestion: request.narrative_event_suggestion.as_ref().map(|nes| {
            wrldbldr_domain::value_objects::NarrativeEventSuggestion {
                event_id: nes.event_id.clone(),
                event_name: nes.event_name.clone(),
                description: nes.description.clone(),
                scene_direction: nes.scene_direction.clone(),
                confidence: nes.confidence.clone(),
                reasoning: nes.reasoning.clone(),
                matched_triggers: nes.matched_triggers.clone(),
                suggested_outcome: nes.suggested_outcome.clone(),
            }
        }),
        player_dialogue: request.player_dialogue.clone(),
        scene_id: request
            .scene_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok().map(SceneId::from_uuid)),
        location_id: request
            .location_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok().map(LocationId::from_uuid)),
        game_time: request.game_time.clone(),
        topics: request.topics.clone(),
    }
}

/// Convert port DmApprovalDecision to domain DmApprovalDecision
fn convert_port_decision(decision: PortDmApprovalDecision) -> DmApprovalDecision {
    match decision {
        PortDmApprovalDecision::Accept => DmApprovalDecision::Accept,
        PortDmApprovalDecision::AcceptWithRecipients { item_recipients } => {
            DmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        PortDmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        PortDmApprovalDecision::Reject { feedback } => DmApprovalDecision::Reject { feedback },
        PortDmApprovalDecision::TakeOver { dm_response } => {
            DmApprovalDecision::TakeOver { dm_response }
        }
        PortDmApprovalDecision::Unknown => DmApprovalDecision::Reject {
            feedback: "Unknown decision type".to_string(),
        },
    }
}

// ============================================================================
// Port Implementation
// ============================================================================

#[async_trait]
impl<Q, I> DmApprovalQueueServicePort for DMApprovalQueueService<Q, I>
where
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    async fn enqueue(&self, approval: ApprovalRequest) -> anyhow::Result<uuid::Uuid> {
        let data = convert_port_to_domain_approval(&approval);
        let priority = approval.urgency as u8;

        self.queue
            .enqueue(data, priority)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn dequeue(&self) -> anyhow::Result<Option<PortApprovalQueueItem>> {
        let item = self
            .queue
            .dequeue()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| {
            convert_domain_to_port_approval(
                &i.payload,
                i.id,
                i.priority,
                i.created_at,
                i.updated_at,
            )
        }))
    }

    async fn complete(
        &self,
        id: uuid::Uuid,
        decision: PortDmApprovalDecision,
    ) -> anyhow::Result<()> {
        // Get the item first to extract world_id
        let item = self
            .queue
            .get(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .ok_or_else(|| anyhow::anyhow!("Approval item not found: {}", id))?;

        let domain_decision = convert_port_decision(decision);

        // Process the decision (this handles accept/reject/takeover logic)
        self.process_decision(item.payload.world_id, id, domain_decision)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }

    async fn get_pending(&self, world_id: WorldId) -> anyhow::Result<Vec<PortApprovalQueueItem>> {
        let items = self
            .queue
            .list_by_world(world_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(items
            .into_iter()
            .map(|i| {
                convert_domain_to_port_approval(
                    &i.payload,
                    i.id,
                    i.priority,
                    i.created_at,
                    i.updated_at,
                )
            })
            .collect())
    }

    async fn get(&self, id: uuid::Uuid) -> anyhow::Result<Option<PortApprovalQueueItem>> {
        let item = self
            .queue
            .get(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| {
            convert_domain_to_port_approval(
                &i.payload,
                i.id,
                i.priority,
                i.created_at,
                i.updated_at,
            )
        }))
    }

    async fn get_history(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> anyhow::Result<Vec<PortApprovalQueueItem>> {
        let items = self
            .queue
            .get_history_by_world(world_id, limit)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(items
            .into_iter()
            .map(|i| {
                convert_domain_to_port_approval(
                    &i.payload,
                    i.id,
                    i.priority,
                    i.created_at,
                    i.updated_at,
                )
            })
            .collect())
    }

    async fn delay(&self, id: uuid::Uuid, until: DateTime<Utc>) -> anyhow::Result<()> {
        self.queue
            .delay(id, until)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn discard_challenge(&self, request_id: &str) -> anyhow::Result<()> {
        self.discard_challenge("", request_id).await;
        Ok(())
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.queue
            .depth()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<PortApprovalQueueItem>> {
        let items = self
            .queue
            .list_by_status(status)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(items
            .into_iter()
            .map(|i| {
                convert_domain_to_port_approval(
                    &i.payload,
                    i.id,
                    i.priority,
                    i.created_at,
                    i.updated_at,
                )
            })
            .collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.cleanup(retention).await
    }

    async fn expire_old(&self, timeout: std::time::Duration) -> anyhow::Result<u64> {
        self.expire_old(timeout).await
    }
}

// ============================================================================
// ApprovalProcessorPort Implementation
// ============================================================================

#[async_trait]
impl<Q, I> ApprovalProcessorPort for DMApprovalQueueService<Q, I>
where
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    async fn process_decision(
        &self,
        world_id: WorldId,
        item_id: uuid::Uuid,
        decision: DmApprovalDecision,
    ) -> Result<ApprovalOutcome, QueueError> {
        // Delegate to the existing process_decision method
        self.process_decision(world_id, item_id, decision).await
    }
}

// ============================================================================
// ApprovalRequestLookupPort Implementation
// ============================================================================

#[async_trait]
impl<Q, I> ApprovalRequestLookupPort for DMApprovalQueueService<Q, I>
where
    Q: ApprovalQueuePort<ApprovalRequestData> + Send + Sync + 'static,
    I: ItemService + Send + Sync + 'static,
{
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ApprovalRequestData>> {
        let maybe_item = DMApprovalQueueService::<Q, I>::get_by_id(self, id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(maybe_item.map(|i| i.payload))
    }
}
