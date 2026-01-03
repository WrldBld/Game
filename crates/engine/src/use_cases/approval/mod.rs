//! DM approval use cases.
//!
//! Handles approval workflows for:
//! - NPC staging (who appears in a region)
//! - LLM suggestions (NPC dialogue, tool calls)
//! - Challenge outcomes

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, DmApprovalDecision, RegionId};

use crate::entities::Staging;
use crate::infrastructure::ports::{QueuePort, RepoError};

/// Container for approval use cases.
pub struct ApprovalUseCases {
    pub approve_staging: Arc<ApproveStaging>,
    pub approve_suggestion: Arc<ApproveSuggestion>,
}

impl ApprovalUseCases {
    pub fn new(
        approve_staging: Arc<ApproveStaging>,
        approve_suggestion: Arc<ApproveSuggestion>,
    ) -> Self {
        Self {
            approve_staging,
            approve_suggestion,
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
            self.staging.unstage_npc(region_id, npc.character_id).await?;
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
}

/// Approve LLM suggestion use case.
///
/// Handles DM approval of LLM-generated content (dialogue, tool calls).
pub struct ApproveSuggestion {
    queue: Arc<dyn QueuePort>,
}

impl ApproveSuggestion {
    pub fn new(queue: Arc<dyn QueuePort>) -> Self {
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
        let (approved, final_dialogue, approved_tools) = match &decision {
            DmApprovalDecision::Accept => (true, None, vec![]),
            DmApprovalDecision::AcceptWithRecipients { .. } => {
                // Item distribution handled separately
                (true, None, vec![])
            }
            DmApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                ..
            } => (true, Some(modified_dialogue.clone()), approved_tools.clone()),
            DmApprovalDecision::Reject { .. } => (false, None, vec![]),
            DmApprovalDecision::TakeOver { dm_response } => (true, Some(dm_response.clone()), vec![]),
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
        })
    }
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
