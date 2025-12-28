# WebSocket Adapter Refactoring Plan

## Overview

This plan addresses architectural violations in the WebSocket layer and establishes guardrails to prevent future issues.

### Problems Identified

1. **Handler Coupling:** WebSocket handlers in `engine-adapters` contain 500-1000+ lines of business logic
2. **Platform Duplication:** Player adapters have ~600 lines of duplicated message construction
3. **Missing Use Cases:** No use case layer exists - handlers call services directly
4. **Missing Arch Enforcement:** No checks prevent handlers from containing business logic

### Solution Summary

1. Create **Use Cases** in `engine-app` for complex workflows
2. Create **BroadcastPort** for notification side-effects
3. Create **ClientMessageBuilder** to deduplicate player adapters
4. Add **arch-check rules** to prevent future violations

---

## Current State Analysis

### Engine Handlers: Classification Summary

| File | Lines | Class | Priority | Issue |
|------|-------|-------|----------|-------|
| `movement.rs` | 964 | HEAVY | **HIGH** | Full staging workflow embedded |
| `staging.rs` | 629 | HEAVY | **HIGH** | Scene building, NPC notification |
| `player_action.rs` | 448 | HEAVY | **HIGH** | 405-line function with travel logic |
| `inventory.rs` | 518 | MIXED | **MEDIUM** | Rollback logic, direct repo calls |
| `challenge.rs` | 817 | MIXED | **MEDIUM** | DM auth repeated 11x |
| `misc.rs` | 465 | MIXED | **MEDIUM** | Observation creation embedded |
| `scene.rs` | 373 | MIXED | **LOW** | Scene loading, acceptable complexity |
| `connection.rs` | 314 | MIXED | **LOW** | Join world lifecycle-specific |
| `narrative.rs` | 84 | THIN | **NONE** | Clean delegation |
| `request.rs` | 81 | THIN | **NONE** | Perfect pattern |

### Cross-Cutting Issues

1. **Connection context boilerplate** - Same 15-20 lines repeated **17+ times**
2. **DM authorization check** - Repeated **10+ times**
3. **SceneChanged building** - Duplicated in movement.rs and staging.rs

### Player Adapters: Duplication

| File | Lines | Unique Logic |
|------|-------|--------------|
| `wasm/adapter.rs` | 354 | SendWrapper, Rc<RefCell> |
| `desktop/adapter.rs` | 496 | tokio::spawn wrapping |
| **Shared logic** | ~40 methods | Message construction (identical) |

---

## Target Architecture

### Hexagonal Layer Separation

```
+---------------------------------------------------------------------+
|                      ADAPTER LAYER (WebSocket)                       |
|  Responsibilities:                                                   |
|    - Parse/validate incoming messages                                |
|    - Extract connection context                                      |
|    - Call use cases                                                  |
|    - Convert domain results to ServerMessage                         |
|    - Send responses via infrastructure                               |
+---------------------------------------------------------------------+
                                    |
                                    v
+---------------------------------------------------------------------+
|                   APPLICATION LAYER (Use Cases)                      |
|  Responsibilities:                                                   |
|    - Orchestrate domain logic                                        |
|    - Return domain results (NOT ServerMessage)                       |
|    - Use ports for side-effect notifications                         |
|    - Transaction boundaries                                          |
+---------------------------------------------------------------------+
                                    |
                                    v
+---------------------------------------------------------------------+
|                         PORTS LAYER                                  |
|  New Outbound Ports:                                                 |
|    - BroadcastPort: notify_staging_required, send_scene_changed      |
|  Domain Events:                                                      |
|    - StagingRequiredEvent, SceneChangedEvent, StagingReadyEvent      |
+---------------------------------------------------------------------+
```

---

## Phase 1: Infrastructure Helpers (2-3 hours)

### 1.1 Create Handler Context Extraction

**File:** `crates/engine-adapters/src/infrastructure/websocket/context.rs`

**Purpose:** Eliminate ~300 lines of boilerplate across 10 handler files.

**Content:**

```rust
//! WebSocket handler context extraction
//!
//! Provides a unified way to extract connection context and perform
//! authorization checks, eliminating boilerplate across handlers.

use uuid::Uuid;
use wrldbldr_domain::{WorldId, PlayerCharacterId};
use wrldbldr_protocol::ServerMessage;
use crate::infrastructure::state::AppState;

/// Extracted context for WebSocket handlers
pub struct HandlerContext {
    pub connection_id: String,
    pub world_id: WorldId,
    pub world_id_uuid: Uuid,
    pub user_id: String,
    pub is_dm: bool,
    pub pc_id: Option<PlayerCharacterId>,
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
            .ok_or_else(not_connected_error)?;

        let world_id_uuid = connection.world_id.ok_or_else(no_world_error)?;
        let world_id = WorldId::from_uuid(world_id_uuid);

        let pc_id = connection.pc_id.map(PlayerCharacterId::from_uuid);

        Ok(Self {
            connection_id: client_id_str,
            world_id,
            world_id_uuid,
            user_id: connection.user_id.clone(),
            is_dm: connection.is_dm(),
            pc_id,
        })
    }

    /// Require DM authorization
    pub fn require_dm(&self) -> Result<(), ServerMessage> {
        if self.is_dm {
            Ok(())
        } else {
            Err(not_authorized_error("Only the DM can perform this action"))
        }
    }

    /// Require player authorization (not DM, not spectator)
    pub fn require_player(&self) -> Result<(), ServerMessage> {
        if !self.is_dm && self.pc_id.is_some() {
            Ok(())
        } else {
            Err(not_authorized_error("Only players can perform this action"))
        }
    }
}

/// Error response helpers
pub fn not_connected_error() -> ServerMessage {
    ServerMessage::Error {
        code: "NOT_CONNECTED".to_string(),
        message: "Client is not connected".to_string(),
    }
}

pub fn no_world_error() -> ServerMessage {
    ServerMessage::Error {
        code: "NO_WORLD".to_string(),
        message: "Not connected to a world".to_string(),
    }
}

pub fn not_authorized_error(message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: "NOT_AUTHORIZED".to_string(),
        message: message.to_string(),
    }
}

pub fn invalid_id_error(entity: &str, id: &str) -> ServerMessage {
    ServerMessage::Error {
        code: format!("INVALID_{}_ID", entity.to_uppercase()),
        message: format!("Invalid {} ID: {}", entity, id),
    }
}

/// Parse a UUID string into a domain ID type, returning ServerMessage error on failure
pub fn parse_uuid(id: &str, entity: &str) -> Result<Uuid, ServerMessage> {
    Uuid::parse_str(id).map_err(|_| invalid_id_error(entity, id))
}

pub fn parse_player_character_id(id: &str) -> Result<PlayerCharacterId, ServerMessage> {
    parse_uuid(id, "PC").map(PlayerCharacterId::from_uuid)
}

pub fn parse_region_id(id: &str) -> Result<wrldbldr_domain::RegionId, ServerMessage> {
    parse_uuid(id, "region").map(wrldbldr_domain::RegionId::from_uuid)
}

pub fn parse_location_id(id: &str) -> Result<wrldbldr_domain::LocationId, ServerMessage> {
    parse_uuid(id, "location").map(wrldbldr_domain::LocationId::from_uuid)
}

pub fn parse_character_id(id: &str) -> Result<wrldbldr_domain::CharacterId, ServerMessage> {
    parse_uuid(id, "character").map(wrldbldr_domain::CharacterId::from_uuid)
}
```

