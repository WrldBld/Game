# ADR-005: WebSocket Protocol Design

## Status

Accepted

## Date

2026-01-13

## Context

WrldBldr requires real-time communication between Player clients and the Engine server for:
- Live game sessions with multiple players
- DM approval flows requiring immediate response
- Real-time NPC dialogue
- Multiplayer position updates
- Challenge resolution broadcasts

Requirements:
1. Bidirectional communication (server can push updates)
2. Low latency for responsive gameplay
3. Support for multiple concurrent clients per world
4. Structured message routing
5. Cross-platform (desktop + web)

## Decision

Use **WebSocket** with a **world-scoped connection model**:

1. **Single WebSocket endpoint** (`/ws`)
2. **World-scoped connections**: Clients join specific worlds, not global sessions
3. **JSON-encoded messages** with `type` field for routing
4. **Request/Response pattern** with `request_id` correlation
5. **Server-push events** for broadcasts (scene updates, dialogue, etc.)

Message taxonomy:
- `ClientMessage`: Player-to-Engine (actions, requests)
- `ServerMessage`: Engine-to-Player (responses, broadcasts)
- `RequestPayload`: Nested enum for CRUD operations within `ClientMessage::Request`

## Consequences

### Positive

- Full-duplex communication enables real-time gameplay
- Low latency compared to polling
- World-scoped model simplifies session management
- JSON is human-readable and debuggable
- Works in browsers (WASM) and native (desktop)

### Negative

- More complex than REST for simple operations
- Connection management overhead (reconnects, heartbeats)
- Stateful connections require careful resource cleanup
- Binary protocols would be more bandwidth-efficient

### Neutral

- Requires different handling on desktop vs WASM platforms
- Message routing adds dispatch complexity

## Protocol Design Decisions

### World-Scoped vs Session-Scoped

Chose world-scoped because:
- Players join worlds, not sessions (a world can have multiple sessions)
- Simpler mental model: "connect to world X as player/DM"
- Allows spectator mode for same world

### Request/Response Pattern

Used `request_id` correlation because:
- Client can track in-flight requests
- Enables async handling (request A, request B, response B, response A)
- Error responses include original request context

### JSON vs Binary

Chose JSON because:
- Easier debugging (can read messages in logs)
- Smaller engineering team (binary requires more tooling)
- Messages are mostly text (dialogue, names, descriptions)
- Performance is acceptable for TTRPG turn-based gameplay

## Message Flow Examples

### Join World
```
Client -> JoinWorld { world_id, role: "player", pc_id }
Server -> WorldJoined { snapshot, connected_users }
```

### Request/Response
```
Client -> Request { request_id: "abc", payload: GetCharacter { id } }
Server -> Response { request_id: "abc", result: Success { data } }
```

### Server Push
```
Server -> DialogueResponse { speaker_id, text, choices }
Server -> ChallengePrompt { challenge_id, difficulty }
```

## Alternatives Considered

### 1. REST API with Polling

Traditional HTTP endpoints with client polling for updates.

**Rejected:** Too high latency for real-time gameplay. Would require frequent polling or long-polling which increases server load.

### 2. Server-Sent Events (SSE)

One-way server-to-client streaming with REST for client-to-server.

**Rejected:** Adds complexity of managing two different transport mechanisms. WebSocket provides unified bidirectional channel.

### 3. gRPC with Bidirectional Streaming

Protocol buffers with streaming RPCs.

**Rejected:** Poor browser support (requires grpc-web proxy). Would complicate WASM client significantly.

### 4. Socket.io

WebSocket abstraction with fallback transports.

**Rejected:** Adds dependency without clear benefit. Native WebSocket is sufficient and more portable.

## References

- [websocket-protocol.md](websocket-protocol.md) - Full protocol documentation
- WebSocket RFC: https://datatracker.ietf.org/doc/html/rfc6455
