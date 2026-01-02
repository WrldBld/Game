//! Narrative Event Use Case
//!
//! Handles DM approval of narrative event suggestions.
//!
//! # Responsibilities
//!
//! - Handle DM decision on narrative event suggestions
//! - Broadcast approved events to all players
//!
//! # Architecture Note
//!
//! This use case delegates to `NarrativeEventApprovalService` for the core
//! approval logic, then broadcasts the result via `BroadcastPort`.

use async_trait::async_trait;
use std::sync::Arc;

use wrldbldr_engine_ports::inbound::{NarrativeEventUseCasePort, UseCaseContext};
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, GameEvent, NarrativeEventApprovalServicePort,
};

use super::errors::NarrativeEventError;

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{
    NarrativeEventDecisionResult as DecisionResult,
    NarrativeEventSuggestionDecisionInput as SuggestionDecisionInput,
};

// =============================================================================
// Use Case
// =============================================================================

pub struct NarrativeEventUseCase {
    approval_service: Arc<dyn NarrativeEventApprovalServicePort>,
    broadcast_port: Arc<dyn BroadcastPort>,
}

impl NarrativeEventUseCase {
    pub fn new(
        approval_service: Arc<dyn NarrativeEventApprovalServicePort>,
        broadcast_port: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            approval_service,
            broadcast_port,
        }
    }

    /// Handle DM's decision on a narrative event suggestion
    ///
    /// If approved, triggers the event and broadcasts to all players.
    /// If rejected, no broadcast is sent.
    pub async fn handle_suggestion_decision(
        &self,
        ctx: UseCaseContext,
        input: SuggestionDecisionInput,
    ) -> Result<DecisionResult, NarrativeEventError> {
        // Verify DM authorization
        if !ctx.is_dm {
            return Err(NarrativeEventError::Unauthorized(
                "Only DM can approve narrative events".to_string(),
            ));
        }

        // Delegate to approval service
        let result = self
            .approval_service
            .handle_decision(
                ctx.world_id,
                input.request_id,
                input.event_id.clone(),
                input.approved,
                input.selected_outcome,
            )
            .await
            .map_err(|e| NarrativeEventError::ApprovalFailed(e.to_string()))?;

        // If approved, broadcast the event
        if let Some(trigger_result) = result {
            self.broadcast_port
                .broadcast(
                    ctx.world_id,
                    GameEvent::NarrativeEventTriggered {
                        event_id: trigger_result.event_id.to_string(),
                        event_name: trigger_result.event_name,
                        outcome_description: trigger_result.outcome_description,
                        scene_direction: trigger_result.scene_direction,
                    },
                )
                .await;

            Ok(DecisionResult { triggered: true })
        } else {
            Ok(DecisionResult { triggered: false })
        }
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl NarrativeEventUseCasePort for NarrativeEventUseCase {
    async fn handle_suggestion_decision(
        &self,
        ctx: UseCaseContext,
        input: SuggestionDecisionInput,
    ) -> Result<DecisionResult, NarrativeEventError> {
        self.handle_suggestion_decision(ctx, input).await
    }
}
