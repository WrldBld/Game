# Queue System

## Overview

WrldBldr uses a queue-based architecture for processing player actions, DM decisions, LLM requests, and asset generation. This provides crash recovery, audit trails, and a foundation for scaling.

---

## Queue Types

| Queue | Purpose | Persistence | Concurrency |
|-------|---------|-------------|-------------|
| `PlayerActionQueue` | Player actions awaiting processing | SQLite | Unlimited |
| `DMActionQueue` | DM actions awaiting processing | SQLite | Unlimited |
| `LLMReasoningQueue` | Ollama requests | SQLite | Semaphore (configurable) |
| `AssetGenerationQueue` | ComfyUI requests | SQLite | 1 (sequential) |
| `DMApprovalQueue` | Decisions awaiting DM approval | SQLite | N/A (waiting) |

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

```rust
#[async_trait]
pub trait QueuePort<T>: Send + Sync {
    async fn enqueue(&self, item: T) -> Result<QueueItemId>;
    async fn dequeue(&self) -> Result<Option<QueueItem<T>>>;
    async fn peek(&self) -> Result<Option<QueueItem<T>>>;
    async fn complete(&self, id: &QueueItemId) -> Result<()>;
    async fn fail(&self, id: &QueueItemId, error: &str) -> Result<()>;
    async fn get_status(&self) -> Result<QueueStatus>;
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
    Cancelled,   // Manually cancelled
}
```

---

## Processing Flow

### Player Action Flow

```
1. Player sends PlayerAction via WebSocket
2. Handler creates PlayerActionItem with session_id
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
1. LLMRequestItem created with prompt, npc_id, context
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

## Session-Aware Processing

All queue items carry `session_id` for:

1. **Isolation**: Items from different sessions don't interfere
2. **Fairness**: Round-robin processing across sessions
3. **Routing**: Results sent to correct session participants

```rust
pub struct PlayerActionItem {
    pub action_id: ActionId,
    pub session_id: SessionId,  // Required
    pub user_id: String,
    pub action_type: String,
    pub target_id: Option<String>,
    pub content: String,
}
```

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
    "sessions": ["session-1", "session-2"]
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

### Engine

| File | Purpose |
|------|---------|
| `crates/engine-ports/src/outbound/queue_port.rs` | Queue port trait |
| `crates/engine-app/src/application/dto/queue_items.rs` | Item types |
| `crates/engine-app/src/application/services/player_action_queue_service.rs` | Player actions |
| `crates/engine-app/src/application/services/dm_action_queue_service.rs` | DM actions |
| `crates/engine-app/src/application/services/llm_queue_service.rs` | LLM processing |
| `crates/engine-app/src/application/services/asset_generation_queue_service.rs` | Asset generation |
| `crates/engine-app/src/application/services/dm_approval_queue_service.rs` | Approvals |
| `crates/engine-adapters/src/infrastructure/queues/sqlite_queue.rs` | SQLite backend |
| `crates/engine-adapters/src/infrastructure/queues/memory_queue.rs` | In-memory backend |
| `crates/engine-adapters/src/infrastructure/queues/factory.rs` | Queue factory |
| `crates/engine-adapters/src/infrastructure/queue_workers.rs` | Worker spawning |

---

## Related Documents

- [WebSocket Protocol](./websocket-protocol.md) - Message flow
- [Hexagonal Architecture](./hexagonal-architecture.md) - Port pattern
