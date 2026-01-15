//! DM approval use cases.
//!
//! Handles approval workflows for:
//! - NPC staging (who appears in a region)
//! - LLM suggestions (NPC dialogue, tool calls)
//! - Challenge outcomes

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, RegionId, WorldId};

use crate::queue_types::DmApprovalDecision;

use crate::infrastructure::ports::RepoError;
use crate::repositories::Queue;
use crate::repositories::staging::Staging;

/// Container for approval use cases.
pub struct ApprovalUseCases {
    pub approve_staging: Arc<ApproveStaging>,
    pub approve_suggestion: Arc<ApproveSuggestion>,
    pub decision_flow: Arc<ApprovalDecisionFlow>,
}

impl ApprovalUseCases {
    pub fn new(
        approve_staging: Arc<ApproveStaging>,
        approve_suggestion: Arc<ApproveSuggestion>,
        decision_flow: Arc<ApprovalDecisionFlow>,
    ) -> Self {
        Self {
            approve_staging,
            approve_suggestion,
            decision_flow,
        }
    }
}

/// Result of staging approval.
#[derive(Debug)]
pub struct StagingApprovalResult {
    /// The region that was staged
    pub region_id: RegionId,
    /// NPCs that are now staged in the region
    pub staged_npcs: Vec<CharacterId>,
}

/// Approve staging use case.
///
/// Handles DM approval of which NPCs appear in a region.
pub struct ApproveStaging {
    staging: Arc<Staging>,
}

impl ApproveStaging {
    pub fn new(staging: Arc<Staging>) -> Self {
        Self { staging }
    }

    /// Approve staging for a region with a specific set of NPCs.
    ///
    /// # Arguments
    /// * `region_id` - The region being staged
    /// * `npc_ids` - The NPCs to stage in the region
    ///
    /// # Returns
    /// * `Ok(StagingApprovalResult)` - Staging was applied
    /// * `Err(ApprovalError)` - Failed to process staging
    pub async fn execute(
        &self,
        region_id: RegionId,
        npc_ids: Vec<CharacterId>,
    ) -> Result<StagingApprovalResult, ApprovalError> {
        // Stage the approved NPCs
        for npc_id in &npc_ids {
            self.staging.stage_npc(region_id, *npc_id).await?;
        }

        Ok(StagingApprovalResult {
            region_id,
            staged_npcs: npc_ids,
        })
    }

    /// Clear staging for a region (remove all NPCs).
    pub async fn clear_staging(&self, region_id: RegionId) -> Result<(), ApprovalError> {
        let current = self.staging.get_staged_npcs(region_id).await?;
        for npc in current {
            self.staging
                .unstage_npc(region_id, npc.character_id)
                .await?;
        }
        Ok(())
    }
}

/// Result of suggestion approval.
#[derive(Debug)]
pub struct SuggestionApprovalResult {
    /// The original suggestion ID
    pub suggestion_id: Uuid,
    /// Whether it was approved
    pub approved: bool,
    /// The final dialogue (possibly modified)
    pub final_dialogue: Option<String>,
    /// Tools that were approved
    pub approved_tools: Vec<String>,
    /// NPC ID (speaker)
    pub npc_id: Option<String>,
    /// NPC name (speaker)
    pub npc_name: Option<String>,
    /// Conversation ID (for dialogue tracking)
    pub conversation_id: Option<Uuid>,
}

/// Approve LLM suggestion use case.
///
/// Handles DM approval of LLM-generated content (dialogue, tool calls).
pub struct ApproveSuggestion {
    queue: Arc<Queue>,
}

impl ApproveSuggestion {
    pub fn new(queue: Arc<Queue>) -> Self {
        Self { queue }
    }

    /// Process a DM decision on an LLM suggestion.
    ///
    /// # Arguments
    /// * `approval_queue_id` - The ID of the approval queue item
    /// * `decision` - The DM's decision (accept, modify, reject, takeover)
    ///
    /// # Returns
    /// * `Ok(SuggestionApprovalResult)` - Decision was processed
    /// * `Err(ApprovalError)` - Failed to process decision
    pub async fn execute(
        &self,
        approval_queue_id: Uuid,
        decision: DmApprovalDecision,
    ) -> Result<SuggestionApprovalResult, ApprovalError> {
        // Get the queue item first to extract NPC info
        let queue_item: Option<crate::queue_types::ApprovalRequestData> = self
            .queue
            .get_approval_request(approval_queue_id)
            .await
            .map_err(|e| ApprovalError::QueueError(e.to_string()))?;

        let (npc_id, npc_name, original_dialogue, conversation_id) = queue_item
            .map(|data| {
                (
                    data.npc_id.map(|id| id.to_string()),
                    Some(data.npc_name),
                    Some(data.proposed_dialogue),
                    data.conversation_id,
                )
            })
            .unwrap_or((None, None, None, None));

        let (approved, final_dialogue, approved_tools) = match &decision {
            DmApprovalDecision::Accept => (true, original_dialogue, vec![]),
            DmApprovalDecision::AcceptWithRecipients { .. } => {
                // Item distribution handled separately
                (true, original_dialogue, vec![])
            }
            DmApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                ..
            } => (
                true,
                Some(modified_dialogue.clone()),
                approved_tools.clone(),
            ),
            DmApprovalDecision::Reject { .. } => (false, None, vec![]),
            DmApprovalDecision::TakeOver { dm_response } => {
                (true, Some(dm_response.clone()), vec![])
            }
        };

