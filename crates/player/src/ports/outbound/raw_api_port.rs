//! Raw API Port - Object-safe HTTP boundary
//!
//! The existing `ApiPort` trait is generic over response/request types which makes it
//! not object-safe. The UI/composition root needs an object-safe abstraction that can
//! be stored behind `Arc<dyn ...>`.
//!
//! `RawApiPort` is the object-safe boundary implemented by adapters.
//! The application layer provides a typed wrapper that implements `ApiPort` on top.

use serde_json::Value;

use super::ApiError;

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait RawApiPort: Send + Sync {
    async fn get_json(&self, path: &str) -> Result<Value, ApiError>;

    async fn get_optional_json(&self, path: &str) -> Result<Option<Value>, ApiError>;

    async fn post_json(&self, path: &str, body: &Value) -> Result<Value, ApiError>;

    async fn post_no_response_json(&self, path: &str, body: &Value) -> Result<(), ApiError>;

    async fn post_empty(&self, path: &str) -> Result<(), ApiError>;

    async fn put_json(&self, path: &str, body: &Value) -> Result<Value, ApiError>;

    async fn put_no_response_json(&self, path: &str, body: &Value) -> Result<(), ApiError>;

    async fn put_empty(&self, path: &str) -> Result<(), ApiError>;

    async fn put_empty_with_response_json(&self, path: &str) -> Result<Value, ApiError>;

    async fn patch_json(&self, path: &str, body: &Value) -> Result<Value, ApiError>;

    async fn delete(&self, path: &str) -> Result<(), ApiError>;
}
