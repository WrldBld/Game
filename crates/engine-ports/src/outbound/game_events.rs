//! Domain events for game notifications
//!
//! These are transport-agnostic event types used by use cases to request
//! notifications. The adapter layer converts these to ServerMessage.
//!
//! # Architecture
//!
//! ```text
//! Use Case Layer                    Adapter Layer
//! ┌─────────────────┐              ┌─────────────────────────┐
//! │ MovementUseCase │──GameEvent──>│ WebSocketBroadcastAdapter│
//! │                 │              │ - Converts to ServerMsg  │
//! │                 │              │ - Routes to recipients   │
//! └─────────────────┘              └─────────────────────────┘
//! ```
//!
//! # Design Rationale (D1)
//!
//! Using a single `GameEvent` enum instead of multiple methods on `BroadcastPort`:
//! - Single routing point in adapter (simpler implementation)
//! - Easier to add new event types (no trait changes)
//! - Cleaner mock setup in tests (`expect_broadcast` once vs many)
//! - Event routing logic centralized in adapter, not scattered across trait methods
//!
//! # Note on ID Types
//!
//! Event structs use domain ID types (RegionId, etc.) for type safety in the
//! application layer. These types don't implement Serialize/Deserialize, so
//! the adapter layer must convert them to protocol types when creating
//! ServerMessage variants.

use chrono::{DateTime, Utc};
use wrldbldr_domain::{
    CharacterId, GameTime, ItemId, LocationId, PlayerCharacterId, RegionId, StagingId,
};

// =============================================================================
// Main GameEvent Enum
// =============================================================================

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

    // === Inventory Events ===
    /// Item picked up (notify player)
    ItemPickedUp {
        user_id: String,
        pc_id: PlayerCharacterId,
        item: ItemInfo,
        quantity: u32,
    },
    /// Item dropped (notify player)
    ItemDropped {
        user_id: String,
        pc_id: PlayerCharacterId,
        item: ItemInfo,
        quantity: u32,
        region_id: RegionId,
    },
    /// Item equipped/unequipped (notify player)
    ItemEquipChanged {
        user_id: String,
        pc_id: PlayerCharacterId,
        item: ItemInfo,
        equipped: bool,
    },

    // === Challenge Events ===
    /// Challenge triggered (DM notification)
    ChallengeTriggerRequested {
        request_id: String,
        challenge_name: String,
        pc_name: String,
        context: String,
    },
    /// Challenge outcome pending approval (DM notification)
    ChallengeOutcomePending {
        request_id: String,
        challenge_name: String,
        pc_name: String,
        roll_result: i32,
        outcome_branch: String,
    },
}

// =============================================================================
// Event Struct Definitions
// =============================================================================

/// DM approval required for NPC staging
#[derive(Debug, Clone)]
pub struct StagingRequiredEvent {
    /// Unique ID for this staging request
    pub request_id: String,
    /// Region being staged
    pub region_id: RegionId,
    /// Region name for display
    pub region_name: String,
    /// Location containing the region
    pub location_id: LocationId,
    /// Location name for display
    pub location_name: String,
    /// Current game time
    pub game_time: GameTime,
    /// NPCs suggested by rules engine
    pub rule_based_npcs: Vec<StagedNpcData>,
    /// NPCs suggested by LLM
    pub llm_based_npcs: Vec<StagedNpcData>,
    /// PCs waiting for this staging to complete
    pub waiting_pcs: Vec<WaitingPcData>,
    /// Previous staging for this region (if any)
    pub previous_staging: Option<PreviousStagingData>,
    /// Default TTL from location settings
    pub default_ttl_hours: i32,
}

/// Staging is ready for a region
#[derive(Debug, Clone)]
pub struct StagingReadyEvent {
    /// Region that was staged
    pub region_id: RegionId,
    /// NPCs now present in the region
    pub npcs_present: Vec<NpcPresenceData>,
    /// PCs that were waiting and should receive scene updates
    pub waiting_pcs: Vec<WaitingPcData>,
}

/// Player is waiting for staging to complete
#[derive(Debug, Clone)]
pub struct StagingPendingEvent {
    /// Region being staged
    pub region_id: RegionId,
    /// Region name for display
    pub region_name: String,
}