        // Mark the queue item based on decision
        if approved {
            self.queue
                .mark_complete(approval_queue_id)
                .await
                .map_err(|e| ApprovalError::QueueError(e.to_string()))?;
        } else {
            self.queue
                .mark_failed(approval_queue_id, "Rejected by DM")
                .await
                .map_err(|e| ApprovalError::QueueError(e.to_string()))?;
        }

        Ok(SuggestionApprovalResult {
            suggestion_id: approval_queue_id,
            approved,
            final_dialogue,
            approved_tools,
            npc_id,
            npc_name,
            conversation_id,
        })
    }
}

/// Full approval decision flow (approval + dialogue persistence).
pub struct ApprovalDecisionFlow {
    approve_suggestion: Arc<ApproveSuggestion>,
    narrative: Arc<crate::use_cases::narrative_operations::Narrative>,
    queue: Arc<Queue>,
}

impl ApprovalDecisionFlow {
    pub fn new(
        approve_suggestion: Arc<ApproveSuggestion>,
        narrative: Arc<crate::use_cases::narrative_operations::Narrative>,
        queue: Arc<Queue>,
    ) -> Self {
        Self {
            approve_suggestion,
            narrative,
            queue,
        }
    }

    pub async fn execute(
        &self,
        approval_id: Uuid,
        decision: DmApprovalDecision,
    ) -> Result<ApprovalDecisionOutcome, ApprovalDecisionError> {
        let approval_data: crate::queue_types::ApprovalRequestData = self
            .queue
            .get_approval_request(approval_id)
            .await
            .map_err(|e| ApprovalDecisionError::QueueError(e.to_string()))?
            .ok_or(ApprovalDecisionError::ApprovalNotFound)?;

        let result = self
            .approve_suggestion
            .execute(approval_id, decision)
            .await
            .map_err(ApprovalDecisionError::Approval)?;

        if result.approved {
            let dialogue = result.final_dialogue.clone().unwrap_or_default();
            if !dialogue.is_empty() {
                if let (Some(pc_id), Some(npc_id)) = (approval_data.pc_id, approval_data.npc_id) {
                    let player_dialogue = approval_data.player_dialogue.clone().unwrap_or_default();
                    if let Err(e) = self
                        .narrative
                        .record_dialogue_exchange(
                            approval_data.world_id,
                            pc_id,
                            npc_id,
                            approval_data.npc_name.clone(),
                            player_dialogue,
                            dialogue,
                            approval_data.topics.clone(),
                            approval_data.scene_id,
                            approval_data.location_id,
                            approval_data.game_time.clone(),
                        )
                        .await
                    {
                        tracing::error!(error = %e, "Failed to record dialogue exchange");
                    }
                }
            }
        }

        Ok(ApprovalDecisionOutcome {
            world_id: approval_data.world_id,
            approved: result.approved,
            final_dialogue: result.final_dialogue,
            approved_tools: result.approved_tools,
            npc_id: result.npc_id,
            npc_name: result.npc_name,
            conversation_id: result.conversation_id,
        })
    }
}

pub struct ApprovalDecisionOutcome {
    pub world_id: WorldId,
    pub approved: bool,
    pub final_dialogue: Option<String>,
    pub approved_tools: Vec<String>,
    pub npc_id: Option<String>,
    pub npc_name: Option<String>,
    pub conversation_id: Option<Uuid>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("Item not found")]
    NotFound,
    #[error("Already processed")]
    AlreadyProcessed,
    #[error("Staging was rejected")]
    Rejected,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[derive(Debug, thiserror::Error)]
pub enum ApprovalDecisionError {
    #[error("Approval request not found")]
    ApprovalNotFound,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Approval error: {0}")]
    Approval(#[from] ApprovalError),
}
