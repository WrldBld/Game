//! Object-safe `ApiPort` adapter
//!
//! The `ApiPort` trait is generic over request/response types and is therefore
//! not object-safe. The UI/composition root can store a `RawApiPort` as a trait
//! object and then wrap it with `ApiPortObject`, which implements `ApiPort`
//! by doing serde_json conversions.

use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

use wrldbldr_player_ports::outbound::{ApiError, RawApiPort};

use super::ApiPort;

#[derive(Clone)]
pub struct ApiPortObject {
    raw: Arc<dyn RawApiPort>,
}

impl ApiPortObject {
    pub fn new(raw: Arc<dyn RawApiPort>) -> Self {
        Self { raw }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl ApiPort for ApiPortObject {
    fn as_raw(&self) -> Arc<dyn RawApiPort> {
        Arc::clone(&self.raw)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let value = self.raw.get_json(path).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn get_optional<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, ApiError> {
        let maybe_value = self.raw.get_optional_json(path).await?;
        match maybe_value {
            None => Ok(None),
            Some(value) => serde_json::from_value(value)
                .map(Some)
                .map_err(|e| ApiError::ParseError(e.to_string())),
        }
    }

    async fn post<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let body_value = serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        let value = self.raw.post_json(path, &body_value).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn post_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let body_value = serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        self.raw.post_no_response_json(path, &body_value).await
    }

    async fn post_empty(&self, path: &str) -> Result<(), ApiError> {
        self.raw.post_empty(path).await
    }

    async fn put<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let body_value = serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        let value = self.raw.put_json(path, &body_value).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn put_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let body_value = serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        self.raw.put_no_response_json(path, &body_value).await
    }

    async fn put_empty(&self, path: &str) -> Result<(), ApiError> {
        self.raw.put_empty(path).await
    }

    async fn put_empty_with_response<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let value = self.raw.put_empty_with_response_json(path).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn patch<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let body_value = serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        let value = self.raw.patch_json(path, &body_value).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        self.raw.delete(path).await
    }
}
