//! Narrative event approval service - encapsulates DM approval of narrative
//! event suggestions, marking events as triggered, recording story events, and
//! constructing `ServerMessage::NarrativeEventTriggered`.
//!
//! Uses `WorldConnectionPort` for world-scoped messaging, maintaining hexagonal architecture.

use std::sync::Arc;

use wrldbldr_engine_ports::outbound::WorldConnectionPort;
use crate::application::services::{NarrativeEventService, StoryEventService};
use wrldbldr_domain::{NarrativeEventId, WorldId};

/// Narrative event triggered message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct NarrativeEventTriggeredMessage {
    r#type: &'static str,
    event_id: String,
    event_name: String,
    outcome_description: String,
    scene_direction: Option<String>,
}

/// Error message DTO
#[derive(Debug, Clone, serde::Serialize)]
struct ErrorMessage {
    r#type: &'static str,
    code: String,
    message: String,
}

/// Service responsible for narrative suggestion approval flows.
///
/// # Architecture Note
///
/// This service uses `WorldConnectionPort` for world-scoped messaging,
/// maintaining hexagonal architecture boundaries.
pub struct NarrativeEventApprovalService<N: NarrativeEventService> {
    world_connection: Arc<dyn WorldConnectionPort>,
    narrative_event_service: Arc<N>,
    story_event_service: Arc<dyn StoryEventService>,
}

impl<N> NarrativeEventApprovalService<N>
where
    N: NarrativeEventService,
{
    pub fn new(
        world_connection: Arc<dyn WorldConnectionPort>,
        narrative_event_service: Arc<N>,
        story_event_service: Arc<dyn StoryEventService>,
    ) -> Self {
        Self {
            world_connection,
            narrative_event_service,
            story_event_service,
        }
    }

    /// Handle `ClientMessage::NarrativeEventSuggestionDecision`.
    pub async fn handle_decision(
        &self,
        world_id: WorldId,
        request_id: String,
        event_id: String,
        approved: bool,
        selected_outcome: Option<String>,
    ) -> Option<serde_json::Value> {
        tracing::debug!(
            "Received narrative event suggestion decision for {}: event={}, approved={}, outcome={:?}",
            request_id,
            event_id,
            approved,
            selected_outcome
        );

        if approved {
            return self
                .approve_and_trigger(
                    world_id,
                    request_id,
                    event_id,
                    selected_outcome,
                )
                .await;
        } else {
            tracing::info!(
                "DM rejected narrative event {} trigger for request {}",
                event_id,
                request_id
            );
        }

        None
    }

    async fn approve_and_trigger(
        &self,
        world_id: WorldId,
        _request_id: String,
        event_id: String,
        selected_outcome: Option<String>,
    ) -> Option<serde_json::Value> {
        let event_uuid = match uuid::Uuid::parse_str(&event_id) {
            Ok(uuid) => NarrativeEventId::from_uuid(uuid),
            Err(_) => {
                tracing::error!("Invalid event_id: {}", event_id);
                let error_msg = ErrorMessage {
                    r#type: "Error",
                    code: "INVALID_EVENT_ID".to_string(),
                    message: "Invalid narrative event ID format".to_string(),
                };
                return serde_json::to_value(&error_msg).ok();
            }
        };

        let narrative_event = match self.narrative_event_service.get(event_uuid).await {
            Ok(Some(event)) => event,
            Ok(None) => {
                tracing::error!("Narrative event {} not found", event_id);
                let error_msg = ErrorMessage {
                    r#type: "Error",
                    code: "EVENT_NOT_FOUND".to_string(),
                    message: format!("Narrative event {} not found", event_id),
                };
                return serde_json::to_value(&error_msg).ok();
            }
            Err(e) => {
                tracing::error!("Failed to load narrative event: {}", e);
                let error_msg = ErrorMessage {
                    r#type: "Error",
                    code: "EVENT_LOAD_ERROR".to_string(),
                    message: format!("Failed to load narrative event: {}", e),
                };
                return serde_json::to_value(&error_msg).ok();
            }
        };

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

        let outcome = match outcome {
            Some(o) => o,
            None => {
                tracing::error!("Narrative event {} has no outcomes", event_id);
                let error_msg = ErrorMessage {
                    r#type: "Error",
                    code: "NO_OUTCOMES".to_string(),
                    message: format!("Narrative event {} has no outcomes", event_id),
                };
                return serde_json::to_value(&error_msg).ok();
            }
        };

        // 3. Mark event as triggered
        if let Err(e) = self
            .narrative_event_service
            .mark_triggered(event_uuid, Some(outcome.name.clone()))
            .await
        {
            tracing::error!("Failed to mark narrative event as triggered: {}", e);
        }

        // 4. Record a StoryEvent for the timeline
        if let Err(e) = self
            .story_event_service
            .record_narrative_event_triggered(
                narrative_event.world_id,
                None, // scene_id
                None, // location_id
                event_uuid,
                narrative_event.name.clone(),
                Some(outcome.name.clone()),
                outcome
                    .effects
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>(),
                vec![], // involved_characters
                None,   // game_time
            )
            .await
        {
            tracing::error!("Failed to record story event: {}", e);
        }

        // 5. Broadcast scene direction to DM via the world connection port
        use wrldbldr_protocol::ServerMessage;
        let server_msg = ServerMessage::NarrativeEventTriggered {
            event_id: event_id.clone(),
            event_name: narrative_event.name.clone(),
            outcome_description: outcome.description.clone(),
            scene_direction: narrative_event.scene_direction.clone(),
        };
        if let Err(e) = self.world_connection.send_to_dm(&world_id, server_msg).await {
            tracing::error!("Failed to send NarrativeEventTriggered to DM: {}", e);
        }

        tracing::info!(
            "Triggered narrative event '{}' with outcome '{}'",
            narrative_event.name,
            outcome.description
        );

        None
    }
}


