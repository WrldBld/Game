# Simplified Architecture

**Status**: ACTIVE  
**Created**: 2026-01-03  
**Branch**: `new-arch`

## Overview

WrldBldr is being restructured from 11+ crates with 128+ port traits to a simpler 4-crate architecture with ~10 port traits. The goal is to maintain hexagonal benefits at real infrastructure boundaries while eliminating over-abstraction internally.

### Core Principle

**Hexagonal architecture is for infrastructure boundaries only.** Internal code calls internal code directly as concrete types.

### Crate Structure

```
crates/
  domain/       # Pure business types (entities, value objects, typed IDs)
  protocol/     # Wire format for Engine <-> Player communication
  engine/       # All server-side code
  player/       # All client-side code
```

---

## What Gets Abstracted (Port Traits)

Only infrastructure that might realistically be swapped:

| Boundary | Port Trait | Why Abstract |
|----------|-----------|--------------|
| Database | `CharacterRepo`, `LocationRepo`, etc. | Could swap Neo4j -> Postgres |
| LLM | `LlmPort` | Could swap Ollama -> Claude/OpenAI |
| Image Generation | `ImageGenPort` | Could swap ComfyUI -> other services |
| Queues | `QueuePort` | Could swap SQLite -> Redis/RabbitMQ |
| Clock | `ClockPort` | For deterministic testing |
| Random | `RandomPort` | For deterministic testing |
| Platform (player) | `StoragePort`, `PlatformPort` | WASM vs Desktop differences |

**Total: ~10 port traits** (down from 128+)

### What Does NOT Get Abstracted

- Internal feature-to-feature calls
- Use case orchestration
- Handler-to-feature calls (same crate)
- Application state management

---

## Engine Crate Structure

```
engine/src/
  entities/           # Entity-focused capabilities (data + operations)
    character.rs      # Character queries and mutations
    location.rs       # Location/region operations
    scene.rs          # Scene management
    challenge.rs      # Challenge/dice operations
    narrative.rs      # Narrative events, triggers, chains
    staging.rs        # NPC presence management
    observation.rs    # Player knowledge tracking
    inventory.rs      # Item operations
    assets.rs         # Asset generation orchestration
    world.rs          # World-level operations
    
  use_cases/          # User story orchestration (cross-entity)
    movement/         # Player movement flows
      enter_region.rs
      exit_location.rs
    conversation/     # NPC dialogue flows
      start.rs
      continue.rs
      tool_execution.rs
    challenge/        # Challenge flows
      roll.rs
      resolve_outcome.rs
    approval/         # DM approval flows
      approve_staging.rs
      approve_suggestion.rs
      approve_challenge.rs
    assets/           # Asset generation flows
      generate.rs
      retry.rs
    world/            # World management flows
      export.rs
      import.rs
      
  infrastructure/     # External dependency implementations
    ports.rs          # All port trait definitions
    neo4j/            # Database implementation
      mod.rs
      character_repo.rs
      location_repo.rs
      scene_repo.rs
      ...
    ollama.rs         # LLM client
    comfyui.rs        # Image generation client  
    queue.rs          # SQLite queue implementation
    clock.rs          # System clock
    random.rs         # Random number generation
    
  api/                # Entry points
    http.rs           # HTTP routes (calls use_cases)
    websocket/        # WebSocket handling (calls use_cases)
      mod.rs
    
  app.rs              # App struct (composition of entities + use_cases)
  main.rs             # Entry point, composition, server startup
  lib.rs              # Public exports
```

### Entity Modules

Each entity module encapsulates all operations for that domain concept:

```rust
// engine/src/entities/character.rs

pub struct Character {
    repo: Arc<dyn CharacterRepo>,
}

impl Character {
    pub fn new(repo: Arc<dyn CharacterRepo>) -> Self {
        Self { repo }
    }
    
    pub async fn get(&self, id: CharacterId) -> Result<Option<domain::Character>> {
        self.repo.get(id).await
    }
    
    pub async fn get_in_region(&self, region_id: RegionId) -> Result<Vec<domain::Character>> {
        self.repo.list_in_region(region_id).await
    }
    
    pub async fn update_position(&self, id: CharacterId, region_id: RegionId) -> Result<()> {
        self.repo.update_position(id, region_id).await
    }
    
    pub async fn get_relationships(&self, id: CharacterId) -> Result<Vec<Relationship>> {
        self.repo.get_relationships(id).await
    }
    
    // All character-related operations...
}
```