### 1.2 Update WebSocket Module Exports

**File:** `crates/engine-adapters/src/infrastructure/websocket/mod.rs` (modify)

Add:
```rust
pub mod context;
pub use context::*;
```

---

## Phase 2: Ports & Domain Events (3-4 hours)

### 2.1 Create BroadcastPort

**File:** `crates/engine-ports/src/outbound/broadcast_port.rs`

```rust
//! Broadcast Port - Outbound port for game event notifications
//!
//! This port abstracts the notification of game events to connected clients,
//! allowing use cases to trigger notifications without depending on WebSocket
//! infrastructure.

use async_trait::async_trait;
use wrldbldr_domain::WorldId;

use super::broadcast_events::*;

/// Port for broadcasting game events to connected clients
///
/// Implementations convert domain events to protocol messages and dispatch
/// via the appropriate transport mechanism.
#[async_trait]
pub trait BroadcastPort: Send + Sync {
    // === Staging Notifications ===

    /// Notify DMs that staging approval is required for a region
    async fn notify_staging_required(&self, world_id: WorldId, event: StagingRequiredEvent);

    /// Notify that staging is ready (sent to DMs and waiting players)
    async fn notify_staging_ready(&self, world_id: WorldId, event: StagingReadyEvent);

    /// Send staging pending notification to a specific player
    async fn send_staging_pending(&self, world_id: WorldId, user_id: &str, event: StagingPendingEvent);

    // === Scene Notifications ===

    /// Send scene changed to a specific user
    async fn send_scene_changed(&self, world_id: WorldId, user_id: &str, event: SceneChangedEvent);

    // === General Notifications ===

    /// Send a notification to a specific user in a world
    async fn send_to_user(&self, world_id: WorldId, user_id: &str, notification: UserNotification);

    /// Broadcast a notification to all DMs in a world
    async fn broadcast_to_dms(&self, world_id: WorldId, notification: DmNotification);

    /// Broadcast a notification to all players in a world
    async fn broadcast_to_players(&self, world_id: WorldId, notification: PlayerNotification);
}
```

### 2.2 Create Domain Events

**File:** `crates/engine-ports/src/outbound/broadcast_events.rs`

```rust
//! Domain events for broadcast notifications
//!
//! These are transport-agnostic event types used by use cases to request
//! notifications. The adapter layer converts these to ServerMessage.

use wrldbldr_domain::{
    CharacterId, GameTime, LocationId, PlayerCharacterId, RegionId, WorldId,
};

// =============================================================================
// Staging Events
// =============================================================================

/// Event indicating DM approval is required for NPC staging
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

/// Event indicating staging is ready for a region
#[derive(Debug, Clone)]
pub struct StagingReadyEvent {
    pub region_id: RegionId,
    pub npcs_present: Vec<NpcPresenceData>,
    /// Players who were waiting for this staging
    pub waiting_pcs: Vec<WaitingPcData>,
}

/// Event indicating a player is waiting for staging
#[derive(Debug, Clone)]
pub struct StagingPendingEvent {
    pub region_id: RegionId,
    pub region_name: String,
}

/// Data for a staged NPC in approval flow
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

/// Data for a PC waiting for staging
#[derive(Debug, Clone)]
pub struct WaitingPcData {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub user_id: String,
}

/// Data about previous staging for DM reference
#[derive(Debug, Clone)]
pub struct PreviousStagingData {
    pub staging_id: String,
    pub approved_at: String,
    pub npcs: Vec<StagedNpcData>,
}

// =============================================================================
// Scene Events
// =============================================================================

/// Event indicating scene has changed for a player
#[derive(Debug, Clone)]
pub struct SceneChangedEvent {
    pub pc_id: PlayerCharacterId,
    pub region: RegionInfo,
    pub npcs_present: Vec<NpcPresenceData>,
    pub navigation: NavigationInfo,
    pub region_items: Vec<RegionItemData>,
}

/// Region information for scene events
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

/// NPC presence data for scenes
#[derive(Debug, Clone)]
pub struct NpcPresenceData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Navigation options from current region
#[derive(Debug, Clone)]
pub struct NavigationInfo {
    pub connected_regions: Vec<NavigationTarget>,
    pub exits: Vec<NavigationExit>,
}

/// A connected region that can be navigated to
#[derive(Debug, Clone)]
pub struct NavigationTarget {
    pub region_id: RegionId,
    pub name: String,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// An exit to another location
#[derive(Debug, Clone)]
pub struct NavigationExit {
    pub location_id: LocationId,
    pub location_name: String,
    pub arrival_region_id: RegionId,
    pub description: Option<String>,
}

/// An item present in a region
#[derive(Debug, Clone)]
pub struct RegionItemData {
    pub item_id: String,
    pub name: String,
    pub description: Option<String>,
    pub quantity: u32,
}

// =============================================================================
// Notification Enums
// =============================================================================

/// Notifications sent to DMs
#[derive(Debug, Clone)]
pub enum DmNotification {
    StagingRequired(StagingRequiredEvent),
    StagingReady(StagingReadyEvent),
    SplitParty(SplitPartyEvent),
    PlayerJoined { user_id: String, pc_name: Option<String> },
    PlayerLeft { user_id: String },
}

/// Notifications sent to specific users
#[derive(Debug, Clone)]
pub enum UserNotification {
    SceneChanged(SceneChangedEvent),
    StagingPending(StagingPendingEvent),
    StagingReady(StagingReadyEvent),
    MovementBlocked { pc_id: PlayerCharacterId, reason: String },
}

/// Notifications broadcast to all players
#[derive(Debug, Clone)]
pub enum PlayerNotification {
    GameTimeUpdated(GameTime),
    WorldEvent { message: String },
}

/// Split party notification for DM
#[derive(Debug, Clone)]
pub struct SplitPartyEvent {
    pub location_groups: Vec<LocationGroup>,
}

/// A group of PCs at the same location
#[derive(Debug, Clone)]
pub struct LocationGroup {
    pub location_id: LocationId,
    pub location_name: String,
    pub pcs: Vec<PcLocationData>,
}

/// PC location data for split party
#[derive(Debug, Clone)]
pub struct PcLocationData {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub region_id: Option<RegionId>,
    pub region_name: Option<String>,
}
```

