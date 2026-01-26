//! Inventory operation errors.

use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{DomainError, ItemId, PlayerCharacterId};

/// Errors that can occur during inventory operations.
#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("Item not found: {0}")]
    ItemNotFound(ItemId),
    #[error("Character not found: {0}")]
    CharacterNotFound(PlayerCharacterId),
    #[error("Item not in inventory: {0}")]
    ItemNotInInventory(ItemId),
    #[error("Item not in current region")]
    ItemNotInRegion,
    #[error("Character not in a region")]
    NotInRegion,
    #[error("Validation error: {0}")]
    Validation(#[from] DomainError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
