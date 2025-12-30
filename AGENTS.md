# WrldBldr Agent Guidelines

## Project Overview

WrldBldr is a **hexagonal-architecture Rust game engine** for AI-powered tabletop roleplaying games.

### Key Facts
- **~450 Rust files** across 15 workspace crates
- **Multi-architecture**: Backend engine server (Axum) + WebAssembly player UI (Dioxus)
- **AI-powered**: Neo4j graph DB + Ollama LLM + ComfyUI image generation
- **Hexagonal/Clean architecture**: domain → ports → adapters → apps → runners
- **Architecture compliance**: 92%+ (tracked via `cargo xtask arch-check`)

### Project Objectives
1. **Pure Graph Model**: All game state in Neo4j as nodes and edges
2. **AI Game Master**: LLM-driven NPC dialogue and narrative generation
3. **DM Approval Flow**: Human oversight of AI-generated content
4. **Session-based Multiplayer**: Real-time WebSocket communication
5. **Asset Generation**: ComfyUI integration for character/scene artwork

---

## Architecture Overview

### Hexagon Structure

```
┌─────────────────────────────────────────────────────────────────────┐
│                           RUNNERS                                    │
│  engine-runner (composition root)    player-runner (composition root)│
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                          ADAPTERS                                    │
│  engine-adapters (Neo4j, Axum, WS)   player-adapters (WS, Storage)  │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                        APPLICATION                                   │
│  engine-app (services, use cases)    player-app (services)          │
│  engine-composition (DI containers)                                  │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                           PORTS                                      │
│  engine-ports (100+ traits)          player-ports (transport traits) │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                      SHARED KERNEL                                   │
│  protocol (wire-format DTOs)         engine-dto (internal DTOs)     │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                          DOMAIN                                      │
│  domain (entities, value objects)    domain-types (shared vocab)    │
└─────────────────────────────────────────────────────────────────────┘
```

### Crate Responsibilities

| Crate | Layer | Purpose |
|-------|-------|---------|
| `domain-types` | Domain | Shared vocabulary (archetypes, monomyth stages) |
| `domain` | Domain | 25+ entities, value objects, 25+ typed IDs |
| `protocol` | Shared Kernel | Wire-format types for Engine↔Player communication |
| `engine-dto` | Shared Kernel | Engine-internal DTOs (queues, persistence) |
| `engine-ports` | Ports | 100+ repository/service traits (ISP-compliant) |
| `player-ports` | Ports | Transport and connection traits |
| `engine-app` | Application | Services, use cases, request handlers |
| `player-app` | Application | Player-side services |
| `engine-composition` | Application | Dependency injection containers |
| `engine-adapters` | Adapters | Neo4j repos, Axum handlers, WebSocket |
| `player-adapters` | Adapters | WebSocket client, platform storage |
| `player-ui` | Presentation | Dioxus components and routes |
| `engine-runner` | Runner | Server entry point, wiring |
| `player-runner` | Runner | Client entry point, WASM/desktop |

---

## Architecture Rules (STRICT)

### 1. Domain Layer Purity

**Location**: `crates/domain/`, `crates/domain-types/`

**Rules**:
- NO external framework dependencies (tokio, axum, neo4rs)
- NO imports from ports, adapters, app, or runner layers
- NO `Utc::now()` in production code - time must be injected via `ClockPort`
- NO `rand` crate - randomness must be injected
- NO file I/O, network calls, or environment access

**Allowed**: serde, uuid, chrono, thiserror

**Known Exception** (ADR-001): `Uuid::new_v4()` is allowed for ID generation (pragmatic trade-off)

### 2. Ports Layer Contracts

**Location**: `crates/engine-ports/`, `crates/player-ports/`

**Rules**:
- Define traits (interfaces) only - no implementations
- Depend only on domain types
- NO adapter or app layer imports
- All async traits use `#[async_trait]`
- Protocol imports allowed only in whitelisted boundary files

**Interface Segregation**: Large traits split into focused sub-traits:
- `CharacterCrudPort`, `CharacterWantPort`, `CharacterInventoryPort`, etc.
- `StoryEventCrudPort`, `StoryEventEdgePort`, `StoryEventQueryPort`, etc.

### 3. Adapters Layer Implementation

**Location**: `crates/engine-adapters/`, `crates/player-adapters/`

**Rules**:
- Implement port traits
- May depend on: domain, ports, protocol, external crates
- NO direct dependencies on app layer
- NO business logic - only translation/mapping
- All Neo4j queries use `.param()` for values (no string concatenation)

### 4. Application Layer Orchestration

**Location**: `crates/engine-app/`, `crates/player-app/`

**Rules**:
- Orchestrate domain objects via port abstractions
- May depend on: domain, ports, protocol (for DTOs)
- NO direct adapter dependencies
- NO direct I/O (database, HTTP, filesystem)
- Delegate domain logic to entities

### 5. Protocol Import Rules

| Layer | Protocol Imports | Rationale |
|-------|------------------|-----------|
| domain | FORBIDDEN | Pure business logic |
| *-ports | FORBIDDEN (except whitelisted) | Infrastructure contracts |
| *-app/use_cases | FORBIDDEN | Business orchestration |
| *-app/services | App-layer DTOs only | Service isolation |
| *-app/handlers | ALLOWED | Boundary layer |
| *-adapters | ALLOWED | Wire-format conversion |
| player-ui | ALLOWED | Presentation boundary |

### 6. No Shim Import Paths

- No re-exports of `wrldbldr_*` from other crates
- No crate aliasing (`use wrldbldr_* as foo`)
- Goal: single canonical import path for every type

