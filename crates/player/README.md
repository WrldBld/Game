# WrldBldr Player

The Player is the frontend client for WrldBldr, providing the visual novel interface for players and DM control panels. It supports both **web** (WASM) and **desktop** (native) builds.

---

## Architecture Overview

The Player is now unified into a single crate (`wrldbldr-player`) with internal modules for UI, application logic, and infrastructure.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           wrldbldr-player                                    │
│                                                                             │
│  src/main.rs           Composition root (desktop + web)                      │
│  src/ui/*              Dioxus routes/views/components/state                  │
│  src/application/*     Services + DTOs                                       │
│  src/infrastructure/*  WebSocket/HTTP/platform adapters (cfg per target)     │
│  src/ports/*           Transitional port traits (will shrink over time)      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Module Responsibilities

### `src/main.rs` (Composition Root)

- Creates platform-specific adapters (WASM vs Desktop)
- Injects dependencies via Dioxus context providers
- Launches the Dioxus application

### `src/ui/` (Presentation)

- Routes, views, reusable components
- Reactive state via Dioxus Signals
- Server event processing/handlers

### `src/application/` (Application)

- Services (API interactions)
- DTOs and error handling

### `src/infrastructure/` (Infrastructure)

- WebSocket (desktop/wasm)
- HTTP client (desktop/wasm)
- Platform (storage, clipboard, URL handling)

---

## Directory Structure

```
crates/
└── player/
    ├── src/
    │   ├── main.rs
    │   ├── ui/
    │   ├── application/
    │   ├── infrastructure/
    │   ├── ports/
    │   └── state/
    ├── assets/
    ├── styles/
    ├── package.json
    ├── tailwind.config.js
    └── Dioxus.toml
```

---

## Key Navigation Guide

### Finding Code by Task

| Task                 | Location                                                                |
| -------------------- | ----------------------------------------------------------------------- |
| Add a new route      | `crates/player/src/ui/routes/mod.rs`                                    |
| Add a new view       | `crates/player/src/ui/presentation/views/`                              |
| Add a component      | `crates/player/src/ui/presentation/components/`                         |
| Add state management | `crates/player/src/ui/presentation/state/`                              |
| Add a service        | `crates/player/src/application/application/services/`                   |
| Add a port trait     | `crates/player/src/ports/outbound/`                                     |
| Handle server events | `crates/player/src/ui/presentation/handlers/session_message_handler.rs` |
| Add platform code    | `crates/player/src/infrastructure/platform/`                            |

### Important Files

| File                                                                    | Purpose                          |
| ----------------------------------------------------------------------- | -------------------------------- |
| `crates/player/src/main.rs`                                             | Application entry point          |
| `crates/player/src/ui/mod.rs`                                           | Dioxus app root, ShellKind       |
| `crates/player/src/ui/routes/mod.rs`                                    | All route definitions            |
| `crates/player/src/ui/presentation/services.rs`                         | Service hooks (use\_\*\_service) |
| `crates/player/src/ui/presentation/handlers/session_message_handler.rs` | Event processing                 |
| `crates/player/src/ports/outbound/game_connection_port.rs`              | WebSocket interface              |

---

## Routes

```rust
pub enum Route {
    // Main screens
    MainMenuRoute {}                    // /
    WorldSelectRoute {}                 // /worlds
    RoleSelectRoute {}                  // /roles

    // Player views
    PCViewRoute { world_id }            // /worlds/:id/play
    PCCreationRoute { world_id }        // /worlds/:id/play/create-character
    SpectatorViewRoute { world_id }     // /worlds/:id/watch

    // DM views
    DMViewRoute { world_id }            // /worlds/:id/dm
    DMViewTabRoute { world_id, tab }    // /worlds/:id/dm/:tab
    DMCreatorSubTabRoute { ... }        // /worlds/:id/dm/creator/:subtab
    DMSettingsSubTabRoute { ... }       // /worlds/:id/dm/settings/:subtab
    DMStoryArcSubTabRoute { ... }       // /worlds/:id/dm/story-arc/:subtab

    NotFoundRoute { route }             // /:..route
}
```

---

## State Management

State is managed via **Dioxus Signals** provided through context:

### State Containers

| State             | Hook                     | Purpose                                 |
| ----------------- | ------------------------ | --------------------------------------- |
| `GameState`       | `use_game_state()`       | World data, scene, NPCs, navigation     |
| `SessionState`    | `use_session_state()`    | Connection, user, approvals, challenges |
| `DialogueState`   | `use_dialogue_state()`   | Current dialogue, typewriter effect     |
| `GenerationState` | `use_generation_state()` | Asset generation queue                  |

### SessionState Composition

`SessionState` is a facade composing:

- `ConnectionState` - WebSocket status, user identity
- `ApprovalState` - Pending approvals, decision history
- `ChallengeState` - Active challenges, results, skills

### Accessing State

```rust
#[component]
fn MyComponent() -> Element {
    let game = use_game_state();
    let session = use_session_state();

    // Read state
    let current_region = game.read().current_region.clone();
    let is_connected = session.read().connection.is_connected();

    // Mutate state
    game.write().update_scene(new_scene);

    rsx! {
        div { "Current region: {current_region:?}" }
    }
}
```

---

## Service Hooks

Services are provided via `Services<A: ApiPort>` context:

```rust
#[component]
fn MyComponent() -> Element {
    let world_service = use_world_service();
    let character_service = use_character_service();

    // Use services
    let _ = use_future(move || async move {
        let worlds = world_service.list().await;
        // ...
    });

    rsx! { /* ... */ }
}
```

### Available Service Hooks

| Hook                            | Service               | Purpose              |
| ------------------------------- | --------------------- | -------------------- |
| `use_world_service()`           | WorldService          | World CRUD           |
| `use_character_service()`       | CharacterService      | Character CRUD       |
| `use_location_service()`        | LocationService       | Location/Region CRUD |
| `use_challenge_service()`       | ChallengeService      | Challenge management |
| `use_skill_service()`           | SkillService          | Skill CRUD           |
| `use_narrative_event_service()` | NarrativeEventService | Event management     |
| `use_event_chain_service()`     | EventChainService     | Event chains         |
| `use_asset_service()`           | AssetService          | Asset gallery        |
| `use_generation_service()`      | GenerationService     | Image generation     |
| `use_workflow_service()`        | WorkflowService       | ComfyUI workflows    |
| `use_settings_service()`        | SettingsService       | App/world settings   |
| `use_story_event_service()`     | StoryEventService     | Story timeline       |

---

## Component Organization

### Visual Novel Components (`components/visual_novel/`)

| Component             | Purpose                      |
| --------------------- | ---------------------------- |
| `backdrop.rs`         | Scene background image       |
| `character_sprite.rs` | NPC sprites with positioning |
| `dialogue_box.rs`     | Typewriter text display      |
| `choice_menu.rs`      | Player dialogue choices      |

### DM Panel Components (`components/dm_panel/`)

| Component                    | Purpose                  |
| ---------------------------- | ------------------------ |
| `approval_popup.rs`          | LLM response approval    |
| `challenge_library/`         | Challenge management     |
| `trigger_challenge_modal.rs` | Manual challenge trigger |
| `director_generate_modal.rs` | Quick asset generation   |
| `staging_approval.rs`        | NPC staging approval     |

### Creator Components (`components/creator/`)

| Component             | Purpose                      |
| --------------------- | ---------------------------- |
| `character_form.rs`   | Character editor             |
| `location_form.rs`    | Location editor              |
| `motivations_tab.rs`  | NPC wants/goals (1513 lines) |
| `asset_gallery.rs`    | Generated assets browser     |
| `generation_queue.rs` | Generation progress          |

### Tactical Components (`components/tactical/`)

| Component           | Purpose            |
| ------------------- | ------------------ |
| `challenge_roll.rs` | Dice rolling modal |
| `skills_display.rs` | Character skills   |

---

## Platform Support

The Player supports both web (WASM) and desktop builds through conditional compilation.

### Platform Detection

```rust
// In player-runner/src/main.rs
#[cfg(target_arch = "wasm32")]
fn main() {
    // WASM initialization
    dioxus::launch(app);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Desktop initialization with tokio runtime
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { /* ... */ });
}
```

### Platform-Specific Code

| Feature     | Desktop             | WASM                  |
| ----------- | ------------------- | --------------------- |
| HTTP Client | `reqwest`           | `gloo-net`            |
| WebSocket   | `tokio-tungstenite` | `gloo-net`            |
| Storage     | File system         | `localStorage`        |
| Random      | `rand`              | `getrandom` (browser) |

---

## Adding a New Feature

### 1. Add a Component

```rust
// crates/player/src/presentation/components/my_component.rs
use dioxus::prelude::*;

