//! Approval state management using Dioxus signals
//!
//! Tracks pending approvals, decision history, and conversation log for DM view.

use dioxus::prelude::*;
use std::sync::Arc;

use wrldbldr_player_app::application::dto::{
    ApprovalDecision, ChallengeSuggestionInfo, NarrativeEventSuggestionInfo, OutcomeBranchData,
    ProposedToolInfo,
};
use wrldbldr_player_ports::outbound::GameConnectionPort;

/// A pending approval request from the LLM that the DM needs to review
#[derive(Debug, Clone, PartialEq)]
pub struct PendingApproval {
    /// Unique request ID for this approval
    pub request_id: String,
    /// Name of the NPC responding
    pub npc_name: String,
    /// The proposed dialogue from the LLM
    pub proposed_dialogue: String,
    /// Internal reasoning from the LLM
    pub internal_reasoning: String,
    /// Proposed tool calls
    pub proposed_tools: Vec<ProposedToolInfo>,
    /// Optional challenge suggestion from the Engine
    pub challenge_suggestion: Option<ChallengeSuggestionInfo>,
    /// Optional narrative event suggestion from the Engine
    pub narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
}

/// A past approval decision for lightweight decision history in the DM view
#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalHistoryEntry {
    /// Request ID this decision relates to
    pub request_id: String,
    /// NPC name involved in the decision
    pub npc_name: String,
    /// Outcome label (e.g. "accepted", "modified", "rejected", "takeover")
    pub outcome: String,
    /// Unix timestamp (seconds) when the decision was made
    pub timestamp: u64,
}

/// A log entry for the conversation
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationLogEntry {
    /// Speaker name (or "System" for system messages)
    pub speaker: String,
    /// The message text
    pub text: String,
    /// Whether this is a system message
    pub is_system: bool,
    /// Timestamp (for ordering)
    pub timestamp: u64,
}

/// Pending challenge outcome awaiting DM approval (P3.3/P3.4)
#[derive(Debug, Clone, PartialEq)]
pub struct PendingChallengeOutcome {
    /// Unique resolution ID for tracking
    pub resolution_id: String,
    /// Challenge name for display
    pub challenge_name: String,
    /// ID of the character who rolled
    pub character_id: String,
    /// Name of the character who rolled
    pub character_name: String,
    /// The dice roll result
    pub roll: i32,
    /// Applied modifier
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Outcome type (success, failure, critical_success, critical_failure)
    pub outcome_type: String,
    /// Generated outcome description
    pub outcome_description: String,
    /// Optional outcome triggers
    pub outcome_triggers: Vec<ProposedToolInfo>,
    /// Roll breakdown string (e.g., "1d20(18) + 3 = 21")
    pub roll_breakdown: Option<String>,
    /// LLM-generated alternative suggestions
    pub suggestions: Option<Vec<String>>,
    /// LLM-generated outcome branches for selection (Phase 22C)
    pub branches: Option<Vec<OutcomeBranchData>>,
    /// Whether suggestions are currently being generated
    pub is_generating_suggestions: bool,
    /// Timestamp for ordering
    pub timestamp: u64,
}

/// Approval state for DM approval workflow
#[derive(Clone)]
pub struct ApprovalState {
    /// Pending approval requests (for DM)
    pub pending_approvals: Signal<Vec<PendingApproval>>,
    /// Recent approval decisions for DM decision history
    pub decision_history: Signal<Vec<ApprovalHistoryEntry>>,
    /// Conversation log (for DM view)
    pub conversation_log: Signal<Vec<ConversationLogEntry>>,
    /// Pending challenge outcomes awaiting DM approval (P3.3/P3.4)
    pub pending_challenge_outcomes: Signal<Vec<PendingChallengeOutcome>>,
}

impl ApprovalState {
    /// Create a new ApprovalState
    pub fn new() -> Self {
        Self {
            pending_approvals: Signal::new(Vec::new()),
            decision_history: Signal::new(Vec::new()),
            conversation_log: Signal::new(Vec::new()),
            pending_challenge_outcomes: Signal::new(Vec::new()),
        }
    }

    /// Add a pending approval request
    pub fn add_pending_approval(&mut self, approval: PendingApproval) {
        self.pending_approvals.write().push(approval);
    }

    /// Remove a pending approval by request_id
    pub fn remove_pending_approval(&mut self, request_id: &str) {
        self.pending_approvals
            .write()
            .retain(|a| a.request_id != request_id);
    }

    /// Add an entry to the approval decision history
    pub fn add_approval_history_entry(&mut self, entry: ApprovalHistoryEntry) {
        self.decision_history.write().push(entry);
    }

    /// Get a snapshot of the approval decision history
    pub fn get_approval_history(&self) -> Vec<ApprovalHistoryEntry> {
        self.decision_history.read().clone()
    }

