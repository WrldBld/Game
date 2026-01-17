//! Time suggestion storage wrapper.

use std::sync::Arc;

use crate::api::websocket::TimeSuggestionStoreImpl;
use crate::infrastructure::ports::TimeSuggestion;

/// Time suggestion store wrapper for use cases.
///
/// Note: This was renamed from `TimeSuggestionStore` to `TimeSuggestionRepository`
/// to avoid confusion with `TimeSuggestionStoreImpl`.
pub struct TimeSuggestionRepository {
    store: Arc<TimeSuggestionStoreImpl>,
}

impl TimeSuggestionRepository {
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

// Keep the old name as a type alias for backwards compatibility
pub type TimeSuggestionStore = TimeSuggestionRepository;
