//! Unified HTTP client for Engine API requests
//!
//! This module provides a platform-agnostic HTTP client that abstracts away
//! the differences between WASM (gloo_net) and desktop (reqwest) environments.
//!
//! # Usage
//!
//! ```ignore
//! use crate::infrastructure::http_client::{HttpClient, ApiError};
//!
//! // GET request
//! let worlds: Vec<WorldData> = HttpClient::get("/api/worlds").await?;
//!
//! // POST request with JSON body
//! let created: CharacterData = HttpClient::post("/api/characters", &new_character).await?;
//!
//! // PUT request
//! HttpClient::put_no_response("/api/characters/123", &update).await?;
//!
//! // DELETE request
//! HttpClient::delete("/api/characters/123").await?;
//! ```

use serde::{de::DeserializeOwned, Serialize};

use super::api::get_engine_url;
use wrldbldr_player_ports::outbound::ApiError;

// Desktop: Use a shared static client for connection reuse and proper timeouts
#[cfg(not(target_arch = "wasm32"))]
mod desktop_client {
    use std::sync::LazyLock;
    use std::time::Duration;
    
    /// Shared HTTP client with 30 second timeout for connection reuse
    pub static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    });
}

#[cfg(not(target_arch = "wasm32"))]
use desktop_client::CLIENT;

/// Unified HTTP client for Engine API
///
/// All methods take a path (e.g., "/api/worlds") and automatically
/// prepend the Engine base URL from configuration.
pub struct HttpClient;

impl HttpClient {
    /// Build full URL from API path
    fn build_url(path: &str) -> String {
        let base = get_engine_url();
        if path.starts_with('/') {
            format!("{}{}", base, path)
        } else {
            format!("{}/{}", base, path)
        }
    }

    /// GET request that returns deserialized JSON
    pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::get(&url);
            // Attach anonymous user header if available (WASM only)
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("GET {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .get(&url)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(status, format!("GET {} failed", path)))
            }
        }
    }

    /// POST request with JSON body, returns deserialized JSON response
    pub async fn post<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let body_str = serde_json::to_string(body)
                .map_err(|e| ApiError::SerializeError(e.to_string()))?;
            let mut request = Request::post(&url).header("Content-Type", "application/json");
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .body(body_str)
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("POST {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .post(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(status, format!("POST {} failed", path)))
            }
        }
    }

    /// POST request with JSON body, no response body expected
    pub async fn post_no_response<B: Serialize>(path: &str, body: &B) -> Result<(), ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let body_str = serde_json::to_string(body)
                .map_err(|e| ApiError::SerializeError(e.to_string()))?;
            let mut request = Request::post(&url).header("Content-Type", "application/json");
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .body(body_str)
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                Ok(())
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("POST {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .post(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                Ok(())
            } else {
                Err(ApiError::HttpError(status, format!("POST {} failed", path)))
            }
        }
    }

    /// POST request without body, no response expected (for toggle endpoints)
    pub async fn post_empty(path: &str) -> Result<(), ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::post(&url);
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request.send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                Ok(())
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("POST {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT.post(&url).send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                Ok(())
            } else {
                Err(ApiError::HttpError(status, format!("POST {} failed", path)))
            }
        }
    }

    /// PUT request with JSON body, returns deserialized JSON response
    pub async fn put<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let body_str = serde_json::to_string(body)
                .map_err(|e| ApiError::SerializeError(e.to_string()))?;

            let mut request = Request::put(&url).header("Content-Type", "application/json");
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .body(body_str)
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("PUT {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .put(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(status, format!("PUT {} failed", path)))
            }
        }
    }

    /// PUT request with JSON body, no response body expected
    pub async fn put_no_response<B: Serialize>(path: &str, body: &B) -> Result<(), ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let body_str = serde_json::to_string(body)
                .map_err(|e| ApiError::SerializeError(e.to_string()))?;

            let mut request = Request::put(&url).header("Content-Type", "application/json");
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .body(body_str)
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                Ok(())
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("PUT {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .put(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                Ok(())
            } else {
                Err(ApiError::HttpError(status, format!("PUT {} failed", path)))
            }
        }
    }

    /// PUT request without body (for toggle endpoints), no response expected
    pub async fn put_empty(path: &str) -> Result<(), ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::put(&url);
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                Ok(())
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("PUT {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .put(&url)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                Ok(())
            } else {
                Err(ApiError::HttpError(status, format!("PUT {} failed", path)))
            }
        }
    }

    /// PUT request without body, returns deserialized JSON response (for toggle endpoints that return state)
    pub async fn put_empty_with_response<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::put(&url);
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("PUT {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .put(&url)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(status, format!("PUT {} failed", path)))
            }
        }
    }

    /// PATCH request with JSON body
    pub async fn patch<T: DeserializeOwned, B: Serialize>(
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let url = Self::build_url(path);
        let json_body =
            serde_json::to_string(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::patch(&url).header("Content-Type", "application/json");
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .body(&json_body)
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("PATCH {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .patch(&url)
                .header("Content-Type", "application/json")
                .body(json_body)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))
            } else {
                Err(ApiError::HttpError(
                    status,
                    format!("PATCH {} failed", path),
                ))
            }
        }
    }

    /// DELETE request
    pub async fn delete(path: &str) -> Result<(), ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::delete(&url);
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.ok() {
                Ok(())
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("DELETE {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .delete(&url)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            let status = response.status().as_u16();
            if response.status().is_success() {
                Ok(())
            } else {
                Err(ApiError::HttpError(status, format!("DELETE {} failed", path)))
            }
        }
    }

    /// GET request that returns Option<T> - returns None for 404, Some(T) for success
    pub async fn get_optional<T: DeserializeOwned>(path: &str) -> Result<Option<T>, ApiError> {
        let url = Self::build_url(path);

        #[cfg(target_arch = "wasm32")]
        {
            use gloo_net::http::Request;

            let mut request = Request::get(&url);
            if let Some(user_id) =
                crate::infrastructure::storage::load(crate::infrastructure::storage::STORAGE_KEY_USER_ID)
            {
                request = request.header("X-User-Id", &user_id);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.status() == 404 {
                return Ok(None);
            }

            if response.ok() {
                let data = response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(Some(data))
            } else {
                Err(ApiError::HttpError(
                    response.status(),
                    format!("GET {} failed", path),
                ))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = CLIENT
                .get(&url)
                .send()
                .await
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }

            let status = response.status().as_u16();
            if response.status().is_success() {
                let data = response
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(Some(data))
            } else {
                Err(ApiError::HttpError(status, format!("GET {} failed", path)))
            }
        }
    }
}

