//! Core CRUD operations for Scene entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{Scene, SceneId};

/// Core CRUD operations for scenes.
///
/// This trait covers basic create, read, update, delete operations
/// plus directorial notes updates.
#[async_trait]
pub trait SceneCrudPort: Send + Sync {
    /// Create a new scene
    async fn create(&self, scene: &Scene) -> Result<()>;

    /// Get a scene by ID
    async fn get(&self, id: SceneId) -> Result<Option<Scene>>;

    /// Update a scene
    async fn update(&self, scene: &Scene) -> Result<()>;

    /// Delete a scene
    async fn delete(&self, id: SceneId) -> Result<()>;

    /// Update directorial notes for a scene
    async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> Result<()>;
}
