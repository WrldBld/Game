//! Port traits for staging storage.

use uuid::Uuid;

use super::types::PendingStagingRequest;
use crate::use_cases::time::TimeSuggestion;

/// Port for storing pending staging requests.
///
/// Abstracts the storage mechanism so use cases don't depend on tokio::sync::RwLock.
#[async_trait::async_trait]
pub trait PendingStagingStore: Send + Sync {
    /// Insert a pending staging request.
    async fn insert(&self, key: String, request: PendingStagingRequest);

    /// Get a pending staging request by key.
    async fn get(&self, key: &str) -> Option<PendingStagingRequest>;

    /// Remove a pending staging request by key.
    async fn remove(&self, key: &str) -> Option<PendingStagingRequest>;
}

/// Port for storing time suggestions.
///
/// Abstracts the storage mechanism so use cases don't depend on tokio::sync::RwLock.
#[async_trait::async_trait]
pub trait TimeSuggestionStore: Send + Sync {
    /// Insert a time suggestion.
    async fn insert(&self, key: Uuid, suggestion: TimeSuggestion);

    /// Remove a time suggestion by key.
    async fn remove(&self, key: Uuid) -> Option<TimeSuggestion>;
}
