# WrldBldr Agent Guidelines

## Architecture

### Core Principles

1. **Hexagonal architecture for infrastructure boundaries only** - Port traits (~10 total) at database, LLM, and external system boundaries
2. **Rustic DDD** - Idiomatic Rust domain-driven design leveraging ownership, newtypes, and enums
3. **Internal code uses concrete types** - No abstraction layers within the same crate

### Rustic DDD Philosophy

Instead of porting Java/C# DDD patterns, we leverage Rust's strengths:

| Java DDD Pattern | Rustic Equivalent |
|------------------|-------------------|
| Private fields + getters | **Newtypes** valid by construction |
| Aggregate root guards | **Ownership** (borrow checker enforces) |
| Value Object immutability | `#[derive(Clone)]` + no `&mut` methods |
| Factory pattern | `::new()` + builder pattern |
| Domain Events | Return enums from mutations |

### Crate Structure (4 crates)

```
crates/
  domain/       # Pure business types (aggregates, value objects, typed IDs, events)
  protocol/     # Wire format for Engine <-> Player communication
  engine/       # All server-side code
  player/       # All client-side code (Dioxus UI + platform adapters)
```

### What Gets Abstracted (Port Traits)

Only infrastructure that might realistically be swapped:

| Boundary | Trait | Why |
|----------|-------|-----|
| Database | `CharacterRepo`, `LocationRepo`, etc. | Could swap Neo4j -> Postgres |
| LLM | `LlmPort` | Could swap Ollama -> Claude/OpenAI |
| Image Generation | `ImageGenPort` | Could swap ComfyUI -> other |
| Queues | `QueuePort` | Could swap SQLite -> Redis |
| Clock/Random | `ClockPort`, `RandomPort` | For testing |

**~10 port traits total.** Everything else is concrete types.

### What Does NOT Get Abstracted

- Aggregate-to-aggregate calls (all in same crate)
- Use case orchestration
- Handler-to-use-case calls
- Application state

---

## Domain Crate Structure

```
domain/src/
  lib.rs              # Re-exports
  error.rs            # DomainError
  ids.rs              # Typed IDs (CharacterId, LocationId, etc.)

  aggregates/         # Aggregate roots (own their data, private fields)
    mod.rs
    character.rs      # Character aggregate
    location.rs       # Location aggregate (owns regions)
    world.rs          # World aggregate
    scene.rs          # Scene aggregate
    player_character.rs
    narrative_event.rs
    challenge.rs
    item.rs

  value_objects/      # Immutable, no identity, validated by construction
    mod.rs
    names.rs          # CharacterName, LocationName, WorldName, Description
    stat_block.rs     # StatBlock with modifiers
    archetype.rs      # CampbellArchetype enum
    mood.rs           # MoodState, DispositionLevel
    expression.rs     # ExpressionConfig

  events/             # Domain events (return types from mutations)
    mod.rs
    character_events.rs  # DamageOutcome, HealOutcome, ArchetypeShift
    combat_events.rs     # ChallengeOutcome
```

### Aggregate Design Rules

1. **Private fields** - All aggregate fields are private
2. **Accessors for reading** - `fn name(&self) -> &CharacterName`
3. **Newtypes for validated data** - `CharacterName` not `String`
4. **Enums for state machines** - `CharacterState` not `is_alive: bool`
5. **Mutations return events** - `fn apply_damage(&mut self, amount: i32) -> DamageOutcome`

**Example Aggregate:**
```rust
// domain/src/aggregates/character.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    id: CharacterId,          // Private
    name: CharacterName,      // Newtype (validated)
    state: CharacterState,    // Enum (not booleans)
    stats: StatBlock,         // Owned
}

impl Character {
    pub fn new(world_id: WorldId, name: CharacterName, archetype: CampbellArchetype) -> Self { ... }

    // Read accessors
    pub fn id(&self) -> CharacterId { self.id }
    pub fn name(&self) -> &CharacterName { &self.name }
    pub fn is_alive(&self) -> bool { self.state.is_alive() }

    // Mutations return events
    pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome {
        // ... modify state ...
        DamageOutcome::Wounded { damage_dealt: amount, remaining_hp: new_hp }
    }
}
```

