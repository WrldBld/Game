# WebSocket Adapter Refactoring Plan - Final Version

## Implementation Status

| Phase | Description | Status | Notes |
|-------|-------------|--------|-------|
| 1 | Infrastructure Helpers | ✅ Complete | HandlerContext, Auth middleware |
| 2 | Ports & Domain Events | ✅ Complete | BroadcastPort, GameEvent, UseCaseContext |
| 3 | Use Cases | ✅ Complete | All 8 use cases created with tests |
| 4 | Adapter Implementation | ✅ Complete | All 8 use cases wired into UseCases container |
| 5 | Player Adapter Deduplication | ⏳ Not Started | Independent stream |
| 6 | Testing | ✅ Complete | 65 tests pass in engine-app |
| 7 | Arch-Check Enhancements | ⏳ Not Started | Independent stream |
| 8 | Documentation & Cleanup | ⏳ Not Started | Depends on handler refactoring |

**Last Updated:** Dec 28, 2024 (Session 3)

### Phase 4 Details (Current)

**Completed:**
- [x] `WebSocketBroadcastAdapter` - converts GameEvent to ServerMessage
- [x] `UseCases` container struct in AppState
- [x] BroadcastSink already removed (done in previous session)
- [x] `StagingStateAdapter` - implements StagingStatePort wrapping WorldStateManager
- [x] `StagingServiceAdapter` - implements StagingServicePort wrapping StagingService
- [x] `ConnectionManagerAdapter` - implements ConnectionManagerPort (not yet wired)
- [x] `MovementUseCase` wired into UseCases container with adapters
- [x] `StagingApprovalUseCase` wired into UseCases container with adapters
- [x] `InventoryUseCase` wired into UseCases container (uses existing ports)
- [x] `PlayerActionQueueAdapter` - implements PlayerActionQueuePort wrapping PlayerActionQueueService
- [x] `DmNotificationAdapter` - implements DmNotificationPort wrapping WorldConnectionManager
- [x] `PlayerActionUseCase` wired into UseCases container with adapters
- [x] `ObservationRepositoryAdapter` - implements ObservationRepositoryPort wrapping Neo4jObservationRepository
- [x] `WorldMessageAdapter` - implements WorldMessagePort wrapping WorldConnectionManager
- [x] `ObservationUseCase` wired into UseCases container with adapters
- [x] `ChallengeResolutionAdapter` - implements ChallengeResolutionPort (adapters created, not yet wired)
- [x] `ChallengeOutcomeApprovalAdapter` - implements ChallengeOutcomeApprovalPort (adapters created, not yet wired)
- [x] `DmApprovalQueueAdapter` - implements DmApprovalQueuePort (adapters created, not yet wired)
- [x] Updated ChallengeResolutionPort to include WorldId in method signatures
- [x] `SceneServiceAdapter` - implements SceneServicePort wrapping SceneService
- [x] `InteractionServiceAdapter` - implements InteractionServicePort wrapping InteractionService
- [x] `SceneWorldStateAdapter` - implements SceneWorldStatePort wrapping WorldStateManager
- [x] `DirectorialContextAdapter` - implements DirectorialContextRepositoryPort wrapping PortDirectorialContextRepositoryPort
- [x] `DmActionQueuePlaceholder` - placeholder for DmActionQueuePort (type mismatch with DTO DMAction)
- [x] `WorldServiceAdapter` - implements WorldServicePort wrapping WorldService
- [x] `PlayerCharacterServiceAdapter` - implements PlayerCharacterServicePort wrapping PlayerCharacterService
- [x] `ConnectionDirectorialContextAdapter` - implements DirectorialContextPort for ConnectionUseCase
- [x] `ConnectionWorldStateAdapter` - implements ConnectionWorldStatePort wrapping WorldStateManager

**All Use Cases Wired:**

| Use Case | Required Port Adapters | Status |
|----------|----------------------|--------|
| MovementUseCase | StagingServiceAdapter, StagingStateAdapter, SceneBuilder | ✅ Wired |
| StagingApprovalUseCase | StagingServiceAdapter, StagingStateAdapter, SceneBuilder | ✅ Wired |
| InventoryUseCase | (uses existing ports directly) | ✅ Wired |
| PlayerActionUseCase | PlayerActionQueueAdapter, DmNotificationAdapter, MovementUseCase | ✅ Wired |
| ObservationUseCase | ObservationRepositoryAdapter, WorldMessageAdapter | ✅ Wired |
| ChallengeUseCase | ChallengeResolutionPlaceholder, ChallengeOutcomeApprovalAdapter, ChallengeDmApprovalQueueAdapter | ✅ Wired |
| SceneUseCase | SceneServiceAdapter, InteractionServiceAdapter, SceneWorldStateAdapter, DirectorialContextAdapter, DmActionQueuePlaceholder | ✅ Wired |
| ConnectionUseCase | ConnectionManagerAdapter, WorldServiceAdapter, PlayerCharacterServiceAdapter, ConnectionDirectorialContextAdapter, ConnectionWorldStateAdapter | ✅ Wired |

**Next Steps (Phase 4.3):**
- [ ] Refactor handlers to use use cases
- [ ] Target: reduce handler files from ~4,693 lines to ~840 lines

**Note on Placeholder Adapters:**
- `ChallengeResolutionPlaceholder` - Returns errors; handlers should call ChallengeResolutionService directly until service refactoring is complete
- `DmActionQueuePlaceholder` - Returns errors; scene approval uses a different approval flow

**Files Created/Modified:**
- `crates/engine-adapters/src/infrastructure/websocket/broadcast_adapter.rs` (~475 lines)
- `crates/engine-adapters/src/infrastructure/state/use_cases.rs` (~200 lines)
- `crates/engine-adapters/src/infrastructure/state/mod.rs` (updated UseCases::new() call)
- `crates/engine-adapters/src/infrastructure/ports/staging_state_adapter.rs` (~200 lines)
- `crates/engine-adapters/src/infrastructure/ports/staging_service_adapter.rs` (~240 lines)
- `crates/engine-adapters/src/infrastructure/ports/connection_manager_adapter.rs` (~190 lines)
- `crates/engine-adapters/src/infrastructure/ports/player_action_adapters.rs` (~98 lines)
- `crates/engine-adapters/src/infrastructure/ports/observation_adapters.rs` (~98 lines)
- `crates/engine-adapters/src/infrastructure/ports/challenge_adapters.rs` (~300 lines)
- `crates/engine-adapters/src/infrastructure/ports/scene_adapters.rs` (~235 lines) - NEW
- `crates/engine-adapters/src/infrastructure/ports/connection_adapters.rs` (~175 lines) - NEW
- `crates/engine-adapters/src/infrastructure/ports/mod.rs` (~60 lines)
- `crates/engine-app/src/application/use_cases/challenge.rs` (updated port signatures with WorldId)

---

## Executive Summary

This plan addresses hexagonal architecture violations in the WebSocket layer by:

1. Creating **Use Cases** in `engine-app` for complex workflows
2. Creating **BroadcastPort** for domain event notifications
3. Creating **Auth Middleware** for HTTP authentication
4. Creating **ClientMessageBuilder** to deduplicate player adapters
5. Adding **arch-check rules** to prevent future violations

**Estimated effort:** 38-48 hours across 8 phases
**Net code change:** ~1,100 lines reduced (4,200 removed, 3,100 added)
**Test coverage:** +50 new unit tests

---

## Table of Contents