### 2.3 Update Port Exports

**File:** `crates/engine-ports/src/outbound/mod.rs` (modify)

Add:
```rust
mod broadcast_port;
mod broadcast_events;

pub use broadcast_port::*;
pub use broadcast_events::*;
```

---

## Phase 3: Use Cases (12-15 hours)

### 3.1 Create Use Case Module Structure

**File:** `crates/engine-app/src/application/use_cases/mod.rs`

```rust
//! Use Cases - Application layer orchestration
//!
//! Use cases coordinate domain services to fulfill specific user intents.
//! They are transport-agnostic and return domain results, not protocol messages.
//!
//! # Guidelines
//!
//! - Use cases must NOT import `wrldbldr_protocol::ServerMessage`
//! - Use cases return domain result types (enums, structs)
//! - Use cases use `BroadcastPort` for side-effect notifications
//! - Use cases are the transaction boundary
//!
//! # Handler Pattern
//!
//! Handlers should call use cases like this:
//!
//! ```rust,ignore
//! let ctx = HandlerContext::extract(state, client_id).await?;
//! match state.my_use_case.do_something(ctx.into(), input).await {
//!     Ok(result) => Some(result.into_server_message()),
//!     Err(e) => Some(e.into_server_message()),
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

pub use errors::*;
pub use movement::*;
pub use staging::*;
pub use player_action::*;
pub use inventory::*;
pub use observation::*;
pub use challenge::*;
pub use scene::*;
pub use connection::*;
```

### 3.2 Create Error Types

**File:** `crates/engine-app/src/application/use_cases/errors.rs`

```rust
//! Use case error types
//!
//! Each use case has its own error type that can be converted to an error code
//! for protocol responses.

use thiserror::Error;
use wrldbldr_domain::{
    CharacterId, LocationId, PlayerCharacterId, RegionId, WorldId,
};

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

impl MovementError {
    pub fn error_code(&self) -> &'static str {
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

    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),

    #[error("Character not found: {0}")]
    CharacterNotFound(CharacterId),

    #[error("Staging approval failed: {0}")]
    ApprovalFailed(String),

    #[error("Regeneration failed: {0}")]
    RegenerationFailed(String),

    #[error("Database error: {0}")]
    Database(String),
}

impl StagingError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::PendingNotFound(_) => "STAGING_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::LocationNotFound(_) => "LOCATION_NOT_FOUND",
            Self::CharacterNotFound(_) => "CHARACTER_NOT_FOUND",
            Self::ApprovalFailed(_) => "STAGING_APPROVAL_FAILED",
            Self::RegenerationFailed(_) => "REGENERATION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Action Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Invalid action type: {0}")]
    InvalidActionType(String),

    #[error("Target not found: {0}")]
    TargetNotFound(String),

    #[error("Scene not found")]
    SceneNotFound,

    #[error("Database error: {0}")]
    Database(String),
}

impl ActionError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::InvalidActionType(_) => "INVALID_ACTION",
            Self::TargetNotFound(_) => "TARGET_NOT_FOUND",
            Self::SceneNotFound => "SCENE_NOT_FOUND",
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

impl InventoryError {
    pub fn error_code(&self) -> &'static str {
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
// Observation Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("NPC not found: {0}")]
    NpcNotFound(CharacterId),

    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    #[error("Database error: {0}")]
    Database(String),
}

impl ObservationError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NpcNotFound(_) => "NPC_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
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

    #[error("Resolution not found: {0}")]
    ResolutionNotFound(String),

    #[error("Invalid roll value")]
    InvalidRoll,

    #[error("Challenge already resolved")]
    AlreadyResolved,

    #[error("Not authorized")]
    NotAuthorized,

    #[error("Service error: {0}")]
    Service(String),
}

impl ChallengeError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::ChallengeNotFound(_) => "CHALLENGE_NOT_FOUND",
            Self::ResolutionNotFound(_) => "RESOLUTION_NOT_FOUND",
            Self::InvalidRoll => "INVALID_ROLL",
            Self::AlreadyResolved => "ALREADY_RESOLVED",
            Self::NotAuthorized => "NOT_AUTHORIZED",
            Self::Service(_) => "SERVICE_ERROR",
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

    #[error("World not found: {0}")]
    WorldNotFound(WorldId),

    #[error("Database error: {0}")]
    Database(String),
}

impl SceneError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::SceneNotFound(_) => "SCENE_NOT_FOUND",
            Self::WorldNotFound(_) => "WORLD_NOT_FOUND",
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

    #[error("Not connected to a world")]
    NotConnected,

    #[error("Database error: {0}")]
    Database(String),
}

impl ConnectionError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::WorldNotFound(_) => "WORLD_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::AlreadyConnected => "ALREADY_CONNECTED",
            Self::NotConnected => "NOT_CONNECTED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Scene Builder Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum SceneBuilderError {
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),

    #[error("Database error: {0}")]
    Database(String),
}
```

### 3.3 Create MovementUseCase

**File:** `crates/engine-app/src/application/use_cases/movement.rs`

**Estimated lines:** 350

This use case extracts business logic from `handlers/movement.rs` (964 lines).

**Key methods:**
- `select_character(pc_id) -> Result<PcSelectedResult, MovementError>`
- `move_to_region(ctx, pc_id, region_id) -> Result<MovementResult, MovementError>`
- `exit_to_location(ctx, pc_id, location_id, arrival_region) -> Result<MovementResult, MovementError>`

**Result types:**
```rust
pub enum MovementResult {
    SceneChanged(SceneChangedEvent),
    StagingPending { region_id: RegionId, region_name: String },
    Blocked { reason: String },
}

