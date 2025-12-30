//! Core CRUD operations for PlayerCharacter entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{PlayerCharacter, PlayerCharacterId};

/// Core CRUD operations for player characters.
///
/// This trait covers basic create, read, update, delete operations
/// plus session unbinding.
#[async_trait]
pub trait PlayerCharacterCrudPort: Send + Sync {
    /// Create a new player character
    async fn create(&self, pc: &PlayerCharacter) -> Result<()>;

    /// Get a player character by ID
    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Update a player character
    async fn update(&self, pc: &PlayerCharacter) -> Result<()>;

    /// Delete a player character
    async fn delete(&self, id: PlayerCharacterId) -> Result<()>;

    /// Unbind a player character from its session
    async fn unbind_from_session(&self, id: PlayerCharacterId) -> Result<()>;
}
