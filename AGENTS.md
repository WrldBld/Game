# WrldBldr Agent Guidelines

## Architecture

### Core Principle

**Hexagonal architecture is for infrastructure boundaries only.** Internal code calls internal code directly as concrete types.

### Crate Structure (4 crates)

```
crates/
  domain/       # Pure business types (entities, value objects, typed IDs)
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

- Entity-to-entity calls (all in same crate)
- Use case orchestration
- Handler-to-use-case calls
- Application state

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

## Engine Structure

```
engine/src/
  entities/           # Repository facades (1-2 repo deps MAX, CRUD + simple queries only)
    character.rs      # Character CRUD operations
    player_character.rs # Player character CRUD
    location.rs       # Location/region CRUD
    location_state.rs # Location state tracking
    region_state.rs   # Region state tracking
    scene.rs          # Scene CRUD
    challenge.rs      # Challenge/dice operations
    narrative.rs      # Narrative CRUD (events, triggers)
    staging.rs        # NPC presence CRUD
    observation.rs    # Player knowledge
    inventory.rs      # Item CRUD
    goal.rs           # Goals (actantial targets)
    act.rs            # Actantial acts
    assets.rs         # Asset operations
    world.rs          # World CRUD
    settings.rs       # Global/world settings
    lore.rs           # Lore entries
    skill.rs          # Skill definitions
    flag.rs           # Game flags
    interaction.rs    # Interaction records

  use_cases/          # Multi-entity orchestration (coordinates entities, NOT repos directly)
    movement/         # Player movement (enter_region, exit_location, scene_change)
    conversation/     # NPC dialogue (start, continue, end)
    challenge/        # Challenge flows
    narrative/        # Narrative orchestration (trigger evaluation, effect execution)
    approval/         # DM approval flows
    staging/          # NPC staging flows
    session/          # Session management (join_world, directorial)
    visual_state/     # Scene visual state resolution
    assets/           # Asset generation flows
    world/            # World import/export
    queues/           # Queue processing
    time/             # Game time advancement
    player_action/    # Player action processing
    actantial/        # Goals, wants context
    ai/               # AI/LLM orchestration
    lore/             # Lore management
    npc/              # NPC behavior
    story_events/     # Story event handling
    location_events/  # Location-based events
    scene/            # Scene resolution logic

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
      mod.rs          # Connection lifecycle + dispatch
      ws_core.rs      # Core RequestPayload handlers
      ws_actantial.rs # Goal/Want/Actantial requests
      ws_approval.rs  # Approval decisions
      ws_challenge.rs # Challenge requests
      ws_conversation.rs # NPC dialogue
      ws_creator.rs   # Generation/AI/Expression requests
      ws_dm.rs        # DM-specific actions
      ws_event_chain.rs # Event chain requests
      ws_inventory.rs # Inventory operations
      ws_location.rs  # Location/Region requests
      ws_lore.rs      # Lore requests
      ws_movement.rs  # Movement requests
      ws_narrative_event.rs # Narrative event requests
      ws_player.rs    # PlayerCharacter requests
      ws_player_action.rs # Player action handling
      ws_scene.rs     # Scene requests
      ws_session.rs   # Session management
      ws_skill.rs     # Skill requests
      ws_staging.rs   # Staging requests
      ws_story_events.rs # Story event requests
      ws_time.rs      # Time advancement

  app.rs              # App struct (composition of entities + use_cases)
  main.rs             # Entry point
```

### Entity Modules

Encapsulate all operations for a domain concept:

```rust
// engine/src/entities/character.rs
pub struct Character {
    repo: Arc<dyn CharacterRepo>,
}

impl Character {
    pub async fn get(&self, id: CharacterId) -> Result<Option<domain::Character>> {
        self.repo.get(id).await
    }

    pub async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<domain::Character>> {
        self.repo.list_in_region(region_id).await
    }

