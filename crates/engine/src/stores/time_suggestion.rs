//! Time suggestion storage wrapper.

use std::sync::Arc;

use crate::api::websocket::TimeSuggestionStoreImpl;
use crate::infrastructure::ports::TimeSuggestion;

/// Time suggestion store wrapper for use cases.
pub struct TimeSuggestionStore {
    store: Arc<TimeSuggestionStoreImpl>,
}

impl TimeSuggestionStore {
    pub fn new(store: Arc<TimeSuggestionStoreImpl>) -> Self {
        Self { store }
    }

    pub async fn insert(&self, key: uuid::Uuid, suggestion: TimeSuggestion) {
        self.store.insert(key, suggestion).await;
    }

    pub async fn remove(&self, key: uuid::Uuid) -> Option<TimeSuggestion> {
        self.store.remove(&key).await
    }
}
