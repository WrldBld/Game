// Unified error - some helper methods for future use
#![allow(dead_code)]

//! Unified infrastructure error types.
//!
//! Provides a single error type that wraps all infrastructure-layer errors,
//! making it easier to handle errors from multiple external systems.

use super::correlation::CorrelationId;
use super::ports::{ImageGenError, LlmError, QueueError, RepoError};
use thiserror::Error;

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

/// Error with correlation ID for request tracing.
///
/// Wraps InfraError with correlation context for debugging.
#[derive(Debug)]
pub struct InfraErrorWithCorrelation {
    /// Correlation ID for the request that caused this error
    pub correlation_id: CorrelationId,
    /// The underlying infrastructure error
    pub error: InfraError,
}

impl InfraErrorWithCorrelation {
    /// Create a new error with correlation ID.
    pub fn new(correlation_id: CorrelationId, error: InfraError) -> Self {
        Self {
            correlation_id,
            error,
        }
    }

    /// Get the correlation ID.
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// Get the underlying error.
    pub fn error(&self) -> &InfraError {
        &self.error
    }

    /// Check if this is a not-found error.
    pub fn is_not_found(&self) -> bool {
        self.error.is_not_found()
    }

    /// Get the entity type if this is a not-found error.
    pub fn not_found_entity(&self) -> Option<&str> {
        self.error.not_found_entity()
    }
}

impl std::fmt::Display for InfraErrorWithCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[correlation_id={}] {}",
            self.correlation_id.short(),
            self.error
        )
    }
}

impl std::error::Error for InfraErrorWithCorrelation {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
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
    use std::error::Error;

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

    #[test]
    fn test_infra_error_with_correlation() {
        let correlation_id = CorrelationId::new();
        let infra_err = InfraError::Repo(RepoError::not_found("Character", "abc123"));
        let err = InfraErrorWithCorrelation::new(correlation_id, infra_err);

        assert_eq!(err.correlation_id(), &correlation_id);
        assert!(err.is_not_found());
        assert_eq!(err.not_found_entity(), Some("Character"));

        let display = format!("{}", err);
        assert!(display.contains(&correlation_id.short().to_string()));
    }

    #[test]
    fn test_infra_error_with_correlation_source() {
        let correlation_id = CorrelationId::new();
        let infra_err = InfraError::Llm(LlmError::RequestFailed("test".to_string()));
        let err = InfraErrorWithCorrelation::new(correlation_id, infra_err);

        // Source returns a trait object, so we check it exists
        assert!(err.source().is_some());
        // And that the error message contains our test string
        assert!(err.to_string().contains("test"));
    }
}
