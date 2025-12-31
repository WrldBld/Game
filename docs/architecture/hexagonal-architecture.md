# Hexagonal Architecture (Target)

> **Status**: Canonical architecture spec for WrldBldr.
>
> This document defines the **target** hexagonal architecture we’re refactoring toward.
> It is written to be:
> - **Internally consistent**
> - **Enforceable** via crate boundaries + `cargo xtask arch-check`
> - **Easy to follow** for new contributors and AI agents
>
> If you only read one doc before making architectural changes, read this one.

## Goals

1. **Clear dependency direction**: business logic never depends on infrastructure.
2. **Stable boundaries**: domain + ports are long-lived; adapters are replaceable.
3. **No type duplication**: each “kind” of data has a single canonical home.
4. **WASM-friendly player**: keep wire-format and player contracts small and stable.
5. **Mechanical enforcement**: where possible, rules are enforced by tooling.

## The hexagon (one sentence)

**Use cases** implement the app’s API (inbound ports) and orchestrate domain logic by calling **outbound ports**, which are implemented by adapters.

## Crates and layers

WrldBldr enforces boundaries primarily through **crate dependencies**.

### Domain (innermost)

- `crates/domain-types` — shared vocabulary (archetypes, story structures)
- `crates/domain` — entities, value objects, invariants, typed IDs

**Domain must be pure**:
- no framework crates (tokio/axum/neo4rs/etc.)
- no I/O
- no `Utc::now()` (inject clock)
- no randomness (inject random)

### Shared kernel

- `crates/protocol` — Engine ↔ Player wire-format types (Serde DTOs)
- `crates/common` — pure helpers (WASM-safe)

**Protocol is not domain**:
- protocol types are for serialization and version-tolerant communication
- business semantics belong in `domain`, not `protocol`

### Ports (contracts)

- `crates/engine-ports`
- `crates/player-ports`

Ports contain **traits + boundary DTOs**. No implementations.

### Application (use cases + services)

- `crates/engine-app`
- `crates/player-app`

Application code implements inbound ports and uses outbound ports.

### Adapters (infrastructure)

- `crates/engine-adapters` — Neo4j, Axum handlers, WebSocket server, LLM/ComfyUI clients
- `crates/player-adapters` — WebSocket client, storage, platform connectors

Adapters implement outbound ports and translate between:
- protocol DTOs ↔ app/port DTOs
- DB rows/records ↔ domain entities

### Runners (composition roots)

- `crates/engine-runner`
- `crates/player-runner`

Only runners are allowed to construct concrete implementations and wire everything together.

## Dependency rules (MUST)

### Crate dependency direction

Allowed dependency edges:

- `domain` → (`domain-types`, `common`*)
- `{engine,player}-ports` → `domain` (and very limited `protocol`, see whitelist)
- `{engine,player}-app` → `domain`, `{engine,player}-ports`
- `{engine,player}-adapters` → `{engine,player}-ports`, `domain`, `protocol`, external crates
- `{engine,player}-runner` → everything
- `player-ui` → `protocol`, `player-app` (presentation boundary)

\* `common` in domain should remain tiny and pure; prefer not depending on it unless necessary.

### Inbound vs outbound ports (canonical meaning)

This repository uses the terms **inbound** and **outbound** in a strict sense:

- **Inbound ports (driving ports)**
    - Define what the application *offers* to the outside world.
    - Called by: adapters (HTTP/WS handlers), UI/presentation.
    - Implemented by: application (use cases).
    - Examples: `MovementUseCasePort`, `SceneUseCasePort`, `ChallengeUseCasePort`.

- **Outbound ports (driven ports)**
    - Define what the application *needs* from the outside world.
    - Implemented by: adapters.
    - Depended on by: application (use cases/services).
    - Examples: repositories (`*CrudPort`), `ClockPort`, `RandomPort`, `BroadcastPort`, `LlmPort`.

**Hard rule**:
> Application **use cases and services may depend only on outbound ports** (plus domain + simple context DTOs).

### Naming and location rule

- Inbound ports live in:
    - `crates/engine-ports/src/inbound/`
    - `crates/player-ports/src/inbound/`

- Outbound ports live in:
    - `crates/engine-ports/src/outbound/`
    - `crates/player-ports/src/outbound/`

If a trait is a dependency of use cases/services, it belongs in **outbound**.

This resolves the current ambiguity where “service-ish” traits live under `inbound`.

## DTO ownership model (no duplication)

To avoid type duplication and adapter/app confusion, every DTO must have a single canonical home.

### 1) Domain types

