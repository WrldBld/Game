use async_trait::async_trait;

use wrldbldr_domain::WorldId;

/// Outbound port for sending messages to the DM.
///
/// Implemented by adapters; used by the application.
#[async_trait]
pub trait DmNotificationPort: Send + Sync {
    /// Send action queued notification to DM
    async fn notify_action_queued(
        &self,
        world_id: &WorldId,
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    );
}
