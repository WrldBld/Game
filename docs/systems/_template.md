# [System Name] System

## Overview

One paragraph explaining what this system does and why it exists in the game.

---

## Game Design

How this system affects player experience. What gameplay loop it enables. Why it matters for storytelling.

---

## User Stories

### Implemented

- [x] **US-001**: As a [role], I can [action] so that [benefit]
  - *Implementation*: Brief summary of how it was done
  - *Files*: `crates/engine-*/src/...`, `crates/player-*/src/...`

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

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/xxx.rs` | Entity definition |
| Domain | `crates/domain/src/value_objects/xxx.rs` | Value objects |
| Ports | `crates/engine-ports/src/outbound/*.rs` | Port traits |
| Application | `crates/engine-app/src/application/services/xxx_service.rs` | Business logic |
| Adapters | `crates/engine-adapters/src/infrastructure/persistence/xxx_repository.rs` | Neo4j impl |
| Adapters | `crates/engine-adapters/src/infrastructure/websocket/handlers/*.rs` | WebSocket handlers |
| Adapters | `crates/engine-adapters/src/infrastructure/http/*.rs` | HTTP routes (uploads, exports) |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Ports | `crates/player-ports/src/outbound/*.rs` | Port traits |
| Application | `crates/player-app/src/application/services/xxx_service.rs` | Client service |
| Adapters | `crates/player-adapters/src/infrastructure/*.rs` | API/WS/platform impls |
| UI | `crates/player-ui/src/presentation/components/xxx.rs` | UI component |
| UI | `crates/player-ui/src/presentation/state/xxx_state.rs` | Reactive state |

---

## Related Systems

- **Depends on**: [System A](./system-a.md), [System B](./system-b.md)
- **Used by**: [System C](./system-c.md), [System D](./system-d.md)

---

## Revision History

| Date | Change |
|------|--------|
| YYYY-MM-DD | Initial version |
