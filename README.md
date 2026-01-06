# WrldBldr

**An AI-Powered TTRPG Engine for Emergent Storytelling**

WrldBldr is a game engine for tabletop roleplaying games that combines AI-generated content with human oversight. Players interact with NPCs through a visual novel interface, while an LLM generates contextually rich dialogue based on character motivations, relationships, and narrative state. Every AI-generated response requires DM approval before players see it.

---

## Vision

```
Player Action → Engine Context → LLM Response → DM Approval → State Updated
     ↑                                                              │
     └──────────────────────────────────────────────────────────────┘
```

### Key Innovations

1. **Pure Graph Model**: All game state in Neo4j as nodes and edges - relationships are first-class citizens
2. **AI Game Master**: LLM-driven NPC dialogue informed by deep narrative context (motivations, relationships, history)
3. **DM Approval Flow**: Human oversight of all AI-generated content before players see it
4. **Actantial Model**: Per-character view of who helps them, opposes them, and what they desire
5. **Asset Generation**: ComfyUI integration for character portraits, sprites, and scene backdrops

---

## Game Systems

| System                                          | Description                                       | Status              |
| ----------------------------------------------- | ------------------------------------------------- | ------------------- |
| [Navigation](docs/systems/navigation-system.md) | Locations, regions, movement, game time           | Engine ✅ Player ✅ |
| [Character](docs/systems/character-system.md)   | NPCs, PCs, archetypes, actantial model, inventory | Engine ✅ Player ✅ |
| [Dialogue](docs/systems/dialogue-system.md)     | LLM integration, DM approval, tool calls          | Engine ✅ Player ✅ |
| [Challenge](docs/systems/challenge-system.md)   | Skill checks, dice, outcomes, rule systems        | Engine ✅ Player ✅ |
| [Narrative](docs/systems/narrative-system.md)   | Events, triggers, effects, chains                 | Engine ✅ Player ✅ |
| [Scene](docs/systems/scene-system.md)           | Visual novel, backdrops, sprites, interactions    | Engine ✅ Player ✅ |
| [Asset](docs/systems/asset-system.md)           | ComfyUI, image generation, gallery                | Engine ✅ Player ✅ |

---

## UI Mockups

