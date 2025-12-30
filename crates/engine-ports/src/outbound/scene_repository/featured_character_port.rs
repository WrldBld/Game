//! Featured character operations for Scene entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{CharacterId, Scene, SceneCharacter, SceneId};

/// Featured character management for scenes.
///
/// This trait covers the FEATURES_CHARACTER relationship between
/// scenes and NPCs, including roles and cues.
#[async_trait]
pub trait SceneFeaturedCharacterPort: Send + Sync {
    /// Add a featured character to the scene
    async fn add_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()>;

    /// Get all featured characters for a scene
    async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<(CharacterId, SceneCharacter)>>;

    /// Update a featured character's role/cue
    async fn update_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()>;

    /// Remove a featured character from the scene
    async fn remove_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Get scenes featuring a specific character
    async fn get_scenes_for_character(&self, character_id: CharacterId) -> Result<Vec<Scene>>;
}
