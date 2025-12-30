# Hexagonal Architecture

## Overview

WrldBldr uses hexagonal (ports & adapters) architecture to separate business logic from external concerns. This enables testing, flexibility, and clean dependencies.

The architecture also incorporates a **Shared Kernel** pattern for the Engine-Player communication boundary, where the `protocol` crate defines wire-format types that must be identical on both sides.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              WrldBldr Architecture                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                           SHARED KERNEL                                     │ │
│  │                                                                             │ │
│  │  ┌─────────────────────────────────────────────────────────────────────┐   │ │
│  │  │  protocol (wrldbldr-protocol)                                        │   │ │
│  │  │  ├── Wire-format DTOs (REST + WebSocket)                             │   │ │
│  │  │  ├── ClientMessage / ServerMessage enums                             │   │ │
│  │  │  ├── RequestPayload / ResponseResult                                 │   │ │
│  │  │  └── No business logic - pure serialization types                    │   │ │
│  │  └─────────────────────────────────────────────────────────────────────┘   │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                           │
│                    ┌─────────────────┴─────────────────┐                        │
│                    │                                   │                        │
│                    ▼                                   ▼                        │
│  ┌─────────────────────────────────┐   ┌─────────────────────────────────┐     │
│  │         ENGINE SIDE              │   │         PLAYER SIDE             │     │
│  ├─────────────────────────────────┤   ├─────────────────────────────────┤     │
│  │                                 │   │                                 │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   domain (innermost)      │  │   │  │   (shares domain crate)   │  │     │
│  │  │   Pure business entities  │  │   │  │                           │  │     │
│  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │     │
│  │              │                  │   │              │                  │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   engine-ports            │  │   │  │   player-ports            │  │     │
│  │  │   Port trait definitions  │  │   │  │   Port trait definitions  │  │     │
│  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │     │
│  │              │                  │   │              │                  │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   engine-app              │  │   │  │   player-app              │  │     │
│  │  │   Use cases & services    │  │   │  │   Use cases & services    │  │     │
│  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │     │
│  │              │                  │   │              │                  │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   engine-adapters         │  │   │  │   player-adapters         │  │     │
│  │  │   Neo4j, HTTP, WebSocket  │  │   │  │   HTTP/WS clients, UI     │  │     │
│  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │     │
│  │              │                  │   │              │                  │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   engine-runner           │  │   │  │   player-runner           │  │     │
│  │  │   Composition root        │  │   │  │   Composition root        │  │     │
│  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │     │
│  │                                 │   │              │                  │     │
│  └─────────────────────────────────┘   │  ┌───────────────────────────┐  │     │
│                                        │  │   player-ui               │  │     │
│                                        │  │   Dioxus presentation     │  │     │
│                                        │  └───────────────────────────┘  │     │
│                                        └─────────────────────────────────┘     │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

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
│   ├── Rule: Pure Rust, no framework dependencies                            │
│   └── Purity: No I/O (rand, Utc::now, env vars) - use injected ports        │
│                                                                             │
│   Ports Layer                                                               │
│   ├── Contains: Trait definitions (inbound + outbound)                      │
│   ├── Depends on: Domain only                                               │
│   ├── Exception: May use Shared Kernel (protocol) for wire-format types     │
│   └── Rule: Interfaces only, no implementations                             │
│                                                                             │
│   Application Layer                                                         │
│   ├── Contains: Services, Use Cases, DTOs                                   │
│   ├── Depends on: Domain, Ports                                             │
│   └── Rule: Orchestrates domain logic via ports                             │
│                                                                             │
│   Adapters Layer (outermost)                                                │
│   ├── Contains: Repositories, External clients, HTTP/WS handlers            │
│   ├── Depends on: Application, Ports, Protocol                              │
│   └── Rule: Implements ports, translates external types                     │
│                                                                             │
│   Runner Layer                                                              │
│   ├── Contains: main(), composition root, CLI                               │
│   ├── Depends on: All layers (wires everything together)                    │
│   └── Rule: Only place where concrete types are constructed                 │
│                                                                             │
│   Presentation Layer (Player only)                                          │
│   ├── Contains: UI Components, Views, Reactive State                        │
│   ├── Depends on: Application services (via signals)                        │
│   └── Rule: Calls services, never adapters directly                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Shared Kernel Pattern

### What is the Shared Kernel?

The `protocol` crate serves as a **Shared Kernel** - a bounded context that both Engine and Player must share for correct WebSocket communication.

