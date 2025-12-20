# Hexagonal Architecture

## Overview

WrldBldr uses hexagonal (ports & adapters) architecture to separate business logic from external concerns. This enables testing, flexibility, and clean dependencies.

---

## Layer Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DEPENDENCY RULES                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Domain Layer (innermost)                                                  │
│   ├── Contains: Entities, Value Objects, Domain Services                    │
│   ├── Depends on: NOTHING external                                          │
│   └── Rule: Pure Rust, no framework dependencies                            │
│                                                                             │
│   Application Layer                                                         │
│   ├── Contains: Services, Use Cases, DTOs, Ports (traits)                   │
│   ├── Depends on: Domain only                                               │
│   └── Rule: Orchestrates domain logic via ports                             │
│                                                                             │
│   Infrastructure Layer (outermost)                                          │
│   ├── Contains: Repositories, External clients, HTTP/WS                     │
│   ├── Depends on: Application (implements ports)                            │
│   └── Rule: Adapts external systems to ports                                │
│                                                                             │
│   Presentation Layer (Player only)                                          │
│   ├── Contains: UI Components, Views, State                                 │
│   ├── Depends on: Application services                                      │
│   └── Rule: Calls services, never repositories directly                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

### Workspace (current)

This repo is mid-migration from "single-crate per app" to a stricter, compile-time-enforced crate DAG.

- `wrldbldr-domain`: typed IDs + core domain types (serde-free)
- `wrldbldr-protocol`: wire DTOs for HTTP/WS (serde-only)
- `wrldbldr-engine-app`: engine application services (use cases)
- `wrldbldr-engine-adapters`: engine infrastructure (http/ws/db/queues)
- `wrldbldr-engine`: thin bin (composition root)
- `wrldbldr-player-app`: player application services
- `wrldbldr-player-adapters`: player infrastructure (http/ws/platform)
- `wrldbldr-player-ui`: Dioxus UI + (approved) composition / launch
- `wrldbldr-player`: thin bin (composition root)

### Engine (legacy layout, referenced by migrated code)

```
Engine/src/
├── domain/                    # Core business logic
│   ├── entities/              # Business objects with identity
│   ├── value_objects/         # Immutable objects (IDs, configs)
│   └── aggregates/            # (Future: DDD aggregates)
│
├── application/               # Use case orchestration
│   ├── services/              # Business logic coordination
│   ├── ports/
│   │   ├── inbound/           # Use case interfaces
│   │   └── outbound/          # Repository interfaces
│   └── dto/                   # Data transfer objects
│
└── infrastructure/            # External adapters
    ├── persistence/           # Neo4j repositories
    ├── http/                  # REST routes
    ├── websocket/             # WebSocket handlers
    ├── queues/                # Queue implementations
    └── ollama.rs              # LLM client
```

### Player

```
Player/src/
├── domain/                    # Client-side entities
│   ├── entities/
│   └── value_objects/
│
├── application/               # Client services
│   ├── services/              # API calls, business logic
│   ├── ports/
│   │   └── outbound/          # ApiPort, GameConnectionPort
│   └── dto/                   # DTOs matching Engine
│
├── infrastructure/            # External adapters
│   ├── websocket/             # WebSocket client
│   ├── platform/              # Platform abstraction
│   └── http_client.rs         # HTTP client
│
└── presentation/              # UI layer
    ├── views/                 # Full-page views
    ├── components/            # Reusable components
    ├── state/                 # Reactive state
    └── handlers/              # Event handlers
```

---

## Import Rules

### NEVER ALLOWED

```rust
// Domain importing from application/infrastructure
use crate::application::*;     // FORBIDDEN in domain/
use crate::infrastructure::*;  // FORBIDDEN in domain/

// Application importing from infrastructure
use crate::infrastructure::*;  // FORBIDDEN in application/
```

### ALWAYS REQUIRED

```rust
// Infrastructure implements application ports
use crate::application::ports::outbound::SomeRepositoryPort;

impl SomeRepositoryPort for Neo4jSomeRepository {
    // ...
}
```

---

## Port Pattern

### Defining a Port (Application Layer)

```rust
// application/ports/outbound/character_repository_port.rs

#[async_trait]
pub trait CharacterRepositoryPort: Send + Sync {
    async fn get_by_id(&self, id: &CharacterId) -> Result<Option<Character>>;
    async fn save(&self, character: &Character) -> Result<()>;
    async fn delete(&self, id: &CharacterId) -> Result<()>;
    async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<Character>>;
}
```

### Implementing a Port (Infrastructure Layer)

```rust
// infrastructure/persistence/character_repository.rs

pub struct Neo4jCharacterRepository {
    pool: Arc<Graph>,
}

#[async_trait]
impl CharacterRepositoryPort for Neo4jCharacterRepository {
    async fn get_by_id(&self, id: &CharacterId) -> Result<Option<Character>> {
        // Neo4j query implementation
    }
    // ...
}
```

### Using a Port (Application Layer)

```rust
// application/services/character_service.rs

pub struct CharacterService {
    character_repo: Arc<dyn CharacterRepositoryPort>,
}

impl CharacterService {
    pub async fn get_character(&self, id: &CharacterId) -> Result<Option<Character>> {
        self.character_repo.get_by_id(id).await
    }
}
```

---

## Architecture Compliance

### Violation Approval Process

1. **Propose in Plan**: Document the violation, explain why, describe trade-offs
2. **Await Approval**: User must explicitly approve
3. **Mark in Code**:

```rust
// ARCHITECTURE VIOLATION: [APPROVED 2025-12-17]
// Reason: <explanation>
// Mitigation: <how we limit the damage>
// Approved by: <user>
use crate::infrastructure::SomeType;  // Normally forbidden
```

### Currently Accepted Violations

| Location | Description | Justification |
|----------|-------------|---------------|
| `Player/src/presentation/services.rs` | Type aliases expose `ApiAdapter` | Composition layer pattern - components use service methods only |
| `Engine/src/domain/entities/*.rs` | Serde derives on some domain types | Required for JSON serialization to Neo4j |

---

## Testing Strategy

### Unit Tests (Domain/Application)

```rust
// Use mock implementations of ports
struct MockCharacterRepo {
    characters: Vec<Character>,
}

impl CharacterRepositoryPort for MockCharacterRepo {
    // In-memory implementation
}

#[test]
fn test_character_service() {
    let mock_repo = MockCharacterRepo::new();
    let service = CharacterService::new(Arc::new(mock_repo));
    // Test business logic without database
}
```

### Integration Tests (Infrastructure)

```rust
// Test with real database
#[tokio::test]
async fn test_neo4j_repository() {
    let pool = setup_test_database().await;
    let repo = Neo4jCharacterRepository::new(pool);
    // Test actual persistence
}
```

---

## Related Documents

- [Neo4j Schema](./neo4j-schema.md) - Database structure
- [WebSocket Protocol](./websocket-protocol.md) - Message types
- [Queue System](./queue-system.md) - Queue architecture
