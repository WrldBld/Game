# WrldBldr Engine

The Engine is the backend server for WrldBldr, providing the game state, AI integration, and real-time communication with Player clients.

---

## Architecture Overview

The Engine follows **hexagonal (ports & adapters) architecture** across 6 crates:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            engine-runner                                     │
│    main() → creates adapters → wires dependencies → starts Axum server       │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
┌──────────────────────────────────┴──────────────────────────────────────────┐
│                          engine-composition                                  │
│    AppState, CoreServices, GameServices, QueueServices (DI containers)       │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
┌──────────────────────────────────┴──────────────────────────────────────────┐
│                            engine-app                                        │
│    Services (55+), Use Cases, Request Handlers, DTOs                         │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
┌──────────────────────────────────┴──────────────────────────────────────────┐
│                           engine-ports                                       │
│    100+ port traits (ISP-split): Repository ports, Service ports             │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
┌──────────────────────────────────┴──────────────────────────────────────────┐
│                          engine-adapters                                     │
│    Neo4j repositories, HTTP routes, WebSocket handlers, Ollama, ComfyUI      │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    ▼                             ▼
             ┌──────────┐                  ┌──────────────┐
             │  domain  │                  │   protocol   │
             └──────────┘                  └──────────────┘
```

---

## Crate Responsibilities

### `engine-runner` (This Crate)
**Layer**: Composition Root

The entry point that wires everything together:
- Creates concrete adapter instances
- Injects dependencies via factory functions
- Starts the Axum HTTP/WebSocket server
- Spawns background workers (queues, cleanup)

### `engine-composition`
**Layer**: Application (DI)

Defines service container types using only `Arc<dyn Trait>`:
- `AppState` - Main container with all services
- `CoreServices` - World, Character, Location, Scene, etc.
- `GameServices` - Challenges, Narrative, Dispositions
- `QueueServices` - LLM, Player Action, Asset queues
- `AssetServices` - Generation, Workflows
- `EventInfra` - Event bus, notifications

### `engine-app`
**Layer**: Application

Business logic orchestration:
- **55+ Services** in `services/` - CRUD and domain operations
- **Use Cases** in `use_cases/` - Complex workflow orchestration
- **Handlers** in `handlers/` - WebSocket request routing
- **DTOs** in `dto/` - Application-layer data transfer objects

### `engine-ports`
**Layer**: Ports

Interface definitions (traits only, no implementations):
- **Inbound** (`inbound/`) - Use case ports called by adapters
- **Outbound** (`outbound/`) - Repository/service ports implemented by adapters
- **ISP-Split** - Large traits split into focused sub-traits

### `engine-adapters`
**Layer**: Infrastructure

Concrete implementations:
- **Persistence** (`persistence/`) - 25+ Neo4j repository implementations
- **HTTP** (`http/`) - REST API routes for uploads, exports, settings
- **WebSocket** (`websocket/`) - Real-time message handling
- **Queues** (`queues/`) - SQLite and in-memory queue backends
- **LLM** (`ollama.rs`) - Ollama API client
- **Image Gen** (`comfyui.rs`) - ComfyUI API client

### `engine-dto`
**Layer**: Shared Kernel (Internal)

Engine-only DTOs (not shared with Player):
- Queue payloads (`LlmQueuePayload`, `ApprovalQueuePayload`)
- Persistence DTOs
- Request context types

---

## Directory Structure

```
crates/
├── engine-runner/
│   └── src/
│       ├── main.rs                    # Entry point
│       ├── composition/
│       │   ├── app_state.rs           # Main wiring (966 lines)
│       │   └── factories/
│       │       ├── repositories.rs    # Neo4j repository factories
│       │       ├── use_cases.rs       # Use case factories
│       │       └── core_services.rs   # Service factories
│       └── run/
│           ├── server.rs              # Axum server setup
│           └── workers.rs             # Background workers
│
├── engine-composition/
│   └── src/
│       ├── app_state.rs               # AppState struct
│       ├── core_services.rs           # CoreServices container
│       ├── game_services.rs           # GameServices container
│       ├── queue_services.rs          # QueueServices container
│       ├── asset_services.rs          # AssetServices container
│       ├── player_services.rs         # PlayerServices container
│       ├── event_infra.rs             # EventInfra container
│       └── use_cases.rs               # UseCases container
│
├── engine-app/
│   └── src/application/
│       ├── services/                  # 55+ services
│       │   ├── world_service.rs
│       │   ├── character_service.rs
│       │   ├── llm_queue_service.rs
│       │   ├── challenge_resolution_service.rs
│       │   └── ...
│       ├── use_cases/                 # Complex workflows
│       │   ├── movement.rs
│       │   ├── staging.rs
│       │   ├── challenge.rs
│       │   └── ...
│       ├── handlers/                  # WebSocket handlers
│       │   └── request_handler.rs
│       └── dto/                       # Application DTOs
│
├── engine-ports/
│   └── src/
│       ├── inbound/                   # Use case ports
│       │   ├── use_cases.rs
│       │   ├── use_case_ports.rs
│       │   └── ...
│       └── outbound/                  # Repository/service ports
│           ├── repository_port.rs     # Legacy large traits
│           ├── character_repository/  # ISP-split traits
│           │   ├── crud_port.rs
│           │   ├── want_port.rs
│           │   └── inventory_port.rs
│           └── ...
│
├── engine-adapters/
│   └── src/infrastructure/
│       ├── persistence/               # Neo4j implementations
│       │   ├── character_repository.rs (2073 lines)
│       │   ├── location_repository.rs
│       │   └── ...
│       ├── http/                      # REST routes
│       │   ├── upload_routes.rs
│       │   ├── export_routes.rs
│       │   └── settings_routes.rs
│       ├── websocket/                 # WebSocket handling
│       │   ├── dispatch.rs
│       │   ├── handlers/
│       │   └── converters.rs
│       ├── queues/                    # Queue backends
│       │   ├── sqlite_queue.rs
│       │   └── memory_queue.rs
│       ├── ollama.rs                  # LLM client
│       ├── comfyui.rs                 # Image gen client
│       └── world_state_manager.rs     # Per-world state
│
└── engine-dto/
    └── src/
        ├── queue.rs                   # Queue payloads
        ├── persistence.rs             # Persistence DTOs
        └── llm.rs                     # LLM DTOs
