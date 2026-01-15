# WrldBldr Documentation

**WrldBldr** is a TTRPG management system with an AI-powered game master assistant.

- **Engine** (Rust/Axum): World creation tool with Neo4j database, Ollama LLM integration
- **Player** (Rust/Dioxus): Gameplay client with visual novel interface (runner owns composition root; UI is presentation-only)

---

## Game Systems

Core gameplay mechanics, each with user stories, UI mockups, and implementation status.

| System                                               | Description                                       | Status              |
| ---------------------------------------------------- | ------------------------------------------------- | ------------------- |
| [Navigation](systems/navigation-system.md)           | Locations, regions, movement, game time           | Engine ✅ Player ⏳ |
| [NPC](systems/npc-system.md)                         | NPC presence, location rules, DM events           | Engine ✅ Player ⏳ |
| [Character](systems/character-system.md)             | NPCs, PCs, archetypes, relationships              | Engine ✅ Player ✅ |
| [Actantial Model](systems/actantial-system.md)       | NPC motivations, wants, helpers/opponents         | Engine ✅ Player ✅ |
| [Inventory](systems/inventory-system.md)             | Items, equipment, containers, region items        | Engine ✅ Player ✅ |
| [Staging](systems/staging-system.md)                 | NPC presence approval, DM workflow                | Engine ✅ Player ⏳ |
| [Observation](systems/observation-system.md)         | Player knowledge tracking, known NPCs             | Engine ✅ Player ⏳ |
| [Challenge](systems/challenge-system.md)             | Skill checks, dice, outcomes, rule systems        | Engine ✅ Player ✅ |
| [Narrative](systems/narrative-system.md)             | Events, triggers, effects, chains                 | Engine ✅ Player ✅ |
| [Dialogue](systems/dialogue-system.md)               | LLM integration, DM approval, tool calls          | Engine ✅ Player ✅ |
| [Scene](systems/scene-system.md)                     | Visual novel, backdrops, sprites, interactions    | Engine ✅ Player ✅ |
| [Asset](systems/asset-system.md)                     | ComfyUI, image generation, gallery                | Engine ✅ Player ✅ |
| [Prompt Template](systems/prompt-template-system.md) | Configurable LLM prompts, per-world customization | Engine ✅ Player ⏳ |

---

## Architecture

Technical reference for developers.

| Document                                                         | Description                              |
| ---------------------------------------------------------------- | ---------------------------------------- |
| [Code Review Guidelines](architecture/review.md)                 | Full Rustic DDD spec, review criteria    |
| [Review Checklist](REVIEW_CHECKLIST.md)                          | Quick reference for code reviewers       |
| [Neo4j Schema](architecture/neo4j-schema.md)                     | Complete graph database schema           |
| [Hexagonal Architecture](architecture/hexagonal-architecture.md) | Ports/adapters patterns, layer rules     |
| [WebSocket Protocol](architecture/websocket-protocol.md)         | All client/server message types          |
| [Queue System](architecture/queue-system.md)                     | Action and approval queue architecture   |
| [E2E Testing](architecture/e2e-testing.md)                       | VCR LLM, event logging, testcontainers   |

### Architecture Decision Records (ADRs)

| ADR | Decision |
| --- | -------- |
| [ADR-001](architecture/ADR-001-uuid-generation-in-domain.md) | UUID generation allowed in domain |
| [ADR-002](architecture/ADR-002-hexagonal-pragmatism.md) | Pragmatic hexagonal (~10 port traits) |
| [ADR-003](architecture/ADR-003-neo4j-graph-model.md) | Neo4j as primary storage |
| [ADR-004](architecture/ADR-004-vcr-llm-testing.md) | VCR-based LLM testing |
| [ADR-005](architecture/ADR-005-websocket-protocol.md) | WebSocket protocol design |
| [ADR-006](architecture/ADR-006-llm-port-design.md) | LLM port trait design |
| [ADR-007](architecture/ADR-007-ttl-cache-ephemeral-state.md) | TtlCache for ephemeral state |

---

## Progress

| Document                                             | Description                               |
| ---------------------------------------------------- | ----------------------------------------- |
| [MVP](progress/MVP.md)                               | Project vision, acceptance criteria       |
| [Roadmap](progress/ROADMAP.md)                       | Remaining work, priority tiers            |
| [Active Development](progress/ACTIVE_DEVELOPMENT.md) | Current phase tracking, user story status |

---

## Plans

| Document                                                                        | Description                                              |
| ------------------------------------------------------------------------------- | -------------------------------------------------------- |
| [Strict Review Remediation Plan](plans/STRICT_REVIEW_REMEDIATION_PLAN.md)       | Active architecture remediation plan (strict layering)   |
| [Mood & Expression System Plan](plans/MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md) | P3.1 feature plan for emotional model + UI               |
| [Playtestable State Plan](plans/PLAYTESTABLE_STATE_PLAN.md)                     | Stabilize core loop for playtesting                      |
| [Player Architecture Simplification](plans/PLAYER_ARCHITECTURE_SIMPLIFICATION.md) | Draft plan to reduce client-side abstraction             |
| [Behavior Testing + TDD Plan](plans/BEHAVIOR_TESTING_TDD_PLAN.md)               | Testing strategy and workflow                            |
| [Implementation Gaps Plan](plans/IMPLEMENTATION_GAPS_PLAN.md)                   | Wiring gaps checklist (engine/player coverage)           |
| [Use Case Wiring Audit](plans/USE_CASE_WIRING_AUDIT.md)                         | Audit of wired vs unwired use cases                       |
| [Systems Review and Fixes](plans/SYSTEMS_REVIEW_AND_FIXES.md)                    | Ongoing systems doc audit and fixes                      |
| [Simplified Architecture](plans/SIMPLIFIED_ARCHITECTURE.md)                     | Active architecture baseline                             |
| [WebSocket Architecture](plans/WEBSOCKET_ARCHITECTURE.md)                       | Proposal for WS-first approach                           |

---

## Quick Links

All commands should run inside the repo Nix shell:

- One-shot: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && <cmd>"`

### Engine

- **Run**: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && task backend"`
- **Check**: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && cargo check -p wrldbldr-engine"`

### Player

- **Run Desktop**: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && task desktop:dev"`
- **Run Web**: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && task web:dev"`

---

## Contributing

1. Read the relevant [system document](systems/) before implementing
2. Follow [hexagonal architecture](architecture/hexagonal-architecture.md) rules
3. Update system doc with implementation summary when complete
4. Keep [ROADMAP](progress/ROADMAP.md) current
