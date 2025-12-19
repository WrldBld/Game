# WrldBldr MVP Plan

**Created:** 2025-12-17  
**Status:** ACTIVE - Primary Implementation Reference  
**Target:** Playable TTRPG game loop without tactical combat

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Project Vision](#2-project-vision)
3. [Architecture Compliance Rules](#3-architecture-compliance-rules)
4. [Acceptance Criteria](#4-acceptance-criteria)

---

## 1. Executive Summary

### Goal

Deliver a **playable TTRPG game loop** where:
1. Players interact with NPCs through a visual novel interface
2. An LLM (Ollama) generates NPC responses informed by deep narrative context
3. The DM approves/modifies all AI-generated content before players see it
4. Character motivations, relationships, and narrative arcs drive emergent storytelling

### Key Innovations

1. **Pure Neo4j Graph Model**: All entities and their relationships stored as nodes and edges - no JSON blobs for relational data
2. **Per-Character Actantial Model**: Every character (PC/NPC) has their own view of who helps them, opposes them, and what they desire
3. **Rich Location Relationships**: Characters have HOME_LOCATION, WORKS_AT, FREQUENTS relationships with locations
4. **Challenge-Location Binding**: Challenges can be location-bound and unlocked by events
5. **Context-Aware LLM**: Rich narrative context with configurable token budgets and automatic summarization
6. **Dual Trigger System**: Both Engine and LLM can suggest narrative event triggers

### Out of Scope

- Tactical combat (grid maps, turn order, pathfinding)
- Multi-platform builds (WASM, Android) - desktop only
- Save/Load system (session state persists in Neo4j, no explicit save files)

---

## 2. Project Vision

### The Game Loop

```
Player Action → Engine Context → LLM Response → DM Approval → State Updated
     ↑                                                              │
     └──────────────────────────────────────────────────────────────┘
```

### Detailed Flow

1. **Player performs action** (speak to NPC, examine object, use item)
2. **Engine builds rich context** via graph traversal
3. **Context budget enforced** with automatic summarization
4. **LLM generates response** with tool calls and suggestions
5. **DM reviews and decides** (accept/modify/reject/takeover)
6. **Approved content delivered** with state updates

### Narrative Frameworks

**Campbell's Monomyth**: Each Act corresponds to a Hero's Journey stage (12 stages from Ordinary World to Return with Elixir)

**Greimas Actantial Model**: Per-character relationship mapping:
- SENDER → OBJECT ← RECEIVER
- HELPER ← SUBJECT → OPPONENT

See [character-system.md](../systems/character-system.md) for full details.

---

## 3. Architecture Compliance Rules

### Hexagonal Architecture Principles

```
Domain Layer (innermost)     → Pure Rust, no framework dependencies
Application Layer            → Services, Use Cases, DTOs, Ports
Infrastructure Layer         → Repositories, External clients, HTTP/WS
Presentation Layer (Player)  → UI Components, Views, State
```

### Import Rules

**NEVER ALLOWED:**
- Domain importing from application/infrastructure
- Application importing from infrastructure

**ALWAYS REQUIRED:**
- Infrastructure implements application ports
- Services depend on port traits, not concrete implementations

### Violation Approval Process

1. Document in this MVP.md with explanation
2. Await explicit user approval
3. Mark in code with `// ARCHITECTURE VIOLATION: [APPROVED DATE]`

### Currently Accepted Violations

| Location | Description | Justification |
|----------|-------------|---------------|
| `Player/src/presentation/services.rs` | Type aliases expose `ApiAdapter` | Composition layer pattern |
| `Engine/src/domain/entities/*.rs` | Serde derives on some domain types | Required for JSON serialization |

See [hexagonal-architecture.md](../architecture/hexagonal-architecture.md) for full architecture documentation.

---

## 4. Acceptance Criteria

### MVP Complete When:

1. **Data Model is Pure Graph**
   - No JSON blobs for entity references
   - All relationships are Neo4j edges
   - Acceptable JSON only for non-relational data

2. **Location System Works**
   - Hierarchy via CONTAINS edges
   - Navigation via CONNECTED_TO edges
   - NPCs have location relationships

3. **Character System Works**
   - Wants as separate nodes
   - Actantial model via VIEWS_AS_* edges
   - Inventory via POSSESSES edges

4. **Challenge System Works**
   - Challenges bound to locations
   - Challenges can unlock locations
   - Challenges enabled/disabled by events

5. **LLM Receives Rich Context**
   - Context built via graph queries
   - All categories included
   - Summarization works when needed

6. **Triggers Work Both Ways**
   - Engine evaluates triggers via graph
   - LLM can suggest triggers
   - All triggers go to DM approval

7. **Game Loop is Playable**
   - Can select world and start session
   - Can speak to NPCs
   - LLM generates contextual responses
   - DM can approve/modify/reject
   - Tool calls execute via graph operations

8. **Architecture is Clean**
   - No unapproved violations
   - All ports properly implemented
   - Services use dependency injection

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [_index.md](../_index.md) | Documentation overview |
| [systems/](../systems/) | Game system specifications |
| [architecture/](../architecture/) | Technical architecture docs |
| [ROADMAP.md](./ROADMAP.md) | Implementation progress tracking |

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-17 | Initial MVP plan created |
| 2025-12-18 | Reorganized: Detailed specs moved to systems/ and architecture/ |