### Value Object Design Rules

1. **Valid by construction** - Constructor validates, returns `Result<Self, DomainError>`
2. **Serde integration** - `#[serde(try_from = "String")]` for automatic validation on deserialize
3. **Display trait** - Implement for easy string conversion
4. **No mutation** - Only `&self` methods (except builder patterns that consume self)

**Example Value Object:**
```rust
// domain/src/value_objects/names.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CharacterName(String);

impl CharacterName {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(DomainError::validation("Character name cannot be empty"));
        }
        if name.len() > 200 {
            return Err(DomainError::validation("Character name cannot exceed 200 characters"));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl TryFrom<String> for CharacterName {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> { Self::new(s) }
}

impl From<CharacterName> for String {
    fn from(name: CharacterName) -> String { name.0 }
}
```

### Domain Event Design Rules

1. **Enums with data** - Each variant carries relevant information
2. **Descriptive variants** - Names describe what happened
3. **Used as return types** - From aggregate mutation methods

**Example Domain Event:**
```rust
// domain/src/events/character_events.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageOutcome {
    AlreadyDead,
    Wounded { damage_dealt: i32, remaining_hp: i32 },
    Killed { damage_dealt: i32 },
    NoHpTracking,
}
```

---

## Engine Structure

```
engine/src/
  repositories/       # Data access wrappers around port traits
    mod.rs
    character.rs      # CharacterRepository (wraps CharacterRepo port)
    location.rs       # LocationRepository
    world.rs          # WorldRepository
    player_character.rs
    scene.rs
    narrative.rs
    staging.rs
    observation.rs
    inventory.rs
    goal.rs
    act.rs
    assets.rs
    settings.rs
    lore.rs
    skill.rs
    flag.rs
    interaction.rs
    location_state.rs
    region_state.rs

  use_cases/          # Multi-repository orchestration
    movement/         # Player movement (enter_region, exit_location)
    conversation/     # NPC dialogue (start, continue, end)
    challenge/        # Challenge flows
    narrative/        # Trigger evaluation, effect execution
    approval/         # DM approval flows
    staging/          # NPC staging flows
    session/          # Session management
    visual_state/     # Scene visual state
    assets/           # Asset generation
    world/            # World import/export
    queues/           # Queue processing
    time/             # Game time
    player_action/    # Player action processing
    actantial/        # Goals, wants
    ai/               # AI/LLM orchestration
    lore/             # Lore management
    npc/              # NPC behavior
    story_events/     # Story events
    location_events/  # Location events
    scene/            # Scene resolution

  infrastructure/     # External dependencies
    ports.rs          # All port trait definitions (~10 traits)
    neo4j/            # Database implementation (23 repository files)
    ollama.rs         # LLM client
    comfyui.rs        # Image generation
    queue.rs          # SQLite queues
    clock.rs          # System clock
    settings.rs       # Settings infrastructure

  api/                # Entry points
    connections.rs    # Connection management
    http.rs           # HTTP routes
    websocket/        # WebSocket handling (24 handler modules)

  app.rs              # App struct (composition root)
  main.rs             # Entry point
```

### Repository Design Rules

1. **Named `*Repository`** - `CharacterRepository`, not `Character`
2. **Wraps port trait** - `repo: Arc<dyn CharacterRepo>`
3. **Async CRUD methods** - `async fn get(&self, id: CharacterId) -> Result<Option<Character>, RepoError>`
4. **No business logic** - Only data access

**Example Repository:**
```rust
// engine/src/repositories/character.rs
pub struct CharacterRepository {
    repo: Arc<dyn CharacterRepo>,
}

impl CharacterRepository {
    pub fn new(repo: Arc<dyn CharacterRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: CharacterId) -> Result<Option<domain::Character>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, character: &domain::Character) -> Result<(), RepoError> {
        self.repo.save(character).await
    }

    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Character>, RepoError> {
        self.repo.list_in_world(world_id).await
    }
}
```

