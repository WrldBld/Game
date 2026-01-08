# WebSocket-First Architecture

**Created**: 2025-12-25  
**Status**: PROPOSAL  
**Estimated Effort**: 2-3 days  
**Related Backlog**: P3.6 in `docs/progress/IMPLEMENTATION_BACKLOG.md`

---

## Background

The WrldBldr application currently uses a hybrid communication model:
- **REST HTTP** for CRUD operations and initial data fetching
- **WebSocket** for real-time updates, game state sync, and LLM streaming

This creates duplication, complexity, and inconsistent patterns across the codebase.

---

## Current State Analysis

### REST Endpoints (engine-adapters/http/)

| Category | Routes File | Purpose |
|----------|-------------|---------|
| Characters | `character_routes.rs` | CRUD for NPCs, stats, inventory |
| Locations | `location_routes.rs` | CRUD for locations, regions |
| Scenes | `scene_routes.rs` | Scene management, backdrops |
| Goals | `goal_routes.rs` | World goal CRUD |
| Challenges | `challenge_routes.rs` | Challenge resolution |
| Story Events | `story_event_routes.rs` | Timeline events |
| Templates | `prompt_template_routes.rs` | Prompt template CRUD |
| Images | `image_routes.rs` | Asset uploads/downloads |
| Generation | `generation_routes.rs` | ComfyUI batch generation |

### WebSocket Messages (protocol crate)

| Category | Message Types | Purpose |
|----------|--------------|---------|
| Session | `SessionJoined`, `JoinSession` | Connection management |
| Scene | `SceneUpdate`, `SceneChanged` | Real-time scene sync |
| Dialogue | `DialogueResponse`, `NpcSpeaks` | LLM conversation streaming |
| Actantial | `NpcWantCreated/Updated/Deleted`, etc. | Motivation system events |
| Staging | `StagingApprovalRequest`, `StagingApproved` | NPC staging workflow |
| Challenges | `ChallengeRollSubmitted`, `ChallengeResolved` | Dice roll resolution |
| Generation | `GenerationProgress`, `GenerationComplete` | Image generation updates |

---

## Problems with Current Architecture

### 1. Duplicate Data Fetching Patterns
- `motivations_tab.rs` fetches via REST service, then subscribes to WebSocket for updates
- `character_routes.rs` returns character data, but so does `NpcActantialContextResponse`
- Forces UI to maintain complex refresh logic (e.g., `actantial_refresh_counter`)

### 2. Inconsistent Error Handling
- REST errors propagate as HTTP status codes → parsed by adapters
- WebSocket errors sent as `ServerMessage::Error` → different error path
- No unified error recovery pattern

### 3. State Synchronization Overhead
- Initial load via REST, then WebSocket for updates
- Race conditions possible if WebSocket message arrives before REST response
- Multiple sources of truth for same data

### 4. Authentication Split
- REST uses session cookie/token validation per request
- WebSocket validates once on connect, then trusts connection
- Different security boundaries

---

## Proposed Solution: WebSocket-First

### Core Principle
All data operations flow through WebSocket. REST remains only for:
- Static asset serving (images, files)
- Initial WebSocket handshake/auth
- Health checks and metrics

### Request-Response Pattern for WebSocket

Add request IDs to support request-response over WebSocket:

```rust
// Client sends
ClientMessage::Request {
    request_id: String,
    payload: RequestPayload,
}

enum RequestPayload {
    GetCharacter { character_id: String },
    ListGoals { world_id: String },
    CreateWant { npc_id: String, data: CreateWantData },
    // ... all current REST operations
}

// Server responds
ServerMessage::Response {
    request_id: String,
    result: ResponseResult,
}

enum ResponseResult {
    Success(serde_json::Value),
    Error { code: String, message: String },
}
```

### Benefits

1. **Single Connection Model**
   - One authenticated connection for all operations
   - Automatic reconnection with session restore
   - Lower latency (no new TCP connections)

2. **Unified Event Flow**
   - All state changes broadcast to all interested clients
   - No more polling or refresh counters needed
   - Natural multiplayer support

3. **Simpler UI Code**
   ```rust
   // Before: REST + WebSocket subscription
   let data = rest_service.get_character(id).await?;
   use_effect(|| { /* subscribe to updates */ });
   
   // After: WebSocket request + automatic updates
   let data = use_resource(|| ws.request(GetCharacter { id }));
   // Updates automatically via existing message handlers
   ```

4. **Transaction Support**
   - Batch operations with rollback
   - Optimistic updates with server confirmation

---

## Migration Plan

### Phase 1: Infrastructure (1 day)
1. Add `Request`/`Response` message types to protocol
2. Implement request routing in `crates/engine/src/api/websocket/mod.rs`
3. Add request timeout handling
4. Create `WebSocketService` in player-app with `request()` method

### Phase 2: Character Operations (0.5 day)
1. Migrate `character_routes.rs` endpoints to WebSocket handlers
2. Update `use_character_service()` to use WebSocket
3. Remove REST-based character fetching from UI
4. Keep REST endpoint as deprecated fallback

### Phase 3: Actantial Operations (0.5 day)
1. Already mostly WebSocket - consolidate `get_actantial_context`
2. Remove `actantial_refresh_counter` pattern
3. Pure reactive updates from message handlers

### Phase 4: Remaining Operations (0.5 day)
1. Goals, scenes, locations → WebSocket
2. Challenge resolution already WebSocket
3. Generation status already WebSocket

### Phase 5: Cleanup (0.5 day)
1. Remove unused REST routes
2. Update API documentation
3. Remove REST adapter code

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Connection drops lose pending requests | Request queue with reconnect replay |
| Large payloads over WebSocket | Chunked responses, pagination |
| Testing complexity | Mock WebSocket server for unit tests |
| Browser WebSocket limits | Connection pooling, message batching |

---

## Decision Points

1. **Keep REST for any operations?**
   - Recommendation: Only static assets and health checks
   
2. **How to handle offline/disconnected state?**
   - Option A: Queue requests, replay on reconnect
   - Option B: Disable UI until connected (current behavior)

3. **Backward compatibility period?**
   - Recommendation: 2 sprints with both paths, then deprecate REST

---

## Related Files

- `crates/engine/src/api/websocket/mod.rs` - Server WebSocket handler + routing
- `crates/player/src/infrastructure/websocket/mod.rs` - Client connection
- `crates/protocol/src/lib.rs` - Message definitions
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs` - UI message handling
