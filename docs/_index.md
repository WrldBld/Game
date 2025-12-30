# WrldBldr Documentation

**WrldBldr** is a TTRPG management system with an AI-powered game master assistant.

- **Engine** (Rust/Axum): World creation tool with Neo4j database, Ollama LLM integration
- **Player** (Rust/Dioxus): Gameplay client with visual novel interface

---

## Game Systems

Core gameplay mechanics, each with user stories, UI mockups, and implementation status.

| System | Description | Status |
|--------|-------------|--------|
| [Navigation](systems/navigation-system.md) | Locations, regions, movement, game time | Engine ✅ Player ⏳ |
| [NPC](systems/npc-system.md) | NPC presence, location rules, DM events | Engine ✅ Player ⏳ |
| [Character](systems/character-system.md) | NPCs, PCs, archetypes, actantial model, relationships, inventory | Engine ✅ Player ✅ |
| [Observation](systems/observation-system.md) | Player knowledge tracking, known NPCs | Engine ✅ Player ⏳ |
| [Challenge](systems/challenge-system.md) | Skill checks, dice, outcomes, rule systems | Engine ✅ Player ✅ |
| [Narrative](systems/narrative-system.md) | Events, triggers, effects, chains | Engine ✅ Player ✅ |
| [Dialogue](systems/dialogue-system.md) | LLM integration, DM approval, tool calls | Engine ✅ Player ✅ |
| [Scene](systems/scene-system.md) | Visual novel, backdrops, sprites, interactions | Engine ✅ Player ✅ |
| [Asset](systems/asset-system.md) | ComfyUI, image generation, gallery | Engine ✅ Player ✅ |

---

## Architecture

Technical reference for developers.

| Document | Description |
|----------|-------------|
| [Neo4j Schema](architecture/neo4j-schema.md) | Complete graph database schema |
| [Hexagonal Architecture](architecture/hexagonal-architecture.md) | Ports/adapters patterns, layer rules |
| [WebSocket Protocol](architecture/websocket-protocol.md) | All client/server message types |
| [Queue System](architecture/queue-system.md) | Action and approval queue architecture |

---

## Progress

| Document | Description |
|----------|-------------|
| [MVP](progress/MVP.md) | Project vision, acceptance criteria |
| [Roadmap](progress/ROADMAP.md) | Remaining work, priority tiers |
| [Active Development](progress/ACTIVE_DEVELOPMENT.md) | Current phase tracking, user story status |

---

## Quick Links

### Engine
- **Run**: `cd Engine && nix-shell --run "cargo run"`
- **Check**: `cd Engine && nix-shell --run "cargo check"`
- [Engine README](../Engine/README.md)

### Player
- **Run Desktop**: `cd Player && nix-shell --run "cargo run"`
- **Run Web**: `cd Player && trunk serve`
- [Player README](../Player/README.md)

---

## Contributing

1. Read the relevant [system document](systems/) before implementing
2. Follow [hexagonal architecture](architecture/hexagonal-architecture.md) rules
3. Update system doc with implementation summary when complete
4. Keep [ROADMAP](progress/ROADMAP.md) current
