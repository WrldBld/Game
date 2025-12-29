//! Session state management using Dioxus signals
//!
//! This is a facade that composes ConnectionState, ApprovalState, and ChallengeState
//! for unified session management. Individual substates can be accessed directly
//! for more focused functionality.

use dioxus::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::presentation::components::tactical::PlayerSkillData;
use wrldbldr_player_app::application::dto::{
    ApprovalDecision, ConnectedUser, OutcomeBranchData, ParticipantRole, WorldRole,
};
use wrldbldr_player_ports::outbound::{GameConnectionPort, Platform};

// Substate types (avoid `pub use crate::...` shims)
use crate::presentation::state::approval_state::{
    ApprovalHistoryEntry, ApprovalState, ConversationLogEntry, PendingApproval,
};
use crate::presentation::state::challenge_state::{
    ChallengePromptData, ChallengeResultData, ChallengeState,
};
use crate::presentation::state::connection_state::{ConnectionState, ConnectionStatus};

/// Session state for connection and user information
///
/// This is a facade that composes ConnectionState, ApprovalState, and ChallengeState.
/// For new code, prefer accessing the substates directly via the `connection`,
/// `approval`, and `challenge` fields.
#[derive(Clone)]
pub struct SessionState {
    /// Connection-related state (status, user, session)
    pub connection: ConnectionState,
    /// Approval workflow state (pending approvals, history, log)
    pub approval: ApprovalState,
    /// Challenge-related state (active challenge, results, skills)
    pub challenge: ChallengeState,
}

impl SessionState {
    /// Create a new SessionState with disconnected status
    pub fn new() -> Self {
        Self {
            connection: ConnectionState::new(),
            approval: ApprovalState::new(),
            challenge: ChallengeState::new(),
        }
    }

    // =========================================================================
    // Backward-compatible field accessors (delegate to substates)
    // =========================================================================

    /// Current connection status
    pub fn connection_status(&self) -> Signal<ConnectionStatus> {
        self.connection.connection_status.clone()
    }

    /// Session ID after joining (legacy)
    pub fn session_id(&self) -> Signal<Option<String>> {
        self.connection.session_id.clone()
    }

    /// World ID after joining (WebSocket-first protocol)
    pub fn world_id(&self) -> Signal<Option<Uuid>> {
        self.connection.world_id.clone()
    }

    /// User ID (local identifier)
    pub fn user_id(&self) -> Signal<Option<String>> {
        self.connection.user_id.clone()
    }

    /// User role (DungeonMaster, Player, Spectator) - legacy
    pub fn user_role(&self) -> Signal<Option<ParticipantRole>> {
        self.connection.user_role.clone()
    }

    /// World role (DM, Player, Spectator) - WebSocket-first protocol
    pub fn world_role(&self) -> Signal<Option<WorldRole>> {
        self.connection.world_role.clone()
    }

    /// Connected users in the current world
    pub fn connected_users(&self) -> Signal<Vec<ConnectedUser>> {
        self.connection.connected_users.clone()
    }

    /// Server URL we're connected to
    pub fn server_url(&self) -> Signal<Option<String>> {
        self.connection.server_url.clone()
    }

    /// Game connection handle (if connected)
    pub fn engine_client(&self) -> Signal<Option<Arc<dyn GameConnectionPort>>> {
        self.connection.engine_client.clone()
    }

    /// Error message if connection failed
    pub fn error_message(&self) -> Signal<Option<String>> {
        self.connection.error_message.clone()
    }

    /// Pending approval requests (for DM)
    pub fn pending_approvals(&self) -> Signal<Vec<PendingApproval>> {
        self.approval.pending_approvals.clone()
    }

    /// Conversation log (for DM view)
    pub fn conversation_log(&self) -> Signal<Vec<ConversationLogEntry>> {
        self.approval.conversation_log.clone()
    }

    /// Active challenge prompt (if any)
    pub fn active_challenge(&self) -> Signal<Option<ChallengePromptData>> {
        self.challenge.active_challenge.clone()
    }

