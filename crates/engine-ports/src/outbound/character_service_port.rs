//! Character service port - Interface for character operations
//!
//! This port abstracts character business logic from infrastructure adapters.
//! It exposes query methods for retrieving characters by various criteria.
//!
//! # Design Notes
//!
//! This port is designed for use by infrastructure adapters that need to query
//! character information. It focuses on read operations used by prompt builders,
//! scene renderers, and dialogue systems.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::Character;
use wrldbldr_domain::{CharacterId, SceneId, WorldId};

/// Port for character service operations used by infrastructure adapters.
///
/// This trait provides read-only access to character data for use in
/// building prompts, gathering context, and scene rendering.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait CharacterServicePort: Send + Sync {
    /// Get a character by ID.
    ///
    /// Returns `Ok(None)` if the character is not found.
    async fn get_character(&self, id: CharacterId) -> Result<Option<Character>>;

    /// List all characters in a world.
    ///
    /// Returns active characters sorted by name.
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Character>>;

    /// List characters featured in a specific scene.
    ///
    /// This retrieves characters that are connected to the scene via
    /// FEATURES_CHARACTER edges.
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of CharacterServicePort for testing.
    pub CharacterServicePort {}

    #[async_trait]
    impl CharacterServicePort for CharacterServicePort {
        async fn get_character(&self, id: CharacterId) -> Result<Option<Character>>;
        async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Character>>;
        async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>>;
    }
}