    // All character operations...
}
```

### Entities vs Use Cases (Classification Rules)

**Entity modules** (`entities/`) wrap repository ports and provide direct domain operations. They are the single source of truth for interacting with domain data.

**Use cases** (`use_cases/`) orchestrate across multiple entities to fulfill user stories. They should **never duplicate** entity functionality.

#### Classification Criteria

| Criteria | Layer | Example |
|----------|-------|---------|
| 1-2 repo dependencies | `entities/` | `Character` (1 repo) |
| 3+ entity dependencies | `use_cases/` | `EnterRegion` (3+ entities) |
| <300 lines, pure CRUD | `entities/` | `Staging` (218 lines) |
| >300 lines, complex logic | `use_cases/` (or split) | `TriggerEvaluator` |
| Coordinates multiple entities | `use_cases/` | `Movement` flow |

#### Layer Rules

| Layer | Purpose | Example |
|-------|---------|---------|
| Entity | Direct domain operations | `inventory.get_pc_inventory(pc_id)` |
| Use Case | Multi-entity orchestration | `EnterRegion` (updates position + triggers events + resolves staging) |

#### Rules to Prevent Duplication

1. **Don't wrap entities in use cases** - If a use case just calls through to an entity, delete the use case
2. **Access entities directly when appropriate** - Handlers can call `app.entities.inventory` for simple operations
3. **Use cases are for orchestration** - Only create use cases when coordinating multiple entities
4. **No `*_operations.rs` in use_cases/** - Files named `*_operations.rs` belong in `entities/`

**Example - Settings access:**
```rust
// Settings is an entity mounted in use_cases for historical reasons
// Access directly - no wrapper layer
app.use_cases.settings.get_global().await?
app.use_cases.settings.get_for_world(world_id).await?

// NOT: app.use_cases.settings.ops.get_global() - this pattern is wrong
```

**Example - Inventory access:**
```rust
// Access entity directly from handlers
app.entities.inventory.get_pc_inventory(pc_id).await?
app.entities.inventory.equip_item(pc_id, item_id).await?

// NOT: app.use_cases.inventory.ops.get_pc_inventory() - dead code, deleted
```

### Use Cases

Orchestrate across entities for user stories:

```rust
// engine/src/use_cases/movement/enter_region.rs
pub struct EnterRegion {
    character: Arc<Character>,
    staging: Arc<Staging>,
    narrative: Arc<Narrative>,
}

impl EnterRegion {
    pub async fn execute(&self, pc_id: PcId, region_id: RegionId) -> Result<EnterRegionResult> {
        let npcs = self.staging.resolve_for_region(region_id).await?;
        let events = self.narrative.check_triggers(region_id, pc_id).await?;
        self.character.update_position(pc_id, region_id).await?;
        Ok(EnterRegionResult { npcs, events })
    }
}
```

### API Layer

Handlers call use cases directly:

```rust
// engine/src/api/http.rs
async fn move_to_region(
    State(app): State<Arc<App>>,
    Json(req): Json<MoveRequest>,
) -> Result<Json<MoveResponse>> {
    let result = app.use_cases.movement.enter_region
        .execute(req.pc_id, req.region_id)
        .await?;
    Ok(Json(result.into()))
}
```

---

## Player Structure

```
player/src/
  application/        # Application logic layer
    api.rs            # API orchestration
    error.rs          # Error types
    services/         # Business services (16+ modules)
      session.rs      # Session management
      challenge.rs    # Challenge handling
      character.rs    # Character operations
      observation.rs  # Knowledge tracking
      ...
    dto/              # Data transfer objects
      requests.rs     # Request types
      player_events.rs
      session_dto.rs
      websocket_messages.rs

  infrastructure/     # External adapters
    websocket/        # Platform-specific WebSocket
      desktop/        # Desktop WebSocket client
      wasm/           # WASM WebSocket client
      bridge.rs       # Platform abstraction
      protocol.rs     # Protocol handling
    messaging/        # Message bus
      command_bus.rs
      event_bus.rs
      connection.rs
    platform/         # Platform abstractions
      desktop.rs
      wasm.rs
    storage.rs        # Storage implementations
    http_client.rs    # HTTP client

  ports/              # Port trait definitions
    outbound/         # Outbound ports
      api_port.rs
      platform_port.rs
      player_events.rs

  ui/                 # User interface (Dioxus)
    presentation/
      views/          # Page-level components (9 views)
        main_menu.rs
        pc_view.rs
        dm_view.rs
        world_select.rs
        pc_creation.rs
        role_select.rs
        ...
      components/     # Reusable UI components (40+ organized by feature)
        visual_novel/   # Dialogue display
        dm_panel/       # DM controls (20+ components)
        story_arc/      # Narrative timeline
        inventory_panel/
        character_sheet_viewer/
        mini_map/
        settings/
        tactical/
        ...
      state/          # UI state management
        connection.rs
        game.rs
        dialogue.rs
        session.rs
        challenge.rs
        ...
      handlers/       # Event handlers
        session_event_handler.rs
        session_message_handler.rs
    routes/           # Dioxus routing

  state/              # Dependency injection
    platform.rs

  main.rs             # Entry point
  lib.rs              # Library exports