```
┌─────────────────────────────────────────────────────────────────┐
│                      SHARED KERNEL                               │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  wrldbldr-protocol                                       │   │
│  │                                                         │   │
│  │  Wire-Format Types (must be identical on both sides):   │   │
│  │  ├── ClientMessage (100+ variants)                      │   │
│  │  ├── ServerMessage (65+ variants)                       │   │
│  │  ├── RequestPayload (95+ variants)                      │   │
│  │  ├── ResponseResult (Success/Error)                     │   │
│  │  └── RequestError (client-side errors)                  │   │
│  │                                                         │   │
│  │  Characteristics:                                       │   │
│  │  ├── Minimal dependencies (serde, uuid, chrono)         │   │
│  │  ├── No business logic                                  │   │
│  │  ├── WASM compatible                                    │   │
│  │  └── Serialization-focused                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Why Not Duplicate These Types?

1. **Correctness**: Both sides must serialize/deserialize identically
2. **Maintainability**: Changes must be synchronized - a single source ensures this
3. **Scale**: 95+ `RequestPayload` variants would mean ~1100 lines of duplicate code
4. **No semantic difference**: Unlike domain types, wire-format types have no independent meaning per side

### Shared Kernel vs Domain Types

| Aspect | Domain Types | Shared Kernel (Protocol) |
|--------|--------------|--------------------------|
| **Purpose** | Business semantics | Wire format |
| **Identity** | Independent per side | Must be identical |
| **Evolution** | Can evolve independently | Must evolve in lockstep |
| **Location** | Domain crate | Protocol crate |
| **Example** | `Character`, `Location` | `RequestPayload`, `ServerMessage` |

### Files Approved for Shared Kernel Usage

These files in `player-ports` are approved to use protocol types directly:

| File | Protocol Types Used | Reason |
|------|---------------------|--------|
| `request_port.rs` | `RequestPayload`, `ResponseResult`, `RequestError` | Defines WebSocket request/response interface |
| `game_connection_port.rs` | Same as above | Parent trait for WebSocket connection |
| `mock_game_connection.rs` | Same as above | Testing infrastructure |

All other ports should define their own types and have adapters translate from protocol types.

---

## Directory Structure (Crate-Based)

Hexagonal boundaries are enforced by **crate dependencies**.

### Core
- `crates/domain` (`wrldbldr-domain`): Core business entities, value objects, typed IDs
- `crates/protocol` (`wrldbldr-protocol`): **Shared Kernel** - wire-format DTOs

### Engine Side
| Crate | Layer | Purpose |
|-------|-------|---------|
| `engine-ports` | Ports | All engine port traits (inbound + outbound) |
| `engine-app` | Application | Services, use cases, app-layer DTOs |
| `engine-adapters` | Infrastructure | Neo4j, HTTP handlers, WebSocket server, LLM clients |
| `engine-runner` | Composition | Entry point, wiring, CLI |

### Player Side
| Crate | Layer | Purpose |
|-------|-------|---------|
| `player-ports` | Ports | All player port traits (+ Shared Kernel exceptions) |
| `player-app` | Application | Services, use cases |
| `player-adapters` | Infrastructure | HTTP/WS clients, platform adapters, message translation |
| `player-ui` | Presentation | Dioxus components, views, reactive state |
| `player-runner` | Composition | Entry point, wiring |

---

## Import Rules (Crate Boundaries)

### NEVER ALLOWED

```rust
// App importing adapters (violates dependency direction)
use wrldbldr_engine_adapters::*;  // in engine-app - FORBIDDEN

// Re-export shim (hides ownership)
pub use wrldbldr_protocol::GameTime;  // FORBIDDEN

// Crate alias shim
use wrldbldr_protocol as messages;  // FORBIDDEN
```

### ALLOWED

```rust
// Ports using Shared Kernel (documented exception)
use wrldbldr_protocol::{RequestPayload, ResponseResult};  // in request_port.rs - OK

// Adapters using protocol for translation
use wrldbldr_protocol::ServerMessage;  // in adapters - OK

