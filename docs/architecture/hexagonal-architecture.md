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

## Directory Structure (crate-based)

Hexagonal boundaries are enforced by **crate dependencies**.

### Core
- `crates/domain` (`wrldbldr-domain`): core domain types + typed IDs (aim: serde-free)
- `crates/protocol` (`wrldbldr-protocol`): wire DTOs for HTTP/WS (serialization-only)

### Ports
- `crates/engine-ports` (`wrldbldr-engine-ports`): all engine port traits (inbound + outbound)
- `crates/player-ports` (`wrldbldr-player-ports`): all player port traits (inbound + outbound)

### Engine
- `crates/engine-app` (`wrldbldr-engine-app`): application services + app-layer DTOs
- `crates/engine-adapters` (`wrldbldr-engine-adapters`): infrastructure (Axum routes, WS server, persistence, LLM clients, queues)
- `crates/engine-runner` (`wrldbldr-engine-runner`): composition root; produces `wrldbldr-engine` binary

### Player
- `crates/player-app` (`wrldbldr-player-app`): application services
- `crates/player-adapters` (`wrldbldr-player-adapters`): infrastructure (HTTP/WS clients, platform/storage adapters)
- `crates/player-ui` (`wrldbldr-player-ui`): Dioxus presentation (no adapter construction)
- `crates/player-runner` (`wrldbldr-player-runner`): composition root; produces `wrldbldr-player` binary

---

## Import Rules (crate boundaries)

### NEVER ALLOWED

- App crates importing adapter crates (e.g. `wrldbldr-engine-app` depending on `wrldbldr-engine-adapters`).
- Non-owner crates re-exporting/aliasing owning crates:

```rust
// Re-export shim (forbidden)
pub use wrldbldr_protocol::GameTime;

// Crate alias shim (forbidden)
use wrldbldr_protocol as messages;
extern crate wrldbldr_protocol as messages;
```

Rationale: keep a single canonical import path for every type.

### ALWAYS REQUIRED

- Application code depends on **port traits**, not concrete adapters.
- Adapters implement those port traits.

---

## Port Pattern (crate based)

### Defining a Port (ports crate)

```rust
// crates/engine-ports/src/outbound/repository_port.rs

#[async_trait]
pub trait CharacterRepositoryPort: Send + Sync {
    async fn get(&self, id: CharacterId) -> Result<Option<Character>>;
    async fn create(&self, character: &Character) -> Result<()>;
    async fn update(&self, character: &Character) -> Result<()>;
    async fn delete(&self, id: CharacterId) -> Result<()>;
}
```

### Implementing a Port (adapters crate)

```rust
// crates/engine-adapters/src/infrastructure/persistence/character_repository.rs

pub struct Neo4jCharacterRepository { /* ... */ }

#[async_trait]
impl CharacterRepositoryPort for Neo4jCharacterRepository {
    async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        // Neo4j query implementation
    }
    // ...
}
```

### Using a Port (app crate)

```rust
// crates/engine-app/src/application/services/character_service.rs

pub struct CharacterService<R: CharacterRepositoryPort> {
    character_repo: Arc<R>,
}
```

---

---

## Architecture Compliance

### Violation Approval Process

1. **Propose in Plan**: Document the violation, explain why, describe trade-offs
2. **Await Approval**: User must explicitly approve
3. **Mark in Code** (and prefer crate-level fixes over module-level exceptions):

```rust
// ARCHITECTURE VIOLATION: [APPROVED YYYY-MM-DD]
// Reason: <explanation>
// Mitigation: <how we limit the damage>
// Approved by: <user>
```

Note: because this repo enforces architecture primarily via crate dependencies, most violations should be solvable by moving code to the correct crate rather than adding an exception.

### Currently Accepted Violations

None tracked in this document. If a violation is required, document it in `docs/progress/HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md` and keep the owning crate boundaries intact.

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
