# WrldBldr Agent Guidelines

## Canonical architecture (read this first)

The source of truth is:

- `docs/architecture/hexagonal-architecture.md` (**Hexagonal Architecture (Target)**)

The single source-of-truth refactor plan to reach that target is:

- `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`

This file (`AGENTS.md`) is a practical, high-signal summary for agents.

### Key Facts

- **~700 Rust files** across 15 workspace crates
- **Multi-architecture**: Backend engine server (Axum) + WebAssembly player UI (Dioxus)
- **AI-powered**: Neo4j graph DB + Ollama LLM + ComfyUI image generation
- **Hexagonal/Clean architecture**: domain → ports → adapters → apps → runners

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
│  protocol (wire-format DTOs)         common (shared utilities)      │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                      ENGINE-INTERNAL                                 │
│  engine-dto (internal DTOs - not shared with player)                │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                          DOMAIN                                      │
│  domain (entities, value objects)    domain-types (shared vocab)    │
└─────────────────────────────────────────────────────────────────────┘
```

### Crate Responsibilities

| Crate                | Layer           | Purpose                                               |
| -------------------- | --------------- | ----------------------------------------------------- |
| `domain-types`       | Domain          | Shared vocabulary (archetypes, monomyth stages)       |
| `domain`             | Domain          | 25+ entities, value objects, 28 typed IDs             |
| `common`             | Shared Kernel   | Shared utilities (datetime parsing, string-to-option) |
| `protocol`           | Shared Kernel   | Wire-format types for Engine↔Player communication     |
| `engine-dto`         | Engine-Internal | Engine-internal DTOs (queues, persistence)            |
| `engine-ports`       | Ports           | 100+ repository/service traits (ISP-compliant)        |
| `player-ports`       | Ports           | Transport and connection traits                       |
| `engine-app`         | Application     | Services, use cases, request handlers                 |
| `player-app`         | Application     | Player-side services                                  |
| `engine-composition` | Application     | Dependency injection containers                       |
| `engine-adapters`    | Adapters        | Neo4j repos, Axum handlers, WebSocket                 |
| `player-adapters`    | Adapters        | WebSocket client, platform storage                    |
| `player-ui`          | Presentation    | Dioxus components and routes                          |
| `engine-runner`      | Runner          | Server entry point, wiring                            |
| `player-runner`      | Runner          | Client entry point, WASM/desktop                      |

---

## Architecture Rules (STRICT)

These rules are enforced (and will increasingly be enforced) by `cargo xtask arch-check`.
**Always run this before committing.**

### 1. Domain Layer Purity

**Location**: `crates/domain/`, `crates/domain-types/`

**DO**:

- Use `serde`, `uuid`, `chrono`, `thiserror` only
- Use typed IDs (`CharacterId`, not raw `Uuid`)
- Implement business logic in entity methods

**DON'T**:

- Import tokio, axum, neo4rs, or any framework crate
- Import from ports, adapters, app, or runner layers
- Call `Utc::now()` - inject time via `ClockPort`
- Use `rand` crate - inject randomness via ports
- Perform file I/O, network calls, or env access

**Known Exception** (ADR-001): `Uuid::new_v4()` is allowed for ID generation.

### 2. Ports Layer Contracts

**Location**: `crates/engine-ports/`, `crates/player-ports/`

**DO**:

- Define traits (interfaces) and small boundary DTOs only
- Depend only on domain (and limited `protocol` only when explicitly whitelisted)
- Use `#[async_trait]` for async traits
- Follow Interface Segregation - small, focused traits

**DON'T**:

- Add implementations (blanket impls, default impls with logic)
- Import from adapters or app layers
- Import protocol types (except whitelisted boundary files)

**Inbound vs outbound meaning (canonical)**:

- **Inbound ports** define what the application offers (implemented by use cases; called by handlers/UI).
- **Outbound ports** define what the application needs from the outside world (depended on by use cases/services; implemented by adapters).

**Mechanical classification rule (use this when in doubt)**:

- If adapters/handlers/UI **call** the trait and application code **implements** it → it is **inbound**.
- If application code **depends on** the trait and adapters implement it → it is **outbound**.
- If both **implementation and callers are inside the application layer** → it is **not a port** (it’s an internal application interface; keep it in `engine-app`/`player-app`, not in `*-ports`).

