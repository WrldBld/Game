//! Core CRUD operations for Character entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{Character, CharacterId, SceneId, WorldId};

/// Core CRUD operations for Character entities.
///
/// This trait covers:
/// - Basic entity operations (create, get, list, update, delete)
/// - Scene-based character retrieval
///
/// # Used By
/// - `CharacterServiceImpl` - For all CRUD operations
/// - Scene loading services - For retrieving characters in a scene
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait CharacterCrudPort: Send + Sync {
    /// Create a new character
    async fn create(&self, character: &Character) -> Result<()>;

    /// Get a character by ID
    async fn get(&self, id: CharacterId) -> Result<Option<Character>>;

    /// List all characters in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Character>>;

    /// Update a character
    async fn update(&self, character: &Character) -> Result<()>;

    /// Delete a character
    async fn delete(&self, id: CharacterId) -> Result<()>;

    /// Get characters by scene
    async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>>;
}
