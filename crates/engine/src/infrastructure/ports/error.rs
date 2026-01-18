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
    #[error("Queue error: {0}")]
    Error(String),
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
