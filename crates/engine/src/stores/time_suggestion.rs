//! Time suggestion storage for runtime state.
//!
//! TTL-based cache for pending time suggestions awaiting DM approval.
//! Suggestions expire after 30 minutes if not resolved.

use uuid::Uuid;

use crate::infrastructure::cache::TtlCache;
use crate::infrastructure::ports::TimeSuggestion;

const TIME_SUGGESTION_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// TTL-based store for time suggestions (30 minute TTL).
pub struct TimeSuggestionStore {
    inner: TtlCache<Uuid, TimeSuggestion>,
}

impl TimeSuggestionStore {
    pub fn new() -> Self {
        Self {
            inner: TtlCache::new(TIME_SUGGESTION_TTL),
        }
    }

    /// Insert a time suggestion.
    pub async fn insert(&self, key: Uuid, suggestion: TimeSuggestion) {
        self.inner.insert(key, suggestion).await;
    }

    /// Get a time suggestion by key.
    pub async fn get(&self, key: &Uuid) -> Option<TimeSuggestion> {
        self.inner.get(key).await
    }

    /// Remove and return a time suggestion.
    pub async fn remove(&self, key: &Uuid) -> Option<TimeSuggestion> {
        self.inner.remove(key).await
    }

    /// Remove all suggestions for a given PC.
    /// This prevents unbounded growth when a player performs multiple actions
    /// before the DM resolves the first suggestion.
    pub async fn remove_for_pc(&self, pc_id: wrldbldr_domain::PlayerCharacterId) {
        let entries = self.inner.entries().await;
        for (key, suggestion) in entries {
            if suggestion.pc_id == pc_id {
                self.inner.remove(&key).await;
            }
        }
    }

    /// Remove expired entries and return count.
    pub async fn cleanup_expired(&self) -> usize {
        self.inner.cleanup_expired().await
    }
}

impl Default for TimeSuggestionStore {
    fn default() -> Self {
        Self::new()
    }
}
