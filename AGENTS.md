# WrldBldr Agent Guidelines

## Project Overview

WrldBldr is a hexagonal-architecture Rust game engine with:
- **390+ Rust files** across 13 workspace crates  
- **Multi-architecture**: Backend engine server + WebAssembly player UI
- **AI-powered**: Neo4j graph DB with ComfyUI/Ollama integrations
- **Hexagonal/Clean architecture**: domain → ports → adapters → apps → runners

## Architecture Layers

### 1. Domain Layer (`crates/domain`)
Core business logic, zero external dependencies
- 20+ entities: Character, Challenge, Location, Item, Event, Scene, etc.
- All entities use: `#[derive(Serialize, Deserialize, Clone, Debug)]`
- UUID v4 for identity, chrono for timestamps, thiserror for errors
- NO async, NO I/O, pure business logic

### 2. Ports Layer (`crates/*-ports`)
Interfaces/contracts that define layer boundaries
- Traits use `#[async_trait]` for async operations
- Pure interfaces, no implementation
- Two crates: player-ports (client-side), engine-ports (server-side)

### 3. Adapters Layer (`crates/*-adapters`)
External system implementations
- **Engine**: Neo4j Cypher queries, Axum HTTP handlers, WebSocket
- **Player**: WebSocket client, HTTP API calls, localStorage
- Handles serialization, database access, web communication

### 4. Application Layer (`crates/*-app`)
Orchestrates domain objects using ports
- Service classes coordinate multiple domain entities
- Manages use cases and workflows
- Sanitizes inputs, validates business rules

### 5. Runners (`crates/*-runner`)
Entry points and infrastructure
- **Engine runner**: Axum server, CLI, handles HTTP/WebSocket
- **Player runner**: Dioxus app, WASM binary for browser

### 6. Protocol (`crates/protocol`)
Shared types for Engine-Player communication
- Serde serializable structs/enums
- Used by both sides without circular dependency

## Technology Stack

### Dependencies (workspace root)

**Core:**
- `tokio = { version = "1.42", features = ["full"] }`
- `serde = { version = "1.0", features = ["derive"] }`
- `serde_json = "1.0"`
- `uuid = { version = "1.11", features = ["v4", "serde", "js"] }`
- `chrono = { version = "0.4", features = ["serde"] }`
- `thiserror = "2.0"`
- `async-trait = "0.1"`

**Engine:**
- `neo4rs = "0.8"` - Graph database
- `axum = { version = "0.8", features = ["ws"] }` - HTTP/WebSocket
- `tower = "0.5"`, `tower-http = { version = "0.6", features = ["cors", "trace"] }`
- `reqwest = { version = "0.12", features = ["json", "stream"] }` - HTTP client
- `sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid"] }`

**Player:**
- `dioxus = "0.7.2"` - Web UI framework
- `tokio-tungstenite = "0.24"` - WebSocket client (desktop)
- `wasm-bindgen = "0.2"`, `wasm-bindgen-futures = "0.4"`
- `web-sys = "0.3"`, `gloo-net = "0.5"`

## Entity Patterns

Standard entity structure:
```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: Uuid,  // Or String for UUID strings from Neo4j
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    // ... fields
}
```

Constructor patterns:
- Use `impl Entity { pub fn new(...) -> Self { ... } }`
- Generate UUID with `Uuid::new_v4()`
- Set `created_at: Utc::now()`, `updated_at: None`

Module structure:
- Each entity in `crates/domain/src/entities/*.rs`
- Module declaration in `crates/domain/src/entities/mod.rs`
- Re-export types with `pub use entity::{Type1, Type2};`

## Porting Patterns

API trait definition (ports layer):
```rust
use async_trait::async_trait;

#[async_trait]
pub trait WorldPort: Send + Sync {
    async fn get_world(&self, id: &Uuid) -> Result<World, DomainError>;
    async fn create_world(&self, world: &World) -> Result<Uuid, DomainError>;
    // etc.
}
```

