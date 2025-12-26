# WebSocket-First Architecture Refactor Plan

**Created**: 2025-12-26  
**Status**: IN PROGRESS  
**Estimated Effort**: 15 days  
**Priority**: P0 (Major Architecture Change)

---

## Executive Summary

This refactor migrates WrldBldr from a hybrid REST+WebSocket architecture to a WebSocket-first model. All game operations will flow through WebSocket, eliminating duplicate code and enabling real-time multiplayer updates.

### Key Decisions

| Decision | Choice |
|----------|--------|
| Migration Strategy | Big bang (no backwards compatibility) |
| Session Concept | Deprecated - scope to worlds instead |
| Conversation History | World-scoped (persisted to world) |
| DM Multi-Screen | Allowed (same user_id, synced state) |
| Spectator Mode | Read-only, player-visible data only |
| Reconnection | Full world snapshot |
| Settings/Workflows REST | Keep (configuration, not gameplay) |
| File Uploads REST | Keep (multipart form data) |
| Authentication | Header-based (existing) |
| Request/Response Types | Protocol crate |
| RequestHandler Trait | engine-ports/inbound |
| Handler Implementation | engine-app/application/handlers |

---

## Architecture Overview

### Layer Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FINAL STRUCTURE                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  protocol/                                                                  │
│  ├── messages.rs         # ClientMessage, ServerMessage                     │
│  ├── requests.rs         # RequestPayload enum (NEW)                        │
│  └── responses.rs        # ResponseResult enum (NEW)                        │
│                                                                             │
│  engine-ports/                                                              │
│  ├── inbound/                                                               │
│  │   └── request_handler.rs  # RequestHandler trait (NEW)                   │
│  └── outbound/               # (existing repository ports, etc.)            │
│                                                                             │
│  engine-app/                                                                │
│  ├── application/                                                           │
│  │   ├── services/           # (existing services)                          │
│  │   └── handlers/           # (NEW)                                        │
│  │       └── request_handler.rs  # impl RequestHandler                      │
│  └── ...                                                                    │
│                                                                             │
│  engine-adapters/                                                           │
│  └── infrastructure/                                                        │
│      ├── websocket.rs              # WebSocket connection handling          │
│      ├── world_connection_manager.rs  # Connection tracking (NEW)           │
│      └── http/                     # Remaining REST routes                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Connection Model

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              WorldConnectionManager                           │
├──────────────────────────────────────────────────────────────────────────────┤
│  World A                                                                      │
│  ├── DM (user_id: abc123)                                                    │
│  │   ├── Connection 1 (main screen)                                          │
│  │   └── Connection 2 (approval queue screen)                                │
│  ├── Player (user_id: def456) - playing as "Thorin"                          │
│  ├── Player (user_id: ghi789) - playing as "Elara"                           │
│  └── Spectator (user_id: jkl012) - spectating "Thorin"                       │
├──────────────────────────────────────────────────────────────────────────────┤
│  World B                                                                      │
│  ├── DM (user_id: xyz999)                                                    │
│  └── Player (user_id: abc123) - same user, different world                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Message Flow

```
WebSocket Message arrives
        │
        ▼
┌───────────────────────┐
│  websocket.rs         │  (infrastructure - engine-adapters)
│  - Parse message      │
│  - Extract Request    │
└───────────┬───────────┘
            │
            ▼
┌───────────────────────┐
│  RequestHandler trait │  (inbound port - engine-ports)
│  .handle(payload,ctx) │
└───────────┬───────────┘
            │
            ▼
┌───────────────────────┐
│  AppRequestHandler    │  (application - engine-app)
│  - Route to service   │
│  - Call service       │
│  - Broadcast changes  │
└───────────┬───────────┘
            │
            ▼
┌───────────────────────┐
│  Services             │  (application - engine-app)
│  - Business logic     │
│  - Use outbound ports │
└───────────────────────┘
```

### Role-Based Access

| Role | Can Do | Sees |
|------|--------|------|
| **DM** | All operations, approve actions, trigger events | Everything |
| **Player** | Actions, navigation, inventory, PC management | Player-visible data, own PC |
| **Spectator** | Nothing (read-only) | Player-visible data for spectated PC |

---

## Phase 1: Protocol & Infrastructure (Days 1-4)

### Day 1: Protocol Types

#### Task 1.1: Create `requests.rs`

**File**: `crates/protocol/src/requests.rs` (NEW)

**Content**:
- `RequestPayload` enum with all CRUD operations
- Data types for create/update if not already existing

**RequestPayload Variants**:

