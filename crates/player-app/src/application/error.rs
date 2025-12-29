//! Service layer error types
//!
//! This module defines errors that can occur in the application service layer,
//! abstracting over transport-specific errors (REST/WebSocket).

use serde::de::DeserializeOwned;
use wrldbldr_protocol::{ErrorCode, RequestError, ResponseResult};

/// Errors that can occur in service operations
#[derive(Debug, Clone)]
pub enum ServiceError {
    /// Request failed to send or was cancelled
    Request(RequestError),
    /// Server returned an error response
    ServerError { code: ErrorCode, message: String },
    /// Response was empty when data was expected
    EmptyResponse,
    /// Failed to parse response data
    ParseError(String),
    /// Not connected to server
    NotConnected,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::Request(e) => write!(f, "Request error: {}", e),
            ServiceError::ServerError { code, message } => {
                write!(f, "Server error ({:?}): {}", code, message)
            }
            ServiceError::EmptyResponse => write!(f, "Server returned empty response"),
            ServiceError::ParseError(msg) => write!(f, "Failed to parse response: {}", msg),
            ServiceError::NotConnected => write!(f, "Not connected to server"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<RequestError> for ServiceError {
    fn from(e: RequestError) -> Self {
        ServiceError::Request(e)
    }
}

impl ServiceError {
    /// Check if this is a "not found" error
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            ServiceError::ServerError {
                code: ErrorCode::NotFound,
                ..
            }
        )
    }

    /// Check if this is an authorization error
    pub fn is_unauthorized(&self) -> bool {
        matches!(
            self,
            ServiceError::ServerError {
                code: ErrorCode::Unauthorized,
                ..
            }
        )
    }
}

/// Helper trait for parsing ResponseResult into typed data
pub trait ParseResponse {
    /// Parse a ResponseResult into the expected type
    fn parse<T: DeserializeOwned>(self) -> Result<T, ServiceError>;

    /// Parse a ResponseResult that may return no data (for delete operations)
    fn parse_empty(self) -> Result<(), ServiceError>;

    /// Parse a ResponseResult that may return Option<T> (for get operations that may 404)
    fn parse_optional<T: DeserializeOwned>(self) -> Result<Option<T>, ServiceError>;
}

impl ParseResponse for ResponseResult {
    fn parse<T: DeserializeOwned>(self) -> Result<T, ServiceError> {
        match self {
            ResponseResult::Success { data } => {
                let data = data.ok_or(ServiceError::EmptyResponse)?;
                serde_json::from_value(data).map_err(|e| ServiceError::ParseError(e.to_string()))
            }
            ResponseResult::Error { code, message, .. } => {
                Err(ServiceError::ServerError { code, message })
            }
            ResponseResult::Unknown => Err(ServiceError::ServerError {
                code: ErrorCode::InternalError,
                message: "Unknown response type".to_string(),
            }),
        }
    }

    fn parse_empty(self) -> Result<(), ServiceError> {
        match self {
            ResponseResult::Success { .. } => Ok(()),
            ResponseResult::Error { code, message, .. } => {
                Err(ServiceError::ServerError { code, message })
            }
            ResponseResult::Unknown => Err(ServiceError::ServerError {
                code: ErrorCode::InternalError,
                message: "Unknown response type".to_string(),
            }),
        }
    }

    fn parse_optional<T: DeserializeOwned>(self) -> Result<Option<T>, ServiceError> {
        match self {
            ResponseResult::Success { data: None } => Ok(None),
            ResponseResult::Success { data: Some(data) } => serde_json::from_value(data)
                .map(Some)
                .map_err(|e| ServiceError::ParseError(e.to_string())),
            ResponseResult::Error {
                code: ErrorCode::NotFound,
                ..
            } => Ok(None),
            ResponseResult::Error { code, message, .. } => {
                Err(ServiceError::ServerError { code, message })
            }
            ResponseResult::Unknown => Err(ServiceError::ServerError {
                code: ErrorCode::InternalError,
                message: "Unknown response type".to_string(),
            }),
        }
    }
}

/// Default request timeout in milliseconds (2 minutes)
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 120_000;

/// Get the request timeout from environment variable or use default
pub fn get_request_timeout_ms() -> u64 {
    std::env::var("WRLDBLDR_REQUEST_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS)
}
