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

use chrono::Utc;

use wrldbldr_engine_ports::outbound::{
    ApprovalQueuePort, QueueError, QueueItem, QueueItemId,
};
use crate::application::services::tool_execution_service::ToolExecutionService;
use crate::application::services::item_service::ItemService;
use crate::application::services::StoryEventService;
use crate::application::dto::{ApprovalItem, DmApprovalDecision, ProposedToolInfo};
use wrldbldr_domain::value_objects::GameTool;
use wrldbldr_domain::entities::AcquisitionMethod;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, SceneId, WorldId};
use std::collections::HashMap;

/// Maximum number of times a response can be rejected before requiring TakeOver
const MAX_RETRY_COUNT: u32 = 3;

/// Service for managing the DM approval queue
pub struct DMApprovalQueueService<Q: ApprovalQueuePort<ApprovalItem>, I: ItemService> {
    pub(crate) queue: Arc<Q>,
    tool_execution_service: ToolExecutionService,
    /// Story event service for recording dialogue exchanges
    story_event_service: Arc<dyn StoryEventService>,
    /// Item service for creating items and managing inventory
    item_service: Arc<I>,
}

impl<Q: ApprovalQueuePort<ApprovalItem>, I: ItemService> DMApprovalQueueService<Q, I> {
    pub fn queue(&self) -> &Arc<Q> {
        &self.queue
    }

    /// Create a new DM approval queue service
    pub fn new(queue: Arc<Q>, story_event_service: Arc<dyn StoryEventService>, item_service: Arc<I>) -> Self {
        Self {
            queue,
            tool_execution_service: ToolExecutionService::new(),
            story_event_service,
            item_service,
        }
    }

    /// Get all pending approvals for a world (for DM UI)
    pub async fn get_pending(&self, world_id: WorldId) -> Result<Vec<QueueItem<ApprovalItem>>, QueueError> {
        self.queue.list_by_world(world_id).await
    }