```rust
pub enum RequestPayload {
    // === World ===
    ListWorlds,
    GetWorld { world_id: String },
    CreateWorld { data: CreateWorldData },
    UpdateWorld { world_id: String, data: UpdateWorldData },
    DeleteWorld { world_id: String },
    
    // === Character ===
    ListCharacters { world_id: String },
    GetCharacter { character_id: String },
    CreateCharacter { world_id: String, data: CreateCharacterData },
    UpdateCharacter { character_id: String, data: UpdateCharacterData },
    DeleteCharacter { character_id: String },
    ChangeArchetype { character_id: String, data: ChangeArchetypeData },
    GetCharacterInventory { character_id: String },
    
    // === Location ===
    ListLocations { world_id: String },
    GetLocation { location_id: String },
    CreateLocation { world_id: String, data: CreateLocationData },
    UpdateLocation { location_id: String, data: UpdateLocationData },
    DeleteLocation { location_id: String },
    GetLocationConnections { location_id: String },
    CreateLocationConnection { data: CreateConnectionData },
    DeleteLocationConnection { from_id: String, to_id: String },
    
    // === Region ===
    ListRegions { location_id: String },
    GetRegion { region_id: String },
    CreateRegion { location_id: String, data: CreateRegionData },
    UpdateRegion { region_id: String, data: UpdateRegionData },
    DeleteRegion { region_id: String },
    GetRegionConnections { region_id: String },
    CreateRegionConnection { from_id: String, to_id: String, data: CreateRegionConnectionData },
    DeleteRegionConnection { from_id: String, to_id: String },
    UnlockRegionConnection { from_id: String, to_id: String },
    GetRegionExits { region_id: String },
    CreateRegionExit { region_id: String, location_id: String },
    DeleteRegionExit { region_id: String, location_id: String },
    ListSpawnPoints { world_id: String },
    
    // === Scene ===
    ListScenes { act_id: String },
    GetScene { scene_id: String },
    CreateScene { act_id: String, data: CreateSceneData },
    UpdateScene { scene_id: String, data: UpdateSceneData },
    DeleteScene { scene_id: String },
    
    // === Act ===
    ListActs { world_id: String },
    CreateAct { world_id: String, data: CreateActData },
    
    // === Interaction ===
    ListInteractions { scene_id: String },
    GetInteraction { interaction_id: String },
    CreateInteraction { scene_id: String, data: CreateInteractionData },
    UpdateInteraction { interaction_id: String, data: UpdateInteractionData },
    DeleteInteraction { interaction_id: String },
    SetInteractionAvailability { interaction_id: String, available: bool },
    
    // === Skill ===
    ListSkills { world_id: String },
    GetSkill { skill_id: String },
    CreateSkill { world_id: String, data: CreateSkillData },
    UpdateSkill { skill_id: String, data: UpdateSkillData },
    DeleteSkill { skill_id: String },
    
    // === Challenge ===
    ListChallenges { world_id: String },
    GetChallenge { challenge_id: String },
    CreateChallenge { world_id: String, data: CreateChallengeData },
    UpdateChallenge { challenge_id: String, data: UpdateChallengeData },
    DeleteChallenge { challenge_id: String },
    SetChallengeActive { challenge_id: String, active: bool },
    SetChallengeFavorite { challenge_id: String, favorite: bool },
    
    // === NarrativeEvent ===
    ListNarrativeEvents { world_id: String },
    GetNarrativeEvent { event_id: String },
    CreateNarrativeEvent { world_id: String, data: CreateNarrativeEventData },
    UpdateNarrativeEvent { event_id: String, data: UpdateNarrativeEventData },
    DeleteNarrativeEvent { event_id: String },
    SetNarrativeEventActive { event_id: String, active: bool },
    SetNarrativeEventFavorite { event_id: String, favorite: bool },
    TriggerNarrativeEvent { event_id: String },
    ResetNarrativeEvent { event_id: String },
    
    // === EventChain ===
    ListEventChains { world_id: String },
    GetEventChain { chain_id: String },
    CreateEventChain { world_id: String, data: CreateEventChainData },
    UpdateEventChain { chain_id: String, data: UpdateEventChainData },
    DeleteEventChain { chain_id: String },
    SetEventChainActive { chain_id: String, active: bool },
    SetEventChainFavorite { chain_id: String, favorite: bool },
    AddEventToChain { chain_id: String, event_id: String, position: Option<u32> },
    RemoveEventFromChain { chain_id: String, event_id: String },
    CompleteChainEvent { chain_id: String, event_id: String },
    ResetEventChain { chain_id: String },
    GetEventChainStatus { chain_id: String },
    
    // === StoryEvent ===
    ListStoryEvents { world_id: String, page: Option<u32>, page_size: Option<u32> },
    GetStoryEvent { event_id: String },
    CreateDmMarker { world_id: String, data: CreateDmMarkerData },
    UpdateStoryEvent { event_id: String, data: UpdateStoryEventData },
    SetStoryEventVisibility { event_id: String, visible: bool },
    
    // === PlayerCharacter ===
    ListPlayerCharacters { world_id: String },
    GetPlayerCharacter { pc_id: String },
    CreatePlayerCharacter { world_id: String, data: CreatePlayerCharacterData },
    UpdatePlayerCharacter { pc_id: String, data: UpdatePlayerCharacterData },
    DeletePlayerCharacter { pc_id: String },
    UpdatePlayerCharacterLocation { pc_id: String, region_id: String },
    
    // === Relationship ===
    GetSocialNetwork { world_id: String },
    CreateRelationship { data: CreateRelationshipData },
    DeleteRelationship { relationship_id: String },
    
    // === Observation ===
    ListObservations { pc_id: String },
    CreateObservation { pc_id: String, data: CreateObservationData },
    DeleteObservation { pc_id: String, npc_id: String },
    
    // === Goal (Actantial) ===
    ListGoals { world_id: String },
    GetGoal { goal_id: String },
    CreateGoal { world_id: String, data: CreateGoalData },
    UpdateGoal { goal_id: String, data: UpdateGoalData },
    DeleteGoal { goal_id: String },
    
    // === Want (Actantial) ===
    ListWants { character_id: String },
    GetWant { want_id: String },
    CreateWant { character_id: String, data: CreateWantData },
    UpdateWant { want_id: String, data: UpdateWantData },
    DeleteWant { want_id: String },
    SetWantTarget { want_id: String, target_id: String, target_type: String },
    RemoveWantTarget { want_id: String },
    
    // === ActantialView ===
    GetActantialContext { character_id: String },
    AddActantialView { character_id: String, data: AddActantialViewData },
    RemoveActantialView { character_id: String, data: RemoveActantialViewData },
    
    // === Game Time ===
    GetGameTime { world_id: String },
    AdvanceGameTime { world_id: String, data: AdvanceTimeData },
    
    // === Character-Region Relationships ===
    ListCharacterRegionRelationships { character_id: String },
    SetCharacterHomeRegion { character_id: String, region_id: String },
    SetCharacterWorkRegion { character_id: String, region_id: String },
    RemoveCharacterRegionRelationship { character_id: String, region_id: String, relationship_type: String },
    ListRegionNpcs { region_id: String },
}
```

**Checklist**:
- [ ] Create `requests.rs` file
- [ ] Define `RequestPayload` enum
- [ ] Add all data types for create/update operations
- [ ] Export from `lib.rs`

#### Task 1.2: Create `responses.rs`

**File**: `crates/protocol/src/responses.rs` (NEW)

**Content**:

```rust
use serde::{Deserialize, Serialize};

/// Result of a request operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResponseResult {
    Success {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    Error {
        code: ErrorCode,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Client errors
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    ValidationError,
    
    // Server errors
    InternalError,
    ServiceUnavailable,
    Timeout,
}

/// Entity change notification for broadcasts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityChangedData {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub change_type: ChangeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// World this entity belongs to
    pub world_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    World,
    Character,
    Location,
    Region,
    Scene,
    Act,
    Interaction,
    Skill,
    Challenge,
    NarrativeEvent,
    EventChain,
    StoryEvent,
    PlayerCharacter,
    Relationship,
    Observation,
    Goal,
    Want,
    ActantialView,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}
```

