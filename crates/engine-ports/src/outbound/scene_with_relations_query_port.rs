use async_trait::async_trait;

use wrldbldr_domain::SceneId;

use super::UseCaseSceneWithRelations;

/// Outbound port for loading a scene (and relations) in use-case DTO form.
#[async_trait]
pub trait SceneWithRelationsQueryPort: Send + Sync {
    /// Get scene with all relations
    async fn get_scene_with_relations(
        &self,
        scene_id: SceneId,
    ) -> Result<Option<UseCaseSceneWithRelations>, String>;
}