```

---

## Key Navigation Guide

### Finding Code by Task

| Task | Location |
|------|----------|
| Add a new entity | `domain/src/entities/` |
| Add a CRUD service | `engine-app/src/application/services/` |
| Add a port trait | `engine-ports/src/outbound/` |
| Implement a repository | `engine-adapters/src/infrastructure/persistence/` |
| Add a REST endpoint | `engine-adapters/src/infrastructure/http/` |
| Add a WebSocket handler | `engine-adapters/src/infrastructure/websocket/handlers/` |
| Wire a new service | `engine-runner/src/composition/factories/` |

### Important Files

| File | Purpose |
|------|---------|
| `engine-runner/src/main.rs` | Application entry point |
| `engine-runner/src/composition/app_state.rs` | Main dependency wiring |
| `engine-app/src/application/handlers/request_handler.rs` | WebSocket request router |
| `engine-adapters/src/infrastructure/websocket/dispatch.rs` | WebSocket message dispatch |
| `engine-ports/src/outbound/repository_port.rs` | Legacy repository traits |

---

## Adding a New Feature

### 1. Domain Entity

```rust
// crates/domain/src/entities/my_entity.rs
use serde::{Deserialize, Serialize};
use crate::MyEntityId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyEntity {
    pub id: MyEntityId,
    pub name: String,
    // ...
}
```

### 2. Port Trait

```rust
// crates/engine-ports/src/outbound/my_entity_repository_port.rs
use async_trait::async_trait;
use wrldbldr_domain::{MyEntity, MyEntityId};

#[async_trait]
pub trait MyEntityCrudPort: Send + Sync {
    async fn get(&self, id: MyEntityId) -> Result<Option<MyEntity>, RepositoryError>;
    async fn create(&self, entity: &MyEntity) -> Result<(), RepositoryError>;
    async fn update(&self, entity: &MyEntity) -> Result<(), RepositoryError>;
    async fn delete(&self, id: MyEntityId) -> Result<(), RepositoryError>;
}
```

### 3. Service

```rust
// crates/engine-app/src/application/services/my_entity_service.rs
use std::sync::Arc;
use wrldbldr_engine_ports::outbound::MyEntityCrudPort;

pub struct MyEntityService {
    repo: Arc<dyn MyEntityCrudPort>,
}

impl MyEntityService {
    pub fn new(repo: Arc<dyn MyEntityCrudPort>) -> Self {
        Self { repo }
    }
    
    pub async fn get(&self, id: MyEntityId) -> Result<Option<MyEntity>> {
        self.repo.get(id).await.map_err(Into::into)
    }
}
```

### 4. Repository Implementation

```rust
// crates/engine-adapters/src/infrastructure/persistence/my_entity_repository.rs
use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_engine_ports::outbound::MyEntityCrudPort;

pub struct Neo4jMyEntityRepository {
    graph: Graph,
}

#[async_trait]
impl MyEntityCrudPort for Neo4jMyEntityRepository {
    async fn get(&self, id: MyEntityId) -> Result<Option<MyEntity>, RepositoryError> {
        let query = query("MATCH (e:MyEntity {id: $id}) RETURN e")
            .param("id", id.to_string());
        // ... execute and map
    }
}
```

### 5. Wire in Composition Root

```rust
// crates/engine-runner/src/composition/factories/repositories.rs
pub fn create_my_entity_repository(graph: Graph) -> Arc<dyn MyEntityCrudPort> {
    Arc::new(Neo4jMyEntityRepository::new(graph))
}

// crates/engine-runner/src/composition/app_state.rs
let my_entity_repo = create_my_entity_repository(graph.clone());
let my_entity_service = MyEntityService::new(my_entity_repo);
```

---

## Architecture Rules

### Domain Layer Purity

The `domain` crate must be pure:
- NO external framework dependencies (tokio, axum, neo4rs)
- NO `Utc::now()` - inject via `ClockPort`
- NO `rand` - inject via `RandomPort`
- NO file I/O, network, or environment access

### Ports Layer Contracts

The `engine-ports` crate:
- Defines traits only - no implementations
- Depends only on domain types
- NO adapter or app layer imports
- All async traits use `#[async_trait]`