#[component]
pub fn MyComponent(name: String) -> Element {
    rsx! {
        div { class: "p-4",
            h1 { "{name}" }
        }
    }
}
```

### 2. Add State

```rust
// crates/player/src/presentation/state/my_state.rs
use dioxus::prelude::*;

#[derive(Clone, Default)]
pub struct MyState {
    pub items: Vec<String>,
}

pub fn use_my_state() -> Signal<MyState> {
    use_context::<Signal<MyState>>()
}
```

### 3. Add a Service

```rust
// crates/player/src/application/services/my_service.rs
use crate::application::{Api, ServiceError};
use wrldbldr_player_ports::outbound::ApiPort;

pub struct MyService<A: ApiPort> {
    api: Api<A>,
}

impl<A: ApiPort> MyService<A> {
    pub fn new(api: Api<A>) -> Self {
        Self { api }
    }

    pub async fn list(&self) -> Result<Vec<MyItem>, ServiceError> {
        self.api.get("/api/my-items").await
    }
}
```

### 4. Add a Service Hook

```rust
// Add to crates/player/src/presentation/services.rs
pub fn use_my_service<A: ApiPort + 'static>() -> MyService<A> {
    let services = use_context::<Services<A>>();
    services.my_service.clone()
}
```

### 5. Handle Server Events

```rust
// In crates/player/src/ui/presentation/handlers/session_message_handler.rs
match event {
    PlayerEvent::MyEvent(data) => {
        game_state.write().handle_my_event(data);
    }
    // ...
}
```

---

## Running the Player

### Web (WASM)

```bash
# Development with hot reload
task web:dev

