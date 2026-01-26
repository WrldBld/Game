// Error sanitizer - constants and helpers for future use
#![allow(dead_code)]

//! Error sanitization for client-facing messages.
//!
//! Prevents leaking internal details (paths, DB errors, stack traces) to clients.

use crate::infrastructure::correlation::CorrelationId;
use tracing;

/// Sanitize an error for client consumption.
///
/// Logs the full error server-side, returns generic message for client.
pub fn sanitize_error<E: std::fmt::Display>(error: &E, context: &str) -> String {
    // Log full error server-side
    tracing::error!(
        error = %error,
        context = context,
        "Internal error occurred"
    );

    // Return generic message to client
    format!("An error occurred while {}", context)
}

/// Sanitize a repository error.
pub fn sanitize_repo_error<E: std::fmt::Display>(error: &E, operation: &str) -> String {
    tracing::error!(
        error = %error,
        operation = operation,
        "Repository error"
    );

    format!("Failed to {} - please try again", operation)
}

/// Sanitize a repository error with correlation ID.
///
/// Logs the full error with correlation context server-side, returns generic message for client.
pub fn sanitize_repo_error_with_cid<E: std::fmt::Display>(
    error: &E,
    operation: &str,
    correlation_id: &CorrelationId,
) -> String {
    tracing::error!(
        error = %error,
        operation = operation,
        correlation_id = %correlation_id,
        correlation_id_short = %correlation_id.short(),
        "Repository error with correlation"
    );

    format!("Failed to {} - please try again", operation)
}

/// Common error messages for client consumption.
pub mod messages {
    pub const INTERNAL_ERROR: &str = "An internal error occurred";
    pub const NOT_FOUND: &str = "The requested resource was not found";
    pub const INVALID_REQUEST: &str = "Invalid request parameters";
    pub const UNAUTHORIZED: &str = "You are not authorized for this action";
}
