//! Approval request lookup port
//!
//! Protocol-free read access to approval requests stored in the approval queue.
//!
//! This is a small outbound port used by application services that need to
//! inspect an approval request payload (e.g., challenge suggestion decisions)
//! without depending on concrete queue service implementations.

use async_trait::async_trait;
use wrldbldr_domain::value_objects::ApprovalRequestData;

/// Outbound port for looking up approval requests by ID.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ApprovalRequestLookupPort: Send + Sync {
    /// Fetch the approval request payload by its string ID.
    ///
    /// Returns `Ok(None)` if no item exists.
    async fn get_by_id(&self, id: &str) -> anyhow::Result<Option<ApprovalRequestData>>;
}