// App using ports
use wrldbldr_engine_ports::outbound::BroadcastPort;  // in app - OK
```

---

## Domain Purity

The domain layer is **pure** - no I/O operations:

| Forbidden in Domain | Use Instead |
|---------------------|-------------|
| `rand::thread_rng()` | Inject `RandomPort` |
| `Utc::now()` | Inject `ClockPort` |
| `std::env::var()` | Inject `EnvPort` |
| File I/O | Inject repository port |
| Network calls | Inject service port |

This ensures:
- **Testability**: All behavior is deterministic when ports are mocked
- **Clarity**: Domain logic is pure business rules
- **Flexibility**: I/O strategies can change without touching domain

---

## Port Patterns

### Interface Segregation (ISP)

Large "god traits" are split into focused sub-traits:

```rust
// Before (god trait - 42 methods)
pub trait CharacterRepositoryPort: Send + Sync {
    async fn create(&self, ...) -> Result<()>;
    async fn get(&self, ...) -> Result<Option<Character>>;
    // ... 40 more methods
}

// After (ISP sub-traits)
pub trait CharacterCrudPort: Send + Sync {
    async fn create(&self, ...) -> Result<()>;
    async fn get(&self, ...) -> Result<Option<Character>>;
    // 6 focused methods
}

pub trait CharacterWantPort: Send + Sync {
    async fn create_want(&self, ...) -> Result<()>;
    // 7 focused methods
}

// Services depend on minimal interface
pub struct MyService {
    crud: Arc<dyn CharacterCrudPort>,  // Only what's needed
}
```

### Outbound vs Inbound Ports

```
┌─────────────────────────────────────────────────────────────────┐
│                       PORT DIRECTIONS                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  INBOUND PORTS (Driving)                                        │
│  ├── Define: What the app offers to external actors             │
│  ├── Called by: Adapters (HTTP handlers, WebSocket handlers)    │
│  ├── Example: ChallengeUseCasePort, MovementUseCasePort         │
│  └── Location: {crate}-ports/src/inbound/                       │
│                                                                 │
│  OUTBOUND PORTS (Driven)                                        │
│  ├── Define: What the app needs from external systems           │
│  ├── Implemented by: Adapters (Neo4j repos, HTTP clients)       │
│  ├── Example: CharacterCrudPort, BroadcastPort, ClockPort       │
│  └── Location: {crate}-ports/src/outbound/                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Composition Root

All dependency injection happens in the **runner** crate:

```rust
// crates/engine-runner/src/composition/app_state.rs

pub async fn build_app_state() -> AppState {
    // Create adapters (concrete types)
    let neo4j = Neo4jCharacterRepository::new(pool.clone());
    let clock = SystemClock::new();
    let broadcast = WebSocketBroadcastAdapter::new(conn_manager.clone());
    
    // Wire into services (as trait objects)
    let character_service = CharacterService::new(
        Arc::new(neo4j) as Arc<dyn CharacterCrudPort>,
        Arc::new(clock) as Arc<dyn ClockPort>,
    );
    
    // Bundle into app state
    AppState { character_service, ... }
}
```

**Only the runner knows about concrete types.** All other layers work with trait objects.

---

## Architecture Enforcement

### Automated Checks

Run `cargo xtask arch-check` to verify:
- No forbidden imports (adapters in app layer)
- No protocol imports outside Shared Kernel exceptions
- File size limits (500 lines max)
- Crate dependency direction

### Shared Kernel Whitelist

The arch-check maintains a whitelist of files approved for Shared Kernel usage:
- `request_port.rs`
- `game_connection_port.rs`
- `mock_game_connection.rs`

Any new file using protocol types in ports layer will fail arch-check.

---

## Testing Strategy

### Unit Tests (Domain/Application)

```rust
// Mock ports for deterministic testing
#[test]
fn test_dice_roll() {
    let fixed_rng = FixedRandomPort::new(vec![15]);  // Always rolls 15
    let formula = DiceFormula::parse("2d6+3")?;
    
    let result = formula.roll(|min, max| fixed_rng.gen_range(min, max));
    assert_eq!(result, 18);  // 15 + 3
}
```

### Integration Tests (Infrastructure)

```rust
#[tokio::test]
async fn test_neo4j_character_crud() {
    let pool = setup_test_database().await;
    let repo = Neo4jCharacterRepository::new(pool);
    
    // Test with real database
    let character = Character::new(...);
    repo.create(&character).await.unwrap();
    
    let loaded = repo.get(character.id).await.unwrap();
    assert_eq!(loaded, Some(character));
}
```

---

## Related Documents

- [Neo4j Schema](./neo4j-schema.md) - Database structure
- [WebSocket Protocol](./websocket-protocol.md) - Message types and wire format
- [Queue System](./queue-system.md) - Async job processing
- [CLAUDE.md](/CLAUDE.md) - AI assistant guidelines
- [AGENTS.md](/AGENTS.md) - Detailed crate-by-crate guidance
