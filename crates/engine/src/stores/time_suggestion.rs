//! Time suggestion storage wrapper.

use std::sync::Arc;

use wrldbldr_domain::TimeSuggestionId;

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

    pub async fn insert(&self, suggestion: TimeSuggestion) {
        self.store.insert(suggestion.id.to_uuid(), suggestion).await;
    }

    pub async fn remove(&self, key: TimeSuggestionId) -> Option<TimeSuggestion> {
        self.store.remove(&key.to_uuid()).await
    }
}