pub struct PcSelectedResult {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub location_id: LocationId,
    pub region_id: Option<RegionId>,
}
```

**Dependencies:**
- `PlayerCharacterRepositoryPort`
- `RegionRepositoryPort`
- `LocationRepositoryPort`
- `StagingService`
- `WorldStateManager`
- `BroadcastPort`
- `SceneBuilder`

### 3.4 Create StagingApprovalUseCase

**File:** `crates/engine-app/src/application/use_cases/staging.rs`

**Estimated lines:** 300

This use case extracts business logic from `handlers/staging.rs` (629 lines).

**Key methods:**
- `approve_staging(ctx, request_id, approved_npcs, ttl_hours, source) -> Result<(), StagingError>`
- `regenerate_suggestions(ctx, request_id, guidance) -> Result<RegeneratedStagingResult, StagingError>`
- `pre_stage_region(ctx, region_id, npcs, ttl_hours) -> Result<(), StagingError>`

**Result types:**
```rust
pub struct RegeneratedStagingResult {
    pub request_id: String,
    pub llm_based_npcs: Vec<StagedNpcData>,
}

pub struct ApprovedNpcInput {
    pub character_id: CharacterId,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: Option<String>,
}
```

### 3.5 Create PlayerActionUseCase

**File:** `crates/engine-app/src/application/use_cases/player_action.rs`

**Estimated lines:** 250

This use case extracts business logic from `handlers/player_action.rs` (448 lines).

**Key methods:**
- `handle_action(ctx, action_type, target, dialogue) -> Result<ActionResult, ActionError>`

**Result types:**
```rust
pub enum ActionResult {
    /// Travel completed, scene changed
    TravelCompleted(SceneChangedEvent),
    /// Action queued for processing
    Queued { action_id: String },
    /// Action requires DM approval
    PendingApproval { request_id: String },
}
```

### 3.6 Create InventoryUseCase

**File:** `crates/engine-app/src/application/use_cases/inventory.rs`

**Estimated lines:** 200

This use case extracts business logic from `handlers/inventory.rs` (518 lines).

**Key methods:**
- `equip_item(ctx, pc_id, item_id) -> Result<EquipResult, InventoryError>`
- `unequip_item(ctx, pc_id, item_id) -> Result<UnequipResult, InventoryError>`
- `drop_item(ctx, pc_id, item_id, quantity) -> Result<DropResult, InventoryError>`
- `pickup_item(ctx, pc_id, item_id) -> Result<PickupResult, InventoryError>`

**Result types:**
```rust
pub struct EquipResult {
    pub pc_id: PlayerCharacterId,
    pub item_id: String,
    pub item_name: String,
}

pub struct DropResult {
    pub pc_id: PlayerCharacterId,
    pub item_id: String,
    pub quantity: u32,
    pub region_id: RegionId,
}

pub struct PickupResult {
    pub pc_id: PlayerCharacterId,
    pub item_id: String,
    pub item_name: String,
}
```

### 3.7 Create ObservationUseCase

**File:** `crates/engine-app/src/application/use_cases/observation.rs`

**Estimated lines:** 150

This use case extracts business logic from `handlers/misc.rs` (share_npc_location, trigger_approach).

**Key methods:**
- `share_npc_location(ctx, npc_id, pc_id) -> Result<(), ObservationError>`
- `trigger_approach_event(ctx, npc_id, pc_id, is_hidden) -> Result<ApproachResult, ObservationError>`

### 3.8 Create ChallengeUseCase

**File:** `crates/engine-app/src/application/use_cases/challenge.rs`

**Estimated lines:** 200

This use case wraps authorization for challenge operations (most logic already in services).

**Key methods:**
- `submit_roll(ctx, challenge_id, roll) -> Result<RollResult, ChallengeError>`
- `submit_roll_input(ctx, challenge_id, input) -> Result<RollResult, ChallengeError>`
- `trigger_challenge(ctx, challenge_id, target_id) -> Result<TriggerResult, ChallengeError>`
- `approve_outcome(ctx, resolution_id, decision) -> Result<(), ChallengeError>`
- `regenerate_outcome(ctx, resolution_id) -> Result<(), ChallengeError>`
- `discard_challenge(ctx, challenge_id) -> Result<(), ChallengeError>`
- `create_adhoc_challenge(ctx, input) -> Result<AdHocResult, ChallengeError>`

### 3.9 Create SceneUseCase

**File:** `crates/engine-app/src/application/use_cases/scene.rs`

**Estimated lines:** 150

This use case extracts business logic from `handlers/scene.rs`.

**Key methods:**
- `request_scene_change(ctx, scene_id) -> Result<SceneResult, SceneError>`
- `update_directorial_context(ctx, context) -> Result<(), SceneError>`
- `handle_approval_decision(ctx, request_id, decision) -> Result<(), SceneError>`

### 3.10 Create ConnectionUseCase

**File:** `crates/engine-app/src/application/use_cases/connection.rs`

**Estimated lines:** 180

This use case extracts business logic from `handlers/connection.rs`.

**Key methods:**
- `join_world(ctx, world_id, user_id, role) -> Result<JoinResult, ConnectionError>`
- `leave_world(ctx) -> Result<(), ConnectionError>`
- `set_spectate_target(ctx, target_pc_id) -> Result<(), ConnectionError>`

### 3.11 Create SceneBuilder Utility

**File:** `crates/engine-app/src/application/services/scene_builder.rs`

**Estimated lines:** 200

Shared utility for building scene data, used by MovementUseCase and StagingApprovalUseCase.

```rust
pub struct SceneBuilder {
    region_repo: Arc<dyn RegionRepositoryPort>,
    location_repo: Arc<dyn LocationRepositoryPort>,
}

impl SceneBuilder {
    /// Build a complete scene for a region
    pub async fn build_scene(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
        staged_npcs: &[StagedNpc],
    ) -> Result<SceneChangedEvent, SceneBuilderError>;

    /// Build navigation data for a region
    pub async fn build_navigation(&self, region_id: RegionId) -> NavigationInfo;

    /// Fetch items present in a region
    pub async fn fetch_region_items(&self, region_id: RegionId) -> Vec<RegionItemData>;
}
```

### 3.12 Update Application Module Exports

**File:** `crates/engine-app/src/application/mod.rs` (modify)

Add:
```rust
pub mod use_cases;
```

**File:** `crates/engine-app/src/lib.rs` (modify)

Ensure use_cases are exported.

---

## Phase 4: Adapter Implementation (5-6 hours)

### 4.1 Create BroadcastPort Implementation

**File:** `crates/engine-adapters/src/infrastructure/websocket/broadcast_adapter.rs`

**Estimated lines:** 200

```rust
//! WebSocket BroadcastPort implementation
//!
//! Converts domain events to ServerMessage and dispatches via WorldConnectionManager.

