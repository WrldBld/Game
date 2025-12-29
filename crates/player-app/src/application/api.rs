//! Typed API wrapper for application services.
//!
//! Application services were historically generic over an `ApiPort` trait that is
//! not object-safe (generic methods). For the split-crate setup we want the
//! composition root (player binary) to store an object-safe port implementation
//! (so UI and services don't depend on adapter types).
//!
//! `Api` wraps an `Arc<dyn RawApiPort>` and implements the typed `ApiPort`
//! interface via serde_json conversions.

use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

use wrldbldr_player_ports::outbound::{ApiError, RawApiPort};

use wrldbldr_player_ports::outbound::ApiPort;

#[derive(Clone)]
pub struct Api {
    raw: Arc<dyn RawApiPort>,
}

impl Api {
    pub fn new(raw: Arc<dyn RawApiPort>) -> Self {
        Self { raw }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl ApiPort for Api {
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
        let body_value =
            serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        let value = self.raw.post_json(path, &body_value).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn post_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let body_value =
            serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
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
        let body_value =
            serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        let value = self.raw.put_json(path, &body_value).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn put_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let body_value =
            serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        self.raw.put_no_response_json(path, &body_value).await
    }

    async fn put_empty(&self, path: &str) -> Result<(), ApiError> {
        self.raw.put_empty(path).await
    }

    async fn put_empty_with_response<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ApiError> {
        let value = self.raw.put_empty_with_response_json(path).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn patch<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let body_value =
            serde_json::to_value(body).map_err(|e| ApiError::SerializeError(e.to_string()))?;
        let value = self.raw.patch_json(path, &body_value).await?;
        serde_json::from_value(value).map_err(|e| ApiError::ParseError(e.to_string()))
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        self.raw.delete(path).await
    }
}