    /// Recent challenge results for display
    pub fn challenge_results(&self) -> Signal<Vec<ChallengeResultData>> {
        self.challenge.challenge_results.clone()
    }

    /// Player character skills with modifiers
    pub fn player_skills(&self) -> Signal<Vec<PlayerSkillData>> {
        self.challenge.player_skills.clone()
    }

    /// Recent approval decisions for DM decision history
    pub fn decision_history(&self) -> Signal<Vec<ApprovalHistoryEntry>> {
        self.approval.decision_history.clone()
    }

    /// ComfyUI connection state
    pub fn comfyui_state(&self) -> Signal<String> {
        self.connection.comfyui_state.clone()
    }

    pub fn comfyui_message(&self) -> Signal<Option<String>> {
        self.connection.comfyui_message.clone()
    }

    pub fn comfyui_retry_in_seconds(&self) -> Signal<Option<u32>> {
        self.connection.comfyui_retry_in_seconds.clone()
    }

    // =========================================================================
    // Backward-compatible methods (delegate to substates)
    // =========================================================================

    /// Set the connection to connecting state
    pub fn start_connecting(&mut self, server_url: &str) {
        self.connection.start_connecting(server_url);
    }

    /// Set the connection to connected state
    pub fn set_connected(&mut self, client: Arc<dyn GameConnectionPort>) {
        self.connection.set_connected(client);
    }

    /// Store the connection handle without changing UI status.
    pub fn set_connection_handle(&mut self, client: Arc<dyn GameConnectionPort>) {
        self.connection.set_connection_handle(client);
    }

    /// Set the session as joined (legacy)
    pub fn set_session_joined(&mut self, session_id: String) {
        self.connection.set_session_joined(session_id);
    }

    /// Set the world as joined (WebSocket-first protocol)
    pub fn set_world_joined(
        &mut self,
        world_id: Uuid,
        role: WorldRole,
        connected_users: Vec<ConnectedUser>,
    ) {
        self.connection
            .set_world_joined(world_id, role, connected_users);
    }

    /// Add a user to the connected users list
    pub fn add_connected_user(&mut self, user: ConnectedUser) {
        self.connection.add_connected_user(user);
    }

    /// Remove a user from the connected users list
    pub fn remove_connected_user(&mut self, user_id: &str) {
        self.connection.remove_connected_user(user_id);
    }

    /// Set user information (legacy)
    pub fn set_user(&mut self, user_id: String, role: ParticipantRole) {
        self.connection.set_user(user_id, role);
    }

    /// Set the connection to disconnected state
    pub fn set_disconnected(&mut self) {
        self.connection.set_disconnected();
    }

    /// Set the connection to failed state with error
    pub fn set_failed(&mut self, error: String) {
        self.connection.set_failed(error);
    }

    /// Set the connection to reconnecting state
    pub fn set_reconnecting(&mut self) {
        self.connection.set_reconnecting();
    }

    /// Clear all session state
    pub fn clear(&mut self) {
        self.connection.clear();
        self.approval.clear();
        self.challenge.clear();
    }

    /// Add a pending approval request
    pub fn add_pending_approval(&mut self, approval: PendingApproval) {
        self.approval.add_pending_approval(approval);
    }

    /// Remove a pending approval by request_id
    pub fn remove_pending_approval(&mut self, request_id: &str) {
        self.approval.remove_pending_approval(request_id);
    }

    /// Add a conversation log entry
    pub fn add_log_entry(
        &mut self,
        speaker: String,
        text: String,
        is_system: bool,
        platform: &Platform,
    ) {
        self.approval
            .add_log_entry(speaker, text, is_system, platform);
    }

    /// Check if we have an active client
    pub fn has_client(&self) -> bool {
        self.connection.has_client()
    }

    /// Set active challenge prompt
    pub fn set_active_challenge(&mut self, challenge: ChallengePromptData) {
        self.challenge.set_active_challenge(challenge);
    }