/// Scene changed for a player
#[derive(Debug, Clone)]
pub struct SceneChangedEvent {
    /// PC whose scene changed
    pub pc_id: PlayerCharacterId,
    /// Region information
    pub region: RegionInfo,
    /// NPCs present in the region
    pub npcs_present: Vec<NpcPresenceData>,
    /// Navigation options from this region
    pub navigation: NavigationInfo,
    /// Items in this region
    pub region_items: Vec<RegionItemData>,
}

/// Party has split across locations
#[derive(Debug, Clone)]
pub struct SplitPartyEvent {
    /// Groups of PCs by location
    pub location_groups: Vec<LocationGroup>,
}

// =============================================================================
// Supporting Data Types
// =============================================================================

/// NPC staging data
#[derive(Debug, Clone)]
pub struct StagedNpcData {
    /// Character ID
    pub character_id: CharacterId,
    /// Character name
    pub name: String,
    /// Sprite asset path
    pub sprite_asset: Option<String>,
    /// Portrait asset path
    pub portrait_asset: Option<String>,
    /// Whether NPC is present
    pub is_present: bool,
    /// Whether NPC is hidden from players
    pub is_hidden_from_players: bool,
    /// Reasoning for presence decision
    pub reasoning: String,
}

/// PC waiting for staging
#[derive(Debug, Clone)]
pub struct WaitingPcData {
    /// Player character ID
    pub pc_id: PlayerCharacterId,
    /// PC name
    pub pc_name: String,
    /// User ID controlling this PC
    pub user_id: String,
}

/// Previous staging data for reference
#[derive(Debug, Clone)]
pub struct PreviousStagingData {
    /// Previous staging ID
    pub staging_id: StagingId,
    /// When it was approved
    pub approved_at: DateTime<Utc>,
    /// NPCs from previous staging
    pub npcs: Vec<StagedNpcData>,
}

/// NPC presence in scene
#[derive(Debug, Clone)]
pub struct NpcPresenceData {
    /// Character ID
    pub character_id: CharacterId,
    /// Character name
    pub name: String,
    /// Sprite asset path
    pub sprite_asset: Option<String>,
    /// Portrait asset path
    pub portrait_asset: Option<String>,
}

/// Region information for scene
#[derive(Debug, Clone)]
pub struct RegionInfo {
    /// Region ID
    pub id: RegionId,
    /// Region name
    pub name: String,
    /// Parent location ID
    pub location_id: LocationId,
    /// Parent location name
    pub location_name: String,
    /// Backdrop asset path
    pub backdrop_asset: Option<String>,
    /// Atmosphere description
    pub atmosphere: Option<String>,
    /// Map asset path (for location)
    pub map_asset: Option<String>,
}

/// Navigation information from region
#[derive(Debug, Clone)]
pub struct NavigationInfo {
    /// Connected regions within same location
    pub connected_regions: Vec<NavigationTarget>,
    /// Exits to other locations
    pub exits: Vec<NavigationExit>,
}

/// Navigation target within location
#[derive(Debug, Clone)]
pub struct NavigationTarget {
    /// Target region ID
    pub region_id: RegionId,
    /// Region name
    pub name: String,
    /// Whether connection is locked
    pub is_locked: bool,
    /// Lock description if locked
    pub lock_description: Option<String>,
}

/// Exit to another location
#[derive(Debug, Clone)]
pub struct NavigationExit {
    /// Target location ID
    pub location_id: LocationId,
    /// Location name
    pub location_name: String,
    /// Arrival region in target location
    pub arrival_region_id: RegionId,
    /// Exit description
    pub description: Option<String>,
}

/// Item in a region
#[derive(Debug, Clone)]
pub struct RegionItemData {
    /// Item ID
    pub item_id: ItemId,
    /// Item name
    pub name: String,
    /// Item description
    pub description: Option<String>,
    /// Quantity available
    pub quantity: u32,
}

/// Group of PCs at a location
#[derive(Debug, Clone)]
pub struct LocationGroup {
    /// Location ID
    pub location_id: LocationId,
    /// Location name
    pub location_name: String,
    /// PCs at this location
    pub pcs: Vec<PcLocationData>,
}

/// PC location data
#[derive(Debug, Clone)]
pub struct PcLocationData {
    /// Player character ID
    pub pc_id: PlayerCharacterId,
    /// PC name
    pub pc_name: String,
    /// Current region (if known)
    pub region_id: Option<RegionId>,
    /// Region name (if known)
    pub region_name: Option<String>,
}

/// Basic item information
#[derive(Debug, Clone)]
pub struct ItemInfo {
    /// Item ID
    pub item_id: ItemId,
    /// Item name
    pub name: String,
}