**Checklist**:
- [ ] Create `responses.rs` file
- [ ] Define `ResponseResult` enum
- [ ] Define `ErrorCode` enum
- [ ] Define `EntityChangedData` struct
- [ ] Define `EntityType` and `ChangeType` enums
- [ ] Export from `lib.rs`

#### Task 1.3: Update `messages.rs`

**File**: `crates/protocol/src/messages.rs` (MODIFY)

**Add to `ClientMessage`**:

```rust
/// Join a world (replaces JoinSession)
JoinWorld {
    world_id: String,
    role: WorldRole,
    /// For Player role, which PC to play as (required for Player)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pc_id: Option<String>,
    /// For Spectator role, which PC to spectate (required for Spectator)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spectate_pc_id: Option<String>,
},

/// Leave the current world
LeaveWorld,

/// Request-response pattern for CRUD and other operations
Request {
    request_id: String,
    payload: RequestPayload,
},

/// Spectator changes which PC they're watching
SetSpectateTarget {
    pc_id: String,
},
```

**Add to `ServerMessage`**:

```rust
/// Successfully joined a world
WorldJoined {
    world_id: String,
    snapshot: WorldSnapshot,
    connected_users: Vec<ConnectedUser>,
    your_role: WorldRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    your_pc: Option<PlayerCharacterData>,
},

/// Another user joined the world
UserJoined {
    user_id: String,
    username: Option<String>,
    role: WorldRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pc: Option<PlayerCharacterData>,
},

/// A user left the world
UserLeft {
    user_id: String,
},

/// Response to a Request
Response {
    request_id: String,
    result: ResponseResult,
},

/// Entity changed broadcast (for cache invalidation / UI updates)
EntityChanged(EntityChangedData),

/// Spectator target changed confirmation
SpectateTargetChanged {
    pc_id: String,
    pc_name: String,
},
```

**Add new types**:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldRole {
    Dm,
    Player,
    Spectator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    pub role: WorldRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pc_id: Option<String>,
    /// Number of connections (for DM with multiple screens)
    pub connection_count: u32,
}
```

**Checklist**:
- [ ] Add `JoinWorld` to `ClientMessage`
- [ ] Add `LeaveWorld` to `ClientMessage`
- [ ] Add `Request` to `ClientMessage`
- [ ] Add `SetSpectateTarget` to `ClientMessage`
- [ ] Add `WorldJoined` to `ServerMessage`
- [ ] Add `UserJoined` to `ServerMessage`
- [ ] Add `UserLeft` to `ServerMessage`
- [ ] Add `Response` to `ServerMessage`
- [ ] Add `EntityChanged` to `ServerMessage`
- [ ] Add `SpectateTargetChanged` to `ServerMessage`
- [ ] Add `WorldRole` enum
- [ ] Add `ConnectedUser` struct
- [ ] Import `RequestPayload` and `ResponseResult`
- [ ] Update lib.rs exports

#### Task 1.4: Update `protocol/lib.rs`

**File**: `crates/protocol/src/lib.rs` (MODIFY)

```rust
pub mod requests;
pub mod responses;

pub use requests::*;
pub use responses::*;
```

**Checklist**:
- [ ] Add module declarations
- [ ] Add re-exports

---

### Day 2: Inbound Port + WorldConnectionManager

#### Task 2.1: Create RequestHandler Port

**File**: `crates/engine-ports/src/inbound/request_handler.rs` (NEW)

```rust
//! Inbound port for handling WebSocket requests
//!
//! This trait defines the contract for processing request/response
//! operations over WebSocket.

use async_trait::async_trait;
use wrldbldr_domain::{CharacterId, WorldId};
use wrldbldr_protocol::{RequestPayload, ResponseResult, WorldRole};

/// Context for a request, including authentication and authorization info
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// The world this request is scoped to (if any)
    pub world_id: Option<WorldId>,
    /// The authenticated user ID
    pub user_id: String,
    /// The user's role in the current world
    pub role: WorldRole,
    /// The player character ID (for Player role)
    pub pc_id: Option<CharacterId>,
    /// The PC being spectated (for Spectator role)
    pub spectate_pc_id: Option<CharacterId>,
}

/// Inbound port for handling WebSocket request/response operations
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle a request and return a response
    ///
    /// The handler should:
    /// 1. Validate the request
    /// 2. Check authorization based on context
    /// 3. Execute the operation
    /// 4. Broadcast any entity changes
    /// 5. Return the result
    async fn handle(
        &self,
        payload: RequestPayload,
        context: RequestContext,
    ) -> ResponseResult;
}
```

**Checklist**:
- [ ] Create `request_handler.rs`
- [ ] Define `RequestContext` struct
- [ ] Define `RequestHandler` trait

#### Task 2.2: Update inbound mod.rs

**File**: `crates/engine-ports/src/inbound/mod.rs` (MODIFY)

```rust
pub mod request_handler;
pub mod use_cases;

pub use request_handler::{RequestContext, RequestHandler};
```

**Checklist**:
- [ ] Add module declaration
- [ ] Add re-exports

#### Task 2.3: Create WorldConnectionManager

**File**: `crates/engine-adapters/src/infrastructure/world_connection_manager.rs` (NEW)

**Responsibilities**:
- Track all WebSocket connections per world
- Track users (with multiple connections for DM multi-screen)
- Enforce one DM per world (by user_id)
- Provide broadcast methods
- Handle join/leave logic

**Key structures**:

```rust
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use wrldbldr_domain::{CharacterId, WorldId};
use wrldbldr_protocol::{ConnectedUser, ServerMessage, WorldRole};

pub type ConnectionId = u64;

/// Manages WebSocket connections organized by world
pub struct WorldConnectionManager {
    /// World ID -> World connection state
    worlds: DashMap<WorldId, WorldState>,
    /// Connection ID -> (WorldId, UserId) for reverse lookup
    connection_index: DashMap<ConnectionId, (WorldId, String)>,
    /// Next connection ID
    next_connection_id: std::sync::atomic::AtomicU64,
}

