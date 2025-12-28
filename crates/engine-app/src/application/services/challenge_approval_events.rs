//! Challenge Approval Events
//!
//! Domain events emitted by `ChallengeOutcomeApprovalService` for async notification.
//! These events are sent through a channel and processed by
//! `ChallengeApprovalEventPublisher`, which converts them to `GameEvent` and
//! broadcasts via `BroadcastPort`.
//!
//! # Architecture
//!
//! ```text
//! ChallengeOutcomeApprovalService
//!        │
//!        │ ChallengeApprovalEvent (via mpsc channel)
//!        ▼
//! ChallengeApprovalEventPublisher
//!        │
//!        │ GameEvent (via BroadcastPort)
//!        ▼
//! WebSocketBroadcastAdapter
//!        │
//!        │ ServerMessage (via WorldConnectionManager)
//!        ▼
//!    Clients
//! ```

use wrldbldr_domain::WorldId;

/// Events emitted by ChallengeOutcomeApprovalService
///
/// These events represent challenge approval workflow state changes that need
/// to be communicated to clients. The publisher converts these to `GameEvent`
/// for routing through the broadcast infrastructure.
#[derive(Debug, Clone)]
pub enum ChallengeApprovalEvent {
    /// Roll submitted, queued for DM approval
    ///
    /// Sent when a player submits a dice roll. The publisher routes this to:
    /// - DM: Full pending data for approval UI (`ChallengeOutcomePending`)
    /// - Players: Status confirmation (`ChallengeRollSubmitted`)
    RollSubmitted {
        world_id: WorldId,
        resolution_id: String,
        challenge_id: String,
        challenge_name: String,
        character_id: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        outcome_triggers: Vec<OutcomeTriggerData>,
    },

    /// Challenge resolved and approved by DM
    ///
    /// Broadcast to all players in the world.
    Resolved {
        world_id: WorldId,
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        individual_rolls: Option<Vec<i32>>,
    },

    /// LLM suggestions ready for DM
    ///
    /// Sent to DM only when AI-generated suggestions are available.
    SuggestionsReady {
        world_id: WorldId,
        resolution_id: String,
        suggestions: Vec<String>,
    },

    /// Outcome branches ready for DM selection
    ///
    /// Sent to DM only when branching outcome options are available.
    BranchesReady {
        world_id: WorldId,
        resolution_id: String,
        outcome_type: String,
        branches: Vec<OutcomeBranchData>,
    },

    /// Character stat updated from outcome trigger
    ///
    /// Broadcast to all players when a stat changes from a challenge outcome.
    StatUpdated {
        world_id: WorldId,
        character_id: String,
        character_name: String,
        stat_name: String,
        old_value: i32,
        new_value: i32,
        delta: i32,
    },
}

/// Outcome trigger data for events
#[derive(Debug, Clone)]
pub struct OutcomeTriggerData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Outcome branch data for events
#[derive(Debug, Clone)]
pub struct OutcomeBranchData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub effects: Vec<String>,
}
