# ADR-007: TtlCache for Ephemeral State

## Status

Accepted

## Date

2026-01-13

## Context

WrldBldr needs to manage ephemeral state that:
1. Lives only in memory (not persisted to database)
2. Has a natural expiration (e.g., pending approvals, staging requests)
3. Must be automatically cleaned up to prevent memory leaks
4. Needs concurrent access from async tasks

Examples of ephemeral state:
- **Pending staging requests**: DM approval for NPC presence (1 hour TTL)
- **Time suggestions**: Pending time advancement requests (30 minute TTL)
- **Generation read state**: Tracking which suggestions user has seen (5 minute TTL)

Previous approach using `HashMap` had issues:
- No automatic expiration
- Required manual cleanup calls
- Risk of unbounded memory growth

## Decision

Implement a **generic `TtlCache<K, V>` wrapper** around `HashMap`:

```rust
pub struct TtlCache<K, V> {
    data: tokio::sync::RwLock<HashMap<K, TtlEntry<V>>>,
    ttl: Duration,
}

struct TtlEntry<V> {
    value: V,
    expires_at: Instant,
}
```

Features:
- **Configurable TTL**: Set at cache creation
- **Lazy expiration**: Expired entries filtered on read
- **Explicit cleanup**: `cleanup_expired()` for periodic maintenance
- **Async-safe**: Uses `tokio::sync::RwLock`

## Consequences

### Positive

- Automatic memory management through TTL
- No unbounded growth
- Simple API (insert, get, remove)
- Thread-safe for async Tokio tasks
- Configurable per use case

### Negative

- Memory not immediately freed on expiration (lazy cleanup)
- Requires periodic `cleanup_expired()` calls for best memory behavior
- Not distributed (single-node only)

### Neutral

- TTL is fixed at cache creation
- No persistence across restarts

## Implementation

```rust
// Create cache with 1 hour TTL
let staging_cache = TtlCache::new(Duration::from_secs(60 * 60));

// Insert
staging_cache.insert(request_id, staging_request).await;

// Get (returns None if expired)
let request = staging_cache.get(&request_id).await;

// Periodic cleanup
let removed = staging_cache.cleanup_expired().await;
```

Usage in `WsState`:
```rust
pub struct WsState {
    pub pending_staging_requests: PendingStagingStoreImpl,  // 1 hour TTL
    pub pending_time_suggestions: TimeSuggestionStoreImpl,  // 30 minute TTL
    pub generation_read_state: GenerationStateStoreImpl,    // 5 minute TTL
}

impl WsState {
    pub async fn cleanup_expired(&self) -> usize {
        let staging = self.pending_staging_requests.cleanup_expired().await;
        let time = self.pending_time_suggestions.cleanup_expired().await;
        let gen = self.generation_read_state.cleanup_expired().await;
        staging + time + gen
    }
}
```

## Alternatives Considered

### 1. External Cache (Redis)

Use Redis for ephemeral state with TTL.

**Rejected:** Adds operational dependency for data that doesn't need to survive restarts. In-memory is simpler and faster.

### 2. Database with Cleanup Job

Store in SQLite/Neo4j with periodic deletion.

**Rejected:** Unnecessary I/O for truly ephemeral data. Database is for persistent state.

### 3. tokio-util's `TtlHashMap`

Use existing crate for TTL cache.

**Rejected:** Would add dependency. Simple wrapper is sufficient for our needs.

### 4. moka Cache

High-performance concurrent cache with TTL.

**Rejected:** Heavyweight for our simple use case. Could consider if performance becomes an issue.

## Memory Safety Notes

To prevent memory leaks:
1. **Set appropriate TTLs**: Match business requirements (staging: 1hr, suggestions: 30min)
2. **Call cleanup periodically**: WebSocket handler calls `cleanup_expired()` on schedule
3. **Monitor cache sizes**: Could add metrics for cache entry counts

The combination of lazy expiration (on read) and periodic cleanup ensures memory stays bounded while keeping the implementation simple.

## References

- `crates/engine/src/infrastructure/cache.rs` - TtlCache implementation
- `crates/engine/src/api/websocket/mod.rs` - WsState and store implementations