Implementation (adapters layer):
```rust
use neo4rs::*;
use async_trait::async_trait;

pub struct Neo4jWorldAdapter {
    graph: Graph,  // neo4rs Graph
}

#[async_trait]
impl WorldPort for Neo4jWorldAdapter {
    async fn get_world(&self, id: &Uuid) -> Result<World, DomainError> {
        let query = query("MATCH (w:World {id: $id}) RETURN w")
            .param("id", id.to_string());
        // ... execute, map results
    }
}
```

Error handling:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Entity not found: {id}")]
    NotFound { id: Uuid },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Database error: {0}")]
    Database(#[from] neo4rs::Error),
    // ...
}
```

## API Patterns

Axum handler structure:
```rust
async fn get_world(
    Path(world_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<WorldResponse>, ApiError> {
    let world = state.world_port.get_world(&world_id).await?;
    Ok(Json(WorldResponse { data: world }))
}
```

AppState sharing:
```rust
#[derive(Clone)]
pub struct AppState {
    pub world_port: Arc<dyn WorldPort>,
    // ... all ports
}
```

## Build & Run

**Development:**
```bash
cd /home/otto/repos/WrldBldr/Game

# Run all services
docker compose up

# Or manually:
cargo run -p engine-runner --bin engine-runner
cargo run -p player-runner --bin player-runner  # For WASM development

# Build WASM
dx build --release  # In player-runner
```

**Tests:**
```bash
# Run tests per crate
cargo test -p wrldbldr-domain
cargo test -p wrldbldr-engine-app
cargo test -p wrldbldr-player-app
```

## Development Guidelines

### Git Branching
- `main`: Production-ready code
- `dev`: Development branch, deploy to staging
- `feature/*`: Feature branches
- `hotfix/*`: Critical bug fixes

### Code Quality
- No unsafe code unless absolutely necessary
- Follow workspace lints in root Cargo.toml
- Clippy: `cargo clippy --workspace --all-targets --all-features`
- Format: `cargo fmt --all`
- Audit: `cargo audit`

###关键约定 (Key Conventions)
- Hexagonal architecture boundaries must be maintained
- External dependencies only in adapters layer
- All entities must be serializable (serde Serialize + Deserialize)
- Use UUIDs for all entity identifiers
- Timestamps: chrono DateTime<Utc>
- Error types: thiserror Error derive
- Async/await using tokio runtime
- Use `#[serde(rename_all = "camelCase")]` for JSON APIs
- WASM compatibility: enable js feature on uuid, getrandom

### Neo4j Cypher Safety
- **NEVER** concatenate user input into Cypher strings
- ALWAYS use query parameters: `.param("key", value)`
- REVIEW ALL Cypher queries for injection vulnerabilities
- Validate UUID format before database queries

## Project-Specific Notes

### Game Engine Logic
- Character-driven narrative generation using AI (ComfyUI, Ollama)
- Event chains for branching storylines  
- Grid-based location system with connection graphs
- Challenge system with difficulty scaling
- Generation batches for AI asset production
- Session management for player state

### Technical Debt Areas
- **No tests** - Establish testing patterns across all crates
- **Error handling** - Standardize error types and propagation
- **API documentation** - Add OpenAPI spec where beneficial
- **WebSocket protocol** - Define formal message protocol specification
- **Neo4j schema** - Maintain schema migrations and constraints

## Agent-Specific Context

When using @entity-designer: Focus on entity relationships, validation, and business logic. Reference existing entities in crates/domain/src/entities/.

When using @protocol-designer: Focus on contract clarity, serialization stability, and proper UUID usage.

When using @security-auditor: Check Neo4j queries for Cypher injection, Axum handlers for proper validation, and WebSocket protocol security.

When using @test-writer: Prioritize domain entity validation tests, then API handler tests, then integration tests. Use mockall for mocking ports.

When using @docker-operator: Use existing docker compose setup in Game/docker/, optimize multi-stage builds, configure health checks and resource limits.
