//! Character sheet operation errors.

use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{CharacterId, DomainError, WorldId};

/// Errors that can occur during character sheet operations.
#[derive(Debug, thiserror::Error)]
pub enum CharacterSheetError {
    #[error("Character not found: {0}")]
    CharacterNotFound(CharacterId),

    #[error("World not found: {0}")]
    WorldNotFound(WorldId),

    #[error("Game system not found: {0}")]
    GameSystemNotFound(String),

    #[error("Character sheet schema not available for system: {0}")]
    #[allow(dead_code)]
    SchemaNotAvailable(String),

    #[error("Field validation failed: {field_id}: {message}")]
    FieldValidation { field_id: String, message: String },

    #[error("Missing required fields: {0}")]
    MissingRequiredFields(String),

    #[error("Invalid character ID format")]
    #[allow(dead_code)]
    InvalidCharacterId,

    #[error("Invalid world ID format")]
    #[allow(dead_code)]
    InvalidWorldId,

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
