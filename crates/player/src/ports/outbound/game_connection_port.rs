//! Game Connection Port - Outbound port for Engine WebSocket operations
//!
//! This port abstracts WebSocket communication with the Engine backend,
//! allowing application services to manage real-time game sessions without
//! depending on concrete WebSocket client implementations.

use std::future::Future;
use std::pin::Pin;

// Import session types from this crate (ports layer owns these DTOs)
use crate::session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DiceInput,
    DirectorialContext, ParticipantRole,
};

// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// This port uses protocol types directly because it defines the primary
// engine-player communication boundary. The protocol crate exists specifically
// to share types across this boundary.
use wrldbldr_protocol::{RequestError, RequestPayload, ResponseResult};

/// Connection state for the game session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected to the server
    Disconnected,
    /// Attempting to establish connection
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection lost, attempting to reconnect
    Reconnecting,
    /// Connection failed
    Failed,
}

/// Game Connection Port trait for Engine WebSocket operations
///
/// This trait provides a platform-agnostic interface for WebSocket communication
/// with the Engine. Both desktop (tokio) and WASM (web-sys) implementations
/// satisfy Send + Sync requirements.
///
/// # Platform Implementations
///
/// - **Desktop (tokio)**: Uses async/await with native threading
/// - **WASM**: Uses SendWrapper to satisfy Send + Sync in single-threaded context
///
/// NOTE: This trait is intentionally **object-safe** so the presentation layer can
/// store an `Arc<dyn GameConnectionPort>` without depending on concrete
/// infrastructure types.
pub trait GameConnectionPort: Send + Sync {
    /// Get the current connection state
    fn state(&self) -> ConnectionState;

    /// Get the server URL
    fn url(&self) -> &str;

    /// Connect to the server
    fn connect(&self) -> anyhow::Result<()>;

    /// Disconnect from the server
    fn disconnect(&self);

    /// Join a world with the given user ID and role.
    ///
    /// # Arguments
    /// * `world_id` - The world to join (required)
    /// * `user_id` - The user's identifier
    /// * `role` - The participant role (Player or DM)
    fn join_world(
        &self,
        world_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> anyhow::Result<()>;

    /// Send a player action to the server
    fn send_action(
        &self,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Start a conversation with an NPC
    fn start_conversation(&self, npc_id: &str, message: &str) -> anyhow::Result<()>;

    /// Continue a conversation with an NPC
    fn continue_conversation(&self, npc_id: &str, message: &str) -> anyhow::Result<()>;

    /// Perform a scene interaction by ID
    fn perform_interaction(&self, interaction_id: &str) -> anyhow::Result<()>;

    /// Request a scene change (DM only)
    fn request_scene_change(&self, scene_id: &str) -> anyhow::Result<()>;

    /// Send a directorial context update (DM only)
    fn send_directorial_update(&self, context: DirectorialContext) -> anyhow::Result<()>;

    /// Send an approval decision (DM only)
    fn send_approval_decision(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> anyhow::Result<()>;

    /// Send a challenge outcome decision (DM only)
    fn send_challenge_outcome_decision(
        &self,
        resolution_id: &str,
        decision: ChallengeOutcomeDecision,
    ) -> anyhow::Result<()>;

    /// Trigger a challenge (DM only)
    fn trigger_challenge(
        &self,
        challenge_id: &str,
        target_character_id: &str,
    ) -> anyhow::Result<()>;

    /// Submit a challenge roll (Player only) - legacy method using raw i32
    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> anyhow::Result<()>;

    /// Submit a challenge roll with dice input (Player only) - supports formulas and manual input
    fn submit_challenge_roll_input(
        &self,
        challenge_id: &str,
        input: DiceInput,
    ) -> anyhow::Result<()>;

    /// Send a heartbeat ping
    fn heartbeat(&self) -> anyhow::Result<()>;

    /// Move PC to a different region within the same location
    fn move_to_region(&self, pc_id: &str, region_id: &str) -> anyhow::Result<()>;

    /// Exit to a different location
    fn exit_to_location(
        &self,
        pc_id: &str,
        location_id: &str,
        arrival_region_id: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Send a staging approval response (DM only)
    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> anyhow::Result<()>;

    /// Request regeneration of staging suggestions (DM only)
    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> anyhow::Result<()>;

    /// Pre-stage a region before player arrival (DM only)
    fn pre_stage_region(
        &self,
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> anyhow::Result<()>;

    /// Create an ad-hoc challenge (DM only)
    fn create_adhoc_challenge(
        &self,
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> anyhow::Result<()>;

    /// Equip an item (Player only)
    fn equip_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()>;

    /// Unequip an item (Player only)
    fn unequip_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()>;

    /// Drop an item (Player only) - currently destroys the item
    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> anyhow::Result<()>;

    /// Pick up an item from current region (Player only)
    fn pickup_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()>;

    /// Request a manual ComfyUI health check
    fn check_comfyui_health(&self) -> anyhow::Result<()>;

    /// Set NPC disposition toward a PC (DM only)
    fn set_npc_disposition(
        &self,
        npc_id: &str,
        pc_id: &str,
        disposition: &str,
        reason: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Set NPC relationship toward a PC (DM only)
    fn set_npc_relationship(
        &self,
        npc_id: &str,
        pc_id: &str,
        relationship: &str,
    ) -> anyhow::Result<()>;

    /// Request NPC dispositions for a PC (fetches current disposition data)
    fn get_npc_dispositions(&self, pc_id: &str) -> anyhow::Result<()>;

    // =========================================================================
    // Time Control (DM only)
    // =========================================================================

    /// Advance game time by a number of minutes
    fn advance_time(&self, world_id: &str, minutes: u32, reason: &str) -> anyhow::Result<()>;

    /// Set game time to a specific day and hour
    fn set_game_time(&self, world_id: &str, day: u32, hour: u8) -> anyhow::Result<()>;

    /// Skip to the next occurrence of a time period (Morning, Afternoon, Evening, Night)
    fn skip_to_period(&self, world_id: &str, period: &str) -> anyhow::Result<()>;

    /// Respond to a time suggestion (approve, modify, or skip)
    fn respond_to_time_suggestion(
        &self,
        suggestion_id: &str,
        decision: &str,
        modified_minutes: Option<u32>,
    ) -> anyhow::Result<()>;

    /// Register a callback for state changes
    fn on_state_change(&self, callback: Box<dyn FnMut(ConnectionState) + Send + 'static>);

    /// Register a callback for server events
    ///
    /// The adapter translates wire-format `ServerMessage` to application-layer
    /// `PlayerEvent` before invoking this callback.
    fn on_message(
        &self,
        callback: Box<dyn FnMut(crate::outbound::player_events::PlayerEvent) + Send + 'static>,
    );

    /// Send a request and await the response
    ///
    /// This is the primary method for WebSocket request-response operations.
    /// The implementation handles request_id generation, pending request tracking,
    /// and response correlation.
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    ///
    /// # Returns
    /// * `Ok(ResponseResult)` - The server's response
    /// * `Err(RequestError)` - If the request failed to send or timed out
    fn request(
        &self,
        payload: RequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>>;

    /// Send a request with a custom timeout
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    /// * `timeout_ms` - Timeout in milliseconds (default is from WRLDBLDR_REQUEST_TIMEOUT_MS env var or 120000)
    fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>>;
}