use std::sync::Arc;
use async_trait::async_trait;

use wrldbldr_engine_ports::outbound::{
    BroadcastPort, DmNotification, PlayerNotification, SceneChangedEvent,
    StagingPendingEvent, StagingReadyEvent, StagingRequiredEvent, UserNotification,
};
use wrldbldr_domain::WorldId;
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::world_connection_manager::WorldConnectionManager;

pub struct WebSocketBroadcastAdapter {
    connection_manager: Arc<WorldConnectionManager>,
}

impl WebSocketBroadcastAdapter {
    pub fn new(connection_manager: Arc<WorldConnectionManager>) -> Self {
        Self { connection_manager }
    }

    // Convert domain events to ServerMessage
    fn staging_required_to_message(event: &StagingRequiredEvent) -> ServerMessage {
        ServerMessage::StagingApprovalRequired {
            request_id: event.request_id.clone(),
            region_id: event.region_id.to_string(),
            region_name: event.region_name.clone(),
            location_id: event.location_id.to_string(),
            location_name: event.location_name.clone(),
            // ... convert all fields
        }
    }

    fn scene_changed_to_message(event: &SceneChangedEvent) -> ServerMessage {
        ServerMessage::SceneChanged {
            pc_id: event.pc_id.to_string(),
            // ... convert all fields
        }
    }

    // ... other conversion methods
}

#[async_trait]
impl BroadcastPort for WebSocketBroadcastAdapter {
    async fn notify_staging_required(&self, world_id: WorldId, event: StagingRequiredEvent) {
        let msg = Self::staging_required_to_message(&event);
        self.connection_manager
            .broadcast_to_dms(world_id.into(), msg)
            .await;
    }

    async fn notify_staging_ready(&self, world_id: WorldId, event: StagingReadyEvent) {
        let msg = ServerMessage::StagingReady {
            region_id: event.region_id.to_string(),
            npcs_present: event.npcs_present.iter().map(/* convert */).collect(),
        };

        // Send to DMs
        self.connection_manager
            .broadcast_to_dms(world_id.into(), msg.clone())
            .await;

        // Send to each waiting PC
        for pc in &event.waiting_pcs {
            self.connection_manager
                .send_to_user_in_world(&world_id.into(), &pc.user_id, msg.clone())
                .await;
        }
    }

    async fn send_staging_pending(&self, world_id: WorldId, user_id: &str, event: StagingPendingEvent) {
        let msg = ServerMessage::StagingPending {
            region_id: event.region_id.to_string(),
            region_name: event.region_name,
        };
        self.connection_manager
            .send_to_user_in_world(&world_id.into(), user_id, msg)
            .await;
    }

    async fn send_scene_changed(&self, world_id: WorldId, user_id: &str, event: SceneChangedEvent) {
        let msg = Self::scene_changed_to_message(&event);
        self.connection_manager
            .send_to_user_in_world(&world_id.into(), user_id, msg)
            .await;
    }

    async fn send_to_user(&self, world_id: WorldId, user_id: &str, notification: UserNotification) {
        let msg = match notification {
            UserNotification::SceneChanged(e) => Self::scene_changed_to_message(&e),
            UserNotification::StagingPending(e) => ServerMessage::StagingPending {
                region_id: e.region_id.to_string(),
                region_name: e.region_name,
            },
            // ... handle other variants
        };
        self.connection_manager
            .send_to_user_in_world(&world_id.into(), user_id, msg)
            .await;
    }

    async fn broadcast_to_dms(&self, world_id: WorldId, notification: DmNotification) {
        let msg = match notification {
            DmNotification::StagingRequired(e) => Self::staging_required_to_message(&e),
            // ... handle other variants
        };
        self.connection_manager
            .broadcast_to_dms(world_id.into(), msg)
            .await;
    }

    async fn broadcast_to_players(&self, world_id: WorldId, notification: PlayerNotification) {
        let msg = match notification {
            // ... handle variants
        };
        self.connection_manager
            .broadcast_to_players(world_id.into(), msg)
            .await;
    }
}
```

### 4.2 Update WebSocket Module Exports

**File:** `crates/engine-adapters/src/infrastructure/websocket/mod.rs` (modify)

Add:
```rust
pub mod broadcast_adapter;
pub use broadcast_adapter::*;
```

### 4.3 Wire Use Cases into AppState

**File:** `crates/engine-adapters/src/infrastructure/state.rs` (modify)

Add use case instances to AppState and initialize them in `AppState::new()`.

### 4.4 Refactor Handlers

Each handler file should be reduced to a thin routing layer.

**Example: movement.rs (after refactoring)**

```rust
//! Movement handlers - thin routing layer
//!
//! These handlers extract context, call use cases, and convert results.

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::context::{
    HandlerContext, parse_player_character_id, parse_region_id, parse_location_id,
};
use wrldbldr_protocol::ServerMessage;

pub async fn handle_select_player_character(
    state: &AppState,
    _client_id: Uuid,
    pc_id: String,
) -> Option<ServerMessage> {
    let pc_uuid = match parse_player_character_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    match state.movement_use_case.select_character(pc_uuid).await {
        Ok(result) => Some(ServerMessage::PcSelected {
            pc_id: result.pc_id.to_string(),
            pc_name: result.pc_name,
            location_id: result.location_id.to_string(),
            region_id: result.region_id.map(|r| r.to_string()),
        }),
        Err(e) => Some(ServerMessage::Error {
            code: e.error_code().to_string(),
            message: e.to_string(),
        }),
    }
}