**Important**:

- Application use cases/services should **not** depend on inbound ports. If you feel you “need” to, that’s usually a sign the trait is miscategorized (should be outbound) or shouldn’t be a port at all (should live in the app crate).

If a trait is a dependency of a use case or service, it belongs in **outbound**.

**Pattern**: Split large traits into focused sub-traits:

```rust
// Good - focused traits
trait CharacterCrudPort { ... }
trait CharacterWantPort { ... }
trait CharacterInventoryPort { ... }

// Bad - god trait with 20+ methods
trait CharacterRepository { /* everything */ }
```

### 3. Adapters Layer Implementation

**Location**: `crates/engine-adapters/`, `crates/player-adapters/`

**DO**:

- Implement port traits
- Depend on domain, ports, protocol, external crates
- Use parameterized queries for all database operations
- Handle and propagate errors properly

**DON'T**:

- Depend on app layer
- Put business logic here - only translation/mapping
- Concatenate user input into queries (injection risk)

### 4. Application Layer Orchestration

**Location**: `crates/engine-app/`, `crates/player-app/`

**DO**:

- Orchestrate domain objects via **outbound port** abstractions
- Depend on domain + ports
- Keep protocol types at the boundary (handlers/adapters)

**DON'T**:

- Import adapter implementations directly
- Store/use concrete adapter types in services/use cases
- Perform direct I/O (database, HTTP, filesystem)
- Duplicate domain logic - delegate to entities

**Red flag**: application code depending on concrete types when a port exists.

### 5. Protocol Import Rules

| Layer            | Protocol Imports               | Why                      |
| ---------------- | ------------------------------ | ------------------------ |
| domain           | FORBIDDEN                      | Pure business logic      |
| \*-ports         | FORBIDDEN (except whitelisted) | Infrastructure contracts |
| \*-app/use_cases | FORBIDDEN                      | Business orchestration   |
| \*-app/services  | FORBIDDEN                      | Business orchestration   |
| \*-app/handlers  | ALLOWED                        | Boundary layer           |
| \*-adapters      | ALLOWED                        | Wire-format conversion   |
| player-ui        | ALLOWED                        | Presentation boundary    |

Note: The target direction is to keep `protocol` at the boundary (handlers/adapters/UI). If a port needs a protocol type, it must be explicitly whitelisted and justified.

### 6. DTO ownership (no duplication)

Single source of truth per DTO:

- Business semantics/invariants → `domain`
- Engine↔Player wire format → `protocol`
- App↔adapter contracts → `engine-ports`/`player-ports`
- Engine-internal glue only → `engine-dto`

Avoid “shadow copies” like `engine_dto::X` duplicating `engine_ports::...::X`.

### 7. No Shim Import Paths

- No re-exports of `wrldbldr_*` from other crates
- No crate aliasing (`use wrldbldr_* as foo`)
- Goal: single canonical import path for every type

### 8. Naming Conventions

Consistent suffixes for trait types:

| Suffix            | Usage                                | Example                               |
| ----------------- | ------------------------------------ | ------------------------------------- |
| `*Port`           | All port traits (general)            | `ClockPort`, `LoggingPort`            |
| `*RepositoryPort` | Data access ports                    | `CharacterRepositoryPort`             |
| `*ServicePort`    | Business operation ports             | `LlmServicePort`                      |
| `*QueryPort`      | Read-only query ports                | `CharacterQueryPort`                  |
| `*Provider`       | Platform abstractions (player-ports) | `StorageProvider`, `PlatformProvider` |

**Guidelines**:

- Use `*Port` suffix for all port traits in `engine-ports` and `player-ports`
- Use `*Provider` for player-side platform abstractions that may have multiple implementations (web, desktop, mobile)
- Prefer specific suffixes (`*RepositoryPort`, `*ServicePort`, `*QueryPort`) over generic `*Port` when the role is clear

---

## Common Pitfalls

### Neo4j Cypher Injection

**CRITICAL**: Never concatenate user input into Cypher strings.

```rust
// CORRECT - parameterized query
let query = query("MATCH (c:Character {id: $id}) RETURN c")
    .param("id", id.to_string());

// WRONG - injection vulnerability
let query = query(&format!("MATCH (c:Character {{id: '{}'}}) RETURN c", id));
```