### Player View - Visual Novel Interface

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │                      [BACKDROP IMAGE]                                │   │
│  │                    The Rusty Anchor Tavern                           │   │
│  │                                                                      │   │
│  │      [NPC Sprite]              [NPC Sprite]              [NPC Sprite]│   │
│  │       (dimmed)                 (speaking)                 (dimmed)   │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [Portrait] Marcus                                                    │   │
│  │                                                                      │   │
│  │ "Welcome to the Rusty Anchor. What brings you here tonight?"        │   │
│  │ [▌ typewriter cursor]                                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│  │ Talk       │ │ Examine    │ │ Travel     │ │ Character  │               │
│  │ [Marcus]   │ │ [Room]     │ │ [Exit]     │ │ [Sheet]    │               │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### DM View - Approval Popup

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  LLM Response - Awaiting Approval                                    [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Player: "Can you help me find the Baron?"                                  │
│                                                                             │
│  ─── NPC Response ──────────────────────────────────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Marcus leans in, lowering his voice. "The Baron? A dangerous man    │   │
│  │ to seek. But I've heard rumors... he frequents the docks at night,  │   │
│  │ meeting with smugglers. If you're serious, I might know someone     │   │
│  │ who can help."                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Internal Reasoning (hidden from player) ───────────────────────────── │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Marcus has information about the Baron due to his past as a         │   │
│  │ mercenary. He's willing to help because the player previously       │   │
│  │ helped him. Suggesting a contact creates a hook for the next event. │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Suggested Tools ───────────────────────────────────────────────────── │
│  ☑ RevealInfo: "Baron frequents docks at night"                            │
│  ☐ ChangeRelationship: Marcus → Player (+0.1)                              │
│                                                                             │
│  [Accept] [Modify] [Reject] [Take Over]                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Challenge Roll Modal

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Persuasion Check                                                    [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Convince the guard to let you pass without papers.                         │
│                                                                             │
│  Difficulty: DC 15 (Medium)                                                 │
│                                                                             │
│  Your Persuasion: +4                                                        │
│                                                                             │
│                      ┌─────────────────┐                                    │
│                      │       🎲        │                                    │
│                      │   [ Roll d20 ]  │                                    │
│                      └─────────────────┘                                    │
│                                                                             │
│  [After rolling]                                                            │
│                                                                             │
│                           🎲 17 + 4 = 21                                    │
│                      ✓ SUCCESS (DC 15)                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Architecture Overview

WrldBldr uses a **simplified hexagonal architecture**: we keep port traits only at true infrastructure boundaries (DB/LLM/queues/clock/random), and internal engine code calls internal engine code directly.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              WrldBldr Architecture                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                           SHARED KERNEL                                     │ │
│  │  ┌─────────────────────────────────────────────────────────────────────┐   │ │
│  │  │  protocol (wrldbldr-protocol)                                        │   │ │
│  │  │  ├── Wire-format DTOs (REST + WebSocket)                             │   │ │
│  │  │  ├── ClientMessage / ServerMessage enums                             │   │ │
│  │  │  └── RequestPayload / ResponseResult                                 │   │ │
│  │  └─────────────────────────────────────────────────────────────────────┘   │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                    ┌─────────────────┴─────────────────┐                        │
│                    │                                   │                        │
│                    ▼                                   ▼                        │
│  ┌─────────────────────────────────┐   ┌─────────────────────────────────┐     │
│  │         ENGINE SIDE              │   │         PLAYER SIDE             │     │
│  │                                  │   │                                 │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   domain (innermost)      │  │   │  │   (shares domain crate)   │  │     │
│  │  │   Pure business entities  │  │   │  │                           │  │     │
│  │  └───────────────────────────┘  │   │  └───────────────────────────┘  │     │
│  │              │                  │   │              │                  │     │
│  │  ┌───────────────────────────┐  │   │  ┌───────────────────────────┐  │     │
│  │  │   engine                  │  │   │  │   player-ports            │  │     │
│  │  │   entities/use_cases/api  │  │   │  │   Port trait definitions  │  │     │
│  │  │   infra behind port traits│  │   │  └───────────────────────────┘  │     │
│  │  └───────────────────────────┘  │   │              │                  │     │
│  │                                  │   │  ┌───────────────────────────┐  │     │
│  │                                  │   │  │   player-app              │  │     │
│  │                                  │   │  │   Client use cases        │  │     │
│  │                                  │   │  └───────────────────────────┘  │     │
│  │                                  │   │              │                  │     │
│  │                                  │   │  ┌───────────────────────────┐  │     │
│  │                                  │   │  │   player-adapters         │  │     │
│  │                                  │   │  │   WS client, storage      │  │     │
│  │                                  │   │  └───────────────────────────┘  │     │
│  │                                  │   │              │                  │     │
│  │                                  │   │  ┌───────────────────────────┐  │     │
│  │                                  │   │  │   player-ui               │  │     │
│  │                                  │   │  │   Dioxus presentation     │  │     │
│  │                                  │   │  └───────────────────────────┘  │     │
│  │                                  │   │              │                  │     │
│  │                                  │   │  ┌───────────────────────────┐  │     │
│  │                                  │   │  │   player-runner           │  │     │
│  │                                  │   │  │   WASM/desktop entrypoint │  │     │
│  │                                  │   │  └───────────────────────────┘  │     │
│  │                                  │   │              │                  │     │
│  └──────────────────────────────────┘   └─────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Crate Overview

| Crate                      | Layer         | Purpose                                                        |
| -------------------------- | ------------- | -------------------------------------------------------------- |
| `wrldbldr-domain`          | Domain        | Pure business types (entities, value objects, typed IDs)       |
| `wrldbldr-protocol`        | Shared Kernel | Wire-format types for Engine↔Player communication              |
| `wrldbldr-engine`          | Engine        | Server-side code (entities/use_cases/api + infra behind ports) |
| `wrldbldr-player-ports`    | Ports         | Player-side ports (transport/storage boundaries)               |
| `wrldbldr-player-app`      | Application   | Player use cases and orchestration                             |
| `wrldbldr-player-adapters` | Adapters      | WebSocket client + platform implementations                    |
| `wrldbldr-player-ui`       | Presentation  | Dioxus components and routes                                   |
| `wrldbldr-player-runner`   | Runner        | Client entry point (WASM/desktop)                              |

---

## Technology Stack

### Engine (Backend)

- **Rust** with Tokio async runtime
- **Axum** for HTTP/WebSocket server
- **Neo4j** graph database for all game state
- **SQLite** for queue persistence
- **Ollama** for LLM integration (OpenAI-compatible API)
- **ComfyUI** for AI image generation

### Player (Frontend)

- **Rust** compiled to WebAssembly
- **Dioxus** for reactive UI
- **Tailwind CSS** for styling
- Supports both **web** (WASM) and **desktop** (native) builds

---

## Quick Start

### Prerequisites

**Option A: Nix (Recommended)**

```bash
nix-shell
```

**Option B: Manual**

- Rust toolchain with `wasm32-unknown-unknown` target
- Dioxus CLI (`cargo install dioxus-cli`)
- Task runner (`brew install go-task` on macOS)
- Node.js 20+ for Tailwind CSS

### Setup

```bash
# 1. Clone and enter the repository
cd WrldBldr/Game

# 2. Install dependencies
task setup

# 3. Start Neo4j database
task docker:up

# 4. Configure environment
cp .env.example .env
# Edit .env with your Neo4j password and service URLs

# 5. Validate architecture
task arch-check

# 6. Run development servers
task dev    # Runs both Engine and Player
```

### Individual Services

```bash
task backend       # Engine server only (port 3000)
task web:dev       # Player web client (port 8080)
task desktop:dev   # Player desktop application
```

---

## Development Commands

| Command           | Description                                    |
| ----------------- | ---------------------------------------------- |
| `task dev`        | Run both backend and frontend                  |
| `task build`      | Build all crates (runs arch-check first)       |
| `task test`       | Run all workspace tests                        |
| `task arch-check` | **Required** - Validate hexagonal architecture |
| `task fmt`        | Format all code                                |
| `task clippy`     | Run Clippy linter                              |
| `task docs`       | Generate and open documentation                |

---

## Environment Variables

| Variable           | Default                     | Description                  |
| ------------------ | --------------------------- | ---------------------------- |
| `NEO4J_URI`        | `bolt://localhost:7687`     | Neo4j connection             |
| `NEO4J_USER`       | `neo4j`                     | Database username            |
| `NEO4J_PASSWORD`   | -                           | Database password (required) |
| `OLLAMA_BASE_URL`  | `http://localhost:11434/v1` | LLM API endpoint             |
| `OLLAMA_MODEL`     | `qwen3-vl:30b`              | LLM model name               |
| `COMFYUI_BASE_URL` | `http://localhost:8188`     | Image generation endpoint    |
| `SERVER_PORT`      | `3000`                      | Engine HTTP port             |

---

## Documentation

| Document                                                                                   | Description                |
| ------------------------------------------------------------------------------------------ | -------------------------- |
| [docs/\_index.md](docs/_index.md)                                                          | Documentation overview     |
| [docs/architecture/hexagonal-architecture.md](docs/architecture/hexagonal-architecture.md) | Architecture deep-dive     |
| [docs/architecture/neo4j-schema.md](docs/architecture/neo4j-schema.md)                     | Database schema            |
| [docs/architecture/websocket-protocol.md](docs/architecture/websocket-protocol.md)         | Wire protocol              |
| [docs/systems/](docs/systems/)                                                             | Game system specifications |
| [AGENTS.md](AGENTS.md)                                                                     | AI assistant guidelines    |

---

## Contributing

1. Read the relevant [system document](docs/systems/) before implementing
2. Follow [hexagonal architecture](docs/architecture/hexagonal-architecture.md) rules
3. Run `task arch-check` before committing - **must pass**
4. Update system docs with implementation summary when complete

See:

- Engine contribution guide lives under [crates/engine/](crates/engine/)
- [crates/player-runner/README.md](crates/player-runner/README.md) for Player contribution guide

---

## License

[Add your license here]
