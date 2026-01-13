//! Unified infrastructure error types.
//!
//! Provides a single error type that wraps all infrastructure-layer errors,
//! making it easier to handle errors from multiple external systems.

use thiserror::Error;

use super::ports::{ImageGenError, LlmError, QueueError, RepoError};

/// Unified infrastructure error for all external system failures.
///
/// This error type aggregates errors from all infrastructure ports,
/// allowing use cases to handle infrastructure failures uniformly.
#[derive(Debug, Error)]
pub enum InfraError {
    /// Database/repository operation failed.
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    /// LLM operation failed.
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    /// Image generation failed.
    #[error("Image generation error: {0}")]
    ImageGen(#[from] ImageGenError),

    /// Queue operation failed.
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),

    /// Generic I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl InfraError {
    /// Check if this is a not-found error from the repository.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Repo(RepoError::NotFound { .. }))
    }

    /// Get the entity type if this is a not-found error.
    pub fn not_found_entity(&self) -> Option<&str> {
        match self {
            Self::Repo(RepoError::NotFound { entity_type, .. }) => Some(entity_type),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_repo_error() {
        let repo_err = RepoError::not_found("Character", "abc123");
        let infra_err: InfraError = repo_err.into();
        assert!(infra_err.is_not_found());
        assert_eq!(infra_err.not_found_entity(), Some("Character"));
    }

    #[test]
    fn test_from_llm_error() {
        let llm_err = LlmError::RequestFailed("timeout".to_string());
        let infra_err: InfraError = llm_err.into();
        assert!(!infra_err.is_not_found());
    }
}
