# Queue System

## Overview

WrldBldr uses a queue-based architecture for processing player actions, DM decisions, LLM requests, and asset generation. This provides crash recovery, audit trails, and a foundation for scaling.

---

## Queue Types

| Queue                   | Purpose                                 | Persistence | Concurrency              |
| ----------------------- | --------------------------------------- | ----------- | ------------------------ |
| `PlayerActionQueue`     | Player actions awaiting processing      | SQLite      | Unlimited                |
| `DMActionQueue`         | DM actions awaiting processing          | SQLite      | Unlimited                |
| `LLMReasoningQueue`     | Ollama requests                         | SQLite      | Semaphore (configurable) |
| `AssetGenerationQueue`  | ComfyUI requests                        | SQLite      | 1 (sequential)           |
| `DMApprovalQueue`       | Decisions awaiting DM approval          | SQLite      | N/A (waiting)            |
| `ChallengeOutcomeQueue` | Challenge outcomes awaiting DM approval | SQLite      | N/A (waiting)            |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Queue Architecture                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   WebSocket Handler                                                         │
│        │                                                                    │
│        │ enqueue()                                                          │
│        ▼                                                                    │
│   ┌─────────────────┐                                                       │
│   │  Queue Service  │ ──────────▶ SQLite (persistence)                     │
│   └────────┬────────┘                                                       │
│            │                                                                │
│            │ Background Worker (tokio::spawn)                               │
│            ▼                                                                │
│   ┌─────────────────┐                                                       │
│   │    Processor    │                                                       │
│   │  - LLM calls    │                                                       │
│   │  - ComfyUI      │                                                       │
│   │  - Approvals    │                                                       │
│   └────────┬────────┘                                                       │
│            │                                                                │
│            │ Results                                                        │
│            ▼                                                                │
│   ┌─────────────────┐                                                       │
│   │ Event Publisher │ ──────────▶ WebSocket broadcast                      │
│   └─────────────────┘                                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Queue Port Interface

The core queue interface is defined in the engine crate’s ports module (`crates/engine/src/infrastructure/ports.rs`).

```rust
#[async_trait]
pub trait QueuePort<T>: Send + Sync
where
    T: Send + Sync + Clone,
{
    /// Add item to queue with given priority (higher = more urgent)
    async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError>;

    /// Get next item for processing (marks as Processing)
    async fn dequeue(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Peek at next item without removing or changing status
    async fn peek(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Mark item as completed
    async fn complete(&self, id: QueueItemId) -> Result<(), QueueError>;

    /// Mark item as failed (may retry based on attempts)
    async fn fail(&self, id: QueueItemId, error: &str) -> Result<(), QueueError>;

    /// Delay item for later processing
    async fn delay(&self, id: QueueItemId, until: DateTime<Utc>) -> Result<(), QueueError>;

    /// Get item by ID
    async fn get(&self, id: QueueItemId) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Get all items with given status
    async fn list_by_status(&self, status: QueueItemStatus) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get queue depth (pending items count)
    async fn depth(&self) -> Result<usize, QueueError>;

    /// Clear completed/failed items older than duration
    async fn cleanup(&self, older_than: Duration) -> Result<usize, QueueError>;
}
```

### Extended Ports

```rust
/// For approval queues with human-facing features
#[async_trait]
pub trait ApprovalQueuePort<T>: QueuePort<T> {
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<QueueItem<T>>, QueueError>;
    async fn get_history_by_world(&self, world_id: WorldId, limit: usize) -> Result<Vec<QueueItem<T>>, QueueError>;
    async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// For processing queues with concurrency control
#[async_trait]
pub trait ProcessingQueuePort<T>: QueuePort<T> {
    fn batch_size(&self) -> usize;
    async fn processing_count(&self) -> Result<usize, QueueError>;
    async fn has_capacity(&self) -> Result<bool, QueueError>;
}
```

---

## Queue Item States

```rust
pub enum QueueItemStatus {
    Pending,     // Waiting to be processed
    Processing,  // Currently being processed
    Completed,   // Successfully processed
    Failed,      // Processing failed
    Delayed,     // Scheduled for later processing
    Expired,     // TTL exceeded (for approval queues)
}
```