### Repositories vs Use Cases

| Criteria | Layer | Example |
|----------|-------|---------|
| 1-2 repo dependencies | `repositories/` | `CharacterRepository` (1 port) |
| 3+ repository dependencies | `use_cases/` | `EnterRegion` (3+ repos) |
| Pure data access | `repositories/` | `get`, `save`, `list` |
| Business orchestration | `use_cases/` | Movement, conversation, challenges |

### Use Case Design Rules

1. **Inject repositories** - Not port traits directly
2. **Orchestrate multiple operations** - Coordinate repos + domain logic
3. **Return domain types** - Not raw database records

**Example Use Case:**
```rust
// engine/src/use_cases/movement/enter_region.rs
pub struct EnterRegion {
    character_repo: Arc<CharacterRepository>,
    staging_repo: Arc<StagingRepository>,
    narrative_repo: Arc<NarrativeRepository>,
}

impl EnterRegion {
    pub async fn execute(&self, pc_id: PlayerCharacterId, region_id: RegionId) -> Result<EnterRegionResult, MovementError> {
        let npcs = self.staging_repo.resolve_for_region(region_id).await?;
        let events = self.narrative_repo.check_triggers(region_id, pc_id).await?;
        self.character_repo.update_position(pc_id, region_id).await?;
        Ok(EnterRegionResult { npcs, events })
    }
}
```

---

## Project Overview

### Key Facts

- **4 crates**: domain, protocol, engine, player
- **Backend**: Axum HTTP + WebSocket server
- **Frontend**: Dioxus (WASM + Desktop)
- **Database**: Neo4j graph database
- **AI**: Ollama LLM + ComfyUI image generation

### Project Objectives

1. **Pure Graph Model**: All game state in Neo4j as nodes and edges
2. **AI Game Master**: LLM-driven NPC dialogue and narrative generation
3. **DM Approval Flow**: Human oversight of AI-generated content
4. **Session-based Multiplayer**: Real-time WebSocket communication
5. **Asset Generation**: ComfyUI integration for character/scene artwork

---

## Player Structure

```
player/src/
  application/        # Application logic layer
    api.rs            # API orchestration
    error.rs          # Error types
    services/         # Business services (16+ modules)
    dto/              # Data transfer objects

  infrastructure/     # External adapters
    websocket/        # Platform-specific WebSocket
    messaging/        # Message bus
    platform/         # Platform abstractions
    storage.rs        # Storage implementations
    http_client.rs    # HTTP client

  ports/              # Port trait definitions
    outbound/         # Outbound ports

  ui/                 # User interface (Dioxus)
    presentation/
      views/          # Page-level components
      components/     # Reusable UI components
      state/          # UI state management
      handlers/       # Event handlers
    routes/           # Dioxus routing

  state/              # Dependency injection
  main.rs             # Entry point
  lib.rs              # Library exports
```

---

## Domain Rules (STRICT)

The domain crate must be **pure**:

**DO**:
- Use `serde`, `uuid`, `chrono`, `thiserror` only
- Use typed IDs (`CharacterId`, not raw `Uuid`)
- Use newtypes for validated strings (`CharacterName`, not `String`)
- Use enums for state machines (`CharacterState`, not booleans)
- Return events from mutations (`DamageOutcome`, not `()`)
- Implement business logic in aggregate methods

**DON'T**:
- Import tokio, axum, neo4rs, or any framework
- Import from engine, player, or protocol
- Call `Utc::now()` - inject via `ClockPort`
- Use `rand` - inject via `RandomPort`
- Perform any I/O
- Use public fields on aggregates

**Exception**: `Uuid::new_v4()` is allowed for ID generation (ADR-001).

---

## Common Pitfalls

### Neo4j Injection

**CRITICAL**: Never concatenate user input into Cypher:

```rust
// CORRECT
let query = query("MATCH (c:Character {id: $id}) RETURN c")
    .param("id", id.to_string());

// WRONG - injection vulnerability
let query = query(&format!("MATCH (c:Character {{id: '{}'}}) RETURN c", id));
```

