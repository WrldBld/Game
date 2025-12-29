//! Narrative event approval service - encapsulates DM approval of narrative
//! event suggestions, marking events as triggered, and recording story events.
//!
//! # Architecture Note
//!
//! This service returns domain result types. The use case layer is responsible
//! for broadcasting events via BroadcastPort.

use std::sync::Arc;

use crate::application::services::{NarrativeEventService, StoryEventService};
use thiserror::Error;
use wrldbldr_domain::{NarrativeEventId, WorldId};

/// Error type for narrative event approval operations
#[derive(Debug, Error)]
pub enum NarrativeEventApprovalError {
    #[error("Invalid event ID: {0}")]
    InvalidEventId(String),

    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Failed to load event: {0}")]
    EventLoadFailed(String),

    #[error("Event has no outcomes: {0}")]
    NoOutcomes(String),

    #[error("Failed to mark event triggered: {0}")]
    MarkTriggeredFailed(String),

    #[error("Failed to record story event: {0}")]
    StoryEventFailed(String),
}

/// Result of a successful narrative event trigger
#[derive(Debug, Clone)]
pub struct NarrativeEventTriggerResult {
    /// The event ID that was triggered
    pub event_id: NarrativeEventId,
    /// Event name
    pub event_name: String,
    /// Selected outcome description
    pub outcome_description: String,
    /// Scene direction for DM (if any)
    pub scene_direction: Option<String>,
    /// Effects that were triggered
    pub effects: Vec<String>,
}

/// Service responsible for narrative suggestion approval flows.
///
/// Returns domain result types for the use case layer to broadcast.
pub struct NarrativeEventApprovalService<N: NarrativeEventService> {
    narrative_event_service: Arc<N>,
    story_event_service: Arc<dyn StoryEventService>,
}

impl<N> NarrativeEventApprovalService<N>
where
    N: NarrativeEventService,
{
    pub fn new(
        narrative_event_service: Arc<N>,
        story_event_service: Arc<dyn StoryEventService>,
    ) -> Self {
        Self {
            narrative_event_service,
            story_event_service,
        }
    }

    /// Handle DM's decision on a narrative event suggestion.
    ///
    /// Returns `Ok(Some(result))` if approved and triggered successfully,
    /// `Ok(None)` if rejected, or `Err` on failure.
    pub async fn handle_decision(
        &self,
        _world_id: WorldId,
        request_id: String,
        event_id: String,
        approved: bool,
        selected_outcome: Option<String>,
    ) -> Result<Option<NarrativeEventTriggerResult>, NarrativeEventApprovalError> {
        tracing::debug!(
            "Received narrative event suggestion decision for {}: event={}, approved={}, outcome={:?}",
            request_id,
            event_id,
            approved,
            selected_outcome
        );

        if approved {
            let result = self.approve_and_trigger(event_id, selected_outcome).await?;
            Ok(Some(result))
        } else {
            tracing::info!(
                "DM rejected narrative event {} trigger for request {}",
                event_id,
                request_id
            );
            Ok(None)
        }
    }

    async fn approve_and_trigger(
        &self,
        event_id: String,
        selected_outcome: Option<String>,
    ) -> Result<NarrativeEventTriggerResult, NarrativeEventApprovalError> {
        // 1. Parse and load the event
        let event_uuid = uuid::Uuid::parse_str(&event_id)
            .map(NarrativeEventId::from_uuid)
            .map_err(|_| NarrativeEventApprovalError::InvalidEventId(event_id.clone()))?;

        let narrative_event = self
            .narrative_event_service
            .get(event_uuid)
            .await
            .map_err(|e| NarrativeEventApprovalError::EventLoadFailed(e.to_string()))?
            .ok_or_else(|| NarrativeEventApprovalError::EventNotFound(event_id.clone()))?;

        // 2. Find the selected outcome (or default to first)
        let outcome = if let Some(outcome_name) = &selected_outcome {
            narrative_event
                .outcomes
                .iter()
                .find(|o| o.name == *outcome_name)
                .cloned()
                .or_else(|| narrative_event.outcomes.first().cloned())
        } else {
            narrative_event.outcomes.first().cloned()
        };

        let outcome =
            outcome.ok_or_else(|| NarrativeEventApprovalError::NoOutcomes(event_id.clone()))?;

        // 3. Mark event as triggered
        self.narrative_event_service
            .mark_triggered(event_uuid, Some(outcome.name.clone()))
            .await
            .map_err(|e| NarrativeEventApprovalError::MarkTriggeredFailed(e.to_string()))?;

        // 4. Record a StoryEvent for the timeline
        let effects: Vec<String> = outcome.effects.iter().map(|e| format!("{:?}", e)).collect();

        self.story_event_service
            .record_narrative_event_triggered(
                narrative_event.world_id,
                None, // scene_id
                None, // location_id
                event_uuid,
                narrative_event.name.clone(),
                Some(outcome.name.clone()),
                effects.clone(),
                vec![], // involved_characters
                None,   // game_time
            )
            .await
            .map_err(|e| NarrativeEventApprovalError::StoryEventFailed(e.to_string()))?;

        tracing::info!(
            "Triggered narrative event '{}' with outcome '{}'",
            narrative_event.name,
            outcome.description
        );

        // 5. Return result for use case to broadcast
        Ok(NarrativeEventTriggerResult {
            event_id: event_uuid,
            event_name: narrative_event.name,
            outcome_description: outcome.description,
            scene_direction: Some(narrative_event.scene_direction),
            effects,
        })
    }
}
