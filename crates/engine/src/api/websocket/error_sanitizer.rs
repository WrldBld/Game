// Error sanitizer - constants and helpers for future use
#![allow(dead_code)]

//! Error sanitization for client-facing messages.
//!
//! Prevents leaking internal details (paths, DB errors, stack traces) to clients.

use crate::infrastructure::correlation::CorrelationId;
use crate::use_cases::visual_state::{CatalogError, GenerationError};
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

/// Sanitize a CatalogError for client consumption.
///
/// Maps CatalogError variants to stable, client-friendly messages:
/// - NotFound variants -> "The requested resource was not found"
/// - Validation variants -> "Invalid request parameters"
/// - All others -> "An internal error occurred"
///
/// Logs the full error server-side with context.
pub fn sanitize_catalog_error(error: &CatalogError, operation: &str) -> String {
    use CatalogError;

    match error {
        // NotFound variants - client-friendly not found message
        CatalogError::LocationNotFound(_)
        | CatalogError::RegionNotFound(_)
        | CatalogError::LocationStateNotFound(_)
        | CatalogError::RegionStateNotFound(_) => {
            tracing::warn!(
                error = %error,
                operation = operation,
                "Catalog not found error"
            );
            messages::NOT_FOUND.to_string()
        }

        // Validation variants - client-friendly validation message
        CatalogError::Validation(_)
        | CatalogError::NameEmpty
        | CatalogError::NameTooLong
        | CatalogError::DescriptionTooLong
        | CatalogError::Domain(_) => {
            tracing::warn!(
                error = %error,
                operation = operation,
                "Catalog validation error"
            );
            messages::INVALID_REQUEST.to_string()
        }

        // Internal errors - log full details, return generic message
        _ => {
            tracing::error!(
                error = %error,
                operation = operation,
                "Catalog internal error"
            );
            messages::INTERNAL_ERROR.to_string()
        }
    }
}

/// Sanitize a GenerationError for client consumption.
///
/// Maps GenerationError variants to stable, client-friendly messages:
/// - Validation -> specific validation message (safe to expose)
/// - Domain -> mapped to stable messages per variant (no raw error strings)
/// - LocationNotFound/RegionNotFound -> clear "not found" message
/// - Repo/Queue -> sanitized generic internal error (no details leaked)
///
/// Logs the full error server-side with context.
pub fn sanitize_generation_error(error: &GenerationError, operation: &str) -> String {
    use wrldbldr_domain::DomainError;
    use GenerationError;

    match error {
        // Validation errors - expose the specific validation message (safe for clients)
        GenerationError::Validation(msg) => {
            tracing::warn!(
                error = %error,
                operation = operation,
                "Generation validation error"
            );
            msg.clone()
        }

        // Domain errors - map variants to stable client-safe messages
        GenerationError::Domain(domain_err) => {
            match domain_err {
                // Validation messages are safe to expose
                DomainError::Validation(msg) => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        "Generation domain validation error"
                    );
                    msg.clone()
                }

                // Invalid ID - stable message
                DomainError::InvalidId(_) => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        "Generation domain invalid ID error"
                    );
                    "Invalid ID".to_string()
                }

                // Not found - stable message
                DomainError::NotFound { entity_type, id } => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        entity_type = entity_type,
                        entity_id = %id,
                        "Generation domain not found error"
                    );
                    "Entity not found".to_string()
                }

                // Constraint violation - generic invalid input
                DomainError::Constraint(_) => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        "Generation domain constraint error"
                    );
                    "Invalid input".to_string()
                }

                // Parse error - generic invalid input
                DomainError::Parse(_) => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        "Generation domain parse error"
                    );
                    "Invalid input".to_string()
                }

                // Invalid state transition - generic invalid input
                DomainError::InvalidStateTransition(_) => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        "Generation domain state transition error"
                    );
                    "Invalid input".to_string()
                }

                // Container full - specific capacity message
                DomainError::ContainerFull { current, max } => {
                    tracing::warn!(
                        error = %error,
                        operation = operation,
                        current = current,
                        max = max,
                        "Generation domain container full error"
                    );
                    "Capacity reached".to_string()
                }
            }
        }

        // Not found errors - clear, specific messages
        GenerationError::LocationNotFound(_) => {
            tracing::warn!(
                error = %error,
                operation = operation,
                "Generation location not found"
            );
            "Location not found".to_string()
        }
        GenerationError::RegionNotFound(_) => {
            tracing::warn!(
                error = %error,
                operation = operation,
                "Generation region not found"
            );
            "Region not found".to_string()
        }

        // Repository/Queue errors - internal details must not leak
        GenerationError::Repo(_) | GenerationError::Queue(_) => {
            tracing::error!(
                error = %error,
                operation = operation,
                "Generation internal error"
            );
            messages::INTERNAL_ERROR.to_string()
        }
    }
}