Dynamic edge types from validated enums are safe:

```rust
let edge_type = match role {
    ActantialRole::Helper => "VIEWS_AS_HELPER",
    ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
};
// Safe - edge_type from controlled enum, not user input
```

### Protocol Enum Forward Compatibility

When adding enums to protocol types, always include `#[serde(other)]` variant:

```rust
#[derive(Serialize, Deserialize)]
pub enum StatusDto {
    Active,
    Inactive,
    #[serde(other)]
    Unknown,  // Handles future variants gracefully
}
```

This prevents deserialization failures when engine sends new variants to older players.

### Typed ID Usage

Always use typed IDs, never raw UUIDs:

```rust
// CORRECT
fn get_character(&self, id: CharacterId) -> Character;

// WRONG - loses type safety
fn get_character(&self, id: Uuid) -> Character;
```

For tests, use `from_uuid()` for deterministic IDs:

```rust
let id = CharacterId::from_uuid(Uuid::nil());
```

### Time and Randomness

Never use `Utc::now()` or `rand` directly in domain/app layers:

```rust
// WRONG - impure, untestable
let timestamp = Utc::now();

// CORRECT - inject via port
let timestamp = clock_port.now();
```

### Error Handling

Never swallow errors or use `.unwrap()` in production code:

```rust
// WRONG
let result = repository.get(id).unwrap();

// CORRECT
let result = repository.get(id).await?;
```

---

## Development Workflow

### Required Checks (Always Run Before Commit)

```bash
cargo xtask arch-check    # Architecture validation (MUST PASS)
cargo check --workspace   # Compilation check
cargo test --workspace    # Run all tests
```

### Quality Checks

```bash
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
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

## Adding New Features

### Adding a New Entity

1. **Domain** (`crates/domain/src/entities/`): Define entity struct and methods
2. **Domain** (`crates/domain/src/ids.rs`): Add typed ID if needed
3. **Ports** (`crates/engine-ports/src/outbound/`): Define repository trait(s)
4. **Adapters** (`crates/engine-adapters/src/infrastructure/persistence/`): Implement Neo4j repository
5. **Composition** (`crates/engine-composition/`): Wire up in DI container
6. **App** (`crates/engine-app/`): Add service/use case if needed

### Adding a New API Endpoint

1. **Protocol** (`crates/protocol/src/`): Define request/response DTOs
2. **App Handler** (`crates/engine-app/src/application/handlers/`): Add handler
3. **Adapters** (`crates/engine-adapters/src/infrastructure/axum/`): Wire route

### Adding a New Player UI Route

1. **Protocol**: Ensure DTOs exist for data needed
2. **Player-UI** (`crates/player-ui/src/routes/`): Add route component
3. **Player-UI** (`crates/player-ui/src/presentation/components/`): Add components
4. **Player-Runner**: Register route in router

---

## Code Patterns

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
}

impl Character {
    pub fn new(world_id: WorldId, name: String) -> Self {
        Self {
            id: CharacterId::new(),
            world_id,
            name,
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

## Key Documentation

| Document                                      | Purpose                         |
| --------------------------------------------- | ------------------------------- |
| `docs/architecture/hexagonal-architecture.md` | Authoritative architecture spec |
| `docs/architecture/neo4j-schema.md`           | Database schema reference       |
| `docs/architecture/queue-system.md`           | Queue processing patterns       |
| `docs/architecture/websocket-protocol.md`     | Client-server protocol          |
| `docs/systems/*.md`                           | Game system specifications      |
| `crates/engine-runner/README.md`              | Engine development guide        |
| `crates/player-runner/README.md`              | Player UI development guide     |

---

## Testing Guidelines

### Domain Tests

- Pure unit tests, no mocking needed
- Test entity behavior and value object validation

### Port Tests

- Use `mockall` for mocking port traits
- Test service orchestration logic

### Adapter Tests

- Integration tests with real dependencies (test containers)
- Verify correct data mapping and query behavior

### Deterministic Testing

```rust
// Use from_uuid() for reproducible IDs
let id = CharacterId::from_uuid(Uuid::nil());

// Inject fixed time via ClockPort mock
let mock_clock = MockClockPort::new();
mock_clock.expect_now().returning(|| fixed_datetime);
```