```

---

## Domain Rules (STRICT)

The domain crate must be **pure**:

**DO**:
- Use `serde`, `uuid`, `chrono`, `thiserror` only
- Use typed IDs (`CharacterId`, not raw `Uuid`)
- Implement business logic in entity methods

**DON'T**:
- Import tokio, axum, neo4rs, or any framework
- Import from engine, player, or protocol
- Call `Utc::now()` - inject via `ClockPort`
- Use `rand` - inject via `RandomPort`
- Perform any I/O

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
    let navigator = use_navigator();  // Always called
    let mut state = use_signal(|| 0);

    if some_condition {
        // Use navigator here, but don't call use_navigator()
        navigator.push(Route::Home {});
    }
    rsx! { ... }
}

// WRONG - hook inside conditional causes RefCell panic
#[component]
pub fn MyComponent() -> Element {
    if some_condition {
        let navigator = use_navigator();  // PANIC! Hook ordering changes
        navigator.push(Route::Home {});
    }
    rsx! { ... }
}
```

**Signal reads**: Avoid nested signal reads that hold borrows across other reads:

```rust
// WRONG - nested reads cause RefCell panic
let location = locations.read().iter()
    .find(|l| l.id == selected_id.read())  // Reading inside iterator
    .map(|l| l.name.clone());

// CORRECT - read signals separately before combining
let selected = selected_id.read().clone();
let location = locations.read().iter()
    .find(|l| Some(&l.id) == selected.as_ref())
    .map(|l| l.name.clone());
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

### New Entity Operations

1. Add to `engine/src/entities/{entity}.rs`
2. Add repository methods to port trait if needed
3. Implement in `engine/src/infrastructure/neo4j/`

### New Use Case

1. Create `engine/src/use_cases/{category}/{name}.rs`
2. Inject required entity modules
3. Add to `App` struct
4. Wire in API layer

### New API Endpoint

1. Add handler to `engine/src/api/http.rs` or `engine/src/api/websocket/mod.rs`
2. Call appropriate use case
3. Add protocol types if needed

---

## Key Documentation

| Document | Purpose |
|----------|---------|
| `docs/architecture/neo4j-schema.md` | Database schema |
| `docs/architecture/websocket-protocol.md` | Client-server protocol |
| `docs/systems/*.md` | Game system specifications |
| `docs/designs/*.md` | Feature design documents |
| `docs/progress/ACTIVE_DEVELOPMENT.md` | Current development status |

---

## Testing

### Unit Tests
- Domain: Pure tests, no mocking
- Entities: Mock repository ports
- Use cases: Mock entity modules

### Integration Tests
- Use testcontainers for Neo4j
- Test full flows with real database

```rust
#[tokio::test]
async fn enter_region_updates_position() {
    let mut repo = MockCharacterRepo::new();
    repo.expect_update_position()
        .returning(|_, _| Ok(()));
        
    let character = Character::new(Arc::new(repo));
    let use_case = EnterRegion::new(character, ...);
    
    let result = use_case.execute(pc_id, region_id).await;
    assert!(result.is_ok());
}
```
