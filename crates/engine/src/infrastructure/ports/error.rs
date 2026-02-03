// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! Error types for port operations.

/// Repository operation errors with context for debugging.
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    /// Entity not found - includes entity type and ID for actionable error messages.
    #[error("{entity_type} not found: {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    /// Database operation failed - includes operation name for tracing.
    #[error("Database error in {operation}: {message}")]
    Database {
        operation: &'static str,
        message: String,
    },

    /// Serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Business constraint violated.
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}

impl RepoError {
    /// Create a NotFound error with entity type and ID context.
    pub fn not_found(entity_type: &'static str, id: impl ToString) -> Self {
        Self::NotFound {
            entity_type,
            id: id.to_string(),
        }
    }

    /// Create a Database error with operation context.
    pub fn database(operation: &'static str, message: impl ToString) -> Self {
        Self::Database {
            operation,
            message: message.to_string(),
        }
    }

    /// Create a Serialization error.
    pub fn serialization(message: impl ToString) -> Self {
        Self::Serialization(message.to_string())
    }

    /// Create a ConstraintViolation error.
    pub fn constraint(message: impl ToString) -> Self {
        Self::ConstraintViolation(message.to_string())
    }

    /// Create a Config error.
    pub fn config(message: impl ToString) -> Self {
        Self::Config(message.to_string())
    }

    /// Check if this is a NotFound error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LlmError {
    #[error("LLM request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ImageGenError {
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Service unavailable")]
    Unavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    /// Validation error - input data is invalid.
    #[error("Validation error: {0}")]
    Validation(String),
    /// Item not found.
    #[error("Queue item not found: {id}")]
    NotFound { id: String },
    /// Database error - includes operation name for tracing.
    #[error("Database error in {operation}: {message}")]
    Database {
        operation: &'static str,
        message: String,
    },
    /// Conflict - item already exists (e.g., unique constraint violation).
    #[error("Conflict: {message}")]
    Conflict { message: String },
    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl QueueError {
    /// Create a NotFound error with item ID context.
    pub fn not_found(id: impl ToString) -> Self {
        Self::NotFound { id: id.to_string() }
    }

    /// Create a Database error with operation and message context.
    pub fn database(operation: &'static str, message: impl ToString) -> Self {
        Self::Database {
            operation,
            message: message.to_string(),
        }
    }

    /// Create a Conflict error with message context.
    pub fn conflict(message: impl ToString) -> Self {
        Self::Conflict {
            message: message.to_string(),
        }
    }

    /// Create a Validation error.
    pub fn validation(message: impl ToString) -> Self {
        Self::Validation(message.to_string())
    }

    /// Create a Serialization error.
    pub fn serialization(message: impl ToString) -> Self {
        Self::Serialization(message.to_string())
    }

    /// Check if this is a NotFound error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Check if this is a Conflict error.
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// Get the operation name if this is a Database error.
    pub fn operation(&self) -> Option<&'static str> {
        match self {
            Self::Database { operation, .. } => Some(operation),
            _ => None,
        }
    }
}

/// Errors from session/connection operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionError {
    #[error("Connection not found: {0}")]
    NotFound(String),
    #[error("DM already connected to this world")]
    DmAlreadyConnected,
    #[error("Not authorized")]
    Unauthorized,
}

/// Errors specific to joining a world.
#[derive(Debug, Clone, thiserror::Error)]
pub enum JoinWorldError {
    #[error("DM already connected: {existing_user_id}")]
    DmAlreadyConnected { existing_user_id: String },
    #[error("Player character not found: pc_id={pc_id} in world_id={world_id}")]
    PcNotFound { world_id: String, pc_id: String },
    #[error("Unknown error")]
    Unknown,
}
