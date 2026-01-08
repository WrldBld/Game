//! Unified error types for the domain layer
//!
//! Provides a common error type that can be used across all domain operations,
//! enabling consistent error handling without forcing adapters to use String or anyhow.

use thiserror::Error;

use crate::value_objects::DiceParseError;

/// Unified error type for domain operations
#[derive(Debug, Error, Clone)]
pub enum DomainError {
    /// Validation failed (e.g., invalid field values)
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Invalid ID format
    #[error("Invalid ID format: {0}")]
    InvalidId(String),

    /// Entity not found
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    /// Business rule violation
    #[error("Constraint violation: {0}")]
    Constraint(String),

    /// Parse error (for value objects)
    #[error("Parse error: {0}")]
    Parse(String),

    /// State transition not allowed
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// Container is at capacity
    #[error("Container full: {current}/{max} items")]
    ContainerFull { current: u32, max: u32 },
}

impl DomainError {
    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a not found error
    pub fn not_found(entity_type: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type,
            id: id.into(),
        }
    }

    /// Create a constraint violation error
    pub fn constraint(msg: impl Into<String>) -> Self {
        Self::Constraint(msg.into())
    }

    /// Create an invalid ID error
    pub fn invalid_id(msg: impl Into<String>) -> Self {
        Self::InvalidId(msg.into())
    }

    /// Create a parse error
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create an invalid state transition error
    pub fn invalid_state_transition(msg: impl Into<String>) -> Self {
        Self::InvalidStateTransition(msg.into())
    }

    /// Create a container full error
    pub fn container_full(current: u32, max: u32) -> Self {
        Self::ContainerFull { current, max }
    }
}

impl From<DiceParseError> for DomainError {
    fn from(err: DiceParseError) -> Self {
        Self::Parse(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let err = DomainError::validation("name cannot be empty");
        assert!(matches!(err, DomainError::Validation(_)));
        assert_eq!(err.to_string(), "Validation failed: name cannot be empty");
    }

    #[test]
    fn test_not_found_error() {
        let err = DomainError::not_found("Character", "123e4567-e89b-12d3-a456-426614174000");
        assert!(matches!(err, DomainError::NotFound { .. }));
        assert!(err.to_string().contains("Character"));
        assert!(err.to_string().contains("123e4567"));
    }

    #[test]
    fn test_constraint_error() {
        let err = DomainError::constraint("character already in party");
        assert!(matches!(err, DomainError::Constraint(_)));
        assert_eq!(
            err.to_string(),
            "Constraint violation: character already in party"
        );
    }

    #[test]
    fn test_from_dice_parse_error() {
        let dice_err = DiceParseError::Empty;
        let domain_err: DomainError = dice_err.into();
        assert!(matches!(domain_err, DomainError::Parse(_)));
        assert!(domain_err.to_string().contains("Empty dice formula"));
    }

    #[test]
    fn test_container_full_error() {
        let err = DomainError::container_full(5, 5);
        assert!(matches!(err, DomainError::ContainerFull { .. }));
        assert_eq!(err.to_string(), "Container full: 5/5 items");
    }
}
