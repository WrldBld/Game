# WrldBldr Agent Guidelines

## Product Vision

**WrldBldr is a digital tabletop RPG platform that merges human Dungeon Masters with AI assistance to create a visual novel-style gameplay experience.**

### The Core Problem

Running a rich, responsive TTRPG world is exhausting for human DMs. You need to voice NPCs on the fly, remember who's where, track relationships, maintain narrative consistency - all while players do unexpected things.

### The Solution: DM-in-the-Loop AI

WrldBldr puts an **AI assistant alongside the DM**, not replacing them. The AI handles the heavy lifting (generating NPC dialogue, deciding who's at a location, suggesting appropriate challenges), while the DM retains **absolute authority** - every AI decision goes through an approval queue before players see it.

### Two User Experiences

**Players** see a visual novel interface:
- Backdrop images of locations (tavern, market, dungeon)
- Character sprites of NPCs present
- Dialogue boxes with typewriter text animation
- Choices and action buttons

**DMs** see a control panel showing:
- What the AI proposes (dialogue, NPC presence, challenges)
- The AI's reasoning and suggested tool calls
- Approve/modify/reject buttons
- Directorial guidance injection

### Key Design Principles

1. **Graph-First World Model**: Everything in Neo4j - locations contain regions, characters have wants targeting goals, NPCs have location affinities, narrative events have triggers and effects. Rich queries enable deep AI context.

2. **Theatre Metaphor (Staging)**: When players enter a region, the system determines "who's on stage" via rule-based logic + LLM reasoning, with DM approval. Results are cached with configurable TTL.

3. **Character Psychology**: NPCs use Campbell's Hero's Journey archetypes and Greimas's Actantial Model (wants, helpers, opponents, senders, receivers) - giving them internal logic the AI can reason about.

4. **Multi-System Support**: Same platform supports D&D, Call of Cthulhu, Fate, etc. - different dice mechanics, same rich narrative tools.

5. **AI-Generated Assets**: ComfyUI integration generates portraits, sprites, and backdrops on demand with style consistency.

### What Makes It Different

| Traditional VTT | WrldBldr |
|-----------------|----------|
| AI runs the game OR human does | AI proposes, human approves |
| NPCs are stat blocks | NPCs have psychology (wants, archetypes, relationships) |
| Relationships in notes | Relationships as graph edges with sentiment |
| Text chat interface | Visual novel presentation |
| Single rule system | Multi-system support |

---

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

### Tiered Encapsulation (ADR-008)

Not all domain types need the same level of encapsulation. Use the right tool for the job:

| Type Category | Encapsulation Level | When to Use |
|---------------|---------------------|-------------|
| **Aggregates** | Private fields + accessors + mutation methods | Types with invariants to protect (e.g., `Character`, `Challenge`, `StatBlock`) |
| **Validated Newtypes** | Newtype wrapper with `::new()` validation | Strings/values with rules (e.g., `CharacterName`, `Description`, `Tag`) |
| **Typed IDs** | Newtype wrapper (always) | All identifiers (e.g., `CharacterId`, `LocationId`) |
| **Simple Data Structs** | Public fields | No invariants, just data grouping (e.g., `MapBounds`, `TimeAdvanceResult`, DTOs) |
| **Enums** | Public variants | State machines, outcomes, choices |

**Decision criteria for encapsulation:**

1. **Does it have invariants?** (e.g., "name cannot be empty", "hp cannot exceed max_hp")
   - Yes → Private fields + validation in constructor
   - No → Public fields are fine

2. **Can invalid states be constructed?**
   - Yes → Encapsulate to prevent
   - No → Public fields are fine

3. **Is it just grouping related data?** (coordinates, results, snapshots)
   - Yes → Public fields, derive `Debug, Clone, Serialize, Deserialize`

**Examples:**

```rust
// AGGREGATE: Has invariants (hp <= max_hp, name validated)
pub struct Character {
    id: CharacterId,        // Private
    name: CharacterName,    // Private, validated newtype
    current_hp: i32,        // Private, constrained by max_hp
    max_hp: i32,
}

// SIMPLE DATA STRUCT: No invariants, just coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBounds {
    pub x: f64,             // Public - no invalid states
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// RESULT/DTO: No invariants, just carries data
#[derive(Debug, Clone)]
pub struct TimeAdvanceResult {
    pub new_time: GameTime,
    pub events_triggered: Vec<NarrativeEvent>,
}

// VALIDATED NEWTYPE: Has rules (non-empty, max length)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct CharacterName(String);

impl CharacterName {
    pub fn new(s: impl Into<String>) -> Result<Self, DomainError> { ... }
}
```

**Anti-pattern: Over-encapsulation**

```rust
// WRONG: Pointless encapsulation for a coordinate struct
pub struct MapBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl MapBounds {
    pub fn x(&self) -> f64 { self.x }
    pub fn y(&self) -> f64 { self.y }
    pub fn width(&self) -> f64 { self.width }
    pub fn height(&self) -> f64 { self.height }
    pub fn with_x(mut self, x: f64) -> Self { self.x = x; self }
    // ... 50 lines of boilerplate for no benefit
}
```

**Reviewing encapsulation (for code reviewers/agents):**

When auditing encapsulation decisions, don't just count getters—analyze the full type:

1. **Read the entire impl block** - A struct with 12 trivial getters may also have `validate_triggers()`, `evaluate_roll()`, or state machine methods that justify encapsulation
2. **Look for mutation methods with constraints** - Methods like `use_slot()`, `start_generating()`, or `reorder_events()` indicate invariants
3. **Check if removing encapsulation would expose violations** - Could callers create invalid states with public fields?
4. **Ask "what would break?"** not "how many lines saved?" - Line count reduction is not the goal; protecting invariants is

**Common false positive:** Counting getters in `entities/` files and assuming they're over-encapsulated. Many entities have business logic methods beyond accessors (e.g., `Challenge.evaluate_roll()`, `EventChain.current_position` tracking).

See [ADR-008](docs/architecture/ADR-008-tiered-encapsulation.md) for rationale.

### Crate Structure (4 crates)

```
crates/
  domain/       # Pure business types (aggregates, value objects, typed IDs, events)
  shared/       # Shared contracts: wire format, game system traits, content types
  engine/       # All server-side code
  player/       # All client-side code (Dioxus UI + platform adapters)
```

**Why `shared` not `protocol`?** The crate contains more than wire format:
- Wire format types (WebSocket messages, DTOs)
- Game system traits (`GameSystem`, `CompendiumProvider`, `CalculationEngine`)
- Content types (`ContentItem`, `ContentType`) for spells, feats, races, etc.

These must be shared because Player needs them for UI rendering and character creation.

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
5. **Business mutations return events** - `fn apply_damage(&mut self, amount: i32) -> DamageOutcome`

**When to return domain events vs `()`:**

| Method Type | Return | Example |
|-------------|--------|---------|
| Multiple possible outcomes | Domain event enum | `apply_damage` → `DamageOutcome::{Wounded, Killed, AlreadyDead}` |
| State machine transitions | Domain event enum | `activate` → `CharacterStateChange::{Activated, AlreadyActive}` |
| Pure setters (one outcome) | `()` | `set_description(&mut self, desc)` - caller knows what they set |

Domain events are valuable when the caller needs to know **what happened**, not just that something happened. Pure setters have exactly one outcome - the value is now set - so events would be ceremony without value.

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
  stores/             # In-memory state (NOT database wrappers)
    mod.rs
    session.rs        # WebSocket connection state (wraps ConnectionManager)
    pending_staging.rs # Pending approval state
    directorial.rs    # Directorial context state
    time_suggestion.rs # Time suggestion state

  use_cases/          # Business logic orchestration (injects port traits directly)
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

### Port Injection (ADR-009)

Use cases inject port traits **directly** - there is no repository wrapper layer.

**Why no repository layer?**
- Port traits already define the data access contract
- Wrapper classes that just delegate add no value (~2,300 lines of boilerplate)
- Business logic belongs in use cases, not data access wrappers
- See [ADR-009](docs/architecture/ADR-009-repository-layer-elimination.md) for full rationale

**What about the `stores/` directory?**

The `stores/` directory contains **in-memory state**, not database wrappers:
- `SessionStore` - WebSocket connection tracking
- `PendingStagingStore` - Approval workflow state
- `DirectorialStore` - DM context state
- `TimeSuggestionStore` - Time suggestion cache

These are legitimate because they manage runtime state, not database access.

### Use Case Design Rules (STRICT)

1. **Inject port traits directly** - `Arc<dyn CharacterRepo>`, not wrapper classes
2. **Orchestrate multiple operations** - Coordinate ports + domain logic
3. **Return domain types or use-case DTOs** - Never shared/wire types as return values

**DO**:
- Return domain aggregates, value objects, or use-case-specific result structs
- Define result structs in the use case module if needed (e.g., `EnterRegionResult`)
- Use domain types for all internal logic

**DON'T**:
- Return `wrldbldr_shared` wire types directly from use cases
- Build wire-format responses (that's the API layer's job)
- Embed `serde_json::Value` in results

**Why?** Use cases are business logic. Wire format concerns belong in the API layer (`api/websocket/`, `api/http.rs`). This separation:
- Allows testing use cases without serialization concerns
- Prevents protocol changes from breaking business logic
- Keeps use cases focused on domain operations

### Protocol Conversion Patterns (ADR-011)

The following patterns are **CORRECT** and should NOT be flagged as violations:

**1. `to_protocol()` helper methods on use case types:**
```rust
// use_cases/staging/types.rs - METHOD lives here
impl StagedNpc {
    pub fn to_protocol(&self) -> wrldbldr_shared::StagedNpcInfo { ... }
}

// api/websocket/ws_staging.rs - CALLED from API layer
let response = staged_npc.to_protocol();  // Conversion happens at correct boundary
```
What matters is WHEN conversion happens (API layer), not WHERE the method is defined.

**2. Shared re-exports of domain types:**
```rust
// These are DOMAIN types, not wire format:
use wrldbldr_shared::{CharacterSheetValues, SheetValue, GameTime};

// Because shared re-exports them from domain:
// shared/src/lib.rs: pub use wrldbldr_domain::types::{CharacterSheetValues, ...};
```
The `shared` crate contains both wire format types AND domain type re-exports. Using re-exported domain types is correct.

**3. `from_protocol()` conversion helpers:**
```rust
impl DirectorialUpdateInput {
    pub fn from_protocol(wire: wrldbldr_shared::DirectorialContext) -> Self {
        Self { context: ports::DirectorialContext { ... } }  // Domain type internally
    }
}
```
The helper is called from API handlers; the use case works with domain types internally.

**Anti-pattern - Architecture Theater:**
Moving `to_protocol()` methods to the API layer would require exposing all internal fields via accessors, breaking encapsulation without changing when conversion happens. See [ADR-011](docs/architecture/ADR-011-protocol-conversion-boundaries.md).

**Example Use Case:**
```rust
// engine/src/use_cases/movement/enter_region.rs
pub struct EnterRegion {
    character_repo: Arc<dyn CharacterRepo>,
    staging_repo: Arc<dyn StagingRepo>,
    narrative_repo: Arc<dyn NarrativeRepo>,
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

- **4 crates**: domain, shared, engine, player
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
- Import from engine, player, or shared
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

### Error Taxonomy and Mapping

The codebase uses a layered error system. Follow these standards for consistency:

#### Error Types by Layer

| Layer | Error Type | Purpose |
|-------|------------|---------|
| Domain | `DomainError` | Validation failures, business rule violations |
| Infrastructure | `RepoError`, `LlmError`, `QueueError` | External system failures |
| Use Cases | Per-use-case enums (`ConversationError`, etc.) | Orchestration failures |
| API/Protocol | `ErrorCode` enum | Client-facing error codes |

#### Domain → Use Case Mapping

Use `#[from]` to preserve error chains:

```rust
// CORRECT - preserves error chain
#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

// WRONG - loses error context
pub enum ConversationError {
    #[error("Queue error: {0}")]
    QueueError(String),  // Don't stringify!
}
```

#### Use Case → API Mapping

Map use case errors to `ErrorCode` consistently:

| Use Case Error Pattern | ErrorCode |
|------------------------|-----------|
| `*::NotFound`, `RepoError::NotFound{..}` | `ErrorCode::NotFound` |
| `*::Unauthorized`, `*::Forbidden` | `ErrorCode::Unauthorized` |
| `*::Validation*`, `DomainError::Validation*` | `ErrorCode::ValidationError` |
| `*::Conflict` | `ErrorCode::Conflict` |
| `*::Timeout`, `LlmError::Timeout` | `ErrorCode::Timeout` |
| Other/Unknown | `ErrorCode::InternalError` |

#### WebSocket Error Responses

Always use typed `ErrorCode`, never string codes:

```rust
// CORRECT - typed error code
use wrldbldr_shared::ErrorCode;

fn handle_error(err: ConversationError) -> ServerMessage {
    let code = match &err {
        ConversationError::NotFound(_) => ErrorCode::NotFound,
        ConversationError::Unauthorized => ErrorCode::Unauthorized,
        _ => ErrorCode::InternalError,
    };
    ServerMessage::Error {
        code: code.as_str().to_string(),
        message: error_sanitizer::sanitize(&err),
    }
}

// WRONG - string-based error code
ServerMessage::Error {
    code: "NOT_FOUND".to_string(),  // Don't use raw strings!
    message: "...".to_string(),
}
```

#### Error Context Requirements

Errors should carry enough context for debugging:

```rust
// CORRECT - includes entity type and ID
RepoError::NotFound { 
    entity_type: "Character", 
    id: character_id.to_string() 
}

// WRONG - no context
RepoError::Generic("not found".to_string())
```

### Fail-Fast Error Philosophy

WrldBldr uses fail-fast error handling where errors bubble up to the appropriate user:

| Error Type | Target | How |
|------------|--------|-----|
| Player action error | Player | WebSocket `Error` message |
| DM action error | DM | WebSocket `Error` message |
| System/infrastructure error | Both + logs | Generic message to user, full context to logs |

**DO:**
- Propagate errors with `?` operator
- Log context before converting to user-friendly errors
- Include entity IDs and operation names in error context

**DON'T:**
- Silently swallow errors with `if let Err(e) = ... { log }` then returns `Ok`
- Use `.ok()` without logging what was lost
- Use `let _ =` on Results without documenting why
- Use `unwrap()` on Results without justification

**When to use fallbacks (warn + default):**
- Non-critical data enrichment (e.g., optional asset paths)
- Backward compatibility during migrations
- Always log a warning so issues are discoverable

**Pattern for fallback with logging:**
```rust
let value = match input.parse::<TargetType>() {
    Ok(v) => v,
    Err(e) => {
        tracing::warn!(
            input = %input,
            error = %e,
            "Failed to parse, using default"
        );
        TargetType::default()
    }
};
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

### New Port Trait

1. Add port trait to `engine/src/infrastructure/ports.rs`
2. Implement in `engine/src/infrastructure/neo4j/{name}.rs`

Note: There is no repository wrapper layer. Use cases inject port traits directly (ADR-009).

### New Use Case

1. Create `engine/src/use_cases/{category}/{name}.rs`
2. Inject required port traits directly (`Arc<dyn *Repo>`)
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
- Infrastructure (Neo4j): Integration tests with testcontainers
- Use cases: Mock port traits directly

### Integration Tests
- Use testcontainers for Neo4j
- Test full flows with real database

### E2E Tests with VCR
- LLM calls are recorded and replayed via VCR cassettes
- Run in `record` mode to capture new LLM responses
- Run in `playback` mode for CI (deterministic, fast)

```rust
#[tokio::test]
async fn enter_region_updates_position() {
    let mut mock_repo = MockCharacterRepo::new();
    mock_repo.expect_update_position()
        .returning(|_, _| Ok(()));

    let character_repo: Arc<dyn CharacterRepo> = Arc::new(mock_repo);
    let use_case = EnterRegion::new(character_repo, ...);

    let result = use_case.execute(pc_id, region_id).await;
    assert!(result.is_ok());
}
```

---

## Code Review

For comprehensive code review guidelines, including:
- Full Rustic DDD pattern specification
- Architecture violation detection
- Anti-pattern identification
- PR and full codebase review checklists

See **[docs/architecture/review.md](docs/architecture/review.md)**

Quick reference: **[docs/REVIEW_CHECKLIST.md](docs/REVIEW_CHECKLIST.md)**