pub async fn handle_move_to_region(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    region_id: String,
    _sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    let ctx = match HandlerContext::extract(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let pc_uuid = match parse_player_character_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    match state.movement_use_case.move_to_region(ctx.into(), pc_uuid, region_uuid).await {
        Ok(result) => Some(result.into_server_message(&pc_id)),
        Err(e) => Some(ServerMessage::Error {
            code: e.error_code().to_string(),
            message: e.to_string(),
        }),
    }
}

pub async fn handle_exit_to_location(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    location_id: String,
    arrival_region_id: Option<String>,
    _sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    let ctx = match HandlerContext::extract(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let pc_uuid = match parse_player_character_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let location_uuid = match parse_location_id(&location_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let arrival_region = match arrival_region_id {
        Some(ref id) => match parse_region_id(id) {
            Ok(r) => Some(r),
            Err(e) => return Some(e),
        },
        None => None,
    };

    match state.movement_use_case
        .exit_to_location(ctx.into(), pc_uuid, location_uuid, arrival_region)
        .await
    {
        Ok(result) => Some(result.into_server_message(&pc_id)),
        Err(e) => Some(ServerMessage::Error {
            code: e.error_code().to_string(),
            message: e.to_string(),
        }),
    }
}
```

**Handler refactoring targets:**

| File | Before | After Target |
|------|--------|--------------|
| `movement.rs` | 964 | ~100 |
| `staging.rs` | 629 | ~80 |
| `player_action.rs` | 448 | ~50 |
| `inventory.rs` | 518 | ~100 |
| `misc.rs` | 465 | ~150 |
| `challenge.rs` | 817 | ~200 |
| `scene.rs` | 373 | ~80 |
| `connection.rs` | 314 | ~80 |

---

## Phase 5: Player Adapter Deduplication (4-5 hours)

### 5.1 Create ClientMessageBuilder

**File:** `crates/player-adapters/src/infrastructure/websocket/message_builder.rs`

**Estimated lines:** 350

```rust
//! ClientMessage construction utilities
//!
//! Centralizes all message construction logic, eliminating duplication
//! between WASM and Desktop adapters.

use wrldbldr_protocol::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecisionData,
    ClientMessage, DiceInputType, DirectorialContext, ParticipantRole, RequestPayload,
};

/// Builder for ClientMessage variants
///
/// All message construction is centralized here to eliminate duplication
/// between WASM and Desktop adapters.
pub struct ClientMessageBuilder;

impl ClientMessageBuilder {
    // =========================================================================
    // Connection Messages
    // =========================================================================

    pub fn join_world(world_id: &str, user_id: &str, role: ParticipantRole) -> ClientMessage {
        ClientMessage::JoinWorld {
            world_id: world_id.to_string(),
            user_id: user_id.to_string(),
            role,
        }
    }

    pub fn leave_world() -> ClientMessage {
        ClientMessage::LeaveWorld
    }

    pub fn heartbeat() -> ClientMessage {
        ClientMessage::Heartbeat
    }

    pub fn set_spectate_target(pc_id: Option<&str>) -> ClientMessage {
        ClientMessage::SetSpectateTarget {
            pc_id: pc_id.map(|s| s.to_string()),
        }
    }

    pub fn select_player_character(pc_id: &str) -> ClientMessage {
        ClientMessage::SelectPlayerCharacter {
            pc_id: pc_id.to_string(),
        }
    }

    // =========================================================================
    // Movement Messages
    // =========================================================================

    pub fn move_to_region(pc_id: &str, region_id: &str) -> ClientMessage {
        ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        }
    }

    pub fn exit_to_location(
        pc_id: &str,
        location_id: &str,
        arrival_region_id: Option<&str>,
    ) -> ClientMessage {
        ClientMessage::ExitToLocation {
            pc_id: pc_id.to_string(),
            location_id: location_id.to_string(),
            arrival_region_id: arrival_region_id.map(|s| s.to_string()),
        }
    }

    // =========================================================================
    // Staging Messages
    // =========================================================================

    pub fn staging_approval(
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> ClientMessage {
        ClientMessage::StagingApprovalResponse {
            request_id: request_id.to_string(),
            approved_npcs,
            ttl_hours,
            source: source.to_string(),
        }
    }

    pub fn staging_regenerate(request_id: &str, guidance: &str) -> ClientMessage {
        ClientMessage::StagingRegenerateRequest {
            request_id: request_id.to_string(),
            guidance: guidance.to_string(),
        }
    }

    pub fn pre_stage_region(
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> ClientMessage {
        ClientMessage::PreStageRegion {
            region_id: region_id.to_string(),
            npcs,
            ttl_hours,
        }
    }

    // =========================================================================
    // Challenge Messages
    // =========================================================================

    pub fn challenge_roll(challenge_id: &str, roll: i32) -> ClientMessage {
        ClientMessage::ChallengeRoll {
            challenge_id: challenge_id.to_string(),
            roll,
        }
    }

    pub fn challenge_roll_input(challenge_id: &str, input: DiceInputType) -> ClientMessage {
        ClientMessage::ChallengeRollInput {
            challenge_id: challenge_id.to_string(),
            input_type: input,
        }
    }

    pub fn trigger_challenge(challenge_id: &str, target_character_id: &str) -> ClientMessage {
        ClientMessage::TriggerChallenge {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        }
    }

    pub fn challenge_outcome_decision(
        resolution_id: &str,
        decision: ChallengeOutcomeDecisionData,
    ) -> ClientMessage {
        ClientMessage::ChallengeOutcomeDecision {
            resolution_id: resolution_id.to_string(),
            decision,
        }
    }

    pub fn create_adhoc_challenge(
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> ClientMessage {
        ClientMessage::CreateAdHocChallenge {
            challenge_name: challenge_name.to_string(),
            skill_name: skill_name.to_string(),
            difficulty: difficulty.to_string(),
            target_pc_id: target_pc_id.to_string(),
            outcomes,
        }
    }

    // ... more challenge messages

    // =========================================================================
    // Inventory Messages
    // =========================================================================

    pub fn equip_item(pc_id: &str, item_id: &str) -> ClientMessage {
        ClientMessage::EquipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        }
    }

    pub fn unequip_item(pc_id: &str, item_id: &str) -> ClientMessage {
        ClientMessage::UnequipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        }
    }

    pub fn drop_item(pc_id: &str, item_id: &str, quantity: u32) -> ClientMessage {
        ClientMessage::DropItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
            quantity,
        }
    }

    pub fn pickup_item(pc_id: &str, item_id: &str) -> ClientMessage {
        ClientMessage::PickupItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        }
    }

    // =========================================================================
    // Scene Messages
    // =========================================================================

    pub fn request_scene_change(scene_id: &str) -> ClientMessage {
        ClientMessage::RequestSceneChange {
            scene_id: scene_id.to_string(),
        }
    }

    pub fn directorial_update(context: DirectorialContext) -> ClientMessage {
        ClientMessage::DirectorialUpdate { context }
    }

    pub fn approval_decision(request_id: &str, decision: ApprovalDecision) -> ClientMessage {
        ClientMessage::ApprovalDecision {
            request_id: request_id.to_string(),
            decision,
        }
    }

    // =========================================================================
    // Action Messages
    // =========================================================================

    pub fn player_action(
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> ClientMessage {
        ClientMessage::PlayerAction {
            action_type: action_type.to_string(),
            target: target.map(|s| s.to_string()),
            dialogue: dialogue.map(|s| s.to_string()),
        }
    }

    // =========================================================================
    // Misc Messages
    // =========================================================================

    pub fn check_comfyui_health() -> ClientMessage {
        ClientMessage::CheckComfyUIHealth
    }

    // =========================================================================
    // Request Wrapper
    // =========================================================================

    pub fn request(payload: RequestPayload) -> ClientMessage {
        ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload,
        }
    }
}
```

### 5.2 Update WebSocket Module Exports

**File:** `crates/player-adapters/src/infrastructure/websocket/mod.rs` (modify)

Add:
```rust
mod message_builder;
pub use message_builder::*;
```

### 5.3 Simplify WASM Adapter

**File:** `crates/player-adapters/src/infrastructure/websocket/wasm/adapter.rs` (modify)

Reduce from 354 lines to ~150 lines by using ClientMessageBuilder.

**Example:**
```rust
fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
    self.inner.client.send(ClientMessageBuilder::move_to_region(pc_id, region_id))
}
```

### 5.4 Simplify Desktop Adapter

**File:** `crates/player-adapters/src/infrastructure/websocket/desktop/adapter.rs` (modify)

Reduce from 496 lines to ~200 lines by using ClientMessageBuilder.

Add helper method:
```rust
fn spawn_send(&self, msg: ClientMessage) -> Result<()> {
    let client = self.client.clone();
    tokio::spawn(async move {
        if let Err(e) = client.send(msg).await {
            tracing::error!("Send failed: {}", e);
        }
    });
    Ok(())
}
```

Then each method becomes:
```rust
fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
    self.spawn_send(ClientMessageBuilder::move_to_region(pc_id, region_id))
}
```

---

## Phase 6: Testing (6-8 hours)

### 6.1 Unit Tests for Use Cases

Create test files alongside each use case:
- `movement_test.rs`
- `staging_test.rs`
- `player_action_test.rs`
- `inventory_test.rs`
- `challenge_test.rs`

**Example test structure:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    // Mock the repository ports
    mock! {
        PlayerCharacterRepo {}
        #[async_trait]
        impl PlayerCharacterRepositoryPort for PlayerCharacterRepo {
            async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>, RepositoryError>;
            // ... other methods
        }
    }

    #[tokio::test]
    async fn move_to_region_with_valid_staging_returns_scene_changed() {
        // Arrange
        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo.expect_get()
            .returning(|_| Ok(Some(/* test PC */)));

        let use_case = MovementUseCase::new(/* mocked deps */);

        // Act
        let result = use_case.move_to_region(ctx, pc_id, region_id).await;

        // Assert
        assert!(matches!(result, Ok(MovementResult::SceneChanged(_))));
    }

    #[tokio::test]
    async fn move_to_region_with_locked_connection_returns_blocked() {
        // ...
    }

    #[tokio::test]
    async fn move_to_region_without_staging_triggers_dm_notification() {
        // ...
    }
}
```