# Or using Dioxus CLI directly
dx serve --platform web

# Build for production
task build:web
```

### Desktop

```bash
# Development
task desktop:dev

# Or using Dioxus CLI
dx serve --platform desktop

# Build for production
cargo build --release -p wrldbldr-player
```

### Configuration

The Player needs the Engine WebSocket URL:

```bash
# Environment variable
ENGINE_WS_URL=ws://localhost:3000/ws

# Or configure in the UI (Main Menu → Settings)
```

---

## Styling

The Player uses **Tailwind CSS** for styling:

```bash
# Build CSS (required after changes to styles)
task css:build

# Watch for changes during development
task css:watch
```

CSS files:

- Source: `crates/player/styles/input.css`
- Output: `crates/player/assets/css/output.css`
- Config: `crates/player/tailwind.config.js`

---

## Architecture Rules

### UI Layer

- Access state via hooks (`use_game_state()`, etc.)
- Access services via hooks (`use_world_service()`, etc.)
- NO direct adapter or infrastructure imports
- Protocol imports ALLOWED (UI is a boundary)

### Application Layer

- Services depend on port traits, not adapters
- DTOs defined here, not in ports
- Protocol imports allowed for boundary DTOs

### Ports Layer

- Traits only - no implementations
- ISP-compliant (focused traits)
- `PlayerEvent` types defined here
- Limited protocol imports (boundary files only)

### Adapters Layer

- Implements port traits
- Platform-specific code via `cfg`
- Message translation (protocol → app events)

---

## WebSocket Communication

### Message Flow

```
User Action
    │
    ▼
Component calls GameConnectionPort method
    │
    ▼
Adapter sends ClientMessage via WebSocket
    │
    ▼
[Engine processes request]
    │
    ▼
Adapter receives ServerMessage
    │
    ▼
message_translator converts to PlayerEvent
    │
    ▼
session_message_handler updates state
    │
    ▼
Components re-render via signals
```

### Handling Events

```rust
// In session_message_handler.rs
pub fn handle_server_message(
    event: PlayerEvent,
    game_state: &mut Signal<GameState>,
    session_state: &mut Signal<SessionState>,
    // ...
) {
    match event {
        PlayerEvent::SceneChanged(data) => {
            game_state.write().update_scene(data);
        }
        PlayerEvent::DialogueResponse(data) => {
            dialogue_state.write().show_dialogue(data);
        }
        // ... 70+ event types
    }
}
```

---

## Testing

```bash
# Run player tests
cargo test -p wrldbldr-player
```

### Mocking Ports

Use the `testing` feature for mock implementations:

```rust
#[cfg(test)]
mod tests {
    use wrldbldr_player::ports::outbound::testing::MockGameConnectionPort;

    #[test]
    fn test_something() {
        let mock = MockGameConnectionPort::new();
        // Configure mock expectations...
    }
}
```

---

## Common Issues

### WASM Build Fails

Ensure WASM target is installed:

```bash
rustup target add wasm32-unknown-unknown
```

### CSS Not Updating

Rebuild Tailwind CSS:

```bash
task css:build
```

### WebSocket Connection Issues

- Check Engine is running at correct URL
- Verify CORS settings in Engine
- Check browser console for errors

### State Not Updating

- Ensure using `.write()` for mutations
- Check that state is provided via context
- Verify event handler is processing the event

---

## Related Documentation

- [Hexagonal Architecture](../../docs/architecture/hexagonal-architecture.md)
- [WebSocket Protocol](../../docs/architecture/websocket-protocol.md)
- [System Documents](../../docs/systems/) - Game system specs with UI mockups
- [Dioxus Documentation](https://dioxuslabs.com/learn/0.5/)
- [AGENTS.md](../../AGENTS.md) - AI assistant guidelines