---

## Processing Flow

### Player Action Flow

```
1. Player sends PlayerAction via WebSocket
2. Handler creates PlayerActionData payload
3. Handler calls PlayerActionQueueService.enqueue()
4. Background worker polls queue
5. Worker builds LLM context, calls LLMQueueService.enqueue()
6. LLM worker processes, creates approval item
7. DMApprovalQueueService.enqueue() notifies DM
8. DM approves, DMActionQueueService processes
9. Results broadcast via WebSocket
```

### LLM Request Flow

```
1. LLMRequestData created with prompt, npc_id, context
2. Semaphore limits concurrent Ollama calls
3. Worker calls Ollama API
4. Response parsed for dialogue, tools, suggestions
5. ChallengeSuggestionInfo / NarrativeEventSuggestionInfo extracted
6. ApprovalItem created for DM review
7. Results stored, DM notified
```

### Asset Generation Flow

```
1. GenerationRequest created with entity, prompt, workflow
2. AssetGenerationQueueService.enqueue()
3. Worker checks ComfyUI health (circuit breaker)
4. Worker submits workflow to ComfyUI
5. Worker polls for completion
6. Generated images saved, GalleryAsset created
7. GenerationComplete broadcast
```

---

## Configuration

```rust
pub struct QueueConfig {
    pub backend: QueueBackend,           // Memory or SQLite
    pub sqlite_path: String,             // Database path
    pub llm_batch_size: u32,             // Concurrent LLM requests
    pub asset_batch_size: u32,           // Concurrent generations (usually 1)
    pub history_retention_hours: u32,    // Keep completed items
    pub approval_timeout_minutes: u32,   // Auto-expire approvals
}
```

Environment variables:

- `QUEUE_BACKEND`: `memory` or `sqlite` (default: sqlite)
- `QUEUE_SQLITE_PATH`: Database path
- `LLM_BATCH_SIZE`: Concurrent LLM requests (default: 2)

---

## World-Scoped Processing

All queue items carry `world_id` for:

1. **Isolation**: Items from different worlds don't interfere
2. **Fairness**: Round-robin processing across worlds
3. **Routing**: Results sent to correct world participants

---

## Health Monitoring

```bash
GET /api/health/queues
```

Response:

```json
{
  "player_action_queue": {
    "pending": 3,
    "processing": 1,
    "worlds": ["world-1", "world-2"]
  },
  "llm_queue": {
    "pending": 2,
    "processing": 1,
    "semaphore_available": 1
  },
  "asset_queue": {
    "pending": 5,
    "processing": 1,
    "comfyui_healthy": true
  },
  "dm_approval_queue": {
    "pending": 2,
    "oldest_age_seconds": 45
  }
}
```

---

## Cleanup Worker

Background task runs hourly:

1. Delete completed items older than `history_retention_hours`
2. Expire approval items older than `approval_timeout_minutes`
3. Mark stale processing items as failed

---

## Crash Recovery

SQLite persistence enables recovery after restart:

1. On startup, query `pending` and `processing` items
2. Reset `processing` items to `pending` (worker died mid-process)
3. Resume processing from queue head

---

## Implementation Files

### Domain Layer

| File                                            | Purpose                                                              |
| ----------------------------------------------- | -------------------------------------------------------------------- |
| `crates/engine/src/queue_types.rs` | Queue payload DTOs (PlayerActionData, LlmRequestData, etc.) |

### Engine Layer

| File                                        | Purpose                                                        |
| ------------------------------------------- | -------------------------------------------------------------- |
| `crates/engine/src/infrastructure/ports.rs` | Queue-related port traits (QueuePort, ApprovalQueuePort, etc.) |
| `crates/engine/src/infrastructure/queue.rs` | SQLite-backed queue implementation + storage schema/indexes    |
| `crates/engine/src/main.rs`                 | Background worker spawning / runtime wiring (when enabled)     |

---

## Related Documents

- [WebSocket Protocol](./websocket-protocol.md) - Message flow
- [Hexagonal Architecture](./hexagonal-architecture.md) - Port pattern