### 6.2 Test Coverage Goals

| Use Case | Tests |
|----------|-------|
| MovementUseCase | 8-10 |
| StagingApprovalUseCase | 6-8 |
| PlayerActionUseCase | 5-6 |
| InventoryUseCase | 8-10 |
| ChallengeUseCase | 6-8 |
| SceneBuilder | 4-5 |
| **Total** | ~40-50 |

---

## Phase 7: Arch-Check Enhancements (3-4 hours)

### 7.1 Add Handler Complexity Check

**File:** `crates/xtask/src/main.rs` (modify)

Add new function:

```rust
/// Check that WebSocket handlers remain thin routing layers
fn check_handler_complexity() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let handlers_dir = workspace_root.join("crates/engine-adapters/src/infrastructure/websocket/handlers");

    if !handlers_dir.exists() {
        return Ok(()); // Skip if directory doesn't exist
    }

    let mut violations = Vec::new();

    // Allowed thin handlers (exemptions)
    let exempt_files: std::collections::HashSet<&str> =
        ["mod.rs", "request.rs"].into_iter().collect();

    // Maximum lines per handler file (after refactor)
    const MAX_HANDLER_LINES: usize = 200;

    // Patterns that indicate business logic in handlers
    let business_logic_patterns = [
        // Building domain DTOs in handler (should be in use case)
        regex_lite::Regex::new(r"(StagedNpcInfo|NpcPresenceData|RegionData|NavigationData)\s*\{")
            .context("compiling DTO pattern")?,
        // Direct repository iteration with complex mapping
        regex_lite::Regex::new(r"\.iter\(\)[\s\S]{50,}\.map\(")
            .context("compiling iter-map pattern")?,
    ];

    for entry in walkdir_rs_files(&handlers_dir)? {
        let file_name = entry
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if exempt_files.contains(file_name) {
            continue;
        }

        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        let line_count = contents.lines().count();

        // Check line count
        if line_count > MAX_HANDLER_LINES {
            violations.push(format!(
                "{}: {} lines exceeds max {} - extract business logic to use cases",
                entry.display(),
                line_count,
                MAX_HANDLER_LINES
            ));
        }

        // Check for business logic patterns
        for pattern in &business_logic_patterns {
            if pattern.is_match(&contents) {
                violations.push(format!(
                    "{}: contains business logic pattern - should be in use case layer",
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
        anyhow::bail!("arch-check failed: handlers contain too much business logic");
    }

    Ok(())
}
```

### 7.2 Add Use Case Layer Check

```rust
/// Check that use case layer exists and is properly structured
fn check_use_case_layer() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let use_cases_dir = workspace_root.join("crates/engine-app/src/application/use_cases");

    // Use cases directory must exist
    if !use_cases_dir.exists() {
        anyhow::bail!(
            "Missing use_cases directory: {}\n\
            Use cases are required for complex workflows.\n\
            See docs/architecture/hexagonal-architecture.md",
            use_cases_dir.display()
        );
    }

    let use_case_files = walkdir_rs_files(&use_cases_dir)?;
    if use_case_files.len() < 2 {
        anyhow::bail!(
            "Insufficient use cases: found {} files, expected at least 2\n\
            Complex workflows should be implemented as use cases, not in handlers.",
            use_case_files.len()
        );
    }

    // Check that use cases don't import protocol ServerMessage
    let forbidden_import =
        regex_lite::Regex::new(r"use\s+wrldbldr_protocol::.*ServerMessage")
            .context("compiling forbidden import regex")?;

    for entry in &use_case_files {
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "mod.rs" {
            continue;
        }

        let contents = std::fs::read_to_string(entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        if forbidden_import.is_match(&contents) {
            anyhow::bail!(
                "{}: Use cases must NOT import ServerMessage\n\
                Use cases should return domain types that handlers convert to ServerMessage.",
                entry.display()
            );
        }
    }

    Ok(())
}
```

