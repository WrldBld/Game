use async_trait::async_trait;

use wrldbldr_domain::SceneId;

use super::InteractionEntity;

/// Outbound port for listing scene interactions in use-case DTO form.
#[async_trait]
pub trait SceneInteractionsQueryPort: Send + Sync {
    /// List interactions for a scene
    async fn list_interactions(&self, scene_id: SceneId) -> Result<Vec<InteractionEntity>, String>;
}
