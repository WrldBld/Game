//! Want management operations for Character entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{CharacterId, CharacterWant, Want, WantId};
use wrldbldr_domain::value_objects::WantTarget;

/// Want management operations for Character entities.
///
/// This trait covers:
/// - Creating and managing character wants (HAS_WANT edges to Want nodes)
/// - Want targeting (TARGETS edges from Want nodes)
/// - Want resolution for determining what a character desires
///
/// # Used By
/// - `ActantialContextServiceImpl` - For retrieving character motivations
/// - `CharacterServiceImpl` - For want CRUD operations
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait CharacterWantPort: Send + Sync {
    /// Create a want and attach it to a character
    async fn create_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()>;

    /// Get all wants for a character
    async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>>;

    /// Update a want
    async fn update_want(&self, want: &Want) -> Result<()>;

    /// Delete a want
    async fn delete_want(&self, want_id: WantId) -> Result<()>;

    /// Set a want's target (creates TARGETS edge)
    /// target_type: "Character", "Item", or "Goal"
    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: String,
        target_type: String,
    ) -> Result<()>;

    /// Remove a want's target (deletes TARGETS edge)
    async fn remove_want_target(&self, want_id: WantId) -> Result<()>;

    /// Get the resolved target of a want
    ///
    /// Returns the target with its name resolved (Character, Item, or Goal).
    async fn get_want_target(&self, want_id: WantId) -> Result<Option<WantTarget>>;
}