### Use Case Modules

Use cases orchestrate across entities for user stories:

```rust
// engine/src/use_cases/movement/enter_region.rs

pub struct EnterRegion {
    character: Arc<Character>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
}

impl EnterRegion {
    pub async fn execute(&self, pc_id: PcId, region_id: RegionId) -> Result<EnterRegionResult> {
        // 1. Validate the move is possible
        let region = self.location.get_region(region_id).await?
            .ok_or(Error::RegionNotFound)?;
            
        // 2. Check/resolve NPC staging
        let npcs = self.staging.resolve_for_region(region_id).await?;
        
        // 3. Update player's observation state
        self.observation.record_visit(pc_id, region_id, &npcs).await?;
        
        // 4. Check for triggered narrative events
        let events = self.narrative.check_triggers(region_id, pc_id).await?;
        
        // 5. Execute the position update
        self.character.update_position(pc_id, region_id).await?;
        
        Ok(EnterRegionResult { region, npcs, events })
    }
}
```

### API Layer

HTTP/WebSocket handlers call use cases directly:

```rust
// engine/src/api/http.rs

pub fn routes(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/worlds", get(list_worlds).post(create_world))
        .route("/api/worlds/:id/export", get(export_world))
        // ... etc
        .with_state(app)
}

async fn export_world(
    State(app): State<Arc<App>>,
    Path(world_id): Path<Uuid>,
) -> Result<Json<WorldExport>, ApiError> {
    let result = app.use_cases.world.export
        .execute(WorldId::from_uuid(world_id))
        .await?;
    Ok(Json(result))
}
```

### App Struct

Simple composition of all entities and use cases:

```rust
// engine/src/app.rs

pub struct App {
    pub entities: Entities,
    pub use_cases: UseCases,
}

pub struct Entities {
    pub character: Arc<Character>,
    pub location: Arc<Location>,
    pub scene: Arc<Scene>,
    pub challenge: Arc<Challenge>,
    pub narrative: Arc<Narrative>,
    pub staging: Arc<Staging>,
    pub observation: Arc<Observation>,
    pub inventory: Arc<Inventory>,
    pub assets: Arc<Assets>,
    pub world: Arc<World>,
}

pub struct UseCases {
    pub movement: MovementUseCases,
    pub conversation: ConversationUseCases,
    pub challenge: ChallengeUseCases,
    pub approval: ApprovalUseCases,
    pub assets: AssetUseCases,
    pub world: WorldUseCases,
}

pub struct MovementUseCases {
    pub enter_region: Arc<EnterRegion>,
    pub exit_location: Arc<ExitLocation>,
}
// ... etc
```

---

## Port Traits

All port traits in one file for simplicity:

