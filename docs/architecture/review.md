# Code Review Guidelines

> **Purpose**: This document provides comprehensive guidelines for reviewing WrldBldr code. It is designed for reviewers (human or AI) who may have no prior context of the application.

---

## Table of Contents

1. [Application Overview](#application-overview)
2. [Rustic DDD Pattern](#rustic-ddd-pattern)
3. [System Architecture](#system-architecture)
4. [Layer Responsibilities](#layer-responsibilities)
5. [Review Criteria](#review-criteria)
6. [Anti-Patterns to Detect](#anti-patterns-to-detect)
7. [Full Codebase Review Checklist](#full-codebase-review-checklist)
8. [PR Review Checklist](#pr-review-checklist)

---

## Application Overview

**WrldBldr** is a TTRPG (tabletop role-playing game) management system with an AI-powered game master assistant.

### Components

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Engine** | Rust/Axum | Backend server: world management, AI orchestration, game state |
| **Player** | Rust/Dioxus | Client UI: visual novel interface, character management |
| **Domain** | Pure Rust | Business logic: aggregates, value objects, domain events |
| **Protocol** | Rust/Serde | Wire format for Engine <-> Player WebSocket communication |

### External Dependencies

| System | Purpose | Abstracted Via |
|--------|---------|----------------|
| Neo4j | Graph database for game state | Repository port traits |
| Ollama/LLM | AI text generation | `LlmPort` trait |
| ComfyUI | AI image generation | `ImageGenPort` trait |
| SQLite | Action/approval queues | `QueuePort` trait |

---

## Rustic DDD Pattern

WrldBldr follows **Rustic DDD** - an idiomatic Rust adaptation of Domain-Driven Design that leverages Rust's type system instead of porting Java/C# patterns.

### Core Principles

1. **Newtypes over runtime validation** - Invalid states are unrepresentable at compile time
2. **Ownership is encapsulation** - The borrow checker enforces aggregate boundaries
3. **Enums over boolean flags** - State machines are explicit and exhaustive
4. **Return types are domain events** - Mutations communicate what happened
5. **Concrete types internally** - Traits only at infrastructure boundaries

### Pattern Mapping

| Traditional DDD | Rustic Equivalent | Rust Mechanism |
|-----------------|-------------------|----------------|
| Private fields + getters | Newtypes valid by construction | `pub struct Name(String)` + validation in `::new()` |
| Aggregate root guards | Ownership enforcement | Struct owns its parts, borrow checker prevents leaks |
| Repository interface | Port traits | `#[async_trait] trait CharacterRepo` |
| Value Object immutability | No `&mut self` methods | `#[derive(Clone)]` + only `&self` methods |
| Factory pattern | Constructor + builder | `::new()` + `.with_*()` methods |
| Domain Events | Return enums from mutations | `fn apply_damage(&mut self) -> DamageOutcome` |
| State pattern | Enum state machines | `enum CharacterState { Active, Inactive, Dead }` |

### Value Object Requirements

Value objects must be:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]  // Auto-validate on deserialize
pub struct CharacterName(String);

impl CharacterName {
    /// Constructor validates and returns Result
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(DomainError::validation("Character name cannot be empty"));
        }
        if name.len() > 200 {
            return Err(DomainError::validation("Character name too long"));
        }
        Ok(Self(name))
    }

    /// Read-only accessor
    pub fn as_str(&self) -> &str { &self.0 }
}

// Required trait implementations for serde integration
impl TryFrom<String> for CharacterName {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> { Self::new(s) }
}

impl From<CharacterName> for String {
    fn from(name: CharacterName) -> String { name.0 }
}

impl Display for CharacterName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}
```

**Review Criteria for Value Objects:**
- [ ] Constructor returns `Result<Self, DomainError>`, not panics
- [ ] `#[serde(try_from, into)]` attributes present for String-based types
- [ ] Only `&self` methods (immutable after construction)
- [ ] `Display` implemented for user-facing types
- [ ] No public fields - inner value accessed via `.as_str()` or similar

### Aggregate Requirements

Aggregates must have:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    // ALL FIELDS PRIVATE
    id: CharacterId,
    world_id: WorldId,
    name: CharacterName,        // Newtype, not String
    state: CharacterState,      // Enum, not booleans
    stats: StatBlock,
}

impl Character {
    /// Constructor with required fields
    pub fn new(world_id: WorldId, name: CharacterName, archetype: CampbellArchetype) -> Self {
        Self {
            id: CharacterId::new(),  // UUID generated in domain is allowed (ADR-001)
            world_id,
            name,
            state: CharacterState::Active,
            stats: StatBlock::default(),
        }
    }

    // Builder methods return Self for chaining
    pub fn with_id(mut self, id: CharacterId) -> Self {
        self.id = id;
        self
    }

    // Read accessors - return references or Copy types
    pub fn id(&self) -> CharacterId { self.id }
    pub fn name(&self) -> &CharacterName { &self.name }
    pub fn is_alive(&self) -> bool { self.state.is_alive() }

    // Mutations RETURN domain events
    pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome {
        match self.state {
            CharacterState::Dead => DamageOutcome::AlreadyDead,
            _ => {
                // ... apply damage logic ...
                if new_hp <= 0 {
                    self.state = CharacterState::Dead;
                    DamageOutcome::Killed { damage_dealt: amount }
                } else {
                    DamageOutcome::Wounded { damage_dealt: amount, remaining_hp: new_hp }
                }
            }
        }
    }

    // Setters for mutable fields
    pub fn set_description(&mut self, desc: Description) {
        self.description = desc;
    }
}
```

**Review Criteria for Aggregates:**
- [ ] All fields are private (no `pub` on struct fields)
- [ ] Constructor is `::new()` with required parameters
- [ ] Builder methods (`.with_*()`) for optional fields, return `Self`
- [ ] Read accessors for all fields that need external access
- [ ] Mutations return domain events (enums describing what happened)
- [ ] Newtypes used for validated strings (`CharacterName`, not `String`)
- [ ] Enums used for state machines (`CharacterState`, not `is_alive: bool`)
- [ ] No I/O or async code in aggregates

### Domain Event Requirements

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageOutcome {
    AlreadyDead,
    Wounded { damage_dealt: i32, remaining_hp: i32 },
    Killed { damage_dealt: i32 },
    NoHpTracking,
}
```

**Review Criteria for Domain Events:**
- [ ] Enum variants describe what happened (past tense or outcome)
- [ ] Variants carry relevant data for callers to react
- [ ] Used as return types from aggregate mutations
- [ ] No side effects - purely data

---

## System Architecture

### Crate Structure

```
crates/
  domain/       # Pure business types (NO async, NO I/O)
    src/
      aggregates/       # Aggregate roots with private fields
      value_objects/    # Immutable validated types
      events/           # Domain events (return types)
      entities/         # Supporting types (not aggregates)
      ids.rs            # Typed IDs (CharacterId, LocationId, etc.)
      error.rs          # DomainError

  protocol/     # Wire format for WebSocket messages
    src/
      messages.rs       # ClientMessage, ServerMessage enums

  engine/       # All server-side code
    src/
      stores/           # In-memory state (sessions, pending staging, etc.)
      use_cases/        # Business orchestration (multi-repository)
      infrastructure/   # External system adapters
        ports.rs        # ~10 port trait definitions
        neo4j/          # Neo4j implementations (23 files)
        ollama.rs       # LLM client
        comfyui.rs      # Image generation
      api/              # Entry points
        http.rs         # HTTP routes
        websocket/      # WebSocket handlers (24 modules)
      app.rs            # Composition root

  player/       # All client-side code
    src/
      application/      # Client business logic
      infrastructure/   # Platform adapters
      ui/               # Dioxus components
```

### Dependency Rules (STRICT)

```
domain  <--  protocol  <--  engine
                              |
                              v
                           player
```

| From | May Import | Must NOT Import |
|------|-----------|-----------------|
| `domain` | `serde`, `uuid`, `chrono`, `thiserror` | `tokio`, `axum`, `neo4rs`, `engine`, `player`, `protocol` |
| `protocol` | `domain`, `serde` | `engine`, `player` |
| `engine` | `domain`, `protocol` | `player` |
| `player` | `domain`, `protocol` | `engine` |

### Port Traits (~10 Total)

Port traits exist ONLY for infrastructure that might realistically be swapped:

| Port | Location | Purpose |
|------|----------|---------|
| `CharacterRepo` | `infrastructure/ports.rs` | Character persistence |
| `LocationRepo` | `infrastructure/ports.rs` | Location persistence |
| `WorldRepo` | `infrastructure/ports.rs` | World persistence |
| `PlayerCharacterRepo` | `infrastructure/ports.rs` | PC persistence |
| `SceneRepo` | `infrastructure/ports.rs` | Scene persistence |
| `NarrativeRepo` | `infrastructure/ports.rs` | Event persistence |
| `LlmPort` | `infrastructure/ports.rs` | LLM text generation |
| `ImageGenPort` | `infrastructure/ports.rs` | Image generation |
| `QueuePort` | `infrastructure/ports.rs` | Action queues |
| `ClockPort` | `infrastructure/ports.rs` | Time (for testing) |
| `RandomPort` | `infrastructure/ports.rs` | Randomness (for testing) |

**What Does NOT Get Abstracted:**
- Use case to use case calls
- Repository to repository calls
- Handler to use case calls
- Internal orchestration

---

## Layer Responsibilities

### Domain Layer (`crates/domain/`)

**Purpose:** Pure business logic, no I/O

**Contains:**
- Aggregates (private fields, behavior methods)
- Value Objects (validated, immutable)
- Domain Events (mutation return types)
- Typed IDs (`CharacterId`, etc.)
- Domain errors

**Rules:**
- NO `async` functions
- NO I/O (database, network, file system)
- NO framework imports (tokio, axum, etc.)
- `Uuid::new_v4()` allowed for ID generation (ADR-001)
- `Utc::now()` NOT allowed (inject via ClockPort)

### Stores Layer (`crates/engine/src/stores/`)

**Purpose:** In-memory runtime state (NOT database wrappers)

**Contains:**
- `SessionStore` - WebSocket connection tracking
- `PendingStagingStore` - Approval workflow state
- `DirectorialStore` - DM context state
- `TimeSuggestionStore` - Time suggestion cache

**Rules:**
- Named `*Store` (not `*Repository`)
- Manages ephemeral runtime state
- Not persistence wrappers

**Note:** There is no repository wrapper layer. Per [ADR-009](ADR-009-repository-layer-elimination.md), use cases inject port traits directly.

### Use Case Layer (`crates/engine/src/use_cases/`)

**Purpose:** Business orchestration, injecting port traits directly

**Contains:**
- One struct per use case
- Injects port traits (`Arc<dyn *Repo>`)
- Coordinates domain logic

**Rules:**
- Named `{Verb}{Noun}` (e.g., `EnterRegion`, `StartConversation`)
- Inject port traits directly (not wrapper classes)
- Return domain types or use-case-specific results
- Has its own error type with context

```rust
pub struct EnterRegion {
    player_character: Arc<dyn PlayerCharacterRepo>,
    staging: Arc<dyn StagingRepo>,
    narrative: Arc<dyn NarrativeRepo>,
}

impl EnterRegion {
    pub async fn execute(&self, input: EnterRegionInput) -> Result<EnterRegionResult, MovementError> {
        // 1. Validate
        // 2. Orchestrate domain logic
        // 3. Persist changes
        // 4. Return result
    }
}
```

### API Layer (`crates/engine/src/api/`)

**Purpose:** HTTP and WebSocket entry points

**Contains:**
- HTTP route handlers
- WebSocket message handlers
- Protocol type conversion

**Rules:**
- Call use cases, not repositories directly
- Convert protocol types to domain types
- Handle authentication/authorization
- Log errors, return appropriate responses

### Infrastructure Layer (`crates/engine/src/infrastructure/`)

**Purpose:** External system implementations

**Contains:**
- Port trait implementations
- Neo4j repository implementations
- Ollama LLM client
- ComfyUI image generation client

**Rules:**
- One implementation per port trait
- Handle external system errors
- Convert external types to domain types

---

## Review Criteria

### 1. Architecture Violations

| Violation | How to Detect | Severity |
|-----------|--------------|----------|
| Domain imports engine | `use crate::*` or `use engine::*` in domain/ | CRITICAL |
| Domain imports tokio/axum | `use tokio::` or `use axum::` in domain/ | CRITICAL |
| Domain performs I/O | `async fn` in domain/, file/network calls | CRITICAL |
| Use case imports API types | `use crate::api::` in use_cases/ | HIGH |
| Public fields on aggregate | `pub field_name:` in aggregates/*.rs | HIGH |
| String instead of newtype | `name: String` instead of `name: CharacterName` | MEDIUM |
| Booleans instead of enum | `is_alive: bool, is_active: bool` | MEDIUM |
| Mutation without return | `fn apply_damage(&mut self)` returns `()` | MEDIUM |
| Port trait in wrong location | Trait defined outside ports.rs | LOW |

### 2. Security Issues

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Cypher injection | `format!()` in Cypher queries without SAFETY comment | CRITICAL |
| Secret in code | Hardcoded passwords, API keys, tokens | CRITICAL |
| Missing input validation | User input used without parsing/validation | HIGH |
| Unwrap on user input | `.unwrap()` on parse results from external input | HIGH |
| Excessive permissions | Endpoint doesn't check authorization | MEDIUM |
| Missing security audit step | No documented secrets scan/authz review | MEDIUM |

```rust
// CORRECT - parameterized query
let query = query("MATCH (c:Character {id: $id}) RETURN c")
    .param("id", id.to_string());

// WRONG - injection vulnerability
let query = query(&format!("MATCH (c:Character {{id: '{}'}}) RETURN c", id));
```

### 3. Error Handling

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Silent unwrap | `.unwrap()` on Result without justification | HIGH |
| Lost error context | `.map_err(|_| SomeError::Generic)` | MEDIUM |
| Panic in library code | `panic!()`, `unreachable!()` without comment | MEDIUM |
| Missing `?` propagation | Manual match on Result when `?` would work | LOW |
| Inconsistent error mapping | Ad-hoc string errors across layers | MEDIUM |

```rust
// WRONG - loses context
repo.get(id).await.map_err(|_| MyError::NotFound)?;

// CORRECT - preserves context
repo.get(id).await.map_err(|e| MyError::repo("get character", e))?;
```

### 4. Type Safety

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Raw Uuid instead of typed ID | `fn get(id: Uuid)` instead of `fn get(id: CharacterId)` | HIGH |
| String instead of newtype | `name: String` for validated data | MEDIUM |
| Option for required field | `Option<T>` when None is never valid | MEDIUM |
| Magic strings | Hardcoded strings that should be enums | LOW |

### 5. Performance

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Unbounded collection | `HashMap::new()` or `Vec::new()` without size limit | MEDIUM |
| Missing index | Query on unindexed field (check neo4j-schema.md) | MEDIUM |
| N+1 queries | Loop with database call inside | MEDIUM |
| Blocking in async | `std::thread::sleep` or sync I/O in async fn | HIGH |

### 6. Code Duplication

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Repeated validation | Same validation logic in multiple places | MEDIUM |
| Repeated queries | Same Cypher pattern in multiple files | MEDIUM |
| Repeated error mapping | Same error conversion in multiple handlers | LOW |
| Copy-paste tests | Identical test structure with different data | LOW |

### 7. Testing

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Missing VCR cassette | New LLM call without recorded response | HIGH |
| Flaky timing | `sleep()` or time-dependent assertions | MEDIUM |
| Missing error case test | Only happy path tested | MEDIUM |
| Test without assertion | Test that runs but doesn't assert | LOW |

---

## Anti-Patterns to Detect

### 1. Anemic Domain Model

**Symptom:** Aggregates are just data containers, all logic in use cases

```rust
// WRONG - anemic aggregate
pub struct Character {
    pub hp: i32,
    pub max_hp: i32,
    pub is_alive: bool,
}

// Use case does all the work
fn apply_damage(character: &mut Character, amount: i32) {
    character.hp -= amount;
    if character.hp <= 0 {
        character.is_alive = false;
    }
}
```

```rust
// CORRECT - rich domain model
impl Character {
    pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome {
        // Logic lives in the aggregate
    }
}
```

### 2. Primitive Obsession

**Symptom:** Using primitives for domain concepts

```rust
// WRONG
pub struct Character {
    pub name: String,           // Could be empty!
    pub email: String,          // Could be invalid!
    pub age: i32,               // Could be negative!
}

// CORRECT
pub struct Character {
    name: CharacterName,        // Guaranteed non-empty, <= 200 chars
    email: Email,               // Guaranteed valid format
    age: Age,                   // Guaranteed >= 0
}
```

### 3. Boolean Blindness

**Symptom:** Multiple booleans for mutually exclusive states

```rust
// WRONG - what if is_alive=false && is_active=true?
pub struct Character {
    pub is_alive: bool,
    pub is_active: bool,
    pub is_hidden: bool,
}

// CORRECT - impossible states are unrepresentable
pub enum CharacterState {
    Active,
    Inactive,
    Dead,
    Hidden,
}
```

### 4. Stringly Typed

**Symptom:** Strings for things that should be enums or typed IDs

```rust
// WRONG
fn get_character(id: &str, character_type: &str) -> Character;

// CORRECT
fn get_character(id: CharacterId, character_type: CharacterType) -> Character;
```

### 5. God Object

**Symptom:** Single struct/module doing too much

```rust
// WRONG - one struct handles everything
pub struct GameManager {
    // 50+ fields
    // 100+ methods
}

// CORRECT - separate concerns
pub struct CharacterRepository { ... }
pub struct LocationRepository { ... }
pub struct EnterRegion { ... }
pub struct StartConversation { ... }
```

### 6. Leaky Abstraction

**Symptom:** Infrastructure details leak into domain

```rust
// WRONG - domain knows about Neo4j
impl Character {
    pub fn to_neo4j_map(&self) -> HashMap<String, Value> { ... }
}

// CORRECT - serialization is infrastructure concern
// Domain just uses #[derive(Serialize, Deserialize)]
```

### 7. Shotgun Surgery

**Symptom:** One change requires modifying many files

If adding a new field requires changes in:
- Domain struct
- Repository save/load
- Multiple use cases
- Multiple handlers
- Multiple tests

This may indicate coupling issues. Consider if the field belongs at a different level.

---

## Full Codebase Review Checklist

Use this checklist when doing a comprehensive review of the entire codebase.

### Architecture

- [ ] All aggregates in `domain/src/aggregates/` have private fields
- [ ] All validated strings use newtypes (check for `name: String` patterns)
- [ ] All state machines use enums (check for `is_*: bool` patterns)
- [ ] All mutations return domain events (check for `fn modify(&mut self)` without return)
- [ ] Domain crate has no async functions
- [ ] Domain crate imports only allowed dependencies (serde, uuid, chrono, thiserror)
- [ ] All port traits are defined in `infrastructure/ports.rs`
- [ ] No more than ~25 port traits total (20 repos + 5 services)
- [ ] In-memory stores are in `stores/`, use cases in `use_cases/`
- [ ] No repository wrapper layer (use cases inject ports directly per ADR-009)
- [ ] Use cases inject port traits (`Arc<dyn *Repo>`), not wrapper classes

### Security

- [ ] All Cypher queries use parameters (search for `format!` in neo4j/)
- [ ] No secrets in code (search for "password", "secret", "token", "key")
- [ ] All user input validated at API boundaries
- [ ] No `.unwrap()` on user-provided data parsing

#### Secrets Scan Commands

Run when auditing for secrets:

```bash
# Search for potential secrets (review matches manually)
rg -i "password|secret|api_key|apikey|token|credential" \
   --type rust -g '!*.md' -g '!target/*' \
   -g '!*test*.rs' -g '!*fixture*.rs'

# Verify .env is not tracked
git ls-files | grep -E '\.env$'  # Should return nothing

# Check for high-entropy strings (potential keys)
rg '[A-Za-z0-9/+=]{32,}' --type rust -g '!target/*'
```

**Acceptable patterns:**
- `TEST_*_PASSWORD` in test harnesses
- `wrldbldr123` default in docker-compose (local dev only)
- `secret_agenda`, `LoreCategory::Secret` - domain terminology
- `TokenUsage`, `max_tokens` - LLM context budgets (not auth tokens)

#### Authentication State (Pre-Auth)

**Current implementation:**
- No server-side authentication
- User identity is client-generated UUID stored in browser localStorage
- `user_id` in `ConnectionInfo` set from client during `JoinWorld`
- DM role check (`require_dm()`) validates connection role, not user identity

**Trust boundary:**
- Client-provided `user_id` is trusted for session continuity only
- Not suitable for access control without server-side verification

**Code locations to audit when implementing auth:**
- `api/connections.rs`: `set_user_id()` should validate tokens
- `api/websocket/mod.rs`: WebSocket upgrade should verify auth
- `use_cases/session/join_world_flow.rs`: Trust server identity, not client

### Consistency

- [ ] Aggregate constructors follow `::new()` + `.with_*()` pattern
- [ ] Value object constructors return `Result<Self, DomainError>`
- [ ] Error types have context (entity type, ID, operation)
- [ ] Repository methods follow `get`, `save`, `delete`, `list_*` naming
- [ ] Use case methods follow `execute` naming
- [ ] Workspace has no unused dependencies

### Testing

- [ ] All LLM calls have VCR cassettes
- [ ] Domain tests don't use mocking (pure functions)
- [ ] Neo4j infrastructure tests use testcontainers
- [ ] Use case tests mock port traits directly
- [ ] No flaky tests (timing, ordering)

### Documentation

- [ ] Public types have doc comments
- [ ] Complex logic has inline comments explaining why
- [ ] ADRs exist for significant architectural decisions
- [ ] Logging/telemetry expectations documented for key flows
- [ ] Lint/format baselines documented (clippy + formatting)

---

## PR Review Checklist

Use this checklist when reviewing a pull request.

### Quick Checks

```bash
cargo check --workspace
cargo test --workspace --lib
cargo clippy --workspace -- -D warnings
```

### Code Changes

- [ ] Files are in the correct layer (domain/repository/use_case/api)
- [ ] New aggregates have private fields
- [ ] New value objects validate in constructor
- [ ] New mutations return domain events
- [ ] No new `pub` fields on existing aggregates
- [ ] No new booleans for state machines
- [ ] No new `String` fields for validated data

### New Features

- [ ] Use case has error type with context
- [ ] Repository methods are async
- [ ] Handler validates input before calling use case
- [ ] Tests cover happy path and error cases
- [ ] VCR cassettes for new LLM calls

### Database Changes

- [ ] Cypher queries use parameters
- [ ] New query patterns have indexes (check neo4j-schema.md)

### API Changes

- [ ] Protocol types added if new messages
- [ ] Backward compatibility considered
- [ ] Error responses are informative but not leaky

---

## Related Documents

| Document | Purpose |
|----------|---------|
| [AGENTS.md](../../AGENTS.md) | Architecture overview for AI agents |
| [REVIEW_CHECKLIST.md](../REVIEW_CHECKLIST.md) | Quick reference checklist |
| [neo4j-schema.md](neo4j-schema.md) | Database schema and indexes |
| [ADR-001](ADR-001-uuid-generation-in-domain.md) | UUID generation in domain |
| [ADR-002](ADR-002-hexagonal-pragmatism.md) | Pragmatic hexagonal architecture |
| [templates/NEW_MODULE.md](../templates/NEW_MODULE.md) | Templates for new code |