### Adapters Layer

The `engine-adapters` crate:
- Implements port traits
- NO direct dependencies on app layer
- NO business logic - only translation/mapping
- All Neo4j queries use `.param()` (no string concatenation)

### Application Layer

The `engine-app` crate:
- Orchestrates domain objects via port abstractions
- NO direct adapter dependencies
- NO direct I/O
- Protocol imports FORBIDDEN in `use_cases/`, ALLOWED in `handlers/`

---

## Neo4j Query Safety

**CRITICAL**: Never concatenate user input into Cypher strings.

```rust
// CORRECT - parameterized query
let query = query("MATCH (c:Character {id: $id}) RETURN c")
    .param("id", id.to_string());

// WRONG - injection vulnerability
let query = query(&format!("MATCH (c:Character {{id: '{}'}}) RETURN c", id));
```

Dynamic edge types from validated enums are acceptable:
```rust
let edge_type = match role {
    ActantialRole::Helper => "VIEWS_AS_HELPER",
    ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
};
// Safe - edge_type is from controlled enum, not user input
```

---

## Running the Engine

```bash
# Development
task backend

# Or directly
cargo run -p wrldbldr-engine-runner

# Check compilation
cargo check -p wrldbldr-engine-runner

# Run tests
cargo test -p wrldbldr-engine-runner
```

### Required Environment Variables

```env
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=your_password
```

### Optional Services

```env
OLLAMA_BASE_URL=http://localhost:11434/v1
OLLAMA_MODEL=qwen3-vl:30b
COMFYUI_BASE_URL=http://localhost:8188
```

---

## Queue System

The Engine uses SQLite-backed queues for async processing:

| Queue | Purpose | Concurrency |
|-------|---------|-------------|
| `PlayerActionQueue` | Player actions | Unlimited |
| `DMActionQueue` | DM actions | Unlimited |
| `LLMReasoningQueue` | Ollama requests | Semaphore (configurable) |
| `AssetGenerationQueue` | ComfyUI requests | Sequential (1) |
| `DMApprovalQueue` | Awaiting DM approval | N/A (waiting) |

Background workers process queues continuously. See `engine-adapters/src/infrastructure/queue_workers.rs`.

---

## WebSocket Protocol

The Engine communicates with Players via WebSocket:

- **Endpoint**: `ws://host:3000/ws`
- **Messages**: JSON with `type` field for routing
- **Protocol crate**: `wrldbldr-protocol` defines all message types

Key message flows:
1. `JoinWorld` → `WorldJoined` (with snapshot)
2. `PlayerAction` → `LLMProcessing` → `ApprovalRequired` → `DialogueResponse`
3. `ChallengeRoll` → `ChallengeOutcomePending` → `ChallengeResolved`

See [docs/architecture/websocket-protocol.md](../../docs/architecture/websocket-protocol.md) for full protocol documentation.

---

## Testing

```bash
# Run all engine tests
cargo test -p wrldbldr-engine-runner
cargo test -p wrldbldr-engine-app
cargo test -p wrldbldr-engine-adapters

# Run specific test
cargo test -p wrldbldr-engine-app test_character_service
```

### Mocking Ports

Use `mockall` for unit testing services:

```rust
#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use super::*;
    
    mock! {
        pub CharacterRepo {}
        #[async_trait]
        impl CharacterCrudPort for CharacterRepo {
            async fn get(&self, id: CharacterId) -> Result<Option<Character>>;
        }
    }
    
    #[tokio::test]
    async fn test_get_character() {
        let mut mock = MockCharacterRepo::new();
        mock.expect_get()
            .with(eq(test_id))
            .returning(|_| Ok(Some(test_character())));
        
        let service = CharacterService::new(Arc::new(mock));
        let result = service.get(test_id).await;
        assert!(result.is_ok());
    }
}
```

---

## Common Issues

### Architecture Check Fails

```bash
cargo xtask arch-check
```

Common violations:
- Importing protocol types in use cases (use domain types instead)
- Importing adapters in app layer (use port traits)
- Re-exporting types from other workspace crates

### Neo4j Connection Issues

- Ensure Neo4j is running: `task docker:up`
- Check credentials in `.env`
- Verify Neo4j browser at http://localhost:7474

### LLM Not Responding

- Check Ollama is running at `OLLAMA_BASE_URL`
- Verify model is available: `ollama list`
- Check logs for timeout/connection errors

---

## Related Documentation

- [Hexagonal Architecture](../../docs/architecture/hexagonal-architecture.md)
- [Neo4j Schema](../../docs/architecture/neo4j-schema.md)
- [WebSocket Protocol](../../docs/architecture/websocket-protocol.md)
- [Queue System](../../docs/architecture/queue-system.md)
- [AGENTS.md](../../AGENTS.md) - AI assistant guidelines
