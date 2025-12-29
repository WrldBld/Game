//! API Port - Outbound port for Engine HTTP API operations
//!
//! This port abstracts HTTP communication with the Engine backend,
//! allowing application services to interact with the API without
//! depending on concrete HTTP client implementations.

use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

/// Error type for API operations
#[derive(Debug, Clone)]
pub enum ApiError {
    /// Network request failed
    RequestFailed(String),
    /// Server returned non-success status
    HttpError(u16, String),
    /// Failed to parse response JSON
    ParseError(String),
    /// Failed to serialize request body
    SerializeError(String),
    /// Resource not found (404)
    NotFound(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            ApiError::HttpError(status, msg) => write!(f, "HTTP {}: {}", status, msg),
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ApiError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<ApiError> for String {
    fn from(err: ApiError) -> String {
        err.to_string()
    }
}

/// API Port trait for Engine HTTP operations
///
/// This trait provides a platform-agnostic interface for making HTTP requests
/// to the Engine API. Implementations handle the platform-specific details
/// (WASM vs desktop) while application services work with this abstraction.
///
/// # Platform Considerations
///
/// Due to differences between WASM and native platforms:
/// - WASM implementation uses `gloo_net` (sync-looking but actually async via JS)
/// - Desktop implementation uses `reqwest` with `async/await`
///
/// The trait methods are async to accommodate both platforms.
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait ApiPort {
    /// GET request that returns deserialized JSON
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError>;

    /// GET request that returns Option<T> - returns None for 404
    async fn get_optional<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, ApiError>;

    /// POST request with JSON body, returns deserialized JSON response
    async fn post<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError>;

    /// POST request with JSON body, no response body expected
    async fn post_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError>;

    /// POST request without body (for toggle endpoints)
    async fn post_empty(&self, path: &str) -> Result<(), ApiError>;

    /// PUT request with JSON body, returns deserialized JSON response
    async fn put<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError>;

    /// PUT request with JSON body, no response body expected
    async fn put_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError>;

    /// PUT request without body (for toggle endpoints)
    async fn put_empty(&self, path: &str) -> Result<(), ApiError>;

    /// PUT request without body, returns deserialized JSON response
    async fn put_empty_with_response<T: DeserializeOwned>(&self, path: &str)
        -> Result<T, ApiError>;

    /// PATCH request with JSON body, returns deserialized JSON response
    async fn patch<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError>;

    /// DELETE request
    async fn delete(&self, path: &str) -> Result<(), ApiError>;
}
