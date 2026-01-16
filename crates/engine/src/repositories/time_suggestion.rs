//! Time suggestion storage wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::{
    TimeSuggestion, TimeSuggestionStore as TimeSuggestionStorePort,
};

/// Time suggestion store wrapper for use cases.
pub struct TimeSuggestionStore {
    store: Arc<dyn TimeSuggestionStorePort>,
}

impl TimeSuggestionStore {
    pub fn new(store: Arc<dyn TimeSuggestionStorePort>) -> Self {
        Self { store }
    }

    pub async fn insert(&self, key: uuid::Uuid, suggestion: TimeSuggestion) {
        self.store.insert(key, suggestion).await;
    }

    pub async fn remove(&self, key: uuid::Uuid) -> Option<TimeSuggestion> {
        self.store.remove(key).await
    }
}
