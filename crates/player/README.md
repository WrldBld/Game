# WrldBldr Player

The **Player** is the frontend client for WrldBldr, a TTRPG (Tabletop Role-Playing Game) world management system. It provides a visual novel-style interface for players and a directorial control panel for Dungeon Masters.

## Goals

- **Visual Novel Gameplay**: Immersive PC experience with backdrops, character sprites, and typewriter dialogue
- **DM Directing**: Control panel for guiding AI-assisted NPC responses and managing scenes
- **World Creation**: Tools for building characters, locations, and assets within the game
- **Cross-Platform**: Run on Desktop (Linux/Windows/macOS), Web (WASM), and Android
- **Real-Time Multiplayer**: Synchronized sessions with multiple players and a DM

## Architecture

The Player follows a **layered architecture** with clear separation between domain, application, infrastructure, and presentation:

```
Player/
├── src/
│   ├── main.rs                        # Dioxus app entry point
│   │
│   ├── domain/                        # Core client-side logic
│   │   ├── entities/
│   │   │   └── player_action.rs       # Player action types (Talk, Examine, Travel, etc.)
│   │   ├── services/                  # (planned)
│   │   └── value_objects/             # (planned)
│   │
│   ├── application/                   # Use cases and services
│   │   ├── services/
│   │   │   ├── session_service.rs     # WebSocket connection lifecycle
│   │   │   └── action_service.rs      # Player action dispatch
│   │   ├── ports/                     # (planned)
│   │   └── dto/                       # (planned)
│   │
│   ├── infrastructure/                # External system adapters
│   │   ├── websocket/
│   │   │   ├── client.rs              # Dual-platform WebSocket (Tokio + WASM)
│   │   │   └── messages.rs            # Client/Server message types
│   │   ├── asset_loader/
│   │   │   └── world_snapshot.rs      # World data deserialization
│   │   └── audio/                     # (planned)
│   │
│   └── presentation/                  # UI layer
│       ├── state/                     # Global state management
│       │   ├── session_state.rs       # Connection status, user role
│       │   ├── game_state.rs          # World, scene, characters
│       │   ├── dialogue_state.rs      # Typewriter animation
│       │   └── generation_state.rs    # Asset generation tracking
│       │
│       ├── views/                     # Top-level screens
│       │   ├── main_menu.rs           # Server connection
│       │   ├── role_select.rs         # DM/Player/Spectator selection
│       │   ├── pc_view.rs             # Player character view
│       │   ├── dm_view.rs             # Dungeon Master view
│       │   └── spectator_view.rs      # Observer view
│       │
│       └── components/                # Reusable UI components
│           ├── visual_novel/          # VN-style components
│           │   ├── backdrop.rs        # Background images
│           │   ├── character_sprite.rs# Character positioning
│           │   ├── dialogue_box.rs    # Typewriter text display
│           │   └── choice_menu.rs     # Player choices
│           │
│           ├── dm_panel/              # DM control components
│           │   ├── approval_popup.rs  # LLM response approval
│           │   ├── scene_preview.rs   # Compact scene display
│           │   ├── conversation_log.rs
│           │   ├── directorial_notes.rs
│           │   ├── npc_motivation.rs
│           │   └── tone_selector.rs
│           │
│           ├── creator/               # World building components
│           │   ├── entity_browser.rs  # Entity tree view
│           │   ├── character_form.rs  # Character creation/editing
│           │   ├── location_form.rs   # Location creation/editing
│           │   ├── asset_gallery.rs   # Generated asset selection
│           │   ├── generation_queue.rs# Generation progress
│           │   └── suggestion_button.rs# AI suggestion requests
│           │
│           ├── settings/              # Configuration components
│           │   ├── workflow_config_editor.rs
│           │   ├── workflow_upload_modal.rs
│           │   └── workflow_slot_list.rs
│           │
│           ├── tactical/              # Combat grid (planned)
│           ├── action_panel.rs        # Interaction buttons
│           └── shared/                # Common utilities
│
├── Cargo.toml                         # Dependencies
├── Trunk.toml                         # WASM bundler config
├── Tailwind.config.js                 # CSS configuration
├── package.json                       # JS dependencies (Tailwind)
├── shell.nix                          # NixOS development environment
└── Taskfile.yml                       # Task runner commands
```

## Views and Roles

### Main Menu
Server connection screen where users enter the Engine WebSocket URL.

### Role Selection
Choose your role in the session:
- **Dungeon Master**: Direct the game, approve AI responses, manage world
- **Player**: Control a player character in the visual novel interface
- **Spectator**: Watch the game unfold (read-only)

### PC View (Player Character)
Visual novel-style gameplay interface:
```
┌─────────────────────────────────────────────────────────────┐
│  [Backdrop Image - Full Screen]                             │
│         ┌─────────┐     ┌─────────┐                         │
│         │  NPC    │     │   PC    │                         │
│         │ Sprite  │     │ Sprite  │                         │
│         └─────────┘     └─────────┘                         │
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ NPC Name:                                               │ │
│ │ "Dialogue text appears here, typewriter style..."       │ │
│ │ [Choice 1] [Choice 2] [Custom input...]                 │ │
│ └─────────────────────────────────────────────────────────┘ │
│ [Inventory]  [Character]  [Map]  [Log]                      │
└─────────────────────────────────────────────────────────────┘
```

