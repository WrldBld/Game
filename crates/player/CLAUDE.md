# Player - Claude Code Instructions

This is the **Player** component of WrldBldr - the frontend application built with Dioxus (Rust) that runs in the browser (WASM) or as a desktop app.

## Environment

This project runs on **NixOS**. Use `nix-shell` for development dependencies:

```bash
nix-shell -p rustc cargo gcc pkg-config openssl.dev webkitgtk_4_1.dev glib.dev gtk3.dev libsoup_3.dev --run "cargo check"
```

For WASM builds:
```bash
nix-shell -p rustc cargo wasm-pack --run "dx build --release"
```

## Architecture

The Player follows **hexagonal architecture** (ports and adapters) with **domain-driven design** principles.

```
src/
├── domain/              # Core business types (INNERMOST - no external deps)
│   ├── entities/        # Domain entities (Character, Scene, Location, etc.)
│   ├── value_objects/   # IDs, types, small immutable types
│   └── services/        # Pure domain logic
├── application/         # Use cases and orchestration
│   ├── services/        # Application services (SessionService, WorldService, etc.)
│   ├── ports/
│   │   ├── inbound/     # Use case interfaces
│   │   └── outbound/    # External service interfaces (ApiPort, GameConnectionPort)
│   └── dto/             # Data transfer objects
├── infrastructure/      # External adapters (OUTERMOST)
│   ├── http_client.rs   # REST API client - implements ApiPort
│   ├── websocket/       # WebSocket client - implements GameConnectionPort
│   ├── asset_loader/    # Data types for API responses (DTOs)
│   ├── api.rs           # API configuration
│   └── storage.rs       # localStorage wrapper
├── presentation/        # UI layer
│   ├── views/           # Page-level components
│   ├── components/      # Reusable UI components
│   └── state/           # Application state (Dioxus signals)
├── routes.rs            # URL routing
└── main.rs
```

## CRITICAL: Hexagonal Architecture Rules

### Dependency Direction (STRICTLY ENFORCED)

```
Domain ← Application ← Infrastructure
                    ← Presentation
```

**NEVER violate these rules:**

1. **Domain layer has NO external dependencies**
   - No `use crate::infrastructure::*`
   - No `use crate::application::*`
   - No `use crate::presentation::*`
   - No framework types (Dioxus, serde on domain types)
   - Domain types are pure Rust structs/enums

2. **Application layer depends ONLY on Domain**
   - `use crate::domain::*` ✓
   - `use crate::infrastructure::*` ✗ FORBIDDEN
   - `use crate::presentation::*` ✗ FORBIDDEN (circular dependency!)
   - Services use TRAIT BOUNDS for external dependencies

3. **Presentation layer uses Application Services**
   - `use crate::application::services::*` ✓
   - `use crate::domain::*` ✓
   - `use crate::infrastructure::*` ✗ FORBIDDEN
   - Components call services, NOT HttpClient directly

4. **Infrastructure implements ports**
   - `HttpClient` implements `ApiPort` trait
   - `EngineClient` implements `GameConnectionPort` trait

### Port Pattern (REQUIRED)

```rust
// application/ports/outbound/api_port.rs
#[async_trait]
pub trait ApiPort: Send + Sync {
    async fn list_worlds(&self) -> Result<Vec<WorldSummary>, ApiError>;
    async fn get_character(&self, id: &str) -> Result<Character, ApiError>;
    async fn save_character(&self, character: &Character) -> Result<Character, ApiError>;
}

// application/services/character_service.rs
pub struct CharacterService<A: ApiPort> {
    api: A,  // Trait bound, NOT concrete type
}

// infrastructure/http_client.rs
impl ApiPort for HttpClient {
    // ... implementation
}
```

### View/Component Pattern (REQUIRED)

Views and components MUST use application services, NOT infrastructure directly:

```rust
// CORRECT - Using service via context
#[component]
pub fn CharacterList(world_id: String) -> Element {
    let character_service = use_context::<CharacterService>();
    let characters = use_resource(move || {
        let svc = character_service.clone();
        let id = world_id.clone();
        async move { svc.list_characters(&id).await }
    });
    // ...
}

// WRONG - Direct infrastructure access
#[component]
pub fn CharacterList(world_id: String) -> Element {
    let characters = use_resource(move || async move {
        HttpClient::get(&format!("/api/worlds/{}/characters", world_id)).await  // VIOLATION!
    });
    // ...
}
```

### State Pattern (REQUIRED)

Presentation state should use domain/application types, NOT infrastructure types:

```rust
// CORRECT - Domain types in state
pub struct GameState {
    pub world: Signal<Option<World>>,           // Domain type
    pub characters: Signal<Vec<Character>>,     // Domain type
}

// WRONG - Infrastructure types in state
pub struct GameState {
    pub world: Signal<Option<WorldSnapshot>>,   // Infrastructure DTO!
    pub client: Signal<Option<Arc<EngineClient>>>,  // Infrastructure type!
}
```

