# [System Name] System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


One paragraph explaining what this system does and why it exists in the game.

---

## Game Design

How this system affects player experience. What gameplay loop it enables. Why it matters for storytelling.

---

## User Stories

### Implemented

- [x] **US-001**: As a [role], I can [action] so that [benefit]
  - *Implementation*: Brief summary of how it was done
  - *Files*: `crates/engine/src/...`, `crates/player-*/src/...`

- [x] **US-002**: As a [role], I can [action] so that [benefit]
  - *Implementation*: Summary
  - *Files*: Key files

### Pending

- [ ] **US-003**: As a [role], I can [action] so that [benefit]

---

## UI Mockups

### [Feature Name]

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ASCII mockup of the UI                                                      │
│                                                                             │
│  ┌──────────┐    ┌──────────┐                                               │
│  │ Button 1 │    │ Button 2 │                                               │
│  └──────────┘    └──────────┘                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented / ⏳ Pending

---

## Data Model

### Neo4j Nodes

```cypher
(:NodeType {
    id: "uuid",
    name: "string",
    property: "type"
})
```

### Neo4j Edges

```cypher
(a:TypeA)-[:EDGE_TYPE {
    property: "value"
}]->(b:TypeB)
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/resource` | List resources | ✅ |
| POST | `/api/resource` | Create resource | ✅ |
| GET | `/api/resource/{id}` | Get by ID | ✅ |
| PUT | `/api/resource/{id}` | Update | ✅ |
| DELETE | `/api/resource/{id}` | Delete | ✅ |

### WebSocket Messages

#### Client → Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `MessageType` | `field1`, `field2` | Description |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `MessageType` | `field1`, `field2` | Description |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Domain Entity | ✅ | - | `entity.rs` |
| Repository | ✅ | - | Neo4j CRUD |
| Service | ✅ | ✅ | Business logic |
| HTTP Routes | ✅ | - | REST API |
| WebSocket | ✅ | ✅ | Real-time |
| UI Component | - | ⏳ | Pending |

---

## Key Files

### Engine (Simplified Architecture)

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/xxx.rs` | Entity definition |
| Domain | `crates/domain/src/value_objects/xxx.rs` | Value objects |
| Repository | `crates/engine/src/repositories/xxx.rs` | Repository wrapper |
| Use Case | `crates/engine/src/use_cases/xxx/mod.rs` | Orchestration logic |
| Ports | `crates/engine/src/infrastructure/ports.rs` | All port traits |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/xxx_repo.rs` | Neo4j implementation |
| API | `crates/engine/src/api/websocket/mod.rs` | WebSocket handlers |
| API | `crates/engine/src/api/http.rs` | HTTP routes |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Ports | `crates/player/src/ports/outbound/*.rs` | Port traits |
| Application | `crates/player/src/application/services/xxx_service.rs` | Client service |
| Infrastructure | `crates/player/src/infrastructure/*.rs` | API/WS/platform impls |
| UI | `crates/player/src/ui/presentation/components/xxx.rs` | UI component |
| UI | `crates/player/src/ui/presentation/state/xxx_state.rs` | Reactive state |

---

## Related Systems

- **Depends on**: System A, System B
- **Used by**: System C, System D

---

## Revision History

| Date | Change |
|------|--------|
| YYYY-MM-DD | Initial version |
