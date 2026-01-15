//! TTL-based cache for ephemeral state.
//!
//! Provides a thread-safe cache with automatic expiration to prevent unbounded
//! memory growth in long-running server processes.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// A thread-safe cache with time-to-live expiration.
///
/// Entries are automatically considered expired after the configured TTL,
/// but are not removed until `cleanup_expired()` is called.
pub struct TtlCache<K, V> {
    entries: RwLock<HashMap<K, TtlEntry<V>>>,
    ttl: Duration,
}

struct TtlEntry<V> {
    value: V,
    inserted_at: Instant,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Create a new cache with the specified TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Insert a value, replacing any existing entry and resetting the TTL.
    pub async fn insert(&self, key: K, value: V) {
        let entry = TtlEntry {
            value,
            inserted_at: Instant::now(),
        };
        self.entries.write().await.insert(key, entry);
    }

    /// Insert a value with an explicit timestamp (tests only).
    #[cfg(test)]
    pub async fn insert_at(&self, key: K, value: V, inserted_at: Instant) {
        let entry = TtlEntry { value, inserted_at };
        self.entries.write().await.insert(key, entry);
    }

    /// Get a value if it exists and hasn't expired.
    pub async fn get(&self, key: &K) -> Option<V> {
        let guard = self.entries.read().await;
        guard.get(key).and_then(|entry| {
            if entry.inserted_at.elapsed() < self.ttl {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    /// Remove and return a value if it exists (regardless of expiration).
    pub async fn remove(&self, key: &K) -> Option<V> {
        self.entries.write().await.remove(key).map(|e| e.value)
    }

    /// Check if a key exists and hasn't expired.
    pub async fn contains(&self, key: &K) -> bool {
        let guard = self.entries.read().await;
        guard
            .get(key)
            .map_or(false, |entry| entry.inserted_at.elapsed() < self.ttl)
    }

    /// Remove all expired entries and return the count of removed entries.
    pub async fn cleanup_expired(&self) -> usize {
        let mut guard = self.entries.write().await;
        let before_count = guard.len();
        guard.retain(|_, entry| entry.inserted_at.elapsed() < self.ttl);
        before_count - guard.len()
    }

    /// Get the current number of entries (including expired ones not yet cleaned).
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    /// Get all non-expired entries as a vec of (key, value) pairs.
    pub async fn entries(&self) -> Vec<(K, V)> {
        let guard = self.entries.read().await;
        guard
            .iter()
            .filter(|(_, entry)| entry.inserted_at.elapsed() < self.ttl)
            .map(|(k, entry)| (k.clone(), entry.value.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn insert_and_get() {
        let cache: TtlCache<String, i32> = TtlCache::new(Duration::from_secs(60));
        cache.insert("key".to_string(), 42).await;
        assert_eq!(cache.get(&"key".to_string()).await, Some(42));
    }

    #[tokio::test]
    async fn get_returns_none_for_missing() {
        let cache: TtlCache<String, i32> = TtlCache::new(Duration::from_secs(60));
        assert_eq!(cache.get(&"missing".to_string()).await, None);
    }

    #[tokio::test]
    async fn remove_returns_value() {
        let cache: TtlCache<String, i32> = TtlCache::new(Duration::from_secs(60));
        cache.insert("key".to_string(), 42).await;
        assert_eq!(cache.remove(&"key".to_string()).await, Some(42));
        assert_eq!(cache.get(&"key".to_string()).await, None);
    }

    #[tokio::test]
    async fn expired_entries_not_returned() {
        let ttl = Duration::from_millis(10);
        let cache: TtlCache<String, i32> = TtlCache::new(ttl);
        let expired_at = Instant::now() - (ttl + Duration::from_millis(1));
        cache.insert_at("key".to_string(), 42, expired_at).await;

        assert_eq!(cache.get(&"key".to_string()).await, None);
    }

    #[tokio::test]
    async fn cleanup_removes_expired() {
        let ttl = Duration::from_millis(10);
        let cache: TtlCache<String, i32> = TtlCache::new(ttl);
        let expired_at = Instant::now() - (ttl + Duration::from_millis(1));
        cache.insert_at("key1".to_string(), 1, expired_at).await;
        cache.insert_at("key2".to_string(), 2, expired_at).await;
        cache.insert("key3".to_string(), 3).await;

        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);
        assert_eq!(cache.len().await, 1);
        assert_eq!(cache.get(&"key3".to_string()).await, Some(3));
    }

    #[tokio::test]
    async fn contains_respects_ttl() {
        let ttl = Duration::from_millis(10);
        let cache: TtlCache<String, i32> = TtlCache::new(ttl);
        cache.insert("key".to_string(), 42).await;

        assert!(cache.contains(&"key".to_string()).await);

        let expired_at = Instant::now() - (ttl + Duration::from_millis(1));
        cache.insert_at("key".to_string(), 42, expired_at).await;

        assert!(!cache.contains(&"key".to_string()).await);
    }

    #[tokio::test]
    async fn entries_filters_expired() {
        let ttl = Duration::from_millis(10);
        let cache: TtlCache<String, i32> = TtlCache::new(ttl);
        let expired_at = Instant::now() - (ttl + Duration::from_millis(1));
        cache.insert_at("old".to_string(), 1, expired_at).await;
        cache.insert("new".to_string(), 2).await;

        let entries = cache.entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], ("new".to_string(), 2));
    }
}