// ============================================================================
// ApiPort Implementation
// ============================================================================

use wrldbldr_player_ports::outbound::{RawApiPort};
use wrldbldr_player_ports::outbound::ApiPort;

/// API adapter that implements the ApiPort trait
///
/// This adapter wraps the static HttpClient methods to provide an instance-based
/// API that can be injected into application services.
#[derive(Clone, Debug, Default)]
pub struct ApiAdapter;

impl ApiAdapter {
    /// Create a new API adapter
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl ApiPort for ApiAdapter {

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        HttpClient::get(path).await
    }

    async fn get_optional<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, ApiError> {
        HttpClient::get_optional(path).await
    }

    async fn post<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        HttpClient::post(path, body).await
    }

    async fn post_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        HttpClient::post_no_response(path, body).await
    }

    async fn post_empty(&self, path: &str) -> Result<(), ApiError> {
        HttpClient::post_empty(path).await
    }

    async fn put<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        HttpClient::put(path, body).await
    }

    async fn put_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        HttpClient::put_no_response(path, body).await
    }

    async fn put_empty(&self, path: &str) -> Result<(), ApiError> {
        HttpClient::put_empty(path).await
    }

    async fn put_empty_with_response<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ApiError> {
        HttpClient::put_empty_with_response(path).await
    }

    async fn patch<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        HttpClient::patch(path, body).await
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        HttpClient::delete(path).await
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl RawApiPort for ApiAdapter {
    async fn get_json(&self, path: &str) -> Result<serde_json::Value, ApiError> {
        self.get(path).await
    }

    async fn get_optional_json(
        &self,
        path: &str,
    ) -> Result<Option<serde_json::Value>, ApiError> {
        self.get_optional(path).await
    }

    async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        self.post(path, body).await
    }

    async fn post_no_response_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<(), ApiError> {
        self.post_no_response(path, body).await
    }

    async fn post_empty(&self, path: &str) -> Result<(), ApiError> {
        ApiPort::post_empty(self, path).await
    }

    async fn put_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        self.put(path, body).await
    }

    async fn put_no_response_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<(), ApiError> {
        self.put_no_response(path, body).await
    }

    async fn put_empty(&self, path: &str) -> Result<(), ApiError> {
        ApiPort::put_empty(self, path).await
    }

    async fn put_empty_with_response_json(
        &self,
        path: &str,
    ) -> Result<serde_json::Value, ApiError> {
        self.put_empty_with_response(path).await
    }

    async fn patch_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        self.patch(path, body).await
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        ApiPort::delete(self, path).await
    }
}
