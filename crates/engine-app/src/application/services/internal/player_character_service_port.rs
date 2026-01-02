//! Player character service port - Interface for player character operations
//!
//! This port abstracts player character business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::PlayerCharacter;
use wrldbldr_domain::{PlayerCharacterId, SkillId, WorldId};

/// Port for player character service operations
///
/// This trait defines the read operations for player character management.
/// Adapters implement this trait by wrapping the PlayerCharacterService.
///
/// # Naming Convention
///
/// Method names match the corresponding `PlayerCharacterService` app-layer trait
/// for consistency across the codebase.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait PlayerCharacterServicePort: Send + Sync {
    /// Get a player character by ID
    ///
    /// Returns the player character if found, or None if not found.
    async fn get_pc(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Get a player character by user ID and world ID
    ///
    /// Returns the player character for the given user in the specified world,
    /// or None if not found.
    async fn get_pc_by_user_and_world(
        &self,
        user_id: &str,
        world_id: &WorldId,
    ) -> Result<Option<PlayerCharacter>>;

    /// Get all player characters in a world
    ///
    /// Returns all player characters belonging to the specified world.
    async fn get_pcs_by_world(&self, world_id: &WorldId) -> Result<Vec<PlayerCharacter>>;

    /// Get a player character's modifier for a specific skill.
    ///
    /// Returns 0 if the PC doesn't have the skill or doesn't have sheet data.
    async fn get_skill_modifier(&self, id: PlayerCharacterId, skill_id: SkillId) -> Result<i32>;
}