struct WorldState {
    /// The DM's user ID (only one allowed per world)
    dm_user_id: Option<String>,
    /// User ID -> User state
    users: HashMap<String, UserState>,
    /// Broadcast channel for this world
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

struct UserState {
    user_id: String,
    username: Option<String>,
    role: WorldRole,
    pc_id: Option<CharacterId>,
    spectate_pc_id: Option<CharacterId>,
    /// Connection IDs for this user (multiple for DM)
    connections: Vec<ConnectionId>,
}
```

**Key methods**:

```rust
impl WorldConnectionManager {
    pub fn new() -> Self;
    
    /// Register a new connection and join a world
    /// Returns error if:
    /// - Trying to join as DM when another DM exists
    /// - Player without pc_id
    /// - Spectator without spectate_pc_id
    pub async fn join_world(
        &self,
        world_id: WorldId,
        user_id: String,
        username: Option<String>,
        role: WorldRole,
        pc_id: Option<CharacterId>,
        spectate_pc_id: Option<CharacterId>,
    ) -> Result<(ConnectionId, broadcast::Receiver<ServerMessage>), JoinError>;
    
    /// Remove a connection
    /// If this was the user's last connection, they leave the world
    pub async fn leave(&self, connection_id: ConnectionId);
    
    /// Get all connected users for a world
    pub fn get_connected_users(&self, world_id: WorldId) -> Vec<ConnectedUser>;
    
    /// Check if DM is connected to a world
    pub fn is_dm_connected(&self, world_id: WorldId) -> bool;
    
    /// Get the DM's user ID for a world
    pub fn get_dm_user_id(&self, world_id: WorldId) -> Option<String>;
    
    /// Broadcast a message to all connections in a world
    pub async fn broadcast_to_world(&self, world_id: WorldId, message: ServerMessage);
    
    /// Broadcast to all except a specific user
    pub async fn broadcast_to_world_except(
        &self,
        world_id: WorldId,
        exclude_user_id: &str,
        message: ServerMessage,
    );
    
    /// Send a message to DM only
    pub async fn send_to_dm(&self, world_id: WorldId, message: ServerMessage);
    
    /// Send a message to a specific user
    pub async fn send_to_user(&self, world_id: WorldId, user_id: &str, message: ServerMessage);
    
    /// Send a message to a specific player (by PC ID)
    pub async fn send_to_player(&self, world_id: WorldId, pc_id: CharacterId, message: ServerMessage);
    
    /// Update spectator's target
    pub fn set_spectate_target(
        &self,
        connection_id: ConnectionId,
        pc_id: CharacterId,
    ) -> Result<(), SpectateError>;
    
