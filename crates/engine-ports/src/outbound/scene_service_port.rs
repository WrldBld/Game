//! Scene service port - Interface for scene operations
//!
//! This port abstracts scene business logic from infrastructure.
//! Adapters can depend on this port instead of directly importing
//! `SceneService` from engine-app.
//!
//! # Usage
//!
//! Adapters implement this trait by wrapping `SceneService`:
//!
//! ```ignore
//! pub struct SceneServiceAdapter {
//!     service: Arc<dyn SceneService>,
//! }
//!
//! impl SceneServicePort for SceneServiceAdapter {
//!     async fn get_scene_with_relations(&self, id: SceneId) -> Result<Option<SceneWithRelations>> {
//!         // Delegate to service, convert types
//!     }
//! }
//! ```

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::{Character, Location, Scene};
use wrldbldr_domain::SceneId;

/// Scene with all related entities loaded
///
/// This struct contains the scene along with its location and
/// featured characters, suitable for rendering or processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneWithRelations {
    /// The scene entity
    pub scene: Scene,
    /// The location where the scene takes place
    pub location: Location,
    /// Characters featured in this scene
    pub featured_characters: Vec<Character>,
}

/// Port for scene service operations
///
/// This port provides access to scene-related business logic
/// without coupling adapters to the application layer implementation.
#[async_trait]
pub trait SceneServicePort: Send + Sync {
    /// Get a scene with all its relations (location, characters)
    ///
    /// Returns the scene along with its location and featured characters.
    /// Returns `Ok(None)` if the scene is not found.
    ///
    /// # Arguments
    ///
    /// * `scene_id` - The ID of the scene to retrieve
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if related
    /// entities (location) cannot be loaded.
    async fn get_scene_with_relations(
        &self,
        scene_id: SceneId,
    ) -> Result<Option<SceneWithRelations>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_with_relations_serialization() {
        // Just verify the struct is serializable
        let _swr: Option<SceneWithRelations> = None;
    }
}