If it has business semantics and invariants, it belongs in `domain`.

Examples:
- `Challenge`, `NarrativeEvent`, `Staging`
- typed IDs (`WorldId`, `CharacterId`)

### 2) Protocol types

If it is part of Engine ↔ Player communication, it belongs in `protocol`.

Rules:
- protocol types are **Serde-focused**
- add `#[serde(other)]` variants for forward-compatible enums
- do not import protocol into domain

### 3) Port boundary DTOs

If it is used *inside the engine* across app↔adapter boundaries, it belongs with the port that owns it:

- `engine-ports/src/outbound/...` for outbound port DTOs
- `engine-ports/src/inbound/...` for inbound request/response DTOs

These DTOs should be stable and (where possible) independent of protocol.

### 4) Engine-internal DTOs (`engine-dto`)

`engine-dto` exists for **engine-internal** data that is not protocol, not domain, and not a port boundary.

Good candidates:
- queue payloads that are internal to the engine runner/services
- persistence snapshots and projections
- internal handler context objects

**Not allowed**:
- duplicating DTOs that already exist as port DTOs (example: duplicate `StagingProposal`)

**Target rule**:
> No “shadow copies” of port DTOs in `engine-dto`.

### Practical rule of thumb

If you have to write a conversion like:
- `engine_dto::X` ↔ `engine_ports::...::X`

…that’s usually a smell and a sign the DTO has the wrong owner.

## System-to-boundary mapping

WrldBldr’s systems map naturally to use cases (inbound ports) and to a small set of outbound ports.

### Core (foundational) systems

- **Navigation**: locations, regions, movement
    - Use cases: movement, region/location browsing
    - Ports: location/region CRUD, connections, time/clock

- **Character**: PCs/NPCs, stats, inventory
    - Use cases: inventory operations, character selection
    - Ports: character CRUD, inventory CRUD, item repository

### AI + DM approval systems

- **Challenge**: dice rolls, outcomes, triggers
    - Use cases: submit roll, trigger challenge, approve outcomes
    - Ports: challenge repository, broadcast, approval queue, rule system, RNG/clock

- **Narrative**: triggers, effects, event chains
    - Use cases: approve narrative suggestions, browse library
    - Ports: narrative CRUD, trigger evaluation inputs, broadcast, approval queue

- **Dialogue**: LLM suggestions, tool calls
    - Use cases: submit player action, DM approve tool/event/challenge suggestions
    - Ports: LLM port, tool execution, queues

- **Staging (NPC presence)**
    - Use cases: staging approval, pre-staging, movement integration
    - Ports: staging repository, NPC relationships, narrative context providers, clock

### Asset system

- **Asset**: ComfyUI generation, workflow config, gallery assets
    - Use cases: queue generation, retry, cancel, set active assets
    - Ports: workflow repository, generation queue, storage/file system adapter, ComfyUI client

## Known current violations (tracked)

These are high-impact known gaps to refactor toward this target architecture:

- **Port taxonomy drift**: some traits currently under `engine-ports/inbound` are used as outbound dependencies.
- **Port adapter anti-pattern**: avoid adding wrapper-forwarder adapters under `engine-adapters/src/infrastructure/ports.rs` (and sibling modules); prefer implementing ports directly on the underlying infrastructure types.
- **Concrete dependencies in application**:
    - use cases depending on concrete use cases
    - services depending on concrete service types instead of ports
- **DTO duplication**:
    - `StagingProposal` exists in both `engine-dto` and `engine-ports`.

See:
- `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`

## Enforcement (tooling)

### Existing

- `cargo xtask arch-check` is the canonical boundary enforcement gate.

### Target (add over time)

We will extend checks to enforce:

1. No use-case or service depends on a trait from `engine-ports/src/inbound/` other than:
     - the inbound port it implements
     - `UseCaseContext` (DTO)

2. No `engine-dto` type duplicates a port DTO (by name or via explicit “X ↔ X” conversions).

3. No composition root stores concrete types where a port trait exists.

## FAQ

### “Can the player depend on the domain crate?”

It can today, but it carries a cost (WASM size and accidental coupling). The target direction is:

- Player UI and player-app should prefer `protocol` types.
- If player needs shared semantics, use `domain-types` (small) instead of full `domain`.

This will be revisited as part of the protocol↔domain decoupling work.

### “Where do I put a new DTO?”

- If it’s wire format: `protocol`
- If it’s business semantics: `domain`
- If it’s app↔adapter contract: the owning port module (`engine-ports`)
- If it’s internal glue: `engine-dto`

