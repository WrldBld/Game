//! Game Connection Port - Outbound port for Engine WebSocket operations
//!
//! This port abstracts WebSocket communication with the Engine backend,
//! allowing application services to manage real-time game sessions without
//! depending on concrete WebSocket client implementations.

use wrldbldr_protocol::{
    AdHocOutcomes,
    ApprovalDecision,
    ApprovedNpcInfo,
    ChallengeOutcomeDecisionData,
    DiceInputType,
    DirectorialContext,
    ParticipantRole,
};

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
/// with the Engine. Different platforms (WASM vs native) have different async
/// models, so this trait is designed to work with both.
///
/// # Platform Differences
///
/// - **Desktop (tokio)**: Uses async/await with Send + Sync requirements
/// - **WASM**: Uses callbacks and single-threaded model
///
/// NOTE: This trait is intentionally **object-safe** so the presentation layer can
/// store an `Arc<dyn GameConnectionPort>` without depending on concrete
/// infrastructure types.
#[cfg(not(target_arch = "wasm32"))]
pub trait GameConnectionPort: Send + Sync {
    /// Get the current connection state
    fn state(&self) -> ConnectionState;

    /// Get the server URL
    fn url(&self) -> &str;

    /// Connect to the server
    fn connect(&self) -> anyhow::Result<()>;

    /// Disconnect from the server
    fn disconnect(&self);

    /// Join a session with the given user ID, role, and optional world context.
    ///
    /// `world_id` should be the world this session belongs to when known. When
    /// `None`, the Engine will create or join a demo session.
    fn join_session(
        &self,
        user_id: &str,
        role: ParticipantRole,
        world_id: Option<String>,
    ) -> anyhow::Result<()>;

    /// Send a player action to the server
    fn send_action(
        &self,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Request a scene change (DM only)
    fn request_scene_change(&self, scene_id: &str) -> anyhow::Result<()>;

    /// Send a directorial context update (DM only)
    fn send_directorial_update(&self, context: DirectorialContext) -> anyhow::Result<()>;

    /// Send an approval decision (DM only)
    fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> anyhow::Result<()>;

    /// Send a challenge outcome decision (DM only)
    fn send_challenge_outcome_decision(&self, resolution_id: &str, decision: ChallengeOutcomeDecisionData) -> anyhow::Result<()>;

    /// Trigger a challenge (DM only)
    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> anyhow::Result<()>;

    /// Submit a challenge roll (Player only) - legacy method using raw i32
    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> anyhow::Result<()>;

    /// Submit a challenge roll with dice input (Player only) - supports formulas and manual input
    fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInputType) -> anyhow::Result<()>;

    /// Send a heartbeat ping
    fn heartbeat(&self) -> anyhow::Result<()>;

    /// Move PC to a different region within the same location
    fn move_to_region(&self, pc_id: &str, region_id: &str) -> anyhow::Result<()>;

    /// Exit to a different location
    fn exit_to_location(&self, pc_id: &str, location_id: &str, arrival_region_id: Option<&str>) -> anyhow::Result<()>;

    /// Send a staging approval response (DM only)
    fn send_staging_approval(&self, request_id: &str, approved_npcs: Vec<ApprovedNpcInfo>, ttl_hours: i32, source: &str) -> anyhow::Result<()>;

    /// Request regeneration of staging suggestions (DM only)
    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> anyhow::Result<()>;

    /// Pre-stage a region before player arrival (DM only)
    fn pre_stage_region(&self, region_id: &str, npcs: Vec<ApprovedNpcInfo>, ttl_hours: i32) -> anyhow::Result<()>;

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

    /// Set NPC mood toward a PC (DM only)
    fn set_npc_mood(&self, npc_id: &str, pc_id: &str, mood: &str, reason: Option<&str>) -> anyhow::Result<()>;

    /// Set NPC relationship toward a PC (DM only)
    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> anyhow::Result<()>;

    /// Request NPC moods for a PC (fetches current mood data)
    fn get_npc_moods(&self, pc_id: &str) -> anyhow::Result<()>;

