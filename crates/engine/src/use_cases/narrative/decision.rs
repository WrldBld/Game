use std::sync::Arc;

use uuid::Uuid;

use crate::entities::Narrative;
use crate::infrastructure::ports::{QueuePort, RepoError};
use crate::use_cases::approval::{ApproveSuggestion, ApprovalError};
use crate::use_cases::narrative::{EffectExecutionContext, ExecuteEffects};
use wrldbldr_domain::{DmApprovalDecision, NarrativeEventId, WorldId};

/// Narrative event suggestion approval flow.
pub struct NarrativeDecisionFlow {
    approve_suggestion: Arc<ApproveSuggestion>,
    queue: Arc<dyn QueuePort>,
    narrative: Arc<Narrative>,
    execute_effects: Arc<ExecuteEffects>,
}

impl NarrativeDecisionFlow {
    pub fn new(
        approve_suggestion: Arc<ApproveSuggestion>,
        queue: Arc<dyn QueuePort>,
        narrative: Arc<Narrative>,
        execute_effects: Arc<ExecuteEffects>,
    ) -> Self {
        Self {
            approve_suggestion,
            queue,
            narrative,
            execute_effects,
        }
    }

    pub async fn execute(
        &self,
        approval_id: Uuid,
        decision: DmApprovalDecision,
        event_id: NarrativeEventId,
        selected_outcome: Option<String>,
    ) -> Result<NarrativeDecisionOutcome, NarrativeDecisionError> {
        let approval_data = self
            .queue
            .get_approval_request(approval_id)
            .await
            .map_err(|e| NarrativeDecisionError::QueueError(e.to_string()))?
            .ok_or(NarrativeDecisionError::ApprovalNotFound)?;

        let result = self
            .approve_suggestion
            .execute(approval_id, decision)
            .await
            .map_err(NarrativeDecisionError::Approval)?;

        if !result.approved {
            tracing::info!(
                event_id = %event_id,
                "Narrative event rejected by approval system"
            );
            return Ok(NarrativeDecisionOutcome {
                world_id: approval_data.world_id,
                triggered: None,
            });
        }

        let event = match self.narrative.get_event(event_id).await? {
            Some(event) => event,
            None => return Err(NarrativeDecisionError::EventNotFound(event_id.to_string())),
        };

        let outcome_name = selected_outcome
            .or_else(|| {
                approval_data
                    .narrative_event_suggestion
                    .as_ref()
                    .and_then(|s| s.suggested_outcome.clone())
            })
            .or_else(|| event.default_outcome.clone())
            .or_else(|| event.outcomes.first().map(|o| o.name.clone()))
            .unwrap_or_default();

        let outcome = event.outcomes.iter().find(|o| o.name == outcome_name);

        if let Some(outcome) = outcome {
            if !outcome.effects.is_empty() {
                let pc_id = approval_data.pc_id.ok_or(NarrativeDecisionError::PcContextRequired)?;
                let context = EffectExecutionContext {
                    pc_id,
                    world_id: approval_data.world_id,
                    current_scene_id: approval_data.scene_id,
                };

                let summary = self
                    .execute_effects
                    .execute(event_id, outcome_name.clone(), &outcome.effects, &context)
                    .await;

                tracing::info!(
                    event_id = %event_id,
                    outcome = %outcome_name,
                    success_count = summary.success_count,
                    failure_count = summary.failure_count,
                    "Executed narrative event effects"
                );
            }
        }

        Ok(NarrativeDecisionOutcome {
            world_id: approval_data.world_id,
            triggered: Some(NarrativeTriggeredPayload {
                event_id: event_id.to_string(),
                event_name: event.name.clone(),
                outcome_description: outcome
                    .map(|o| o.description.clone())
                    .unwrap_or_default(),
                scene_direction: event.scene_direction.clone(),
            }),
        })
    }
}

pub struct NarrativeDecisionOutcome {
    pub world_id: WorldId,
    pub triggered: Option<NarrativeTriggeredPayload>,
}

pub struct NarrativeTriggeredPayload {
    pub event_id: String,
    pub event_name: String,
    pub outcome_description: String,
    pub scene_direction: String,
}

#[derive(Debug, thiserror::Error)]
pub enum NarrativeDecisionError {
    #[error("Approval request not found")]
    ApprovalNotFound,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Approval error: {0}")]
    Approval(#[from] ApprovalError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Event not found: {0}")]
    EventNotFound(String),
    #[error("PC context required for effect execution")]
    PcContextRequired,
}
