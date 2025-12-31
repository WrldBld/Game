use async_trait::async_trait;

use wrldbldr_domain::{ActionId, PlayerCharacterId, WorldId};

/// Outbound port for player action queue operations.
///
/// Implemented by adapters; used by the application.
#[async_trait]
pub trait PlayerActionQueuePort: Send + Sync {
    /// Enqueue an action
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        player_id: String,
        pc_id: Option<PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<ActionId, String>;

    /// Get current queue depth
    async fn depth(&self) -> Result<usize, String>;
}