### Typed IDs

Always use typed IDs:

```rust
// CORRECT
fn get_character(&self, id: CharacterId) -> Character;

// WRONG
fn get_character(&self, id: Uuid) -> Character;
```

### Newtypes for Validated Data

Always use newtypes for validated strings:

```rust
// CORRECT
pub struct Character {
    name: CharacterName,  // Guaranteed valid
}

// WRONG
pub struct Character {
    pub name: String,  // Could be empty or too long
}
```

### State Enums over Booleans

Use enums for mutually exclusive states:

```rust
// CORRECT
pub enum CharacterState { Active, Inactive, Dead }

// WRONG
pub is_alive: bool,
pub is_active: bool,  // What if is_alive=false && is_active=true?
```

### Return Events from Mutations

Return what happened, don't just mutate:

```rust
// CORRECT
pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome { ... }

// WRONG
pub fn apply_damage(&mut self, amount: i32) { ... }  // Caller has no idea what happened
```

### Error Handling

Never swallow errors:

```rust
// WRONG
let result = repo.get(id).unwrap();

// CORRECT
let result = repo.get(id).await?;
```

### Dioxus Hooks (CRITICAL)

**Hooks must be called unconditionally at the top of components.** Never call hooks inside conditionals, loops, or closures.

```rust
// CORRECT - hooks at top level
#[component]
pub fn MyComponent() -> Element {
    let navigator = use_navigator();
    let mut state = use_signal(|| 0);
    // ...
}

// WRONG - hook inside conditional causes RefCell panic
#[component]
pub fn MyComponent() -> Element {
    if some_condition {
        let navigator = use_navigator();  // PANIC!
    }
    // ...
}
```

---

## Development Workflow

### Before Committing

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```

### Running

```bash
# Engine
cargo run -p wrldbldr-engine

# Player (desktop)
cargo run -p wrldbldr-player

# Player (WASM)
dx serve --platform web
```

---

## Adding Features

### New Aggregate

1. Create `domain/src/aggregates/{name}.rs` with private fields
2. Add value objects if needed in `domain/src/value_objects/`
3. Add domain events if needed in `domain/src/events/`
4. Export from `domain/src/aggregates/mod.rs`

### New Repository

1. Create `engine/src/repositories/{name}.rs`
2. Add port trait methods if needed in `infrastructure/ports.rs`
3. Implement in `engine/src/infrastructure/neo4j/`

### New Use Case

1. Create `engine/src/use_cases/{category}/{name}.rs`
2. Inject required repositories
3. Add to `App` struct
4. Wire in API layer

### New API Endpoint

1. Add handler to `engine/src/api/http.rs` or `engine/src/api/websocket/`
2. Call appropriate use case
3. Add protocol types if needed

---

## Key Documentation

| Document | Purpose |
|----------|---------|
| `docs/plans/RUSTIC_DDD_REFACTOR.md` | Current refactoring plan |
| `docs/architecture/neo4j-schema.md` | Database schema |
| `docs/architecture/websocket-protocol.md` | Client-server protocol |
| `docs/architecture/ADR-*.md` | Architecture decision records |
| `docs/systems/*.md` | Game system specifications |
| `docs/designs/*.md` | Feature design documents |

---

## Testing

### Unit Tests
- Domain: Pure tests, no mocking (aggregates, value objects)
- Repositories: Mock port traits
- Use cases: Mock repositories

### Integration Tests
- Use testcontainers for Neo4j
- Test full flows with real database

```rust
#[tokio::test]
async fn enter_region_updates_position() {
    let mut repo = MockCharacterRepo::new();
    repo.expect_update_position()
        .returning(|_, _| Ok(()));

    let character_repo = CharacterRepository::new(Arc::new(repo));
    let use_case = EnterRegion::new(character_repo, ...);

    let result = use_case.execute(pc_id, region_id).await;
    assert!(result.is_ok());
}
```
