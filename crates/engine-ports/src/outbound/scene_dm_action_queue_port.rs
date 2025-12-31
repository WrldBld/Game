use async_trait::async_trait;

use wrldbldr_domain::WorldId;

use super::SceneDmAction;

/// Outbound port for enqueueing DM actions for the scene use case.
#[async_trait]
pub trait SceneDmActionQueuePort: Send + Sync {
    /// Enqueue a DM action
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        dm_id: String,
        action: SceneDmAction,
    ) -> Result<(), String>;
}