```rust
// engine/src/infrastructure/ports.rs

use async_trait::async_trait;
use domain::*;

// =============================================================================
// Database Ports (one per entity type)
// =============================================================================

#[async_trait]
pub trait CharacterRepo: Send + Sync {
    async fn get(&self, id: CharacterId) -> Result<Option<Character>>;
    async fn save(&self, character: &Character) -> Result<()>;
    async fn delete(&self, id: CharacterId) -> Result<()>;
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Character>>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Character>>;
    async fn update_position(&self, id: CharacterId, region_id: RegionId) -> Result<()>;
    async fn get_relationships(&self, id: CharacterId) -> Result<Vec<Relationship>>;
    async fn get_inventory(&self, id: CharacterId) -> Result<Vec<Item>>;
    // All character-related database operations
}

#[async_trait]
pub trait LocationRepo: Send + Sync {
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>>;
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>>;
    async fn save_location(&self, location: &Location) -> Result<()>;
    async fn save_region(&self, region: &Region) -> Result<()>;
    async fn get_regions_in_location(&self, location_id: LocationId) -> Result<Vec<Region>>;
    async fn get_connections(&self, region_id: RegionId) -> Result<Vec<Connection>>;
    // All location-related database operations
}

#[async_trait]
pub trait SceneRepo: Send + Sync {
    async fn get(&self, id: SceneId) -> Result<Option<Scene>>;
    async fn save(&self, scene: &Scene) -> Result<()>;
    async fn get_current_for_world(&self, world_id: WorldId) -> Result<Option<Scene>>;
    async fn get_for_region(&self, region_id: RegionId) -> Result<Vec<Scene>>;
    // All scene-related database operations
}

// Similar for: ChallengeRepo, NarrativeRepo, StagingRepo, 
//              ObservationRepo, InventoryRepo, AssetRepo, WorldRepo

// =============================================================================
// External Service Ports
// =============================================================================

#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse>;
    async fn generate_with_tools(&self, request: LlmRequest, tools: Vec<Tool>) -> Result<LlmResponse>;
}

#[async_trait]
pub trait ImageGenPort: Send + Sync {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult>;
    async fn check_health(&self) -> Result<bool>;
    async fn get_workflows(&self) -> Result<Vec<Workflow>>;
}

#[async_trait]
pub trait QueuePort: Send + Sync {
    async fn enqueue<T: Serialize>(&self, queue: QueueType, item: T) -> Result<QueueItemId>;
    async fn dequeue<T: DeserializeOwned>(&self, queue: QueueType) -> Result<Option<QueueItem<T>>>;
    async fn mark_complete(&self, id: QueueItemId) -> Result<()>;
    async fn mark_failed(&self, id: QueueItemId, error: &str) -> Result<()>;
}

// =============================================================================
// Testability Ports
// =============================================================================

pub trait ClockPort: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub trait RandomPort: Send + Sync {
    fn gen_range(&self, min: i32, max: i32) -> i32;
    fn gen_uuid(&self) -> Uuid;
}
```

---

## Player Crate Structure

```
player/src/
  screens/            # UI screens (Dioxus routes)
    connection.rs     # Server connection screen
    game.rs           # Main game screen
    dm_dashboard.rs   # DM control panel
    settings.rs       # Settings screen
    
  components/         # Reusable UI components
    scene_view.rs
    character_panel.rs
    dialogue_box.rs
    inventory_grid.rs
    map_view.rs
    action_bar.rs
    
  use_cases/          # Player-initiated actions
    connection/
      connect.rs
      disconnect.rs
    game/
      move_to_region.rs
      talk_to_npc.rs
      pick_up_item.rs
      roll_dice.rs
    dm/
      approve_staging.rs
      approve_suggestion.rs
      set_directorial_context.rs
      
  features/           # Client-side state management
    connection.rs     # WebSocket connection state
    game_state.rs     # Current scene, characters, etc.
    settings.rs       # Local preferences
    asset_cache.rs    # Image caching
    
  infrastructure/
    ports.rs          # Port traits for platform differences
    websocket/        # WebSocket implementation
      mod.rs
    storage/          # Platform-specific storage
      web.rs
      desktop.rs
      
  app.rs              # Player app struct
  main.rs             # Entry point
  lib.rs
```

### Player Use Cases

From the player's perspective:

```rust
// player/src/use_cases/game/move_to_region.rs

pub struct MoveToRegion {
    connection: Arc<Connection>,
    game_state: Arc<GameState>,
}

impl MoveToRegion {
    pub async fn execute(&self, region_id: RegionId) -> Result<()> {
        // Send request to server
        let response = self.connection
            .request(Request::MoveToRegion { region_id })
            .await?;
            
        // Update local state based on response
        match response {
            Response::SceneChanged(scene) => {
                self.game_state.set_current_scene(scene);
            }
            Response::StagingPending => {
                self.game_state.set_waiting_for_staging(true);
            }
            Response::Error(e) => return Err(e.into()),
        }
        
        Ok(())
    }
}
```

---

## Domain Crate

Pure business types with no I/O:

```
domain/src/
  entities/
    character.rs
    location.rs
    region.rs
    scene.rs
    challenge.rs
    narrative_event.rs
    item.rs
    world.rs
    ...
    
  value_objects/
    archetype.rs
    disposition.rs
    dice_formula.rs
    game_time.rs
    ...
    
  ids.rs              # Typed IDs (CharacterId, RegionId, etc.)
  error.rs            # Domain errors
  lib.rs
```

### Domain Purity Rules

The domain crate must NOT:
- Import tokio, axum, neo4rs, or any framework
- Perform I/O (file, network, database)
- Call `Utc::now()` or use random
- Import from engine, player, or protocol crates