### 7.3 Add Adapter Import Check

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

    // Adapters should NOT import internal types from services
    // Pattern: use wrldbldr_engine_app::application::services::some_service::InternalType
    let forbidden_service_internals = regex_lite::Regex::new(
        r"use\s+wrldbldr_engine_app::application::services::\w+::\w+",
    )
    .context("compiling forbidden service internals regex")?;

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&adapters_dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        if let Some((line_no, line)) = first_match_line(&forbidden_service_internals, &contents) {
            violations.push(format!(
                "{}:{}: Adapter imports service internals - use ports or use cases instead:\n    {}",
                entry.display(),
                line_no,
                line.trim()
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
    // Existing checks
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
        .context("running cargo metadata")?;

    if !output.status.success() {
        anyhow::bail!("cargo metadata failed")
    }

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("parsing cargo metadata JSON")?;

    // ... existing internal deps check code ...

    check_no_cross_crate_shims()?;

    // NEW: Additional architecture checks
    check_handler_complexity()?;
    check_use_case_layer()?;
    check_adapter_imports()?;

    println!("arch-check OK ({checked} workspace crates checked)");
    Ok(())
}
```

---

## Phase 8: Documentation & Cleanup (2-3 hours)

### 8.1 Update Architecture Documentation

**File:** `docs/architecture/hexagonal-architecture.md` (modify)

Add section on use cases:

```markdown
## Use Cases

Use cases are the orchestration layer between adapters and domain services.
They coordinate complex workflows while remaining transport-agnostic.

### When to Create a Use Case

Create a use case when:
- A handler would exceed 100 lines
- Multiple services need coordination
- Business logic needs to be tested independently of transport
- The same workflow is needed from multiple entry points

### Use Case Structure

```rust
pub struct MyUseCase {
    // Dependencies are ports, not concrete implementations
    repo: Arc<dyn SomeRepositoryPort>,
    service: Arc<SomeService>,
    broadcast: Arc<dyn BroadcastPort>,
}

impl MyUseCase {
    // Methods return domain types, not ServerMessage
    pub async fn do_something(&self, ctx: Context, input: Input)
        -> Result<DomainResult, DomainError>;
}
```

### Handler Pattern

Handlers should be thin routing layers:

```rust
pub async fn handle_something(
    state: &AppState,
    client_id: Uuid,
    /* params */
) -> Option<ServerMessage> {
    let ctx = HandlerContext::extract(state, client_id).await.ok()?;

    match state.my_use_case.do_something(ctx.into(), input).await {
        Ok(result) => Some(result.into_server_message()),
        Err(e) => Some(ServerMessage::Error {
            code: e.error_code().to_string(),
            message: e.to_string(),
        }),
    }
}
```
```

### 8.2 Remove Dead Code

- Remove unused converters in `crates/engine-adapters/src/infrastructure/websocket/converters.rs`
- Remove any orphaned helper functions

### 8.3 Update AGENTS.md

Add guidance about the use case layer to the agent guidelines.

---

## Summary

### Files Created (~3,100 lines)

| Crate | File | Lines |
|-------|------|-------|
| engine-ports | `outbound/broadcast_port.rs` | 80 |
| engine-ports | `outbound/broadcast_events.rs` | 200 |
| engine-app | `application/use_cases/mod.rs` | 30 |
| engine-app | `application/use_cases/errors.rs` | 200 |
| engine-app | `application/use_cases/movement.rs` | 350 |
| engine-app | `application/use_cases/staging.rs` | 300 |
| engine-app | `application/use_cases/player_action.rs` | 250 |
| engine-app | `application/use_cases/inventory.rs` | 200 |
| engine-app | `application/use_cases/observation.rs` | 150 |
| engine-app | `application/use_cases/challenge.rs` | 200 |
| engine-app | `application/use_cases/scene.rs` | 150 |
| engine-app | `application/use_cases/connection.rs` | 180 |
| engine-app | `application/services/scene_builder.rs` | 200 |
| engine-app | `application/use_cases/*_test.rs` | 400 |
| engine-adapters | `websocket/context.rs` | 100 |
| engine-adapters | `websocket/broadcast_adapter.rs` | 200 |
| player-adapters | `websocket/message_builder.rs` | 350 |
| xtask | New check functions | 150 |

### Files Modified (net reduction ~3,500 lines)

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
| `converters.rs` | - | - | -40 |
| Various mod.rs files | - | - | +50 |

### Net Change

- **New code:** ~3,100 lines
- **Removed code:** ~4,200 lines
- **Net reduction:** ~1,100 lines
- **Test coverage:** +40-50 new tests

### Effort Estimate

| Phase | Description | Hours |
|-------|-------------|-------|
| 1 | Infrastructure Helpers | 2-3 |
| 2 | Ports & Domain Events | 3-4 |
| 3 | Use Cases | 12-15 |
| 4 | Adapter Implementation | 5-6 |
| 5 | Player Adapter Deduplication | 4-5 |
| 6 | Testing | 6-8 |
| 7 | Arch-Check Enhancements | 3-4 |
| 8 | Documentation & Cleanup | 2-3 |
| **Total** | | **38-48 hours** |

---

## Verification Checklist

After implementation, verify:

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` passes
- [ ] `cargo xtask arch-check` passes (including new checks)
- [ ] Manual test: PC movement triggers staging approval
- [ ] Manual test: DM staging approval sends scene to waiting PCs
- [ ] Manual test: Player actions queue correctly
- [ ] Manual test: WASM client connects and operates
- [ ] Manual test: Desktop client connects and operates
- [ ] All handler files < 200 lines
- [ ] No ServerMessage imports in use cases
- [ ] No service internal imports in adapters

---

## Implementation Order

For the implementing agent, proceed in this order:

1. **Phase 1:** Create `context.rs` helper (unblocks all handler refactoring)
2. **Phase 2:** Create ports and domain events (unblocks use cases)
3. **Phase 3:** Create use cases one at a time:
   - Start with `MovementUseCase` (most complex, validates pattern)
   - Then `StagingApprovalUseCase` (shares SceneBuilder)
   - Then remaining use cases
4. **Phase 4:** Create broadcast adapter, refactor handlers
5. **Phase 5:** Create ClientMessageBuilder, simplify player adapters
6. **Phase 6:** Add tests as each use case is created
7. **Phase 7:** Add arch-check enhancements
8. **Phase 8:** Documentation and cleanup

After each phase, run `cargo check --workspace` to catch issues early.
