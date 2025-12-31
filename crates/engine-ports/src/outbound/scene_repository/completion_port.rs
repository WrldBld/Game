//! Scene completion tracking operations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{PlayerCharacterId, SceneId};

/// Scene completion tracking for player characters.
///
/// This trait covers the COMPLETED_SCENE relationship between
/// player characters and scenes they have finished.
#[async_trait]
pub trait SceneCompletionPort: Send + Sync {
    /// Mark a scene as completed by a player character
    async fn mark_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId)
        -> Result<()>;

    /// Check if a player character has completed a scene
    async fn is_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId)
        -> Result<bool>;

    /// Get all scenes completed by a player character
    async fn get_completed_scenes(&self, pc_id: PlayerCharacterId) -> Result<Vec<SceneId>>;
}
