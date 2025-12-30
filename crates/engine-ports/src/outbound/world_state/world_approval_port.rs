//! World Approval Port - Pending DM approval management.
//!
//! This port handles pending approvals awaiting DM review.

use wrldbldr_domain::value_objects::PendingApprovalItem;
use wrldbldr_domain::WorldId;

/// Port for managing pending DM approvals within a world.
///
/// Pending approvals are items that require DM review before
/// being applied to the game state (e.g., NPC dialogue, events).
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldApprovalPort: Send + Sync {
    /// Add an item pending DM approval.
    fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem);

    /// Remove a pending approval by its ID.
    ///
    /// Returns the removed item if found.
    fn remove_pending_approval(
        &self,
        world_id: &WorldId,
        approval_id: &str,
    ) -> Option<PendingApprovalItem>;

    /// Get all pending approvals for a world.
    fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldApprovalPort {}

    impl WorldApprovalPort for WorldApprovalPort {
        fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem);
        fn remove_pending_approval(&self, world_id: &WorldId, approval_id: &str) -> Option<PendingApprovalItem>;
        fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem>;
    }
}