### Application Service Pattern (REQUIRED)

```rust
// CORRECT - Trait bound for dependencies
pub struct SessionService<C: GameConnectionPort> {
    connection: C,
}

// WRONG - Concrete infrastructure type
pub struct SessionService {
    client: Arc<EngineClient>,  // VIOLATION!
}

// VERY WRONG - Importing presentation state
use crate::presentation::state::SessionState;  // CIRCULAR DEPENDENCY!
```

## Key Conventions

### Routing (CRITICAL)

**All navigation MUST use proper URL routing:**

- Top-level tabs: `/worlds/{world_id}/dm/{tab}` (director, creator, story-arc, settings)
- Sub-tabs: `/worlds/{world_id}/dm/{tab}/{subtab}`
- Use `Link` component from Dioxus router, NOT onClick handlers for navigation
- Tab state should come from URL parameters, not local signals

**NEVER use `use_effect` + `navigator.replace()` for redirects!**

This causes race conditions. Instead:
1. **Link directly to the final route**
2. **Render content directly** with default subtab

### Dioxus RSX Patterns

- Use `Signal<T>` for reactive state
- Pre-compute format values before RSX blocks
- For Props with complex types, implement `PartialEq` manually
- Read signals into variables before using in closures

### Components

- Components go in `src/presentation/components/`
- Group related components in subdirectories (e.g., `story_arc/`)
- Export from module's `mod.rs`
- Props structs should derive `Props, Clone, PartialEq`
- **Components receive data via props and emit events - no direct API calls**

### Platform-Specific Code

Platform-specific code belongs ONLY in infrastructure layer:

```rust
// ONLY in infrastructure/http_client.rs
#[cfg(target_arch = "wasm32")]
{
    // WASM-specific code (gloo_net)
}
#[cfg(not(target_arch = "wasm32"))]
{
    // Desktop-specific code (reqwest)
}
```

**NEVER put `#[cfg(target_arch = ...)]` in presentation or application layers!**

## File Placement Rules

| If you're creating... | Put it in... |
|-----------------------|--------------|
| Business entity (Character, Scene) | `domain/entities/` |
| ID type, value object | `domain/value_objects/` |
| Pure business logic | `domain/services/` |
| External service trait (API, WebSocket) | `application/ports/outbound/` |
| Business orchestration | `application/services/` |
| HTTP client implementation | `infrastructure/http_client.rs` |
| WebSocket implementation | `infrastructure/websocket/` |
| API response DTOs | `infrastructure/asset_loader/` or `application/dto/` |
| UI component | `presentation/components/` |
| Page/view | `presentation/views/` |
| Reactive state | `presentation/state/` |

## Running

```bash
# Check compilation
cargo check

# Run desktop app
dx serve

# Build for WASM
dx build --release
```

## Architecture Violations to Avoid

1. **Importing infrastructure in presentation layer**
   ```rust
   // WRONG in presentation/views/*.rs or presentation/components/*.rs
   use crate::infrastructure::http_client::HttpClient;
   use crate::infrastructure::websocket::EngineClient;
   ```

2. **Importing infrastructure in application layer**
   ```rust
   // WRONG in application/services/*.rs
   use crate::infrastructure::websocket::EngineClient;
   ```

3. **Importing presentation in application layer**
   ```rust
   // VERY WRONG - circular dependency!
   use crate::presentation::state::SessionState;
   ```

4. **Direct HTTP calls in components**
   ```rust
   // WRONG in any component
   HttpClient::get("/api/...").await
   ```

5. **Infrastructure types in presentation state**
   ```rust
   // WRONG in presentation/state/*.rs
   pub engine_client: Signal<Option<Arc<EngineClient>>>,
   pub proposed_tools: Vec<ProposedTool>,  // Infrastructure type!
   ```

6. **Concrete types instead of traits in services**
   ```rust
   // WRONG
   pub struct ActionService {
       client: Arc<EngineClient>,
   }
   // CORRECT
   pub struct ActionService<C: GameConnectionPort> {
       connection: C,
   }
   ```

## Migration Notes

The codebase is currently being refactored to comply with hexagonal architecture.
See `/home/otto/repos/WrldBldr/plans/Hexagonal_refactor.md` for the detailed plan.

Current violations being addressed:
- Application services depend on concrete infrastructure types
- Presentation components call HttpClient directly
- Infrastructure types leak into presentation state
- Application layer imports presentation state (circular dependency)

## See Also

- `/home/otto/repos/WrldBldr/plans/Hexagonal_refactor.md` - Detailed refactoring plan
- `/home/otto/repos/WrldBldr/plans/CLAUDE.md` - Project planning conventions