    /// Get connection's context
    pub fn get_context(&self, connection_id: ConnectionId) -> Option<ConnectionContext>;
}

#[derive(Debug)]
pub enum JoinError {
    DmAlreadyConnected { existing_user_id: String },
    PlayerRequiresPc,
    SpectatorRequiresTarget,
    InvalidWorld,
}

#[derive(Debug)]
pub enum SpectateError {
    NotSpectator,
    ConnectionNotFound,
}

#[derive(Debug, Clone)]
pub struct ConnectionContext {
    pub world_id: WorldId,
    pub user_id: String,
    pub role: WorldRole,
    pub pc_id: Option<CharacterId>,
    pub spectate_pc_id: Option<CharacterId>,
}
```

**Checklist**:
- [ ] Create `world_connection_manager.rs`
- [ ] Implement `WorldConnectionManager` struct
- [ ] Implement `WorldState` and `UserState`
- [ ] Implement `join_world` method
- [ ] Implement `leave` method
- [ ] Implement `get_connected_users` method
- [ ] Implement broadcast methods
- [ ] Implement `set_spectate_target` method
- [ ] Implement `get_context` method
- [ ] Add to `infrastructure/mod.rs`

---

### Day 3: Application Request Handler

#### Task 3.1: Create handlers module

**File**: `crates/engine-app/src/application/handlers/mod.rs` (NEW)

```rust
//! Request handlers for WebSocket operations
//!
//! This module contains the implementation of the RequestHandler port,
//! routing incoming requests to the appropriate services.

mod request_handler;

pub use request_handler::AppRequestHandler;
```

#### Task 3.2: Create AppRequestHandler

**File**: `crates/engine-app/src/application/handlers/request_handler.rs` (NEW)

This is the core of the refactor - it routes all requests to services and broadcasts changes.

**Structure**:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use wrldbldr_engine_ports::inbound::{RequestContext, RequestHandler};
use wrldbldr_protocol::{
    RequestPayload, ResponseResult, ErrorCode,
    EntityChangedData, EntityType, ChangeType,
};

use crate::application::services::{
    WorldService, CharacterService, LocationService,
    SceneService, ChallengeService, SkillService,
    NarrativeEventService, EventChainService, StoryEventService,
    PlayerCharacterService, ActantialContextService,
    // ... other services
};

/// Callback for broadcasting entity changes
pub type BroadcastFn = Arc<dyn Fn(EntityChangedData) + Send + Sync>;

pub struct AppRequestHandler {
    // Services
    world_service: Arc<dyn WorldService>,
    character_service: Arc<dyn CharacterService>,
    location_service: Arc<dyn LocationService>,
    // ... all services
    
    // Broadcast callback
    broadcast: BroadcastFn,
}

impl AppRequestHandler {
    pub fn new(
        world_service: Arc<dyn WorldService>,
        character_service: Arc<dyn CharacterService>,
        // ... all services
        broadcast: BroadcastFn,
    ) -> Self {
        Self {
            world_service,
            character_service,
            // ...
            broadcast,
        }
    }
    
    // Helper to broadcast entity changes
    fn broadcast_change(&self, data: EntityChangedData) {
        (self.broadcast)(data);
    }
    
    // Helper to create success response
    fn success<T: serde::Serialize>(data: T) -> ResponseResult {
        ResponseResult::Success {
            data: Some(serde_json::to_value(data).unwrap_or_default()),
        }
    }
    
    fn success_empty() -> ResponseResult {
        ResponseResult::Success { data: None }
    }
    
    // Helper to create error response
    fn error(code: ErrorCode, message: impl Into<String>) -> ResponseResult {
        ResponseResult::Error {
            code,
            message: message.into(),
            details: None,
        }
    }
    
    // Authorization helpers
    fn require_dm(&self, ctx: &RequestContext) -> Result<(), ResponseResult> {
        if ctx.role != WorldRole::Dm {
            return Err(Self::error(ErrorCode::Forbidden, "DM role required"));
        }
        Ok(())
    }
    
    fn require_dm_or_player(&self, ctx: &RequestContext) -> Result<(), ResponseResult> {
        if ctx.role == WorldRole::Spectator {
            return Err(Self::error(ErrorCode::Forbidden, "Spectators cannot modify data"));
        }
        Ok(())
    }
}

#[async_trait]
impl RequestHandler for AppRequestHandler {
    async fn handle(&self, payload: RequestPayload, ctx: RequestContext) -> ResponseResult {
        match payload {
            // === World ===
            RequestPayload::ListWorlds => self.list_worlds(&ctx).await,
            RequestPayload::GetWorld { world_id } => self.get_world(&ctx, world_id).await,
            RequestPayload::CreateWorld { data } => self.create_world(&ctx, data).await,
            RequestPayload::UpdateWorld { world_id, data } => {
                self.update_world(&ctx, world_id, data).await
            }
            RequestPayload::DeleteWorld { world_id } => self.delete_world(&ctx, world_id).await,
            
            // === Character ===
            RequestPayload::ListCharacters { world_id } => {
                self.list_characters(&ctx, world_id).await
            }
            // ... all other handlers
            
            _ => Self::error(ErrorCode::BadRequest, "Unknown request type"),
        }
    }
}

// Implement handlers for each entity type
impl AppRequestHandler {
    // === World handlers ===
    async fn list_worlds(&self, ctx: &RequestContext) -> ResponseResult {
        match self.world_service.list_worlds().await {
            Ok(worlds) => Self::success(worlds),
            Err(e) => Self::error(ErrorCode::InternalError, e.to_string()),
        }
    }
    
    async fn create_world(&self, ctx: &RequestContext, data: CreateWorldData) -> ResponseResult {
        match self.world_service.create_world(data.into()).await {
            Ok(world) => {
                // No broadcast for world creation (not scoped to a world)
                Self::success(world)
            }
            Err(e) => Self::error(ErrorCode::InternalError, e.to_string()),
        }
    }
    
    // === Character handlers ===
    async fn list_characters(&self, ctx: &RequestContext, world_id: String) -> ResponseResult {
        let world_id = match parse_world_id(&world_id) {
            Ok(id) => id,
            Err(e) => return e,
        };
        
        match self.character_service.list_characters(world_id).await {
            Ok(characters) => Self::success(characters),
            Err(e) => Self::error(ErrorCode::InternalError, e.to_string()),
        }
    }
    
    async fn create_character(
        &self,
        ctx: &RequestContext,
        world_id: String,
        data: CreateCharacterData,
    ) -> ResponseResult {
        self.require_dm(ctx)?;
        
        let world_id = match parse_world_id(&world_id) {
            Ok(id) => id,
            Err(e) => return e,
        };
        
        match self.character_service.create_character(data.into()).await {
            Ok(character) => {
                self.broadcast_change(EntityChangedData {
                    entity_type: EntityType::Character,
                    entity_id: character.id.to_string(),
                    change_type: ChangeType::Created,
                    data: Some(serde_json::to_value(&character).unwrap_or_default()),
                    world_id: world_id.to_string(),
                });
                Self::success(character)
            }
            Err(e) => Self::error(ErrorCode::InternalError, e.to_string()),
        }
    }
    
    // ... implement all other handlers
}

fn parse_world_id(id: &str) -> Result<WorldId, ResponseResult> {
    uuid::Uuid::parse_str(id)
        .map(WorldId::from_uuid)
        .map_err(|_| AppRequestHandler::error(ErrorCode::BadRequest, "Invalid world ID"))
}
```

**Checklist**:
- [ ] Create `handlers/mod.rs`
- [ ] Create `handlers/request_handler.rs`
- [ ] Implement `AppRequestHandler` struct
- [ ] Implement `RequestHandler` trait
- [ ] Implement World handlers (List, Get, Create, Update, Delete)
- [ ] Implement Character handlers
- [ ] Implement Location handlers
- [ ] Implement Region handlers
- [ ] Implement Scene handlers
- [ ] Implement Skill handlers
- [ ] Implement Challenge handlers
- [ ] Implement NarrativeEvent handlers
- [ ] Implement EventChain handlers
- [ ] Implement StoryEvent handlers
- [ ] Implement PlayerCharacter handlers
- [ ] Implement Relationship handlers
- [ ] Implement Observation handlers
- [ ] Implement Goal handlers
- [ ] Implement Want handlers
- [ ] Implement ActantialView handlers
- [ ] Implement GameTime handlers
- [ ] Add authorization checks (DM-only for mutations)
- [ ] Add broadcast calls for all mutations

#### Task 3.3: Update application mod.rs

**File**: `crates/engine-app/src/application/mod.rs` (MODIFY)

Add:
```rust
pub mod handlers;
pub use handlers::AppRequestHandler;
```

**Checklist**:
- [ ] Add handlers module
- [ ] Export AppRequestHandler

---

### Day 4: WebSocket Integration

#### Task 4.1: Update websocket.rs

**File**: `crates/engine-adapters/src/infrastructure/websocket.rs` (MODIFY)

**Changes**:
1. Add `WorldConnectionManager` to handler state
2. Replace session-based logic with world-based logic
3. Handle `JoinWorld` message
4. Handle `LeaveWorld` message
5. Handle `Request` message (delegate to RequestHandler)
6. Handle `SetSpectateTarget` message
7. Update existing handlers to use world-scoped broadcasts

**Key changes**:

```rust
// Add to handler state
world_connections: Arc<WorldConnectionManager>,
request_handler: Arc<dyn RequestHandler>,

// Handle new message types
ClientMessage::JoinWorld { world_id, role, pc_id, spectate_pc_id } => {
    // 1. Parse world_id
    // 2. Call world_connections.join_world(...)
    // 3. Load world snapshot
    // 4. Send WorldJoined response
    // 5. Broadcast UserJoined to others
}

ClientMessage::LeaveWorld => {
    // 1. Call world_connections.leave(connection_id)
    // 2. Broadcast UserLeft to others
}

ClientMessage::Request { request_id, payload } => {
    // 1. Get context from world_connections
    // 2. Build RequestContext
    // 3. Call request_handler.handle(payload, context)
    // 4. Send Response with request_id
}

ClientMessage::SetSpectateTarget { pc_id } => {
    // 1. Call world_connections.set_spectate_target(...)
    // 2. Send SpectateTargetChanged response
}
```

**Checklist**:
- [ ] Add WorldConnectionManager to handler state
- [ ] Add RequestHandler to handler state
- [ ] Implement JoinWorld handler
- [ ] Implement LeaveWorld handler
- [ ] Implement Request handler
- [ ] Implement SetSpectateTarget handler
- [ ] Update broadcast calls to use WorldConnectionManager
- [ ] Remove/update session-based code

#### Task 4.2: Update AppState

**File**: `crates/engine-adapters/src/infrastructure/state/mod.rs` (MODIFY)

Add:
```rust
pub world_connections: Arc<WorldConnectionManager>,
```

Wire up in initialization.

**Checklist**:
- [ ] Add WorldConnectionManager to AppState
- [ ] Initialize WorldConnectionManager
- [ ] Initialize AppRequestHandler with services
- [ ] Wire broadcast callback to WorldConnectionManager

#### Task 4.3: Update infrastructure mod.rs

**File**: `crates/engine-adapters/src/infrastructure/mod.rs` (MODIFY)

Add:
```rust
pub mod world_connection_manager;
pub use world_connection_manager::WorldConnectionManager;
```

**Checklist**:
- [ ] Add module declaration
- [ ] Add re-export

---

## Phase 2: Migrate CRUD Operations (Days 5-9)

### Day 5: World + Character

All handlers for World and Character entities.

**Checklist**:
- [ ] Test ListWorlds
- [ ] Test GetWorld
- [ ] Test CreateWorld
- [ ] Test UpdateWorld
- [ ] Test DeleteWorld
- [ ] Test ListCharacters
- [ ] Test GetCharacter
- [ ] Test CreateCharacter
- [ ] Test UpdateCharacter
- [ ] Test DeleteCharacter
- [ ] Test ChangeArchetype
- [ ] Test GetCharacterInventory
- [ ] Verify broadcasts work

### Day 6: Location + Region

**Checklist**:
- [ ] Test all Location operations
- [ ] Test all Region operations
- [ ] Test Region connections
- [ ] Test Region exits
- [ ] Verify broadcasts work

### Day 7: Scene + Skill + Challenge

**Checklist**:
- [ ] Test all Scene operations
- [ ] Test all Act operations
- [ ] Test all Interaction operations
- [ ] Test all Skill operations
- [ ] Test all Challenge operations
- [ ] Test active/favorite toggles
- [ ] Verify broadcasts work

### Day 8: NarrativeEvent + EventChain + StoryEvent

**Checklist**:
- [ ] Test all NarrativeEvent operations
- [ ] Test trigger/reset
- [ ] Test all EventChain operations
- [ ] Test chain event management
- [ ] Test all StoryEvent operations
- [ ] Verify broadcasts work

### Day 9: PlayerCharacter + Relationship + Observation

**Checklist**:
- [ ] Test all PlayerCharacter operations
- [ ] Test location update
- [ ] Test Relationship operations
- [ ] Test Observation operations
- [ ] Test SocialNetwork query
- [ ] Verify broadcasts work

---

## Phase 3: Consolidate & Remove Duplication (Days 10-11)

### Day 10: Consolidate Existing WebSocket Handlers

**Tasks**:
1. Move actantial operations (Goals, Wants, Views) to use Request pattern
2. Move game time operations to use Request pattern
3. Move NPC mood operations to use Request pattern
4. Remove duplicate ClientMessage variants
5. Update all broadcast calls to use WorldConnectionManager

**Files to modify**:
- `websocket.rs` - remove old handlers
- `messages.rs` - remove old message types (breaking change OK)

**Checklist**:
- [ ] Remove `CreateNpcWant`, `UpdateNpcWant`, `DeleteNpcWant` from ClientMessage
- [ ] Remove `SetWantTarget`, `RemoveWantTarget` from ClientMessage
- [ ] Remove `AddActantialView`, `RemoveActantialView` from ClientMessage
- [ ] Remove `GetNpcActantialContext`, `GetWorldGoals` from ClientMessage
- [ ] Remove `CreateGoal`, `UpdateGoal`, `DeleteGoal` from ClientMessage
- [ ] Remove `AdvanceGameTime` from ClientMessage (use Request instead)
- [ ] Remove `SetNpcMood`, `SetNpcRelationship`, `GetNpcMoods` from ClientMessage
- [ ] Update corresponding ServerMessage types
- [ ] Update websocket.rs to remove old handlers
- [ ] Verify all operations work through Request pattern

### Day 11: Remove Session Concept

**Tasks**:
1. Rename SessionManager → WorldConnectionManager (if not already done)
2. Remove session-related code paths
3. Move conversation history to world-scoped storage
4. Update AsyncSessionPort → AsyncWorldPort (or remove if WorldConnectionManager handles all)
5. Update all session_id references to world_id

**Files to modify**:
- Remove `session_routes.rs`
- Update `session_join_service.rs` → merge into world join logic
- Update `AsyncSessionPort` or replace with `WorldConnectionManager`
- Update all services that reference sessions

**Checklist**:
- [ ] Remove JoinSession from ClientMessage
- [ ] Remove SessionJoined from ServerMessage
- [ ] Remove session_routes.rs
- [ ] Update or remove session_join_service.rs
- [ ] Update conversation history storage
- [ ] Search/replace session_id → world_id where appropriate
- [ ] Remove SessionManager if fully replaced
- [ ] Update AsyncSessionPort trait or remove

---

## Phase 4: Remove REST Endpoints (Days 12-13)

### Day 12: Delete Route Files

**Files to DELETE**:
```
crates/engine-adapters/src/infrastructure/http/
├── character_routes.rs          DELETE
├── location_routes.rs           DELETE
├── region_routes.rs             DELETE
├── scene_routes.rs              DELETE
├── act_routes.rs                DELETE (if exists)
├── challenge_routes.rs          DELETE
├── skill_routes.rs              DELETE
├── narrative_event_routes.rs    DELETE
├── event_chain_routes.rs        DELETE
├── story_event_routes.rs        DELETE
├── player_character_routes.rs   DELETE
├── session_routes.rs            DELETE
├── want_routes.rs               DELETE
├── goal_routes.rs               DELETE
├── game_time_routes.rs          DELETE (if exists)
├── observation_routes.rs        DELETE
├── relationship_routes.rs       DELETE (if exists)
├── social_network_routes.rs     DELETE (if exists)
├── spawn_points_routes.rs       DELETE (if exists)
└── interaction_routes.rs        DELETE
```

**Files to KEEP**:
```
crates/engine-adapters/src/infrastructure/http/
├── mod.rs                       MODIFY (remove deleted modules)
├── assets_routes.rs             KEEP (file uploads)
├── gallery_routes.rs            KEEP (if exists, for file operations)
├── health_routes.rs             KEEP (health checks)
├── world_routes.rs              MODIFY (keep only export endpoint)
├── rule_system_routes.rs        KEEP (static reference data)
├── settings_routes.rs           KEEP (configuration)
├── workflow_routes.rs           KEEP (ComfyUI configuration)
├── prompt_template_routes.rs    KEEP (configuration)
├── comfyui_routes.rs            KEEP (ComfyUI status/config)
├── generation_routes.rs         KEEP (generation queue)
└── suggest_routes.rs            KEEP (async suggestions)
```

**Checklist**:
- [ ] Delete character_routes.rs
- [ ] Delete location_routes.rs
- [ ] Delete region_routes.rs
- [ ] Delete scene_routes.rs
- [ ] Delete challenge_routes.rs
- [ ] Delete skill_routes.rs
- [ ] Delete narrative_event_routes.rs
- [ ] Delete event_chain_routes.rs
- [ ] Delete story_event_routes.rs
- [ ] Delete player_character_routes.rs
- [ ] Delete session_routes.rs
- [ ] Delete want_routes.rs
- [ ] Delete goal_routes.rs
- [ ] Delete observation_routes.rs
- [ ] Delete interaction_routes.rs
- [ ] Update http/mod.rs to remove deleted modules
- [ ] Update router configuration
- [ ] Verify remaining routes still work

### Day 13: Update Player-UI

**Tasks**:
1. Create WebSocket request client in player-adapters
2. Update all services to use WebSocket requests
3. Remove HTTP adapter code for migrated operations
4. Update game_state.rs for world-centric model

**Files to modify in player-adapters**:
- Create/update `websocket_client.rs` with request/response support

**Files to modify in player-app**:
- Update all services in `application/services/` to use WebSocket

**Files to modify in player-ui**:
- Update `game_state.rs` for world-centric model
- Update `session_message_handler.rs` for new messages
- Remove session-related UI if any

**Checklist**:
- [ ] Add request/response support to WebSocket client
- [ ] Update WorldService to use WebSocket
- [ ] Update CharacterService to use WebSocket
- [ ] Update LocationService to use WebSocket
- [ ] Update all other services to use WebSocket
- [ ] Remove HTTP adapters for migrated operations
- [ ] Update game_state.rs
- [ ] Update message handlers
- [ ] Test end-to-end

---

## Phase 5: Testing & Polish (Days 14-15)

### Day 14: Testing

**Unit Tests**:
- [ ] WorldConnectionManager tests
- [ ] AppRequestHandler tests
- [ ] Request/Response serialization tests

**Integration Tests**:
- [ ] JoinWorld flow
- [ ] CRUD operations via WebSocket
- [ ] Broadcast verification
- [ ] Multi-user scenarios
- [ ] Spectator mode
- [ ] DM multi-screen

**Manual Testing**:
- [ ] DM can create/edit all entities
- [ ] Player can perform actions
- [ ] Spectator sees updates
- [ ] Reconnection works
- [ ] Multiple worlds work

### Day 15: Documentation & Cleanup

**Tasks**:
- [ ] Run `cargo check --workspace`
- [ ] Run `cargo xtask arch-check`
- [ ] Fix all warnings
- [ ] Update WebSocket protocol documentation
- [ ] Update API documentation for remaining REST endpoints
- [ ] Archive/remove old documentation

---

## Files Summary

### New Files

| File | Purpose |
|------|---------|
| `protocol/src/requests.rs` | RequestPayload enum |
| `protocol/src/responses.rs` | ResponseResult enum |
| `engine-ports/src/inbound/request_handler.rs` | RequestHandler trait |
| `engine-app/src/application/handlers/mod.rs` | Handlers module |
| `engine-app/src/application/handlers/request_handler.rs` | AppRequestHandler impl |
| `engine-adapters/.../world_connection_manager.rs` | Connection tracking |

### Major Modifications

| File | Changes |
|------|---------|
| `protocol/src/messages.rs` | New message types |
| `protocol/src/lib.rs` | Export new modules |
| `engine-ports/src/inbound/mod.rs` | Export RequestHandler |
| `engine-app/src/application/mod.rs` | Export handlers |
| `engine-adapters/.../websocket.rs` | Integrate new handlers |
| `engine-adapters/.../state/mod.rs` | Add WorldConnectionManager |
| `engine-adapters/.../http/mod.rs` | Remove deleted routes |
| All player-app services | Use WebSocket instead of REST |
| `player-ui/.../game_state.rs` | World-centric model |

### Deleted Files

~20 REST route files.

---

## Success Criteria

- [ ] All CRUD operations work via WebSocket request/response
- [ ] All mutations broadcast EntityChanged to world
- [ ] DM can have multiple screens with synced state
- [ ] Player can play with real-time updates
- [ ] Spectator can view player-visible data
- [ ] No duplicate code between REST and WebSocket
- [ ] Only essential REST endpoints remain
- [ ] `cargo check --workspace` passes
- [ ] `cargo xtask arch-check` passes
- [ ] Basic test coverage for new components

---

## Progress Log

| Date | Phase | Task | Status | Notes |
|------|-------|------|--------|-------|
| 2025-12-26 | Plan | Create plan document | Done | |
| 2025-12-26 | Phase 1 | Day 1: Protocol Types | Done | requests.rs, responses.rs created |
| 2025-12-26 | Phase 1 | Day 2: RequestHandler + WorldConnectionManager | Done | trait + impl created |
| 2025-12-26 | Phase 1 | Day 3: AppRequestHandler | Done | ~53 handlers implemented |
| 2025-12-26 | Phase 1 | Day 4: WebSocket Integration | Done | Request pattern integrated |
| 2025-12-26 | Phase 2 | GameTime Migration | Done | GameTime moved to World entity, persisted to DB |
| 2025-12-26 | Phase 2 | Character Create/Update | Done | CreateCharacter, UpdateCharacter, ChangeArchetype handlers |
| 2025-12-26 | Phase 2 | Location Create/Update | Done | CreateLocation, UpdateLocation handlers |
| 2025-12-26 | Phase 2 | Skill Create/Update | Done | CreateSkill, UpdateSkill handlers |
| 2025-12-26 | Phase 2 | Scene Create/Update | Done | CreateScene, UpdateScene handlers |
| 2025-12-26 | Phase 2 | GameTime Handlers | Done | GetGameTime, AdvanceGameTime handlers |
| 2025-12-26 | Phase 2 | CreateAct Handler | Done | Uses WorldService::create_act() |
| 2025-12-26 | Phase 2 | Interaction Handlers | Done | CreateInteraction, UpdateInteraction |
| 2025-12-26 | Phase 2 | Challenge Handlers | Done | CreateChallenge, UpdateChallenge + parse_difficulty() helper |
| 2025-12-26 | Phase 2 | NarrativeEvent Handlers | Done | CreateNarrativeEvent, UpdateNarrativeEvent |
| 2025-12-26 | Phase 2 | EventChain Handlers | Done | CreateEventChain, UpdateEventChain |
| 2025-12-26 | Phase 2 | PlayerCharacter Handlers | Done | CreatePC (stub), UpdatePC, UpdatePCLocation |
| 2025-12-26 | Phase 2 | Relationship Handlers | Done | CreateRelationship + parse_relationship_type() helper |
| 2025-12-26 | Phase 2 | NPC Mood Handlers | Done | SetNpcMood, SetNpcRelationship + helper functions |
| 2025-12-26 | Phase 2 | ActantialView Handlers | Done | AddActantialView, RemoveActantialView |
| 2025-12-26 | Protocol Fix | RemoveActantialView | Done | Added target_type field to avoid error-as-flow-control |
| 2025-12-26 | Phase 2 | Goal Handlers | Done | ListGoals, GetGoal (stub), CreateGoal, UpdateGoal, DeleteGoal |
| 2025-12-26 | Phase 2 | Want Handlers | Done | ListWants, GetWant (stub), CreateWant, UpdateWant, DeleteWant |
| 2025-12-26 | Phase 2 | Want Target Handlers | Done | SetWantTarget, RemoveWantTarget + convert_want_visibility() |
| 2025-12-26 | Phase 2 | Location Connection Handlers | Done | CreateLocationConnection, DeleteLocationConnection |
| 2025-12-26 | Phase 2 | Region Handlers | Done | ListRegions, GetRegion (stub), CreateRegion, UpdateRegion (stub), DeleteRegion (stub), plus 6 connection/exit stubs |
| 2025-12-26 | Phase 2 | StoryEvent Handlers | Done | ListStoryEvents (stub), GetStoryEvent (stub), UpdateStoryEvent (stub), SetStoryEventVisibility (stub) |
| 2025-12-26 | Phase 2 | Character-Region Handlers | Done | ListCharacterRegionRelationships (stub), SetCharacterHomeRegion (stub), SetCharacterWorkRegion (stub), RemoveCharacterRegionRelationship (stub), ListRegionNpcs (stub) |
| 2025-12-26 | Phase 2 | CreateDmMarker Handler | Done | Stub handler for DM story markers |
| 2025-12-26 | Phase 2 | Observation Handlers | Done | ListObservations, CreateObservation, DeleteObservation (stubs) |
| 2025-12-26 | Phase 2 | ListSpawnPoints Handler | Done | Stub handler for spawn point regions |
| 2025-12-26 | Phase 2 | AI Suggestion Handlers | Done | SuggestDeflectionBehavior, SuggestBehavioralTells, SuggestWantDescription, SuggestActantialReason (stubs) |
| 2025-12-26 | Phase 2 | **ALL HANDLERS COMPLETE** | Done | **126 RequestPayload variants fully covered (2482 lines)** |
| 2025-12-26 | Phase 2 | Service Integration | Done | StoryEventService extracted to trait, ObservationRepo + RegionRepo wired to handler |

---

## Service Integration Summary (Completed 2025-12-26)

### Changes Made

#### 1. StoryEventService Trait Extraction
**File**: `crates/engine-app/src/application/services/story_event_service.rs`
- Created `trait StoryEventService` with all methods
- Created `struct StoryEventServiceImpl` implementing the trait
- Fixed `MarkerImportance::Normal` → `MarkerImportance::Minor`

#### 2. Services Updated to Use Arc<dyn StoryEventService>
| File | Status |
|------|--------|
| `dm_approval_queue_service.rs` | Done |
| `narrative_event_approval_service.rs` | Done |
| `staging_context_provider.rs` | Done |
| `staging_service.rs` | Done |
| `game_services.rs` | Done |

#### 3. RegionRepositoryPort Extended
**File**: `crates/engine-adapters/src/infrastructure/persistence/region_repository.rs`
- Added `update()` and `delete()` methods to trait
- Implemented both methods in `Neo4jRegionRepository`

#### 4. AppRequestHandler Updated
**File**: `crates/engine-app/src/application/handlers/request_handler.rs`
- Added 3 new fields:
  - `story_event_service: Arc<dyn StoryEventService>`
  - `observation_repo: Arc<dyn ObservationRepositoryPort>`
  - `region_repo: Arc<dyn RegionRepositoryPort>`
- Constructor now takes 16 arguments

#### 5. AppState Wiring Fixed
**File**: `crates/engine-adapters/src/infrastructure/state/mod.rs`
- Cloned `observation_repo` before `scene_resolution_service` consumes it
- Created `region_repo_for_handler`
- Fixed double-wrapping of `story_event_service` in `NarrativeEventApprovalService::new()`
- Updated `AppRequestHandler::new()` call with all 16 arguments

### Build Status
- `cargo check --workspace` - **PASSED** (warnings only)
- `cargo xtask arch-check` - **PASSED** (12 crates checked)