## Edit policy

- This document is the **source of truth**.
- If the codebase contradicts this document, we either:
    - refactor the code to match, or
    - document an explicit exception (with a link to an ADR / plan).

---

## Shared Kernel Pattern

### What is the Shared Kernel?

The **Shared Kernel** layer contains crates that provide shared functionality across both Engine and Player sides without containing business logic.

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
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  wrldbldr-common                                         │   │
│  │                                                         │   │
│  │  Pure Utility Functions:                                │   │
│  │  ├── datetime: parse_datetime, parse_datetime_or        │   │
│  │  └── string: none_if_empty, some_if_not_empty           │   │
│  │                                                         │   │
│  │  Characteristics:                                       │   │
│  │  ├── Minimal dependencies (only chrono)                 │   │
│  │  ├── Pure functions only - no side effects, no I/O      │   │
│  │  ├── No domain type dependencies                        │   │
│  │  ├── WASM compatible                                    │   │
│  │  └── Used by: engine-adapters                           │   │
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

These files in ports layers are approved to use protocol types directly:

#### Engine-Ports

| File | Protocol Types Used | Reason |
|------|---------------------|--------|
| `request_handler.rs` | `RequestPayload`, `ResponseResult` | Defines primary engine-player communication boundary |
| `dm_approval_queue_service_port.rs` | `ChallengeSuggestionInfo`, `NarrativeEventSuggestionInfo`, `ProposedToolInfo` | Queue items contain wire-format suggestion types |

#### Player-Ports

| File | Protocol Types Used | Reason |
|------|---------------------|--------|
| `request_port.rs` | `RequestPayload`, `ResponseResult`, `RequestError` | Defines WebSocket request/response interface |
| `game_connection_port.rs` | Same as above | Parent trait for WebSocket connection |
| `mock_game_connection.rs` | Same as above | Testing infrastructure |
| `session_types.rs` | Multiple DTO types | Provides `From` conversions between port-layer and protocol types |
| `player_events.rs` | `ChallengeSuggestionInfo`, `NarrativeEventSuggestionInfo`, `ProposedToolInfo` | Re-exports for event payloads |

All other ports should define their own types and have adapters translate from protocol types.

---

## Directory Structure (Crate-Based)

Hexagonal boundaries are enforced by **crate dependencies**.

### Core
- `crates/domain` (`wrldbldr-domain`): Core business entities, value objects, typed IDs

### Shared Kernel
- `crates/protocol` (`wrldbldr-protocol`): Wire-format DTOs for Engine↔Player communication
- `crates/common` (`wrldbldr-common`): Pure utility functions (datetime, string helpers)

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
│  ├── Implemented by: Use cases in application layer             │
│  ├── Example: ChallengeUseCasePort, MovementUseCasePort         │
│  └── Location: {crate}-ports/src/inbound/                       │
│                                                                 │
│  OUTBOUND PORTS (Driven)                                        │
│  ├── Define: What the app needs from external systems           │
│  ├── Implemented by: Adapters (Neo4j repos, HTTP clients)       │
│  ├── Depended on by: Use cases and services                     │
│  ├── Example: CharacterCrudPort, BroadcastPort, ClockPort       │
│  └── Location: {crate}-ports/src/outbound/                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Use Case Dependency Rule

**Use cases MUST depend only on outbound ports, never on inbound ports.**

```rust
// CORRECT: Use case depends on outbound ports
pub struct InventoryUseCase {
    pc_crud: Arc<dyn PlayerCharacterCrudPort>,      // outbound
    pc_inventory: Arc<dyn PlayerCharacterInventoryPort>, // outbound
    broadcast: Arc<dyn BroadcastPort>,              // outbound
}

// WRONG: Use case depends on inbound port
pub struct BadUseCase {
    some_service: Arc<dyn SomeInboundPort>,  // ANTI-PATTERN
}
```

**Why?**
- Inbound ports define what the application *offers* - use cases *implement* them
- Outbound ports define what the application *needs* - use cases *depend on* them
- Depending on inbound ports creates circular abstractions

**Exception**: Use cases may depend on `UseCaseContext` (a DTO, not a port trait).

See `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` for the single source-of-truth remediation plan.

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

The arch-check maintains a whitelist of files approved for Shared Kernel usage.

**Engine-Ports whitelist:**
- `request_handler.rs`
- `dm_approval_queue_service_port.rs`

**Player-Ports whitelist:**
- `request_port.rs`
- `game_connection_port.rs`
- `mock_game_connection.rs`
- `session_types.rs`
- `player_events.rs`

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