    /// Add a conversation log entry
    pub fn add_log_entry(
        &mut self,
        speaker: String,
        text: String,
        is_system: bool,
        platform: &dyn wrldbldr_player_ports::outbound::PlatformPort,
    ) {
        let timestamp = platform.now_unix_secs();
        self.conversation_log.write().push(ConversationLogEntry {
            speaker,
            text,
            is_system,
            timestamp,
        });
    }

    /// Record an approval decision: send it to the Engine, log it locally with
    /// a real timestamp, and remove it from the pending queue.
    pub fn record_approval_decision(
        &mut self,
        request_id: String,
        decision: &ApprovalDecision,
        platform: &dyn wrldbldr_player_ports::outbound::PlatformPort,
        engine_client: &Option<Arc<dyn GameConnectionPort>>,
    ) {
        // Send to Engine if we have a client
        if let Some(client) = engine_client.as_ref() {
            // Use DmControlPort method directly (available via blanket impl)
            if let Err(e) = client.send_approval_decision(&request_id, decision.clone()) {
                tracing::error!("Failed to send approval decision: {}", e);
            }
        }

        // Normalize outcome label
        let outcome_label = match decision {
            ApprovalDecision::Accept => "accepted",
            ApprovalDecision::AcceptWithRecipients { .. } => "accepted",
            ApprovalDecision::AcceptWithModification { .. } => "modified",
            ApprovalDecision::Reject { .. } => "rejected",
            ApprovalDecision::TakeOver { .. } => "takeover",
        }
        .to_string();

        // Resolve NPC name from current pending approvals
        let npc_name = self
            .pending_approvals
            .read()
            .iter()
            .find(|a| a.request_id == request_id)
            .map(|a| a.npc_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Use Platform to get a real timestamp
        let timestamp = platform.now_unix_secs();

        let entry = ApprovalHistoryEntry {
            request_id: request_id.clone(),
            npc_name,
            outcome: outcome_label,
            timestamp,
        };
        self.add_approval_history_entry(entry);

        // Remove from pending approvals
        self.remove_pending_approval(&request_id);
    }

    /// Clear all approval state
    pub fn clear(&mut self) {
        self.pending_approvals.set(Vec::new());
        self.decision_history.set(Vec::new());
        self.conversation_log.set(Vec::new());
        self.pending_challenge_outcomes.set(Vec::new());
    }

    /// Add a pending challenge outcome for DM approval (P3.3/P3.4)
    pub fn add_pending_challenge_outcome(&mut self, outcome: PendingChallengeOutcome) {
        self.pending_challenge_outcomes.write().push(outcome);
    }

    /// Remove a pending challenge outcome by resolution_id (P3.3/P3.4)
    pub fn remove_pending_challenge_outcome(&mut self, resolution_id: &str) {
        self.pending_challenge_outcomes
            .write()
            .retain(|o| o.resolution_id != resolution_id);
    }

    /// Update suggestions for a pending challenge outcome (P3.3/P3.4)
    pub fn update_challenge_suggestions(&mut self, resolution_id: &str, suggestions: Vec<String>) {
        let mut outcomes = self.pending_challenge_outcomes.write();
        if let Some(outcome) = outcomes
            .iter_mut()
            .find(|o| o.resolution_id == resolution_id)
        {
            outcome.suggestions = Some(suggestions);
            outcome.is_generating_suggestions = false;
        }
    }

    /// Update branches for a pending challenge outcome (Phase 22C)
    pub fn update_challenge_branches(
        &mut self,
        resolution_id: &str,
        _outcome_type: String,
        branches: Vec<OutcomeBranchData>,
    ) {
        let mut outcomes = self.pending_challenge_outcomes.write();
        if let Some(outcome) = outcomes
            .iter_mut()
            .find(|o| o.resolution_id == resolution_id)
        {
            outcome.branches = Some(branches);
            outcome.is_generating_suggestions = false;
        }
    }

    /// Mark a challenge outcome as generating suggestions (P3.3/P3.4)
    pub fn set_challenge_generating_suggestions(&mut self, resolution_id: &str, generating: bool) {
        let mut outcomes = self.pending_challenge_outcomes.write();
        if let Some(outcome) = outcomes
            .iter_mut()
            .find(|o| o.resolution_id == resolution_id)
        {
            outcome.is_generating_suggestions = generating;
        }
    }

    /// Get pending challenge outcomes for display (P3.3/P3.4)
    pub fn get_pending_challenge_outcomes(&self) -> Vec<PendingChallengeOutcome> {
        self.pending_challenge_outcomes.read().clone()
    }
}

impl Default for ApprovalState {
    fn default() -> Self {
        Self::new()
    }
}