    /// Get an approval item by its string ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<QueueItem<ApprovalItem>>, QueueError> {
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
                self.handle_accept(world_id, &item.payload, HashMap::new()).await?
            }
            DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
                self.handle_accept(world_id, &item.payload, item_recipients).await?
            }
            DmApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            } => {
                self.handle_accept_modified(world_id, &item.payload, &modified_dialogue, &approved_tools, &rejected_tools, item_recipients).await?
            }
            DmApprovalDecision::Reject { feedback } => {
                self.handle_reject(&item.payload, &feedback).await?
            }
            DmApprovalDecision::TakeOver { dm_response } => {
                self.handle_takeover(world_id, &item.payload, &dm_response).await?
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
                    .delay(item_id, Utc::now() + Duration::from_secs(1))
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
        approval: &ApprovalItem,
        item_recipients: HashMap<String, Vec<String>>,
    ) -> Result<ApprovalOutcome, QueueError> {
        // Record dialogue exchange as a story event
        self.record_dialogue_event(world_id, approval, &approval.proposed_dialogue)
            .await;

        // Execute approved tool calls
        let mut executed_tools = Vec::new();
        for tool_info in &approval.proposed_tools {
            // Check if this is a give_item with DM-specified recipients
            if tool_info.name == "give_item" {
                self.execute_give_item_with_recipients(
                    world_id,
                    tool_info,
                    item_recipients.get(&tool_info.id),
                ).await;
                executed_tools.push(tool_info.name.clone());
            } else {
                // Convert ProposedToolInfo to GameTool
                // Parse the tool arguments JSON to determine tool type
                if let Ok(tool) = self.parse_tool_from_info(tool_info) {
                    if let Err(e) = self
                        .tool_execution_service
                        .execute_tool(&tool)
                        .await
                    {
                        tracing::warn!("Failed to execute tool {}: {}", tool_info.name, e);
                        // Continue with other tools even if one fails
                    } else {
                        executed_tools.push(tool_info.name.clone());
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
        approval: &ApprovalItem,
        modified_dialogue: &str,
        approved_tools: &[String],
        _rejected_tools: &[String],
        item_recipients: HashMap<String, Vec<String>>,
    ) -> Result<ApprovalOutcome, QueueError> {
        // Record dialogue exchange as a story event (with modified dialogue)
        self.record_dialogue_event(world_id, approval, modified_dialogue)
            .await;

        // Execute approved tool calls (filter based on approved_tools list)
        // approved_tools contains tool IDs that should be executed
        let mut executed_tools = Vec::new();
        for tool_info in &approval.proposed_tools {
            // Check if this tool is in the approved list
            if approved_tools.contains(&tool_info.id) {
                // Check if this is a give_item with DM-specified recipients
                if tool_info.name == "give_item" {
                    self.execute_give_item_with_recipients(
                        world_id,
                        tool_info,
                        item_recipients.get(&tool_info.id),
                    ).await;
                    executed_tools.push(tool_info.name.clone());
                } else {
                    if let Ok(tool) = self.parse_tool_from_info(tool_info) {
                        if let Err(e) = self
                            .tool_execution_service
                            .execute_tool(&tool)
                            .await
                        {
                            tracing::warn!("Failed to execute tool {}: {}", tool_info.name, e);
                        } else {
                            executed_tools.push(tool_info.name.clone());
                        }
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
        approval: &ApprovalItem,
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
        approval: &ApprovalItem,
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
        approval: &ApprovalItem,
        npc_response: &str,
    ) {
        // Parse NPC ID from approval item
        let npc_id = match &approval.npc_id {
            Some(id_str) => match uuid::Uuid::parse_str(id_str) {
                Ok(uuid) => CharacterId::from_uuid(uuid),
                Err(e) => {
                    tracing::warn!("Cannot record dialogue event: invalid npc_id '{}': {}", id_str, e);
                    return;
                }
            },
            None => {
                tracing::warn!(
                    "Cannot record dialogue event: npc_id not set in approval for '{}'",
                    approval.npc_name
                );
                return;
            }
        };

        // Record the dialogue exchange with full context from ApprovalItem
        let scene_id = approval.scene_id.as_ref().and_then(|s| {
            uuid::Uuid::parse_str(s).ok().map(SceneId::from_uuid)
        });
        let location_id = approval.location_id.as_ref().and_then(|s| {
            uuid::Uuid::parse_str(s).ok().map(LocationId::from_uuid)
        });
        
        if let Err(e) = self
            .story_event_service
            .record_dialogue_exchange(
                world_id,
                scene_id,
                location_id,
                npc_id,
                approval.npc_name.clone(),
                approval.player_dialogue.clone().unwrap_or_default(),
                npc_response.to_string(),
                approval.topics.clone(),
                None,       // tone
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
            let pc_id: PlayerCharacterId = pc_id.into();
            if let Err(e) = self
                .story_event_service
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
        self.queue.delay(item_id, Utc::now() + duration).await
    }

    /// Get decision history for a world
    pub async fn get_history(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<QueueItem<ApprovalItem>>, QueueError> {
        self.queue.get_history_by_world(world_id, limit).await
    }

    /// Expire old pending approvals
    pub async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError> {
        self.queue.expire_old(older_than).await
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

    /// Parse ProposedToolInfo into GameTool
    fn parse_tool_from_info(
        &self,
        tool_info: &ProposedToolInfo,
    ) -> Result<GameTool, QueueError> {
        // Parse tool based on name and arguments (arguments is serde_json::Value)
        let args = &tool_info.arguments;
        
        match tool_info.name.as_str() {
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
            _ => Err(QueueError::Backend(format!("Unknown tool: {}", tool_info.name))),
        }
    }

    /// Execute a give_item tool with DM-specified recipients
    ///
    /// If recipients is None or empty, the item is not given (DM chose not to give it).
    /// If recipients has PC IDs, creates the item and gives it to each recipient.
    async fn execute_give_item_with_recipients(
        &self,
        world_id: WorldId,
        tool_info: &ProposedToolInfo,
        recipients: Option<&Vec<String>>,
    ) {
        let args = &tool_info.arguments;

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
            Some(ids) if !ids.is_empty() => {
                ids.iter()
                    .filter_map(|id| {
                        uuid::Uuid::parse_str(id)
                            .ok()
                            .map(PlayerCharacterId::from_uuid)
                    })
                    .collect()
            }
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
    MaxRetriesExceeded {
        feedback: String,
    },
}