The domain crate MAY:
- Use serde for serialization attributes
- Use uuid for ID generation (exception documented in ADR-001)
- Use chrono for time types (not Utc::now())
- Use thiserror for error definitions

---

## Protocol Crate

Wire format for Engine <-> Player communication:

```
protocol/src/
  messages.rs         # ClientMessage, ServerMessage enums
  requests.rs         # Request payload types
  responses.rs        # Response payload types  
  events.rs           # Server-pushed event types
  lib.rs
```

### Protocol Rules

- All types must derive `Serialize`, `Deserialize`
- Enums should have `#[serde(other)]` variants for forward compatibility
- Protocol types are for serialization, not business logic
- Keep minimal - only what crosses the wire

---

## Migration Progress

### Phase 1: Documentation & Planning
- [x] Create architecture plan (this document)
- [x] Update AGENTS.md
- [x] Update/archive hexagonal-architecture.md
- [x] Commit documentation (fbea16c)

### Phase 2: Create New Engine Structure
- [x] Create `engine/src/entities/` modules (10 entity modules)
- [x] Create `engine/src/use_cases/` modules (7 use case categories)
- [x] Create `engine/src/infrastructure/ports.rs` (~10 port traits)
- [x] Create `engine/src/api/` modules (http.rs, websocket/mod.rs)
- [x] Create `engine/src/app.rs`
- [x] Commit skeleton (f101c87)

### Phase 3: Migrate Existing Code

#### 3.1 Neo4j Repositories
- [x] CharacterRepo (from engine-adapters/src/infrastructure/persistence/character_repository/) - DONE
- [x] LocationRepo (from engine-adapters/src/infrastructure/persistence/location_repository.rs) - DONE
- [x] RegionRepo (merged into LocationRepo) - DONE
- [x] SceneRepo (from engine-adapters/src/infrastructure/persistence/scene_repository.rs) - DONE
- [x] ChallengeRepo (from engine-adapters/src/infrastructure/persistence/challenge_repository.rs) - DONE
- [x] NarrativeRepo (narrative_event_repository/, story_event_repository/, event_chain_repository.rs) - DONE (~1800 lines)
- [x] StagingRepo (from engine-adapters/src/infrastructure/persistence/staging/) - DONE
- [x] ObservationRepo (from engine-adapters/src/infrastructure/persistence/observation_repository.rs) - DONE
- [x] ItemRepo (inventory handling in character_repository/) - DONE
- [x] WorldRepo (from engine-adapters/src/infrastructure/persistence/world_repository.rs) - DONE
- [x] AssetRepo (from engine-adapters/src/infrastructure/persistence/asset/) - DONE
- [x] PlayerCharacterRepo (from engine-adapters/src/infrastructure/persistence/player_character_repository.rs) - DONE

#### 3.2 External Services
- [x] LLM client (from engine-adapters/src/infrastructure/ollama/) - DONE
- [x] ComfyUI client (from engine-adapters/src/infrastructure/comfyui/) - DONE
- [x] Queue implementations (from engine-adapters/src/infrastructure/queue/) - DONE

#### 3.3 Entity Module Logic
Note: Entity modules are thin wrappers around repos. Complex logic goes in use cases.
- [x] character.rs - wraps CharacterRepo with all operations - DONE
- [x] scene.rs - wraps SceneRepo with all operations - DONE
- [x] challenge.rs - wraps ChallengeRepo with all operations - DONE
- [x] narrative.rs - wraps NarrativeRepo with all operations - DONE
- [x] staging.rs - wraps StagingRepo with all operations - DONE

#### 3.4 Use Cases
- [x] movement/ - enter_region, exit_location - DONE
- [x] conversation/ - start, continue - DONE
- [x] challenge/ - roll, resolve - DONE  
- [x] approval/ - staging, suggestion - DONE
- [x] assets/ - generate, queue_generation - DONE
- [x] world/ - export, import - DONE
- [x] queues/ - process_player_action, process_llm_request - DONE