    /// Clear active challenge
    pub fn clear_active_challenge(&mut self) {
        self.challenge.clear_active_challenge();
    }

    /// Add a challenge result
    pub fn add_challenge_result(&mut self, result: ChallengeResultData) {
        self.challenge.add_challenge_result(result);
    }

    /// Set player skills
    pub fn set_player_skills(&mut self, skills: Vec<PlayerSkillData>) {
        self.challenge.set_player_skills(skills);
    }

    /// Add a player skill
    pub fn add_player_skill(&mut self, skill: PlayerSkillData) {
        self.challenge.add_player_skill(skill);
    }

    /// Add an entry to the approval decision history
    pub fn add_approval_history_entry(&mut self, entry: ApprovalHistoryEntry) {
        self.approval.add_approval_history_entry(entry);
    }

    /// Get a snapshot of the approval decision history
    pub fn get_approval_history(&self) -> Vec<ApprovalHistoryEntry> {
        self.approval.get_approval_history()
    }

    /// Record an approval decision: send it to the Engine, log it locally with
    /// a real timestamp, and remove it from the pending queue.
    pub fn record_approval_decision(
        &mut self,
        request_id: String,
        decision: &ApprovalDecision,
        platform: &Platform,
    ) {
        let engine_client = self.connection.engine_client.read().clone();
        self.approval
            .record_approval_decision(request_id, decision, platform, &engine_client);
    }

    // =========================================================================
    // P3.3/P3.4: Challenge Outcome Approval
    // =========================================================================

    /// Set roll as awaiting DM approval
    pub fn set_awaiting_approval(
        &mut self,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
    ) {
        self.challenge
            .set_awaiting_approval(roll, modifier, total, outcome_type);
    }

    /// Set challenge result as ready to display
    pub fn set_result_ready(&mut self, result: ChallengeResultData) {
        self.challenge.set_result_ready(result);
    }

    /// Dismiss the result display
    pub fn dismiss_result(&mut self) {
        self.challenge.dismiss_result();
    }

    /// Clear the roll status
    pub fn clear_roll_status(&mut self) {
        self.challenge.clear_roll_status();
    }

    /// Roll submission status accessor
    pub fn roll_status(
        &self,
    ) -> Signal<crate::presentation::state::challenge_state::RollSubmissionStatus> {
        self.challenge.roll_status.clone()
    }

    /// Add a pending challenge outcome for DM approval
    pub fn add_pending_challenge_outcome(
        &mut self,
        outcome: crate::presentation::state::approval_state::PendingChallengeOutcome,
    ) {
        self.approval.add_pending_challenge_outcome(outcome);
    }

    /// Remove a pending challenge outcome by resolution_id
    pub fn remove_pending_challenge_outcome(&mut self, resolution_id: &str) {
        self.approval
            .remove_pending_challenge_outcome(resolution_id);
    }

    /// Update suggestions for a pending challenge outcome
    pub fn update_challenge_suggestions(&mut self, resolution_id: &str, suggestions: Vec<String>) {
        self.approval
            .update_challenge_suggestions(resolution_id, suggestions);
    }

    /// Update branches for a pending challenge outcome (Phase 22C)
    pub fn update_challenge_branches(
        &mut self,
        resolution_id: &str,
        outcome_type: String,
        branches: Vec<OutcomeBranchData>,
    ) {
        self.approval
            .update_challenge_branches(resolution_id, outcome_type, branches);
    }

    /// Mark a challenge outcome as generating suggestions
    pub fn set_challenge_generating_suggestions(&mut self, resolution_id: &str, generating: bool) {
        self.approval
            .set_challenge_generating_suggestions(resolution_id, generating);
    }

    /// Pending challenge outcomes accessor
    pub fn pending_challenge_outcomes(
        &self,
    ) -> Signal<Vec<crate::presentation::state::approval_state::PendingChallengeOutcome>> {
        self.approval.pending_challenge_outcomes.clone()
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}