    /// Register a callback for state changes
    fn on_state_change(&self, callback: Box<dyn FnMut(ConnectionState) + Send + 'static>);

    /// Register a callback for server messages
    fn on_message(&self, callback: Box<dyn FnMut(serde_json::Value) + Send + 'static>);
}

#[cfg(target_arch = "wasm32")]
pub trait GameConnectionPort {
    /// Get the current connection state
    fn state(&self) -> ConnectionState;

    /// Get the URL this client is configured for
    fn url(&self) -> &str;

    /// Connect to the Engine server
    ///
    /// # Errors
    /// Returns an error if the connection cannot be established.
    fn connect(&self) -> anyhow::Result<()>;

    /// Disconnect from the Engine server
    fn disconnect(&self);

    /// Join a game session
    ///
    /// # Arguments
    /// * `user_id` - Unique identifier for this user
    /// * `role` - The role this participant will have in the session
    /// * `world_id` - Optional world this session is associated with
    fn join_session(
        &self,
        user_id: &str,
        role: ParticipantRole,
        world_id: Option<String>,
    ) -> anyhow::Result<()>;

    /// Send a player action
    ///
    /// # Arguments
    /// * `action_type` - Type of action (e.g., "talk", "examine", "use")
    /// * `target` - Optional target of the action
    /// * `dialogue` - Optional dialogue text
    fn send_action(
        &self,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Request a scene change
    fn request_scene_change(&self, scene_id: &str) -> anyhow::Result<()>;

    /// Send directorial context update (DM only)
    fn send_directorial_update(&self, context: DirectorialContext) -> anyhow::Result<()>;

    /// Send approval decision (DM only)
    fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> anyhow::Result<()>;

    /// Send a challenge outcome decision (DM only)
    fn send_challenge_outcome_decision(&self, resolution_id: &str, decision: ChallengeOutcomeDecisionData) -> anyhow::Result<()>;

    /// Trigger a challenge for a character (DM only)
    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> anyhow::Result<()>;

    /// Submit a challenge roll (Player only) - legacy method using raw i32
    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> anyhow::Result<()>;

    /// Submit a challenge roll with dice input (Player only) - supports formulas and manual input
    fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInputType) -> anyhow::Result<()>;

    /// Send a heartbeat ping
    fn heartbeat(&self) -> anyhow::Result<()>;

    /// Move PC to a different region within the same location
    fn move_to_region(&self, pc_id: &str, region_id: &str) -> anyhow::Result<()>;

    /// Exit to a different location
    fn exit_to_location(&self, pc_id: &str, location_id: &str, arrival_region_id: Option<&str>) -> anyhow::Result<()>;

    /// Send a staging approval response (DM only)
    fn send_staging_approval(&self, request_id: &str, approved_npcs: Vec<ApprovedNpcInfo>, ttl_hours: i32, source: &str) -> anyhow::Result<()>;

    /// Request regeneration of staging suggestions (DM only)
    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> anyhow::Result<()>;

    /// Pre-stage a region before player arrival (DM only)
    fn pre_stage_region(&self, region_id: &str, npcs: Vec<ApprovedNpcInfo>, ttl_hours: i32) -> anyhow::Result<()>;

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

    /// Set NPC mood toward a PC (DM only)
    fn set_npc_mood(&self, npc_id: &str, pc_id: &str, mood: &str, reason: Option<&str>) -> anyhow::Result<()>;

    /// Set NPC relationship toward a PC (DM only)
    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> anyhow::Result<()>;

    /// Request NPC moods for a PC (fetches current mood data)
    fn get_npc_moods(&self, pc_id: &str) -> anyhow::Result<()>;

    /// Register a callback for state changes
    ///
    /// The callback will be invoked whenever the connection state changes.
    fn on_state_change(&self, callback: Box<dyn FnMut(ConnectionState) + 'static>);

    /// Register a callback for server messages
    ///
    /// The raw JSON value allows the presentation layer to handle specific
    /// message types as needed.
    fn on_message(&self, callback: Box<dyn FnMut(serde_json::Value) + 'static>);
}