### DM View (Dungeon Master)
Tabbed interface with Director Mode, Creator Mode, and Settings:
```
┌─────────────────────────────────────────────────────────────┐
│  [Director Mode]  [Creator Mode]  [Settings]       [← Back] │
├───────────────────────────────────┬─────────────────────────┤
│  Scene Preview                    │  Directorial Panel      │
│  ┌─────────────────────────────┐  │  Scene Notes: [...]     │
│  │  Backdrop + Characters      │  │  NPC Motivations        │
│  └─────────────────────────────┘  │  Tone: [Dropdown]       │
│                                   │                         │
│  Conversation Log                 │  Active NPCs:           │
│  ┌─────────────────────────────┐  │  [x] Bartender          │
│  │ PC: "Hello there"           │  │  [ ] Guard              │
│  │ [Awaiting response...]      │  │                         │
│  └─────────────────────────────┘  │  [Social Graph]         │
│                                   │                         │
│  ┌─ Approval Popup ─────────────┐ │                         │
│  │ NPC will say: "..."          │ │                         │
│  │ [Accept] [Modify] [Reject]   │ │                         │
│  └──────────────────────────────┘ │                         │
└───────────────────────────────────┴─────────────────────────┘
```

## Running the Player

### Prerequisites

- **Rust** (latest stable)
- **Node.js** (for Tailwind CSS)
- **Trunk** (for WASM builds): `cargo install trunk`

### Using Nix (Recommended)

```bash
# Enter development environment
nix-shell

# Install JS dependencies
npm install

# Run desktop version
task desktop

# Or run web version
task web
```

### Desktop Build

```bash
# Install dependencies
npm install

# Build and run (Linux with GTK)
cargo run

# Or with release optimizations
cargo run --release
```

### Web/WASM Build

```bash
# Install trunk if not present
cargo install trunk

# Serve with hot reload
trunk serve

# Build for production
trunk build --release

# Output in dist/
```

### Android Build

```bash
# Requires Android SDK and NDK
cargo apk build --release
```

## Configuration

The Player connects to an Engine server via WebSocket. The default URL is `ws://localhost:3000/ws`.

### Build Targets

| Target | Command | Notes |
|--------|---------|-------|
| Desktop (Linux) | `cargo run` | Uses GTK/WebKitGTK |
| Desktop (Windows) | `cargo run` | Uses WebView2 |
| Desktop (macOS) | `cargo run` | Uses WKWebView |
| Web (WASM) | `trunk serve` | Runs at localhost:8080 |
| Android | `cargo apk build` | Requires Android toolchain |

## State Management

The Player uses Dioxus signals with context providers for global state:

### SessionState
- Connection status (Disconnected, Connecting, Connected, etc.)
- Session ID and user info
- WebSocket client reference

### GameState
- Loaded WorldSnapshot
- Current scene and characters
- Available interactions

### DialogueState
- Current speaker and text
- Typewriter animation progress
- Available choices

### GenerationState
- Asset generation batch tracking
- Progress updates from Engine

## WebSocket Protocol

### Client → Server
```rust
JoinSession { user_id, role }      // Join game session
PlayerAction { action_type, target, dialogue }  // PC action
DirectorialUpdate { scene_notes, npc_motivations, tone }
ApprovalDecision { Accept | Modify | Reject | TakeOver }
RequestSceneChange { scene_id }
Heartbeat
```

### Server → Client
```rust
SessionJoined { session_id, world_snapshot }
SceneUpdate { scene, characters }
DialogueResponse { speaker, text, choices }
LLMProcessing { action_id }        // AI is thinking
ApprovalRequired { dialogue, proposed_tools }  // DM approval needed
ResponseApproved { dialogue, executed_tools }
GenerationEvent { batch_id, status, progress }
Error { message }
```

## Development

### Task Commands

```bash
# Using Taskfile
task desktop    # Run desktop version
task web        # Run web version with hot reload
task build      # Build release
task check      # Run clippy and format check
```

### Styling

The Player uses Tailwind CSS with a TTRPG-themed design:
- **Colors**: Parchment backgrounds, ink text, blood accents, gold highlights
- **Typography**: Fantasy-inspired fonts
- **Components**: Card-based layouts, gradient buttons

### Platform-Specific Code

The codebase handles platform differences with conditional compilation:

```rust
#[cfg(not(target_arch = "wasm32"))]
// Desktop: Uses tokio for async, tokio-tungstenite for WebSocket

#[cfg(target_arch = "wasm32")]
// WASM: Uses gloo-timers, web-sys WebSocket API
```

## Current Status

### Implemented
- Dual-platform WebSocket client (Desktop + WASM)
- Session and action services
- WorldSnapshot loading and parsing
- State management (Session, Game, Dialogue, Generation)
- Visual novel components (backdrop, sprites, dialogue, choices)
- Typewriter effect with punctuation-aware timing
- Main menu and role selection
- PC view with visual novel interface
- DM view structure with tabs
- Creator mode UI components
- Workflow configuration UI

### In Progress
- Creator mode API integration
- Workflow config save functionality
- DM approval flow wiring

### Planned
- Spectator view scene display
- Tactical combat grid
- Audio system
- Mobile-optimized layouts

## Related

- [Engine README](../Engine/README.md) - Backend server documentation
- [Master Plan](../plans/00-master-plan.md) - Full project specification
- [Phase 4: Player Basics](../plans/04-player-basics.md) - Player implementation details
- [Phase 11: Director Creation UI](../plans/11-director-creation-ui.md) - Creator mode specification
