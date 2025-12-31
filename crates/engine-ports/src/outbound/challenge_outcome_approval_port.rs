use async_trait::async_trait;

use wrldbldr_domain::WorldId;

use super::{ApprovalItem, OutcomeDecision};

/// Outbound port for challenge outcome approval operations.
///
/// Implemented by adapters; used by the application.
#[async_trait]
pub trait ChallengeOutcomeApprovalPort: Send + Sync {
    /// Process DM's decision on an outcome
    async fn process_decision(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        decision: OutcomeDecision,
    ) -> Result<(), String>;

    /// Request outcome branches
    async fn request_branches(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<(), String>;

    /// Select a specific branch
    async fn select_branch(
        &self,
        world_id: &WorldId,
        resolution_id: &str,
        branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<(), String>;

    /// Get an approval item by ID
    async fn get_by_id(&self, request_id: &str) -> Result<Option<ApprovalItem>, String>;
}
