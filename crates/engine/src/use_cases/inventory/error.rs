//! Inventory operation errors.

use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::DomainError;

/// Errors that can occur during inventory operations.
#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Character not found")]
    CharacterNotFound,
    #[error("Item not in inventory")]
    ItemNotInInventory,
    #[error("Item not in current region")]
    ItemNotInRegion,
    #[error("Character not in a region")]
    NotInRegion,
    #[error("Validation error: {0}")]
    Validation(#[from] DomainError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
