use async_trait::async_trait;

use super::ApprovalItem;

/// Outbound port for DM approval queue operations used by the Challenge use case.
///
/// This is a dependency of the application layer and is implemented by adapters.
#[async_trait]
pub trait ChallengeDmApprovalQueuePort: Send + Sync {
    /// Get an approval item by ID
    async fn get_by_id(&self, request_id: &str) -> Result<Option<ApprovalItem>, String>;

    /// Discard a challenge from the queue
    async fn discard_challenge(&self, dm_id: &str, request_id: &str);
}
