//! Challenge Approval Event Publisher
//!
//! Background task that receives `ChallengeApprovalEvent` from a channel
//! and broadcasts them via `BroadcastPort` as `GameEvent`.
//!
//! # Usage
//!
//! ```rust,ignore
//! // Create channel
//! let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
//!
//! // Create publisher with broadcast port
//! let publisher = ChallengeApprovalEventPublisher::new(broadcast_port);
//!
//! // Spawn publisher as background task
//! tokio::spawn(publisher.run(rx));
//!
//! // Pass tx to ChallengeOutcomeApprovalService
//! let service = ChallengeOutcomeApprovalService::new(...).with_event_sender(tx);
//! ```

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedReceiver;
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, GameEvent, OutcomeBranchInfo, OutcomeTriggerInfo,
};

use super::challenge_approval_events::{ChallengeApprovalEvent, OutcomeBranchData, OutcomeTriggerData};

/// Publisher that converts `ChallengeApprovalEvent` to `GameEvent` and broadcasts
pub struct ChallengeApprovalEventPublisher {
    broadcast_port: Arc<dyn BroadcastPort>,
}

impl ChallengeApprovalEventPublisher {
    /// Create a new publisher
    pub fn new(broadcast_port: Arc<dyn BroadcastPort>) -> Self {
        Self { broadcast_port }
    }

    /// Run the publisher, consuming events from the channel
    ///
    /// This should be spawned as a background task. It runs until the
    /// channel sender is dropped.
    pub async fn run(self, mut rx: UnboundedReceiver<ChallengeApprovalEvent>) {
        tracing::info!("ChallengeApprovalEventPublisher started");

        while let Some(event) = rx.recv().await {
            let world_id = self.extract_world_id(&event);
            let game_event = self.map_to_game_event(event);
            self.broadcast_port.broadcast(world_id, game_event).await;
        }

        tracing::info!("ChallengeApprovalEventPublisher stopped");
    }

    /// Extract world_id from an event
    fn extract_world_id(&self, event: &ChallengeApprovalEvent) -> wrldbldr_domain::WorldId {
        match event {
            ChallengeApprovalEvent::RollSubmitted { world_id, .. } => *world_id,
            ChallengeApprovalEvent::Resolved { world_id, .. } => *world_id,
            ChallengeApprovalEvent::SuggestionsReady { world_id, .. } => *world_id,
            ChallengeApprovalEvent::BranchesReady { world_id, .. } => *world_id,
            ChallengeApprovalEvent::StatUpdated { world_id, .. } => *world_id,
        }
    }

    /// Map a ChallengeApprovalEvent to a GameEvent
    fn map_to_game_event(&self, event: ChallengeApprovalEvent) -> GameEvent {
        match event {
            ChallengeApprovalEvent::RollSubmitted {
                world_id,
                resolution_id,
                challenge_id,
                challenge_name,
                character_id,
                character_name,
                roll,
                modifier,
                total,
                outcome_type,
                outcome_description,
                roll_breakdown,
                outcome_triggers,
            } => GameEvent::ChallengeRollSubmitted {
                world_id,
                resolution_id,
                challenge_id,
                challenge_name,
                character_id,
                character_name,
                roll,
                modifier,
                total,
                outcome_type,
                outcome_description,
                roll_breakdown,
                individual_rolls: None,
                outcome_triggers: convert_triggers(outcome_triggers),
            },

            ChallengeApprovalEvent::Resolved {
                world_id,
                challenge_id,
                challenge_name,
                character_name,
                roll,
                modifier,
                total,
                outcome,
                outcome_description,
                roll_breakdown,
                individual_rolls,
            } => GameEvent::ChallengeResolved {
                world_id,
                challenge_id,
                challenge_name,
                character_name,
                roll,
                modifier,
                total,
                outcome,
                outcome_description,
                roll_breakdown,
                individual_rolls,
                state_changes: vec![], // State changes broadcast separately
            },

            ChallengeApprovalEvent::SuggestionsReady {
                world_id: _,
                resolution_id,
                suggestions,
            } => GameEvent::ChallengeSuggestionsReady {
                resolution_id,
                suggestions,
            },

            ChallengeApprovalEvent::BranchesReady {
                world_id: _,
                resolution_id,
                outcome_type,
                branches,
            } => GameEvent::ChallengeBranchesReady {
                resolution_id,
                outcome_type,
                branches: convert_branches(branches),
            },

            ChallengeApprovalEvent::StatUpdated {
                world_id,
                character_id,
                character_name,
                stat_name,
                old_value,
                new_value,
                delta,
            } => GameEvent::CharacterStatUpdated {
                world_id,
                character_id,
                character_name,
                stat_name,
                old_value,
                new_value,
                delta,
                source: "challenge_outcome".to_string(),
            },
        }
    }
}

/// Convert internal trigger data to port trigger info
fn convert_triggers(triggers: Vec<OutcomeTriggerData>) -> Vec<OutcomeTriggerInfo> {
    triggers
        .into_iter()
        .map(|t| OutcomeTriggerInfo {
            id: t.id,
            name: t.name,
            description: t.description,
            arguments: t.arguments,
        })
        .collect()
}

/// Convert internal branch data to port branch info
fn convert_branches(branches: Vec<OutcomeBranchData>) -> Vec<OutcomeBranchInfo> {
    branches
        .into_iter()
        .map(|b| OutcomeBranchInfo {
            branch_id: b.id,
            title: b.title,
            description: b.description,
            effects: b.effects,
        })
        .collect()
}