#### 3.5 API Layer
- [x] HTTP handlers - basic routes (health, worlds, export) - DONE
- [x] WebSocket handlers - core protocol (JoinWorld, Movement, Request/Response) - DONE
- [x] Connection management (ConnectionManager with client tracking) - DONE
- [x] WebSocket handlers - inventory (EquipItem, UnequipItem, DropItem, PickupItem) - DONE
- [x] WebSocket handlers - challenges (ChallengeRoll, ChallengeRollInput, TriggerChallenge) - DONE
- [x] WebSocket handlers - staging (StagingApprovalResponse, StagingRegenerateRequest, PreStageRegion) - DONE
- [x] WebSocket handlers - approval (ApprovalDecision, ChallengeSuggestionDecision, NarrativeEventSuggestionDecision) - DONE
- [x] WebSocket handlers - DM actions (DirectorialUpdate, TriggerApproachEvent, TriggerLocationEvent, ShareNpcLocation) - DONE
- [x] WebSocket handlers - player actions (PlayerAction with action acknowledgement) - DONE
- [ ] Remaining HTTP handlers (assets, workflows, settings) - PENDING (requires additional entities)

Note: Phase 3.5 API Layer is functionally complete. All core WebSocket handlers are implemented.
Remaining HTTP handlers are for Creator Mode features that may not be MVP-critical.

### Phase 4: Delete Old Structure
- [x] Delete `engine-ports` crate - DONE
- [x] Delete `engine-app` crate - DONE
- [x] Delete `engine-adapters` crate - DONE
- [x] Delete `engine-runner` crate - DONE
- [x] Delete `engine-composition` crate (if exists) - N/A (already merged)
- [x] Update workspace Cargo.toml - DONE

Note: Phase 4 complete. Old engine crates removed. Only the new `engine` crate remains.

### Phase 5: Player Restructure (DEFERRED)
**Status**: Deferred - Player works with current structure
**Reason**: Engine simplification was the main bottleneck. Player communicates via protocol only.
**Future Work**: When player complexity becomes a bottleneck, consolidate:
- player-ports + player-app + player-adapters → player
- Keep player-ui and player-runner separate (UI concerns)

- [ ] Create new player structure
- [ ] Migrate existing player code  
- [ ] Delete old player crates

### Phase 6: Cleanup
- [x] Update architecture documentation - DONE
- [ ] Remove obsolete files (if any)
- [x] Verify full workspace compiles - DONE

---

## Current State Summary (as of 2026-01-03)

**Engine**: Fully simplified to single crate with ~10 port traits
- All WebSocket handlers implemented (movement, inventory, challenges, staging, approval, DM actions)
- HTTP handlers for basic routes
- Ready for feature development

**Player**: Works with existing multi-crate structure
- Communicates with Engine via WebSocket protocol
- Can be simplified later when needed

**Next Steps**: Resume novel feature work on the engine

---

## Testing Strategy

### Unit Tests
- Domain logic: Pure unit tests, no mocking needed
- Entity modules: Mock repository ports
- Use cases: Mock entity modules or repository ports

### Integration Tests  
- Use testcontainers for Neo4j
- Test full use case flows with real database
- Test API endpoints end-to-end

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    
    #[tokio::test]
    async fn enter_region_updates_position() {
        let mut char_repo = MockCharacterRepo::new();
        char_repo.expect_update_position()
            .with(eq(pc_id), eq(region_id))
            .returning(|_, _| Ok(()));
            
        let character = Character::new(Arc::new(char_repo));
        // ... setup other mocks
        
        let use_case = EnterRegion::new(character, location, staging, observation, narrative);
        let result = use_case.execute(pc_id, region_id).await;
        
        assert!(result.is_ok());
    }
}
```

---

## Decisions Log

| Decision | Rationale |
|----------|-----------|
| 4 crates (domain, protocol, engine, player) | Minimal structure that still enforces domain purity |
| Entity modules, not services | Clearer mental model - operations grouped by what they operate on |
| Use cases for cross-cutting | User stories often span multiple entities |
| ~10 port traits | Only abstract real infrastructure boundaries |
| Capability-based repos (per entity) | Avoids duplication, maps naturally to domain |
| Axum in engine crate | Pragmatic - we won't swap web frameworks |
| Handlers call use cases directly | No inbound ports, no internal traits |

---

## Open Questions

1. **Queue handling**: One `QueuePort` trait or separate per queue type?
2. **WebSocket handlers**: Part of `api/` or separate module?
3. **Event broadcasting**: Where does this fit? Entity module or infrastructure?

---

## Related Documents

- [Neo4j Schema](../architecture/neo4j-schema.md)
- [WebSocket Protocol](../architecture/websocket-protocol.md)
- [Domain Entities](../../crates/domain/src/entities/)
