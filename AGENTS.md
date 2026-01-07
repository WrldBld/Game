# WrldBldr Agent Guidelines

## Architecture (Read This First)

**Source of truth**: `docs/plans/SIMPLIFIED_ARCHITECTURE.md`

### Core Principle

**Hexagonal architecture is for infrastructure boundaries only.** Internal code calls internal code directly as concrete types.

### Crate Structure (4 crates)

```
crates/
  domain/       # Pure business types (entities, value objects, typed IDs)
  protocol/     # Wire format for Engine <-> Player communication
  engine/       # All server-side code
  player/       # All client-side code
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
  entities/           # Entity operations (one file per domain entity)
    character.rs      # All character operations
    location.rs       # Location/region operations
    scene.rs          # Scene management
    challenge.rs      # Challenge/dice operations
    narrative.rs      # Events, triggers, chains
    staging.rs        # NPC presence
    observation.rs    # Player knowledge
    inventory.rs      # Items
    assets.rs         # Asset generation
    world.rs          # World operations
    
  use_cases/          # User story orchestration
    movement/         # Player movement
      enter_region.rs
      exit_location.rs
    conversation/     # NPC dialogue
      start.rs
      continue.rs
    challenge/        # Dice rolls
      roll.rs
      resolve.rs
    approval/         # DM approval flows
      staging.rs
      suggestion.rs
    ...
    
  infrastructure/     # External dependencies
    ports.rs          # All port trait definitions
    neo4j/            # Database implementation
    ollama.rs         # LLM client
    comfyui.rs        # Image generation
    queue.rs          # SQLite queues
    clock.rs          # System clock
    
  api/                # Entry points
    http.rs           # HTTP routes
    websocket/        # WebSocket handling + routing
      mod.rs          # Connection lifecycle + dispatch
      ws_core.rs      # Core RequestPayload handlers
      ws_creator.rs   # Generation/AI/Expression requests
      ws_lore.rs      # Lore requests
      ws_story_events.rs # Story event requests
    
  app.rs              # App struct
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
  screens/            # UI screens (Dioxus routes)
    connection.rs
    game.rs
    dm_dashboard.rs
    
  components/         # Reusable UI components
    scene_view.rs
    dialogue_box.rs
    inventory_grid.rs
    
  use_cases/          # Player actions
    connection/
      connect.rs
    game/
      move_to_region.rs
      talk_to_npc.rs
    dm/
      approve_staging.rs
      
  features/           # Client-side state
    connection.rs
    game_state.rs
    settings.rs
    
  infrastructure/
    websocket/        # WebSocket client
      mod.rs
    storage/
      web.rs
      desktop.rs
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
| `docs/plans/SIMPLIFIED_ARCHITECTURE.md` | Architecture spec and migration progress |
| `docs/architecture/neo4j-schema.md` | Database schema |
| `docs/architecture/websocket-protocol.md` | Client-server protocol |
| `docs/systems/*.md` | Game system specifications |

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