---

## Technology Stack

### Core Dependencies
```toml
tokio = { version = "1.42", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.11", features = ["v4", "serde", "js"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2.0"
async-trait = "0.1"
```

### Engine-Specific
```toml
neo4rs = "0.8"                    # Graph database
axum = { version = "0.8", features = ["ws"] }  # HTTP/WebSocket
sqlx = "0.8"                      # SQLite for queues
reqwest = "0.12"                  # HTTP client (LLM, ComfyUI)
```

### Player-Specific
```toml
dioxus = "0.7.2"                  # Web UI framework
tokio-tungstenite = "0.24"        # WebSocket (desktop)
gloo-net = "0.5"                  # WebSocket (WASM)
```

---

## Development Commands

### Required Checks (Always Run)
```bash
cargo xtask arch-check    # Architecture validation (MUST PASS)
cargo check --workspace   # Compilation check
cargo test --workspace    # Run all tests
```

### Quality Checks
```bash
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
cargo audit
```

### Running the Application
```bash
# Engine server
cargo run -p wrldbldr-engine-runner

# Player (desktop)
cargo run -p wrldbldr-player-runner

# Player (WASM) - requires dx CLI
dx serve --platform web
```

---

## Current Architecture Status

### Compliance Score: 92/100

**Completed**:
- adapters→app coupling removed
- God traits split (ISP compliance)
- Domain purity (rand, Utc::now abstracted)
- Protocol forward compatibility
- Broadcast consolidation
- Player-UI protocol isolation
- arch-check passing (15 crates)

**Remaining Issues**:

| Issue | Severity | Location |
|-------|----------|----------|
| `anyhow` in domain | Medium | `domain/Cargo.toml` - should use `thiserror` |
| UUID generation impure | Low | `domain/src/ids.rs` - documented in ADR-001 |
| 7 god traits remain | Medium | 15+ methods each (e.g., `LocationRepositoryPort`) |
| `request_handler.rs` size | Low | 1115 lines - consider splitting |
| `app_state.rs` size | Medium | 1314 lines - needs factory functions |

---

## File Organization Patterns

### Entity Definition
```rust
// crates/domain/src/entities/character.rs
use serde::{Deserialize, Serialize};
use crate::{CharacterId, WorldId};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: CharacterId,
    pub world_id: WorldId,
    pub name: String,
    // ... fields
}

impl Character {
    pub fn new(world_id: WorldId, name: String) -> Self {
        Self {
            id: CharacterId::new(),
            world_id,
            name,
            // ...
        }
    }
}
```

### Port Trait Definition
```rust
// crates/engine-ports/src/outbound/character_repository.rs
use async_trait::async_trait;
use wrldbldr_domain::{Character, CharacterId};

#[async_trait]
pub trait CharacterCrudPort: Send + Sync {
    async fn get(&self, id: CharacterId) -> Result<Character, RepositoryError>;
    async fn create(&self, character: &Character) -> Result<(), RepositoryError>;
    async fn update(&self, character: &Character) -> Result<(), RepositoryError>;
    async fn delete(&self, id: CharacterId) -> Result<(), RepositoryError>;
}
```

### Adapter Implementation
```rust
// crates/engine-adapters/src/infrastructure/persistence/character_repository.rs
use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_engine_ports::outbound::CharacterCrudPort;

pub struct Neo4jCharacterRepository {
    graph: Graph,
}

#[async_trait]
impl CharacterCrudPort for Neo4jCharacterRepository {
    async fn get(&self, id: CharacterId) -> Result<Character, RepositoryError> {
        let query = query("MATCH (c:Character {id: $id}) RETURN c")
            .param("id", id.to_string());
        // ... execute and map
    }
}
```

---

## Neo4j Cypher Safety

**CRITICAL**: Never concatenate user input into Cypher strings.

```rust
// CORRECT - parameterized query
let query = query("MATCH (c:Character {id: $id}) RETURN c")
    .param("id", id.to_string());

// WRONG - SQL injection vulnerability
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

## Key Documentation

| Document | Purpose |
|----------|---------|
| `docs/architecture/hexagonal-architecture.md` | Authoritative architecture spec |
| `docs/architecture/neo4j-schema.md` | Database schema reference |
| `docs/architecture/queue-system.md` | Queue processing patterns |
| `docs/architecture/websocket-protocol.md` | Client-server protocol |
| `docs/progress/ACTIVE_DEVELOPMENT.md` | Current sprint tracking |
| `docs/progress/MVP.md` | MVP acceptance criteria |
| `docs/plans/CODE_QUALITY_REMEDIATION_PLAN.md` | Quality improvement tracking |

---

## Agent-Specific Guidance

### For Architecture Work
- Run `cargo xtask arch-check` before and after changes
- Check `docs/architecture/hexagonal-architecture.md` for rules
- Respect whitelisted exemptions in `crates/xtask/src/main.rs`

### For Domain Changes
- No external I/O - inject via ports
- Use typed IDs (`CharacterId`, not `Uuid`)
- Implement business logic in entities, not services

### For Adapter Changes
- Implement port traits, don't modify them
- All database queries use parameters
- Handle errors, don't swallow them

### For Protocol Changes
- Add `#[serde(other)]` to enums for forward compatibility
- Document breaking changes
- Keep wire format minimal

### For Testing
- Use `mockall` for mocking ports
- Domain tests: pure unit tests
- Adapter tests: integration tests with test containers
- Use `from_uuid()` for deterministic ID testing