1. [Current State Analysis](#current-state-analysis)
2. [Target Architecture](#target-architecture)
3. [Parallelization Strategy](#parallelization-strategy)
4. [Phase 1: Infrastructure Helpers](#phase-1-infrastructure-helpers)
5. [Phase 2: Ports & Domain Events](#phase-2-ports--domain-events)
6. [Phase 3: Use Cases](#phase-3-use-cases)
7. [Phase 4: Adapter Implementation](#phase-4-adapter-implementation)
8. [Phase 5: Player Adapter Deduplication](#phase-5-player-adapter-deduplication)
9. [Phase 6: Testing](#phase-6-testing)
10. [Phase 7: Arch-Check Enhancements](#phase-7-arch-check-enhancements)
11. [Phase 8: Documentation & Cleanup](#phase-8-documentation--cleanup)
12. [Summary](#summary)
13. [Verification Checklist](#verification-checklist)
14. [Implementation Order](#implementation-order)
15. [Key Design Decisions](#key-design-decisions)

---

## Current State Analysis

### Handler Complexity Summary

| File | Lines | Classification | Priority | Notes |
|------|-------|----------------|----------|-------|
| `movement.rs` | 964 | HEAVY | **HIGH** | Staging workflow, navigation building |
| `challenge.rs` | 817 | HEAVY | **HIGH** | Already delegates to services but has 11 handler functions with boilerplate |
| `staging.rs` | 629 | HEAVY | **HIGH** | Scene construction, NPC enrichment |
| `inventory.rs` | 518 | MIXED | **MEDIUM** | Transaction/rollback logic |
| `misc.rs` | 465 | MIXED | **MEDIUM** | Observation creation logic |
| `player_action.rs` | 448 | MIXED | **MEDIUM** | Travel action workflow |
| `scene.rs` | 373 | MIXED | **LOW** | Scene building logic |
| `connection.rs` | 314 | MIXED | **LOW** | Join world workflow |
| `narrative.rs` | 84 | THIN | **NONE** | Already thin |
| `request.rs` | 81 | THIN | **NONE** | Delegates to AppRequestHandler |

**Total: ~4,693 lines** in handler layer that should be ~840 lines.

**Note on challenge.rs:** While this handler already delegates most logic to `ChallengeResolutionService`, it still has 11 handler functions with repeated context extraction, authorization checks, and error handling boilerplate. Full extraction to `ChallengeUseCase` ensures consistent patterns and reduces the handler to a thin routing layer.

### Violations Identified

| Violation | Count | Impact |
|-----------|-------|--------|
| Connection context extraction boilerplate | 18+ | ~300 duplicate lines |
| DM authorization checks | 11+ | ~110 duplicate lines |
| UUID parsing boilerplate | 25+ | ~200 duplicate lines |
| Complex DTO building in handlers | 15+ | Business logic leaking |
| Direct repository calls in handlers | 30+ | Layer boundary violation |

### Player Adapter Duplication

| File | Lines | Duplicated Logic |
|------|-------|------------------|
| `wasm/adapter.rs` | 354 | 28 trait methods, 22 with duplicated message construction |
| `desktop/adapter.rs` | 496 | Same 28 methods, wrapped in `tokio::spawn` |
| **Extractable** | ~500 | `ClientMessage` construction logic |

The 28 `GameConnectionPort` trait methods (in `player-ports/src/outbound/game_connection_port.rs`) break down as: connection (5), movement (2), staging (3), challenge (5), inventory (4), scene (3), actions (1), disposition (3), misc (2). Of these, 22 have identical message construction logic that can be extracted to `ClientMessageBuilder`.

**Note:** This count refers specifically to `GameConnectionPort` (the client-side WebSocket interface), not all ports in the system. The engine-side has 40+ port traits with 300+ methods total, but those are not relevant to player adapter deduplication.

---

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ADAPTER LAYER (Infrastructure)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  HTTP Middleware:                                                            │
│    - Auth middleware (extracts user_id from header)                         │
│    - Future: JWT validation                                                  │
│                                                                              │
│  WebSocket Handlers:                                                         │
│    - Parse/validate incoming messages                                        │
│    - Extract HandlerContext                                                  │
│    - Call use cases                                                          │
│    - Convert domain results to ServerMessage                                 │
│    - Max ~100 lines per handler                                              │
│                                                                              │
│  WebSocketBroadcastAdapter:                                                  │
│    - Implements BroadcastPort                                                │
│    - Uses WorldConnectionManager internally                                  │
│    - Routes GameEvents to appropriate recipients                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     APPLICATION LAYER (Use Cases)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Use Cases:                                                                  │
│    - Orchestrate domain services                                             │
│    - Return domain result types (NOT ServerMessage)                          │
│    - Use BroadcastPort for side-effect notifications                         │
│    - Transaction/workflow boundaries                                         │
│                                                                              │
│  Examples:                                                                   │
│    - MovementUseCase::move_to_region() → MovementResult                      │
│    - StagingApprovalUseCase::approve() → Result<(), StagingError>           │
│    - InventoryUseCase::equip_item() → EquipResult                           │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DOMAIN SERVICES                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  Existing Services (unchanged):                                              │
│    - StagingService                                                          │
│    - ChallengeResolutionService                                              │
│    - SceneResolutionService                                                  │
│    - NarrativeEventApprovalService                                           │
│                                                                              │
│  New Utilities:                                                              │
│    - SceneBuilder (builds SceneChangedEvent from region data)               │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PORTS LAYER                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  Outbound Ports:                                                             │
│    - BroadcastPort: broadcast(world_id, GameEvent)                          │
│    - WorldConnectionPort (existing): raw message sending                     │
│    - Repository ports (existing): data access                                │
│                                                                              │
│  Domain Events:                                                              │
│    - GameEvent enum with all broadcastable events                           │
│    - Transport-agnostic, adapter converts to ServerMessage                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Parallelization Strategy

Three independent work streams can execute in parallel:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PARALLEL WORK STREAMS                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  STREAM A: Engine Core (Phases 1-4, 6)         ← Primary, blocking          │
│  ├── Phase 1: HandlerContext helper                                         │
│  ├── Phase 2: BroadcastPort + GameEvent                                     │
│  ├── Phase 3: Use Cases (sequential within)                                 │
│  ├── Phase 4: WebSocketBroadcastAdapter + handler refactor                  │
│  └── Phase 6A: Use case tests                                               │
│                                                                              │
│  STREAM B: Player Adapters (Phase 5)           ← Independent                │
│  ├── ClientMessageBuilder                                                    │
│  ├── WASM adapter simplification                                             │
│  └── Desktop adapter simplification                                          │
│                                                                              │
│  STREAM C: Tooling (Phases 7, 8)               ← Independent                │
│  ├── Phase 7: xtask arch-check enhancements                                 │
│  └── Phase 8: Documentation updates                                          │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  DEPENDENCIES:                                                               │
│  - Stream A must complete Phase 2 before Phase 3                            │
│  - Stream A Phase 3 use cases are sequential (movement → staging → others) │
│  - Stream B can start immediately (no dependencies)                          │
│  - Stream C can start immediately (no dependencies)                          │
│  - Final verification requires all streams complete                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Infrastructure Helpers

**Duration:** 2-3 hours
**Dependencies:** None
**Stream:** A

### 1.1 Create HandlerContext

**File:** `crates/engine-adapters/src/infrastructure/websocket/context.rs`

```rust
//! WebSocket handler context extraction and authorization
//!
//! Provides unified context extraction and authorization checks,
//! eliminating boilerplate across handlers.

use uuid::Uuid;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::state::AppState;

/// Extracted context for WebSocket handlers
#[derive(Debug, Clone)]
pub struct HandlerContext {
    pub connection_id: String,
    pub world_id: WorldId,
    pub world_id_uuid: Uuid,
    pub user_id: String,
    pub is_dm: bool,
    pub pc_id: Option<PlayerCharacterId>,
}

/// Context for DM-only operations
#[derive(Debug, Clone)]
pub struct DmContext {
    pub connection_id: String,
    pub world_id: WorldId,
    pub world_id_uuid: Uuid,
    pub user_id: String,
}

/// Context for player-only operations
#[derive(Debug, Clone)]
pub struct PlayerContext {
    pub connection_id: String,
    pub world_id: WorldId,
    pub world_id_uuid: Uuid,
    pub user_id: String,
    pub pc_id: PlayerCharacterId,
}

impl HandlerContext {
    /// Extract context from connection state
    ///
    /// Returns error ServerMessage if:
    /// - Client is not connected
    /// - Client is not in a world
    pub async fn extract(state: &AppState, client_id: Uuid) -> Result<Self, ServerMessage> {
        let client_id_str = client_id.to_string();
        let connection = state
            .world_connection_manager
            .get_connection_by_client_id(&client_id_str)
            .await
            .ok_or_else(|| error_response("NOT_CONNECTED", "Client is not connected"))?;

        let world_id_uuid = connection
            .world_id
            .ok_or_else(|| error_response("NO_WORLD", "Not connected to a world"))?;

        Ok(Self {
            connection_id: client_id_str,
            world_id: WorldId::from_uuid(world_id_uuid),
            world_id_uuid,
            user_id: connection.user_id.clone(),
            is_dm: connection.is_dm(),
            pc_id: connection.pc_id.map(PlayerCharacterId::from_uuid),
        })
    }

    /// Require DM authorization, returning DmContext
    pub fn require_dm(self) -> Result<DmContext, ServerMessage> {
        if self.is_dm {
            Ok(DmContext {
                connection_id: self.connection_id,
                world_id: self.world_id,
                world_id_uuid: self.world_id_uuid,
                user_id: self.user_id,
            })
        } else {
            Err(error_response(
                "NOT_AUTHORIZED",
                "Only the DM can perform this action",
            ))
        }
    }

    /// Require player authorization (not DM, has PC), returning PlayerContext
    pub fn require_player(self) -> Result<PlayerContext, ServerMessage> {
        match (self.is_dm, self.pc_id) {
            (false, Some(pc_id)) => Ok(PlayerContext {
                connection_id: self.connection_id,
                world_id: self.world_id,
                world_id_uuid: self.world_id_uuid,
                user_id: self.user_id,
                pc_id,
            }),
            _ => Err(error_response(
                "NOT_AUTHORIZED",
                "Only players can perform this action",
            )),
        }
    }
}

// Conversion to use case context types
// NOTE: UseCaseContext is defined in engine-ports/src/inbound/use_case_context.rs
// to avoid circular dependencies between engine-adapters and engine-app.
use wrldbldr_engine_ports::inbound::UseCaseContext;

impl From<HandlerContext> for UseCaseContext {
    fn from(ctx: HandlerContext) -> Self {
        Self {
            world_id: ctx.world_id,
            user_id: ctx.user_id,
            is_dm: ctx.is_dm,
            pc_id: ctx.pc_id,
        }
    }
}

impl From<DmContext> for UseCaseContext {
    fn from(ctx: DmContext) -> Self {
        UseCaseContext::dm(ctx.world_id, ctx.user_id)
    }
}

impl From<PlayerContext> for UseCaseContext {
    fn from(ctx: PlayerContext) -> Self {
        UseCaseContext::player(ctx.world_id, ctx.user_id, ctx.pc_id)
    }
}

// =============================================================================
// Error Response Helpers
// =============================================================================

pub fn error_response(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

pub fn not_found_error(entity: &str, id: &str) -> ServerMessage {
    error_response(
        &format!("{}_NOT_FOUND", entity.to_uppercase()),
        &format!("{} not found: {}", entity, id),
    )
}

pub fn invalid_id_error(entity: &str, id: &str) -> ServerMessage {
    error_response(
        &format!("INVALID_{}_ID", entity.to_uppercase()),
        &format!("Invalid {} ID: {}", entity, id),
    )
}

// =============================================================================
// ID Parsing Helpers
// =============================================================================

pub fn parse_uuid(id: &str, entity: &str) -> Result<Uuid, ServerMessage> {
    Uuid::parse_str(id).map_err(|_| invalid_id_error(entity, id))
}

pub fn parse_world_id(id: &str) -> Result<WorldId, ServerMessage> {
    parse_uuid(id, "world").map(WorldId::from_uuid)
}

pub fn parse_player_character_id(id: &str) -> Result<PlayerCharacterId, ServerMessage> {
    parse_uuid(id, "PC").map(PlayerCharacterId::from_uuid)
}

pub fn parse_region_id(id: &str) -> Result<RegionId, ServerMessage> {
    parse_uuid(id, "region").map(RegionId::from_uuid)
}

pub fn parse_location_id(id: &str) -> Result<LocationId, ServerMessage> {
    parse_uuid(id, "location").map(LocationId::from_uuid)
}

pub fn parse_character_id(id: &str) -> Result<CharacterId, ServerMessage> {
    parse_uuid(id, "character").map(CharacterId::from_uuid)
}
```

### 1.2 Create HTTP Auth Middleware

**File:** `crates/engine-adapters/src/infrastructure/http/middleware/auth.rs`

```rust
//! Authentication middleware for HTTP routes
//!
//! Currently extracts user_id from X-User-Id header.
//! Future: JWT validation will be added here.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// User ID extracted from request headers
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
}

/// Extension key for authenticated user
pub const AUTH_USER_KEY: &str = "authenticated_user";

/// Middleware that extracts user_id from X-User-Id header
///
/// # Future Enhancement
/// This will be replaced with proper JWT validation when
/// production authentication is implemented.
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let user_id = request
        .headers()
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(user_id) = user_id {
        request.extensions_mut().insert(AuthenticatedUser { user_id });
    }
    // Note: We don't reject requests without user_id yet
    // Some endpoints may be public

    Ok(next.run(request).await)
}

/// Middleware that requires authentication
pub async fn require_auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.extensions().get::<AuthenticatedUser>().is_none() {
        let user_id = request
            .headers()
            .get("X-User-Id")
            .and_then(|v| v.to_str().ok());

        if user_id.is_none() {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(request).await)
}

/// Extractor for authenticated user in handlers
#[derive(Debug, Clone)]
pub struct Auth(pub AuthenticatedUser);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .map(Auth)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}
```

### 1.3 Create Middleware Module

**File:** `crates/engine-adapters/src/infrastructure/http/middleware/mod.rs`

```rust
mod auth;

pub use auth::*;
```

### 1.4 Update Module Exports

**File:** `crates/engine-adapters/src/infrastructure/websocket/mod.rs` (modify)

Add:
```rust
pub mod context;
pub use context::*;
```

**File:** `crates/engine-adapters/src/infrastructure/http/mod.rs` (modify)

Add:
```rust
pub mod middleware;
```

---

## Phase 2: Ports & Domain Events

**Duration:** 3-4 hours
**Dependencies:** None (can start parallel to Phase 1)
**Stream:** A

### 2.1 Create GameEvent Enum

**File:** `crates/engine-ports/src/outbound/game_events.rs`

```rust
//! Domain events for game notifications
//!
//! These are transport-agnostic event types used by use cases to request
//! notifications. The adapter layer converts these to ServerMessage.

use wrldbldr_domain::{
    CharacterId, GameTime, LocationId, PlayerCharacterId, RegionId,
};

/// All broadcastable game events
///
/// Use cases emit these events via BroadcastPort.
/// The adapter layer routes and converts them to protocol messages.
#[derive(Debug, Clone)]
pub enum GameEvent {
    // === Staging Events ===
    /// DM approval required for NPC staging
    StagingRequired(StagingRequiredEvent),
    /// Staging is ready, notify waiting players
    StagingReady(StagingReadyEvent),
    /// Player is waiting for staging (sent to specific user)
    StagingPending {
        user_id: String,
        event: StagingPendingEvent,
    },

    // === Scene Events ===
    /// Scene changed for a player (sent to specific user)
    SceneChanged {
        user_id: String,
        event: SceneChangedEvent,
    },

    // === Movement Events ===
    /// Movement was blocked (sent to specific user)
    MovementBlocked {
        user_id: String,
        pc_id: PlayerCharacterId,
        reason: String,
    },

    // === Party Events ===
    /// Party has split across locations (DM notification)
    SplitParty(SplitPartyEvent),

    // === Time Events ===
    /// Game time has advanced (broadcast to all players)
    GameTimeUpdated(GameTime),

    // === Player Events ===
    /// Player joined the world (DM notification)
    PlayerJoined {
        user_id: String,
        pc_name: Option<String>,
    },
    /// Player left the world (DM notification)
    PlayerLeft { user_id: String },
}

// =============================================================================
// Event Struct Definitions
// =============================================================================

/// DM approval required for NPC staging
#[derive(Debug, Clone)]
pub struct StagingRequiredEvent {
    pub request_id: String,
    pub region_id: RegionId,
    pub region_name: String,
    pub location_id: LocationId,
    pub location_name: String,
    pub game_time: GameTime,
    pub rule_based_npcs: Vec<StagedNpcData>,
    pub llm_based_npcs: Vec<StagedNpcData>,
    pub waiting_pcs: Vec<WaitingPcData>,
    pub previous_staging: Option<PreviousStagingData>,
    pub default_ttl_hours: i32,
}

/// Staging is ready for a region
#[derive(Debug, Clone)]
pub struct StagingReadyEvent {
    pub region_id: RegionId,
    pub npcs_present: Vec<NpcPresenceData>,
    pub waiting_pcs: Vec<WaitingPcData>,
}

/// Player is waiting for staging to complete
#[derive(Debug, Clone)]
pub struct StagingPendingEvent {
    pub region_id: RegionId,
    pub region_name: String,
}

/// Scene changed for a player
#[derive(Debug, Clone)]
pub struct SceneChangedEvent {
    pub pc_id: PlayerCharacterId,
    pub region: RegionInfo,
    pub npcs_present: Vec<NpcPresenceData>,
    pub navigation: NavigationInfo,
    pub region_items: Vec<RegionItemData>,
}

/// Party has split across locations
#[derive(Debug, Clone)]
pub struct SplitPartyEvent {
    pub location_groups: Vec<LocationGroup>,
}

// =============================================================================
// Supporting Data Types
// =============================================================================

#[derive(Debug, Clone)]
pub struct StagedNpcData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct WaitingPcData {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub user_id: String,
}

#[derive(Debug, Clone)]
pub struct PreviousStagingData {
    pub staging_id: String,
    pub approved_at: String,
    pub npcs: Vec<StagedNpcData>,
}

#[derive(Debug, Clone)]
pub struct NpcPresenceData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub id: RegionId,
    pub name: String,
    pub location_id: LocationId,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_asset: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NavigationInfo {
    pub connected_regions: Vec<NavigationTarget>,
    pub exits: Vec<NavigationExit>,
}

#[derive(Debug, Clone)]
pub struct NavigationTarget {
    pub region_id: RegionId,
    pub name: String,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NavigationExit {
    pub location_id: LocationId,
    pub location_name: String,
    pub arrival_region_id: RegionId,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegionItemData {
    pub item_id: String,
    pub name: String,
    pub description: Option<String>,
    pub quantity: u32,
}

#[derive(Debug, Clone)]
pub struct LocationGroup {
    pub location_id: LocationId,
    pub location_name: String,
    pub pcs: Vec<PcLocationData>,
}

#[derive(Debug, Clone)]
pub struct PcLocationData {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub region_id: Option<RegionId>,
    pub region_name: Option<String>,
}
```

### 2.2 Create BroadcastPort

**File:** `crates/engine-ports/src/outbound/broadcast_port.rs`

```rust
//! Broadcast Port - Outbound port for game event notifications
//!
//! This port abstracts the notification of game events to connected clients,
//! allowing use cases to trigger notifications without depending on WebSocket
//! infrastructure.

use async_trait::async_trait;
use wrldbldr_domain::WorldId;

use super::game_events::GameEvent;

/// Port for broadcasting game events to connected clients
///
/// Implementations:
/// - Convert GameEvent to appropriate ServerMessage(s)
/// - Route to correct recipients based on event type
/// - Use WorldConnectionManager or similar for actual delivery
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait BroadcastPort: Send + Sync {
    /// Broadcast a game event
    ///
    /// The implementation routes the event to appropriate recipients:
    /// - DM-targeted events go to DMs
    /// - Player-targeted events go to specific players
    /// - World-wide events go to all participants
    async fn broadcast(&self, world_id: WorldId, event: GameEvent);
}
```

**Required Cargo.toml changes for mockall support:**

```toml
# crates/engine-ports/Cargo.toml

[features]
default = []
testing = ["mockall"]

[dependencies]
mockall = { version = "0.11", optional = true }

[dev-dependencies]
mockall = "0.11"
```

### 2.2.1 Deprecate and Remove BroadcastSink

**Background:** An existing `BroadcastSink` trait exists in `engine-ports/src/inbound/request_handler.rs` (lines 105-161). It takes `ServerMessage` directly, which violates hexagonal architecture by coupling the application layer to protocol types.

**Current BroadcastSink methods:**
- `broadcast_entity_change(world_id, EntityChangedData)`
- `send_to_connection(connection_id, ServerMessage)`
- `send_to_user(user_id, world_id, ServerMessage)`
- `broadcast_to_dms(world_id, ServerMessage)`
- `broadcast_to_players(world_id, ServerMessage)`

**Why replace instead of extend:**
1. `BroadcastSink` takes `ServerMessage` (protocol type) - violates hexagonal architecture
2. `BroadcastPort` takes `GameEvent` (domain type) - proper abstraction
3. `BroadcastSink` is an inbound port but semantically it's outbound (app → infrastructure)
4. Clean break is better than carrying legacy API

**Migration steps:**

1. **Create `BroadcastPort`** in `engine-ports/src/outbound/` (as shown above)

2. **Update `AppRequestHandler`** to use `BroadcastPort` instead of `BroadcastSink`:
   
   **File:** `crates/engine-app/src/application/handlers/request_handler.rs`
   ```rust
   // BEFORE
   broadcast_sink: Option<Arc<dyn BroadcastSink>>,
   
   // AFTER
   broadcast_port: Option<Arc<dyn BroadcastPort>>,
   ```

3. **Remove `BroadcastSink`** from `engine-ports/src/inbound/request_handler.rs`

4. **Update `engine-ports/src/inbound/mod.rs`** to remove the re-export

**Note:** `WorldConnectionPort` in `engine-ports/src/outbound/world_connection_port.rs` remains unchanged. It provides low-level message routing used by `WebSocketBroadcastAdapter` internally. The difference:
- `WorldConnectionPort`: Low-level, takes `ServerMessage`, used by adapters
- `BroadcastPort`: High-level, takes `GameEvent`, used by use cases

### 2.3 Create UseCaseContext (Shared Contract)

**File:** `crates/engine-ports/src/inbound/use_case_context.rs`

```rust
//! Use case execution context
//!
//! This context is passed from handlers to use cases, containing identity
//! and authorization information. Defined in ports layer so both adapters
//! and application layer can use it without circular dependencies.

use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// Context for use case execution
///
/// Passed from handlers to use cases, contains identity and authorization info.
/// Defined in engine-ports to avoid circular dependencies between adapters and app.
#[derive(Debug, Clone)]
pub struct UseCaseContext {
    pub world_id: WorldId,
    pub user_id: String,
    pub is_dm: bool,
    pub pc_id: Option<PlayerCharacterId>,
}

impl UseCaseContext {
    /// Create a DM context
    pub fn dm(world_id: WorldId, user_id: String) -> Self {
        Self {
            world_id,
            user_id,
            is_dm: true,
            pc_id: None,
        }
    }

    /// Create a player context
    pub fn player(world_id: WorldId, user_id: String, pc_id: PlayerCharacterId) -> Self {
        Self {
            world_id,
            user_id,
            is_dm: false,
            pc_id: Some(pc_id),
        }
    }

    /// Create a spectator context (not DM, no PC)
    pub fn spectator(world_id: WorldId, user_id: String) -> Self {
        Self {
            world_id,
            user_id,
            is_dm: false,
            pc_id: None,
        }
    }
}
```

### 2.4 Update Port Exports

**File:** `crates/engine-ports/src/outbound/mod.rs` (modify)

Add:
```rust
mod broadcast_port;
mod game_events;

pub use broadcast_port::*;
pub use game_events::*;
```

**File:** `crates/engine-ports/src/inbound/mod.rs` (modify)

Add:
```rust
mod use_case_context;

pub use use_case_context::*;
```

---

## Phase 3: Use Cases

**Duration:** 12-15 hours
**Dependencies:** Phase 2 (BroadcastPort, GameEvent)
**Stream:** A

### 3.1 Create Use Case Module Structure

**File:** `crates/engine-app/src/application/use_cases/mod.rs`

```rust
//! Use Cases - Application layer orchestration
//!
//! Use cases coordinate domain services to fulfill specific user intents.
//! They are transport-agnostic and return domain results, not protocol messages.
//!
//! # Scope Clarification
//!
//! Use cases handle **WebSocket message handlers** (gameplay events):
//! - movement.rs, staging.rs, challenge.rs, inventory.rs, etc.
//!
//! They do NOT replace **AppRequestHandler** which handles:
//! - Request/Response CRUD operations via `ClientMessage::Request { request_id, payload }`
//! - Located in `engine-app/src/application/handlers/request_handler.rs`
//!
//! The distinction:
//! - **Use cases**: Complex workflows with side-effects (broadcasts, state changes)
//! - **AppRequestHandler**: Simple CRUD operations that return a single response
//!
//! # Architecture Rules
//!
//! 1. Use cases must NOT import `wrldbldr_protocol::ServerMessage`
//! 2. Use cases return domain result types (enums, structs)
//! 3. Use cases use `BroadcastPort` for side-effect notifications
//! 4. Use cases orchestrate domain services, they don't replace them
//! 5. Use cases are the transaction/workflow boundary
//! 6. Use cases import `UseCaseContext` from `engine-ports::inbound`
//!
//! # Handler Pattern
//!
//! Handlers should call use cases like this:
//!
//! ```rust,ignore
//! let ctx = HandlerContext::extract(state, client_id).await?;
//! match state.use_cases.movement.move_to_region(ctx.into(), input).await {
//!     Ok(result) => Some(result.into_server_message()),
//!     Err(e) => Some(e.into_server_error()),
//! }
//! ```

mod errors;
mod movement;
mod staging;
mod player_action;
mod inventory;
mod observation;
mod challenge;
mod scene;
mod connection;
mod builders;

// Re-export UseCaseContext from ports (defined there to avoid circular deps)
pub use wrldbldr_engine_ports::inbound::UseCaseContext;

pub use errors::*;
pub use movement::*;
pub use staging::*;
pub use player_action::*;
pub use inventory::*;
pub use observation::*;
pub use challenge::*;
pub use scene::*;
pub use connection::*;
pub use builders::*;
```

### 3.2 Re-export UseCaseContext

**Note:** `UseCaseContext` is defined in `engine-ports/src/inbound/use_case_context.rs` (see Phase 2.3) to avoid circular dependencies. Use cases should import it from ports:

```rust
// In any use case file
use wrldbldr_engine_ports::inbound::UseCaseContext;
```

The use_cases module re-exports it for convenience:

```rust
// In use_cases/mod.rs
pub use wrldbldr_engine_ports::inbound::UseCaseContext;
```

### 3.3 Create Error Types

**File:** `crates/engine-app/src/application/use_cases/errors.rs`

```rust
//! Use case error types with protocol conversion support

use std::fmt::Display;
use thiserror::Error;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId, WorldId};

// =============================================================================
// ErrorCode Trait
// =============================================================================

/// Trait for converting errors to protocol error codes
///
/// Implemented by all use case error types to provide standardized
/// conversion to ServerMessage::Error format.
pub trait ErrorCode: Display {
    /// Get the error code string (e.g., "PC_NOT_FOUND")
    fn code(&self) -> &'static str;

    /// Convert to a ServerMessage::Error
    ///
    /// Note: This returns the protocol type directly. While use cases
    /// should not import ServerMessage, the errors module is allowed
    /// to provide this conversion for handler convenience.
    fn into_server_error(&self) -> wrldbldr_protocol::ServerMessage {
        wrldbldr_protocol::ServerMessage::Error {
            code: self.code().to_string(),
            message: self.to_string(),
        }
    }
}

// =============================================================================
// Movement Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum MovementError {
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),

    #[error("Connection is locked: {0}")]
    ConnectionLocked(String),

    #[error("No arrival region available for location")]
    NoArrivalRegion,

    #[error("Region does not belong to target location")]
    RegionLocationMismatch,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Staging error: {0}")]
    Staging(String),
}

impl ErrorCode for MovementError {
    fn code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::LocationNotFound(_) => "LOCATION_NOT_FOUND",
            Self::ConnectionLocked(_) => "CONNECTION_LOCKED",
            Self::NoArrivalRegion => "NO_ARRIVAL_REGION",
            Self::RegionLocationMismatch => "REGION_MISMATCH",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Staging(_) => "STAGING_ERROR",
        }
    }
}

// =============================================================================
// Staging Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum StagingError {
    #[error("Pending staging not found: {0}")]
    PendingNotFound(String),

    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    #[error("Character not found: {0}")]
    CharacterNotFound(CharacterId),

    #[error("Staging approval failed: {0}")]
    ApprovalFailed(String),

    #[error("Regeneration failed: {0}")]
    RegenerationFailed(String),

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for StagingError {
    fn code(&self) -> &'static str {
        match self {
            Self::PendingNotFound(_) => "STAGING_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::CharacterNotFound(_) => "CHARACTER_NOT_FOUND",
            Self::ApprovalFailed(_) => "STAGING_APPROVAL_FAILED",
            Self::RegenerationFailed(_) => "REGENERATION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Inventory Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error("Item not in inventory")]
    NotInInventory,

    #[error("Item already owned by another character")]
    AlreadyOwned,

    #[error("Insufficient quantity")]
    InsufficientQuantity,

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for InventoryError {
    fn code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::ItemNotFound(_) => "ITEM_NOT_FOUND",
            Self::NotInInventory => "NOT_IN_INVENTORY",
            Self::AlreadyOwned => "ITEM_ALREADY_OWNED",
            Self::InsufficientQuantity => "INSUFFICIENT_QUANTITY",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Challenge Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ChallengeError {
    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),

    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Target character not found: {0}")]
    TargetNotFound(CharacterId),

    #[error("Roll already submitted for this challenge")]
    RollAlreadySubmitted,

    #[error("Invalid roll value: {0}")]
    InvalidRoll(String),

    #[error("Challenge outcome pending approval")]
    OutcomePending,

    #[error("Not authorized to approve this outcome")]
    NotAuthorized,

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ChallengeError {
    fn code(&self) -> &'static str {
        match self {
            Self::ChallengeNotFound(_) => "CHALLENGE_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::TargetNotFound(_) => "TARGET_NOT_FOUND",
            Self::RollAlreadySubmitted => "ROLL_ALREADY_SUBMITTED",
            Self::InvalidRoll(_) => "INVALID_ROLL",
            Self::OutcomePending => "OUTCOME_PENDING",
            Self::NotAuthorized => "NOT_AUTHORIZED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Observation Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("NPC not found: {0}")]
    NpcNotFound(CharacterId),

    #[error("NPC not in current region")]
    NpcNotInRegion,

    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),

    #[error("Event generation failed: {0}")]
    EventGenerationFailed(String),

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ObservationError {
    fn code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::NpcNotFound(_) => "NPC_NOT_FOUND",
            Self::NpcNotInRegion => "NPC_NOT_IN_REGION",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::LocationNotFound(_) => "LOCATION_NOT_FOUND",
            Self::EventGenerationFailed(_) => "EVENT_GENERATION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Scene Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum SceneError {
    #[error("Scene not found: {0}")]
    SceneNotFound(String),

    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    #[error("Scene change request pending approval")]
    RequestPending,

    #[error("Invalid directorial context: {0}")]
    InvalidContext(String),

    #[error("Not authorized to approve scene changes")]
    NotAuthorized,

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for SceneError {
    fn code(&self) -> &'static str {
        match self {
            Self::SceneNotFound(_) => "SCENE_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::RequestPending => "REQUEST_PENDING",
            Self::InvalidContext(_) => "INVALID_CONTEXT",
            Self::NotAuthorized => "NOT_AUTHORIZED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Connection Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),

    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Already connected to a world")]
    AlreadyConnected,

    #[error("Not connected to any world")]
    NotConnected,

    #[error("Character already claimed by another player")]
    CharacterClaimed,

    #[error("Invalid spectate target: {0}")]
    InvalidSpectateTarget(String),

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ConnectionError {
    fn code(&self) -> &'static str {
        match self {
            Self::WorldNotFound(_) => "WORLD_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::AlreadyConnected => "ALREADY_CONNECTED",
            Self::NotConnected => "NOT_CONNECTED",
            Self::CharacterClaimed => "CHARACTER_CLAIMED",
            Self::InvalidSpectateTarget(_) => "INVALID_SPECTATE_TARGET",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Action Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("No player character selected")]
    NoPcSelected,

    #[error("Missing target for action")]
    MissingTarget,

    #[error("Invalid action type: {0}")]
    InvalidActionType(String),

    #[error("Movement failed: {0}")]
    MovementFailed(String),

    #[error("Movement blocked: {0}")]
    MovementBlocked(String),

    #[error("Action queue failed: {0}")]
    QueueFailed(String),

    #[error("Action requires DM approval")]
    RequiresApproval,

    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ActionError {
    fn code(&self) -> &'static str {
        match self {
            Self::NoPcSelected => "NO_PC_SELECTED",
            Self::MissingTarget => "MISSING_TARGET",
            Self::InvalidActionType(_) => "INVALID_ACTION_TYPE",
            Self::MovementFailed(_) => "MOVEMENT_FAILED",
            Self::MovementBlocked(_) => "MOVEMENT_BLOCKED",
            Self::QueueFailed(_) => "QUEUE_FAILED",
            Self::RequiresApproval => "REQUIRES_APPROVAL",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}
```

All error types follow the same pattern:
- Derive `Debug` and `thiserror::Error`
- Include domain-specific variants with meaningful context
- Implement `ErrorCode` trait for protocol conversion
- Use consistent error codes (SCREAMING_SNAKE_CASE)

### 3.4 Create SceneBuilder

**File:** `crates/engine-app/src/application/use_cases/builders/scene_builder.rs`

Builds `SceneChangedEvent` from region and staging data. Shared across `MovementUseCase` and `StagingApprovalUseCase`.

### 3.5-3.11 Create Use Cases

Each use case follows the pattern:
1. Takes `UseCaseContext` + inputs
2. Calls domain services
3. Returns domain result types
4. Uses `BroadcastPort` for side-effects
5. Includes tests

**Files to create:**
- `movement.rs` - `MovementUseCase` with `select_character`, `move_to_region`, `exit_to_location`
- `staging.rs` - `StagingApprovalUseCase` with `approve_staging`, `regenerate_suggestions`, `pre_stage_region`
- `inventory.rs` - `InventoryUseCase` with `equip_item`, `unequip_item`, `drop_item`, `pickup_item`
- `challenge.rs` - `ChallengeUseCase` with `submit_roll`, `trigger_challenge`, `approve_outcome`, `create_adhoc`
- `observation.rs` - `ObservationUseCase` with `share_npc_location`, `trigger_approach_event`, `trigger_location_event` (extracts from `misc.rs`)
- `scene.rs` - `SceneUseCase` with `request_scene_change`, `update_directorial_context`, `handle_approval`
- `connection.rs` - `ConnectionUseCase` with `join_world`, `leave_world`, `set_spectate_target`
- `player_action.rs` - `PlayerActionUseCase` with `handle_action`

### 3.13 ObservationUseCase: Extraction from misc.rs

The current `misc.rs` handler (465 lines) contains:
- `handle_check_comfyui_health` (35 lines) - stays in misc.rs (infrastructure concern)
- `handle_share_npc_location` (121 lines) - extract to `ObservationUseCase`
- `handle_trigger_approach_event` (163 lines) - extract to `ObservationUseCase`
- `handle_trigger_location_event` (60 lines) - extract to `ObservationUseCase`

After refactoring, `misc.rs` will only contain the ComfyUI health check (~50 lines).

### 3.14 PlayerActionUseCase: Travel Action Handling

**Important:** The current `player_action.rs` handler (448 lines) has special handling for "travel" actions that bypass the normal queue and trigger scene changes. The `PlayerActionUseCase` must handle this correctly.

**Struct definition with dependency injection:**

```rust
use std::sync::Arc;
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::BroadcastPort;

/// Use case for handling player actions
///
/// Handles both immediate actions (travel) and queued actions (speak, interact).
/// Delegates travel to MovementUseCase to avoid duplicating movement logic.
pub struct PlayerActionUseCase {
    /// Movement use case for travel actions
    movement: Arc<MovementUseCase>,
    /// Queue service for non-immediate actions
    action_queue_service: Arc<PlayerActionQueueService>,
    /// Scene resolution for post-travel scene building
    scene_resolution_service: Arc<SceneResolutionService>,
    /// Broadcast port for side-effect notifications
    broadcast: Arc<dyn BroadcastPort>,
}

impl PlayerActionUseCase {
    pub fn new(
        movement: Arc<MovementUseCase>,
        action_queue_service: Arc<PlayerActionQueueService>,
        scene_resolution_service: Arc<SceneResolutionService>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            movement,
            action_queue_service,
            scene_resolution_service,
            broadcast,
        }
    }

    pub async fn handle_action(
        &self,
        ctx: UseCaseContext,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> Result<ActionResult, ActionError> {
        // Travel actions are special - they don't queue, they execute immediately
        if action_type == "travel" {
            let target_id = target.ok_or(ActionError::MissingTarget)?;
            let pc_id = ctx.pc_id.ok_or(ActionError::NoPcSelected)?;
            
            return self.handle_travel(ctx, pc_id, target_id).await;
        }
        
        // Non-travel actions go through the queue
        self.queue_action(ctx, action_type, target, dialogue).await
    }
    
    async fn handle_travel(
        &self,
        ctx: UseCaseContext,
        pc_id: PlayerCharacterId,
        target: &str,
    ) -> Result<ActionResult, ActionError> {
        // Determine if target is a region (same location) or location (exit)
        // by checking if it parses as a known region ID in current location
        
        // Delegate to movement use case
        let movement_result = self.movement
            .move_to_region(ctx.clone(), pc_id, parse_region_id(target)?)
            .await
            .map_err(|e| ActionError::MovementFailed(e.to_string()))?;
        
        match movement_result {
            MovementResult::SceneChanged(event) => Ok(ActionResult::TravelCompleted(event)),
            MovementResult::StagingPending { .. } => Ok(ActionResult::TravelPending),
            MovementResult::Blocked { reason } => Err(ActionError::MovementBlocked(reason)),
        }
    }
    
    async fn queue_action(
        &self,
        ctx: UseCaseContext,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> Result<ActionResult, ActionError> {
        let action_id = self.action_queue_service
            .enqueue(ctx.world_id, ctx.pc_id.unwrap(), action_type, target, dialogue)
            .await
            .map_err(|e| ActionError::QueueFailed(e.to_string()))?;
        
        Ok(ActionResult::Queued { action_id })
    }
}

pub enum ActionResult {
    /// Travel completed, scene changed (not queued)
    TravelCompleted(SceneChangedEvent),
    /// Travel pending staging approval
    TravelPending,
    /// Action queued for LLM processing
    Queued { action_id: String },
    /// Action requires DM approval
    PendingApproval { request_id: String },
}
```

**Key point:** `PlayerActionUseCase` takes `Arc<MovementUseCase>` in its constructor, enabling delegation of travel actions without duplicating movement logic.

### 3.12 Update Application Module Exports

**File:** `crates/engine-app/src/application/mod.rs` (modify)

Add:
```rust
pub mod use_cases;
```

---

## Phase 4: Adapter Implementation

**Duration:** 5-6 hours
**Dependencies:** Phase 3 (Use Cases)
**Stream:** A

### 4.1 Create WebSocketBroadcastAdapter

**File:** `crates/engine-adapters/src/infrastructure/websocket/broadcast_adapter.rs`

Implements `BroadcastPort`:
- Converts `GameEvent` to `ServerMessage`
- Routes to correct recipients via `WorldConnectionManager`
- Handles all event types (staging, scene, movement, party, time, player)

### 4.2 Wire Use Cases into AppState

**File:** `crates/engine-adapters/src/infrastructure/state/mod.rs` (modify)

Add `UseCases` struct following the existing grouped service pattern:

```rust
use std::sync::Arc;
use wrldbldr_engine_app::application::use_cases::*;

/// Container for all use cases
///
/// Follows the same pattern as CoreServices, GameServices, QueueServices.
/// Use cases are constructed with their dependencies during AppState initialization.
pub struct UseCases {
    pub movement: Arc<MovementUseCase>,
    pub staging: Arc<StagingApprovalUseCase>,
    pub inventory: Arc<InventoryUseCase>,
    pub challenge: Arc<ChallengeUseCase>,
    pub observation: Arc<ObservationUseCase>,
    pub scene: Arc<SceneUseCase>,
    pub connection: Arc<ConnectionUseCase>,
    pub player_action: Arc<PlayerActionUseCase>,
}

impl UseCases {
    pub fn new(
        // Repository ports
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
        location_repo: Arc<dyn LocationRepositoryPort>,
        character_repo: Arc<dyn CharacterRepositoryPort>,
        // Existing services
        staging_service: Arc<StagingService>,
        scene_resolution_service: Arc<SceneResolutionService>,
        challenge_resolution_service: Arc<ChallengeResolutionService>,
        // ... other dependencies
        // Broadcast port
        broadcast: Arc<dyn BroadcastPort>,
        // World state
        world_state: Arc<WorldStateManager>,
    ) -> Self {
        // Build scene builder first (shared)
        let scene_builder = Arc::new(SceneBuilder::new(
            Arc::clone(&region_repo),
            Arc::clone(&location_repo),
        ));

        // Build use cases with their dependencies
        let movement = Arc::new(MovementUseCase::new(
            Arc::clone(&pc_repo),
            Arc::clone(&region_repo),
            Arc::clone(&location_repo),
            Arc::clone(&staging_service),
            Arc::clone(&world_state),
            Arc::clone(&broadcast),
            Arc::clone(&scene_builder),
        ));

        let staging = Arc::new(StagingApprovalUseCase::new(
            Arc::clone(&staging_service),
            Arc::clone(&character_repo),
            Arc::clone(&region_repo),
            Arc::clone(&world_state),
            Arc::clone(&broadcast),
            Arc::clone(&scene_builder),
        ));

        // ... construct other use cases

        // PlayerActionUseCase takes movement as dependency
        let player_action = Arc::new(PlayerActionUseCase::new(
            Arc::clone(&movement),  // Delegation for travel
            action_queue_service,
            Arc::clone(&scene_resolution_service),
            Arc::clone(&broadcast),
        ));

        Self {
            movement,
            staging,
            inventory,
            challenge,
            observation,
            scene,
            connection,
            player_action,
        }
    }
}

// Add to AppState
pub struct AppState {
    // Existing service groups (unchanged)
    pub core: CoreServices,
    pub game: GameServices<OllamaClient>,
    pub queues: QueueServices,
    pub player: PlayerServices,
    
    // New use case container
    pub use_cases: UseCases,
    
    // Other existing fields...
    pub world_connection_manager: Arc<WorldConnectionManager>,
    pub world_state: Arc<WorldStateManager>,
    // ...
}
```

**Key points:**
- `UseCases` follows the same pattern as existing service groups
- Dependencies are injected via constructor
- `MovementUseCase` is shared with `PlayerActionUseCase` via `Arc`
- `SceneBuilder` is shared between movement and staging use cases

### 4.2.1 Migrate AppRequestHandler from BroadcastSink to BroadcastPort

**File:** `crates/engine-app/src/application/handlers/request_handler.rs`

The `AppRequestHandler` currently has an optional `BroadcastSink` dependency. This must be migrated to use `BroadcastPort` instead.

**Changes required:**

```rust
// BEFORE (line ~81)
broadcast_sink: Option<Arc<dyn BroadcastSink>>,

// AFTER
broadcast_port: Option<Arc<dyn BroadcastPort>>,
```

```rust
// BEFORE (line ~148)
pub fn with_broadcast_sink(mut self, sink: Arc<dyn BroadcastSink>) -> Self {
    self.broadcast_sink = Some(sink);
    self
}

// AFTER
pub fn with_broadcast_port(mut self, port: Arc<dyn BroadcastPort>) -> Self {
    self.broadcast_port = Some(port);
    self
}
```

**Note:** Any calls to `broadcast_sink.broadcast_entity_change()` or similar must be converted to emit `GameEvent` variants via `broadcast_port.broadcast()`.

### 4.2.2 Remove BroadcastSink Trait

**File:** `crates/engine-ports/src/inbound/request_handler.rs`

Remove the `BroadcastSink` trait definition (lines 105-161).

**File:** `crates/engine-ports/src/inbound/mod.rs`

Remove the re-export:
```rust
// REMOVE this line
pub use request_handler::{BroadcastSink, RequestContext, RequestHandler};

// REPLACE with
pub use request_handler::{RequestContext, RequestHandler};
```

### 4.3 Refactor Handlers

Each handler is reduced to a thin routing layer (~100 lines or less):

**Handler refactoring targets:**

| File | Before | After | Change |
|------|--------|-------|--------|
| `movement.rs` | 964 | ~100 | -864 |
| `staging.rs` | 629 | ~80 | -549 |
| `player_action.rs` | 448 | ~50 | -398 |
| `inventory.rs` | 518 | ~100 | -418 |
| `misc.rs` | 465 | ~150 | -315 |
| `challenge.rs` | 817 | ~200 | -617 |
| `scene.rs` | 373 | ~80 | -293 |
| `connection.rs` | 314 | ~80 | -234 |

### 4.4 Note on dispatch.rs

**File:** `crates/engine-adapters/src/infrastructure/websocket/dispatch.rs` (306 lines)

**No changes required.** The dispatch module routes 34 `ClientMessage` variants to handler functions via a match statement. This routing logic is already at the correct abstraction level for the adapter layer.

**Current pattern (preserved after refactoring):**
```rust
pub async fn handle_message(
    msg: ClientMessage,
    state: &AppState,
    client_id: Uuid,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::MoveToRegion { pc_id, region_id } => {
            movement::handle_move_to_region(state, client_id, pc_id, region_id).await
        }
        ClientMessage::StagingApprovalResponse { ... } => {
            staging::handle_staging_approval_response(state, client_id, ...).await
        }
        // ... 32 more variants
    }
}
```

**What changes:** The handler *implementations* (e.g., `movement::handle_move_to_region`) become thin adapters that call use cases via `state.use_cases.*`. The dispatch routing itself remains unchanged.

**Message type breakdown by handler module:**
- Connection: 4 (Heartbeat, JoinWorld, LeaveWorld, SetSpectateTarget)
- Movement: 3 (SelectPlayerCharacter, MoveToRegion, ExitToLocation)
- Staging: 3 (StagingApprovalResponse, StagingRegenerateRequest, PreStageRegion)
- Challenge: 11 (ChallengeRoll, TriggerChallenge, etc.)
- Inventory: 4 (EquipItem, UnequipItem, DropItem, PickupItem)
- Scene: 3 (RequestSceneChange, DirectorialUpdate, ApprovalDecision)
- Misc: 4 (CheckComfyUIHealth, ShareNpcLocation, TriggerApproachEvent, TriggerLocationEvent)
- Narrative: 1 (NarrativeEventSuggestionDecision)
- Request: 1 (Request - delegates to AppRequestHandler)

---

## Phase 5: Player Adapter Deduplication

**Duration:** 4-5 hours
**Dependencies:** None (can run parallel to other phases)
**Stream:** B (Parallel)

### 5.1 Create ClientMessageBuilder

**File:** `crates/player-adapters/src/infrastructure/websocket/message_builder.rs`

Centralizes all `ClientMessage` construction (~350 lines):
- Connection messages (join, leave, heartbeat, spectate, select PC)
- Movement messages (move to region, exit to location)
- Staging messages (approval, regenerate, pre-stage)
- Challenge messages (roll, trigger, outcome, adhoc)
- Inventory messages (equip, unequip, drop, pickup)
- Scene messages (change, directorial, approval)
- Action messages (player action)
- Misc messages (comfyui, approach, location event, share NPC, time)
- Request wrapper

### 5.2 Simplify WASM Adapter

**File:** `crates/player-adapters/src/infrastructure/websocket/wasm/adapter.rs` (modify)

Reduce from 354 to ~150 lines using `ClientMessageBuilder`.

### 5.3 Simplify Desktop Adapter

**File:** `crates/player-adapters/src/infrastructure/websocket/desktop/adapter.rs` (modify)

Reduce from 496 to ~200 lines using `ClientMessageBuilder` and `spawn_send` helper.

### 5.4 Update Module Exports

**File:** `crates/player-adapters/src/infrastructure/websocket/mod.rs` (modify)

Add:
```rust
mod message_builder;
pub use message_builder::*;
```

---

## Phase 6: Testing

**Duration:** 6-8 hours
**Dependencies:** Phase 3 (Use Cases)
**Stream:** A (integrated with Phase 3)

Tests are written alongside each use case (test-as-you-go approach).

### 6.1 Test Coverage Goals

| Use Case | Test Count | Priority |
|----------|------------|----------|
| MovementUseCase | 10 | HIGH |
| StagingApprovalUseCase | 8 | HIGH |
| InventoryUseCase | 8 | MEDIUM |
| ChallengeUseCase | 6 | MEDIUM |
| ConnectionUseCase | 5 | MEDIUM |
| ObservationUseCase | 4 | LOW |
| SceneUseCase | 4 | LOW |
| SceneBuilder | 5 | MEDIUM |
| **Total** | ~50 | |

### 6.2 Test Location Convention

Tests are placed as inline `#[cfg(test)]` modules at the bottom of each use case file:

```rust
// movement.rs
pub struct MovementUseCase { /* ... */ }

impl MovementUseCase { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use wrldbldr_engine_ports::outbound::MockBroadcastPort;
    
    #[tokio::test]
    async fn move_to_region_with_valid_staging_returns_scene_changed() {
        // ...
    }
}
```

**Rationale:** Inline tests are preferred because:
- Tests stay close to the code they test
- Private functions can be tested directly
- Refactoring moves tests along with code
- `cargo test` automatically finds and runs them

### 6.3 Test Pattern

Each use case test module:
1. Uses `mockall` for port mocking (enabled via `#[cfg_attr(test, mockall::automock)]` on port traits)
2. Tests success paths
3. Tests error paths
4. Tests edge cases
5. Verifies broadcast calls using `mock.expect_broadcast().times(1).returning(|_, _| ())`

---

## Phase 7: Arch-Check Enhancements

**Duration:** 3-4 hours
**Dependencies:** None (can run parallel)
**Stream:** C (Parallel)

### 7.1 Add Handler Complexity Check

**File:** `crates/xtask/src/main.rs` (modify)

```rust
/// Check that WebSocket handlers remain thin routing layers
fn check_handler_complexity() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let handlers_dir = workspace_root
        .join("crates/engine-adapters/src/infrastructure/websocket/handlers");

    if !handlers_dir.exists() {
        return Ok(());
    }

    let mut violations = Vec::new();

    // Exempt files that are allowed to be thin or special
    let exempt_files: std::collections::HashSet<&str> = 
        ["mod.rs", "request.rs"].into_iter().collect();

    const MAX_HANDLER_LINES: usize = 200;

    // Patterns indicating business logic that should be in use cases
    let business_logic_patterns = [
        // Building protocol DTOs directly in handlers
        regex_lite::Regex::new(
            r"(StagedNpcInfo|NpcPresenceData|RegionData|NavigationData|WaitingPcInfo)\s*\{"
        )?,
        // Complex iterator chains (mapping domain to protocol)
        regex_lite::Regex::new(r"\.iter\(\)[\s\S]{80,}\.map\(")?,
        // Direct repository calls with complex result handling
        regex_lite::Regex::new(r"state\.repository\.\w+\(\)\.\w+\([^)]+\)\.await[\s\S]{50,}match")?,
    ];

    for entry in walkdir_rs_files(&handlers_dir)? {
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if exempt_files.contains(file_name) {
            continue;
        }

        let contents = std::fs::read_to_string(&entry)?;
        let line_count = contents.lines().count();

        if line_count > MAX_HANDLER_LINES {
            violations.push(format!(
                "{}: {} lines exceeds max {} - extract to use case",
                entry.display(), line_count, MAX_HANDLER_LINES
            ));
        }

        for pattern in &business_logic_patterns {
            if pattern.is_match(&contents) {
                violations.push(format!(
                    "{}: contains business logic pattern - move to use case layer",
                    entry.display()
                ));
                break;
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("Handler complexity violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: handlers too complex");
    }

    Ok(())
}
```

### 7.2 Add Use Case Protocol Import Check

```rust
/// Check that use cases don't import protocol types
fn check_use_case_layer() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let use_cases_dir = workspace_root
        .join("crates/engine-app/src/application/use_cases");

    if !use_cases_dir.exists() {
        anyhow::bail!(
            "Missing use_cases directory: {}\n\
            Complex workflows require use cases. See docs/architecture/hexagonal-architecture.md",
            use_cases_dir.display()
        );
    }

    // Forbidden: importing ServerMessage in use cases
    let forbidden_import = regex_lite::Regex::new(
        r"use\s+wrldbldr_protocol::[^;]*ServerMessage"
    )?;

    // Also forbidden: importing ClientMessage (use cases are server-side only)
    let forbidden_client = regex_lite::Regex::new(
        r"use\s+wrldbldr_protocol::[^;]*ClientMessage"
    )?;

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&use_cases_dir)? {
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "mod.rs" {
            continue;
        }

        let contents = std::fs::read_to_string(&entry)?;

        if forbidden_import.is_match(&contents) {
            violations.push(format!(
                "{}: imports ServerMessage - use cases must return domain types",
                entry.display()
            ));
        }

        if forbidden_client.is_match(&contents) {
            violations.push(format!(
                "{}: imports ClientMessage - use cases are server-side only",
                entry.display()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!("Use case layer violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: use cases import protocol types");
    }

    Ok(())
}
```

### 7.3 Add Adapter Service Internal Import Check

```rust
/// Check that adapters don't import service implementation internals
fn check_adapter_imports() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let adapters_dir = workspace_root.join("crates/engine-adapters/src");

    if !adapters_dir.exists() {
        return Ok(());
    }

    // Forbidden: importing internal types from services
    // e.g., `use wrldbldr_engine_app::application::services::staging_service::ApprovedNpcData`
    let forbidden_pattern = regex_lite::Regex::new(
        r"use\s+wrldbldr_engine_app::application::services::\w+::\w+"
    )?;

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&adapters_dir)? {
        let contents = std::fs::read_to_string(&entry)?;

        if let Some((line_no, line)) = first_match_line(&forbidden_pattern, &contents) {
            violations.push(format!(
                "{}:{}: imports service internals - use ports or use_cases instead:\n    {}",
                entry.display(), line_no, line.trim()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!("Adapter import violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: adapters import service internals");
    }

    Ok(())
}
```

### 7.4 Update Main arch_check Function

```rust
fn arch_check() -> anyhow::Result<()> {
    // ... existing checks ...
    
    check_no_cross_crate_shims()?;

    // New architecture checks
    check_handler_complexity()?;
    check_use_case_layer()?;
    check_adapter_imports()?;

    println!("arch-check OK ({checked} workspace crates checked)");
    Ok(())
}
```

---

## Phase 8: Documentation & Cleanup

**Duration:** 2-3 hours
**Dependencies:** Phases 1-7
**Stream:** C (After other phases)

### 8.1 Update Architecture Documentation

**File:** `docs/architecture/hexagonal-architecture.md` (modify)

Add section on use cases and handler patterns.

### 8.2 Update AGENTS.md

Add guidance about use case layer for AI agents.

### 8.3 Remove Dead Code

- Remove unused converters in `websocket/converters.rs`
- Remove orphaned helper functions from handlers
- Clean up any commented-out code

---

## Summary

### Files Created (~3,100 lines)

| Crate | File | Lines |
|-------|------|-------|
| engine-adapters | `websocket/context.rs` | 150 |
| engine-adapters | `http/middleware/auth.rs` | 80 |
| engine-adapters | `websocket/broadcast_adapter.rs` | 250 |
| engine-ports | `outbound/broadcast_port.rs` | 30 |
| engine-ports | `outbound/game_events.rs` | 200 |
| engine-app | `use_cases/mod.rs` | 40 |
| engine-app | `use_cases/context.rs` | 40 |
| engine-app | `use_cases/errors.rs` | 250 |
| engine-app | `use_cases/movement.rs` | 350 |
| engine-app | `use_cases/staging.rs` | 300 |
| engine-app | `use_cases/inventory.rs` | 200 |
| engine-app | `use_cases/challenge.rs` | 200 |
| engine-app | `use_cases/observation.rs` | 150 |
| engine-app | `use_cases/scene.rs` | 150 |
| engine-app | `use_cases/connection.rs` | 180 |
| engine-app | `use_cases/player_action.rs` | 200 |
| engine-app | `use_cases/builders/scene_builder.rs` | 200 |
| engine-app | tests | 400 |
| player-adapters | `websocket/message_builder.rs` | 350 |
| xtask | New check functions | 100 |

### Files Modified (net reduction ~4,200 lines)

| File | Before | After | Change |
|------|--------|-------|--------|
| `handlers/movement.rs` | 964 | 100 | -864 |
| `handlers/staging.rs` | 629 | 80 | -549 |
| `handlers/player_action.rs` | 448 | 50 | -398 |
| `handlers/inventory.rs` | 518 | 100 | -418 |
| `handlers/misc.rs` | 465 | 150 | -315 |
| `handlers/challenge.rs` | 817 | 200 | -617 |
| `handlers/scene.rs` | 373 | 80 | -293 |
| `handlers/connection.rs` | 314 | 80 | -234 |
| `wasm/adapter.rs` | 354 | 150 | -204 |
| `desktop/adapter.rs` | 496 | 200 | -296 |

### Net Change

- **New code:** ~3,100 lines
- **Removed code:** ~4,200 lines
- **Net reduction:** ~1,100 lines
- **Test coverage:** +50 new tests

### Effort Estimate

| Phase | Stream | Hours |
|-------|--------|-------|
| 1 - Infrastructure Helpers | A | 2-3 |
| 2 - Ports & Domain Events | A | 3-4 |
| 3 - Use Cases | A | 12-15 |
| 4 - Adapter Implementation | A | 5-6 |
| 5 - Player Adapter Deduplication | B (parallel) | 4-5 |
| 6 - Testing | A (integrated) | 6-8 |
| 7 - Arch-Check Enhancements | C (parallel) | 3-4 |
| 8 - Documentation & Cleanup | C | 2-3 |
| **Total** | | **38-48** |

With parallelization (Streams B and C running alongside A), effective elapsed time: **~30-38 hours**

---

## Verification Checklist

After implementation, verify:

**Build & Test:**
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` passes
- [ ] `cargo xtask arch-check` passes (including new checks)

**Manual Tests:**
- [ ] PC movement triggers staging approval
- [ ] DM staging approval sends scene to waiting PCs
- [ ] Player actions queue correctly
- [ ] Challenge workflow (trigger, roll, outcome) works end-to-end
- [ ] WASM client connects and operates
- [ ] Desktop client connects and operates

**Architecture Compliance:**
- [ ] All handler files < 200 lines
- [ ] No `ServerMessage` imports in use cases (except errors.rs)
- [ ] No service internal imports in adapters
- [ ] `BroadcastSink` trait removed from codebase
- [ ] `BroadcastPort` used by all use cases for notifications
- [ ] `AppRequestHandler` migrated from `BroadcastSink` to `BroadcastPort`

---

## Implementation Order

1. **Start parallel streams B and C immediately** (no dependencies)
2. **Stream A: Phase 1** → Create `context.rs` helper
3. **Stream A: Phase 2** → Create `BroadcastPort`, `GameEvent`, `UseCaseContext`
   - Also: Deprecate `BroadcastSink` (add `#[deprecated]` attribute)
4. **Stream A: Phase 3** → Create use cases (sequential order):
   - `errors.rs`, `builders/scene_builder.rs`
   - `movement.rs` (with tests) - validates pattern
   - `staging.rs` (with tests) - shares SceneBuilder
   - `challenge.rs` (with tests) - full extraction despite existing service delegation
   - Remaining use cases with tests
5. **Stream A: Phase 4** → Create `WebSocketBroadcastAdapter`, refactor handlers
   - Migrate `AppRequestHandler` from `BroadcastSink` to `BroadcastPort`
   - Remove `BroadcastSink` trait entirely
6. **Converge all streams** → Phase 8 cleanup and documentation
7. **Final verification** → Run all checks

After each phase, run `cargo check --workspace` to catch issues early.

---

## Key Design Decisions

This section documents the rationale for major architectural choices in this refactoring.

### D1: Unified GameEvent Enum vs Multiple Methods

**Decision:** Use a single `BroadcastPort::broadcast(world_id, GameEvent)` method with a unified `GameEvent` enum.

**Alternatives considered:**
- Multiple methods: `notify_staging_required()`, `send_scene_changed()`, etc.

**Rationale:**
- Single routing point in adapter (simpler implementation)
- Easier to add new event types (no trait changes)
- Cleaner mock setup in tests (`expect_broadcast` once vs many)
- Event routing logic centralized in adapter, not scattered across trait methods

### D2: Typed Context Variants (DmContext, PlayerContext)

**Decision:** `require_dm()` and `require_player()` return typed context structs, not just `Result<(), Error>`.

**Alternatives considered:**
- Simple boolean checks with early return
- Single context type with runtime checks

**Rationale:**
- Compile-time guarantee that DM-only code has DM context
- `PlayerContext` guarantees `pc_id` is `Some` (no unwrap needed)
- Self-documenting handler signatures
- Prevents bugs where wrong context is used

### D3: Travel Actions Delegate to MovementUseCase

**Decision:** `PlayerActionUseCase::handle_action()` delegates "travel" actions to `MovementUseCase` rather than duplicating movement logic.

**Alternatives considered:**
- Inline travel handling in PlayerActionUseCase
- Separate TravelActionUseCase

**Rationale:**
- Single source of truth for movement logic
- MovementUseCase already handles staging, scene building, etc.
- Avoids code duplication
- Cleaner separation: PlayerActionUseCase handles queueable actions, MovementUseCase handles immediate navigation

### D4: Inline Tests vs Separate Test Files

**Decision:** Tests are placed as `#[cfg(test)]` modules at the bottom of each use case file.

**Alternatives considered:**
- Separate `*_test.rs` files
- `tests/` directory with integration tests

**Rationale:**
- Tests stay close to code they test
- Private functions can be tested directly
- Refactoring moves tests with code automatically
- Consistent with Rust conventions for unit tests
- Integration tests can still live in `tests/` if needed later

### D5: BroadcastPort in engine-ports (Not domain)

**Decision:** `BroadcastPort` and `GameEvent` live in `engine-ports`, not `domain`.

**Alternatives considered:**
- Domain layer (since events are domain concepts)

**Rationale:**
- Events contain IDs but no business logic (value objects, not entities)
- Port is an application-layer contract, not domain logic
- Avoids domain depending on async-trait
- Keeps domain layer pure and zero-dependency
- GameEvent is still transport-agnostic (adapter converts to ServerMessage)

### D6: Handler Max Line Limit (200 lines)

**Decision:** Arch-check enforces max 200 lines per handler file (except `mod.rs`, `request.rs`).

**Alternatives considered:**
- No limit (rely on code review)
- Stricter limit (100 lines)

**Rationale:**
- 200 lines allows reasonable complexity for handlers with many variants
- Forces extraction when handlers grow beyond thin routing
- `request.rs` exempt because it's already the ideal pattern
- Can tighten to 150 lines after initial refactoring if desired

### D7: Error Types Per Use Case Domain

**Decision:** Separate error types for each use case domain (`MovementError`, `StagingError`, etc.) rather than a unified error type.

**Alternatives considered:**
- Single `UseCaseError` enum with all variants
- Using `anyhow::Error`

**Rationale:**
- Domain-specific error codes (e.g., `PC_NOT_FOUND` vs `REGION_NOT_FOUND`)
- Exhaustive match in handlers for each domain
- Cleaner API: methods return their specific error type
- `ErrorCode` trait standardizes conversion to protocol error codes

### D8: Replace BroadcastSink with BroadcastPort

**Decision:** Deprecate and remove the existing `BroadcastSink` trait, replacing it with the new `BroadcastPort`.

**Existing `BroadcastSink`** (in `engine-ports/src/inbound/request_handler.rs`):
- Takes `ServerMessage` directly (protocol type)
- Located in inbound ports but semantically outbound
- Used optionally by `AppRequestHandler`

**New `BroadcastPort`** (in `engine-ports/src/outbound/broadcast_port.rs`):
- Takes `GameEvent` (domain type)
- Correctly located in outbound ports
- Used by all use cases

**Alternatives considered:**
- Extend `BroadcastSink` with new methods
- Keep both traits for backward compatibility

**Rationale:**
- `BroadcastSink` violates hexagonal architecture by coupling app to protocol
- Clean break avoids API confusion and legacy code paths
- `BroadcastPort` with `GameEvent` is the correct abstraction
- `AppRequestHandler` uses `BroadcastSink` optionally, migration is straightforward
- `WorldConnectionPort` remains for low-level adapter-internal use

### D9: Full Extraction for challenge.rs Despite Service Delegation

**Decision:** Fully extract `challenge.rs` handler logic to `ChallengeUseCase` even though it already delegates to `ChallengeResolutionService`.

**Current state:**
- 817 lines, 11 handler functions
- Most logic delegated to `ChallengeResolutionService`
- Repeated boilerplate: context extraction, authorization, error handling

**Alternatives considered:**
- Leave as-is since core logic is already in services
- Partial extraction (only some handlers)

**Rationale:**
- Consistency: all handlers should follow the same thin-adapter pattern
- Boilerplate reduction: 11 functions × ~30 lines of repeated patterns = ~330 lines
- Testability: use case layer is easier to unit test than handlers
- Future-proofing: challenge workflow may need cross-cutting concerns (auditing, metrics)
- Target: reduce from 817 to ~200 lines
