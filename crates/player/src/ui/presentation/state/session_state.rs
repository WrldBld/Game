//! Session state management using Dioxus signals
//!
//! This is a facade that composes ConnectionState, ApprovalState, and ChallengeState
//! for unified session management. Individual substates can be accessed directly
//! for more focused functionality.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::application::dto::{
    ApprovalDecision, ConnectedUser, OutcomeBranchData, ParticipantRole, WorldRole,
};
use crate::presentation::components::tactical::PlayerSkillData;

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
/// This is a facade that composes ConnectionState, ApprovalState and ChallengeState.
/// For new code, prefer accessing the substates directly via `connection`,
/// `approval`, and `challenge` fields.
#[derive(Clone)]
pub struct SessionState {
    /// Connection-related state (status, user, session)
    pub connection: ConnectionState,
    /// Approval workflow state (pending approvals, history, log)
    pub approval: ApprovalState,
    /// Challenge-related state (active challenge, results, skills)
    pub challenge: ChallengeState,
    /// Whether to show time to players (from TimeConfigUpdated)
    pub show_time_to_players: Signal<bool>,
}

impl SessionState {
    /// Create a new SessionState with disconnected status
    pub fn new() -> Self {
        Self {
            connection: ConnectionState::new(),
            approval: ApprovalState::new(),
            challenge: ChallengeState::new(),
            show_time_to_players: Signal::new(true),
        }
    }

    /// Accessor for show_time_to_players flag
    pub fn should_show_time_to_players(&self) -> Signal<bool> {
        self.show_time_to_players.clone()
    }

    /// Update show_time_to_players flag
    pub fn set_show_time_to_players(&mut self, show: bool) {
        self.show_time_to_players.set(show);
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
        platform: &dyn crate::ports::outbound::PlatformPort,
    ) {
        self.approval
            .add_log_entry(speaker, text, is_system, platform);
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

    /// Record an approval decision locally: log it in history and remove from pending queue.
    /// Note: The actual sending to Engine is done via CommandBus through the ApprovalService.
    pub fn record_approval_decision(
        &mut self,
        request_id: String,
        decision: &ApprovalDecision,
        platform: &dyn crate::ports::outbound::PlatformPort,
    ) {
        self.approval
            .record_approval_decision(request_id, decision, platform);
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
        self.challenge.roll_status
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
        self.approval.pending_challenge_outcomes
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}
