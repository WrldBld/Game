//! Narrative Event Service - Application service for narrative event management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and managing narrative events (future story events). It uses
//! WebSocket for real-time communication with the Engine.

use wrldbldr_shared::{NarrativeEventRequest, RequestPayload};

use crate::application::dto::{CreateNarrativeEventRequest, NarrativeEventData};
use crate::application::error::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;

/// Narrative event service for managing narrative events
///
/// This service provides methods for narrative event-related operations
/// while depending only on the `CommandBus`, not concrete
/// infrastructure implementations.
#[derive(Clone)]
pub struct NarrativeEventService {
    commands: CommandBus,
}

impl NarrativeEventService {
    /// Create a new NarrativeEventService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// List all narrative events for a world
    pub async fn list_narrative_events(
        &self,
        world_id: &str,
    ) -> Result<Vec<NarrativeEventData>, ServiceError> {
        let payload = RequestPayload::NarrativeEvent(NarrativeEventRequest::ListNarrativeEvents {
            world_id: world_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }

    /// List pending (active but not triggered) narrative events
    ///
    /// This fetches all events and filters client-side for those that are
    /// active but not yet triggered.
    pub async fn list_pending_events(
        &self,
        world_id: &str,
    ) -> Result<Vec<NarrativeEventData>, ServiceError> {
        let all_events = self.list_narrative_events(world_id).await?;
        Ok(all_events
            .into_iter()
            .filter(|e| e.is_active && !e.is_triggered)
            .collect())
    }

    /// Toggle favorite status for a narrative event
    ///
    /// Returns the new favorite state after toggling
    pub async fn toggle_favorite(&self, event_id: &str) -> Result<bool, ServiceError> {
        // First get current state by fetching the event
        let payload = RequestPayload::NarrativeEvent(NarrativeEventRequest::GetNarrativeEvent {
            event_id: event_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        let event: NarrativeEventData = response.parse()?;
        let new_favorite = !event.is_favorite;

        // Set new state
        let set_payload =
            RequestPayload::NarrativeEvent(NarrativeEventRequest::SetNarrativeEventFavorite {
                event_id: event_id.to_string(),
                favorite: new_favorite,
            });
        let set_response = self
            .commands
            .request_with_timeout(set_payload, get_request_timeout_ms())
            .await?;
        set_response.parse_empty()?;
        Ok(new_favorite)
    }

    /// Set active status for a narrative event
    pub async fn set_active(&self, event_id: &str, active: bool) -> Result<(), ServiceError> {
        let payload =
            RequestPayload::NarrativeEvent(NarrativeEventRequest::SetNarrativeEventActive {
                event_id: event_id.to_string(),
                active,
            });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse_empty()
    }

    /// Create a new narrative event
    pub async fn create_narrative_event(
        &self,
        world_id: &str,
        request: CreateNarrativeEventRequest,
    ) -> Result<NarrativeEventData, ServiceError> {
        // Build trigger conditions JSON if provided
        let trigger_conditions = match (&request.trigger_conditions, &request.trigger_logic) {
            (Some(conditions), Some(logic)) if !conditions.is_empty() => Some(serde_json::json!({
                "logic": logic,
                "conditions": conditions
            })),
            (Some(conditions), None) if !conditions.is_empty() => Some(serde_json::json!({
                "logic": "all",
                "conditions": conditions
            })),
            _ => None,
        };

        let data = wrldbldr_shared::CreateNarrativeEventData {
            name: request.name,
            description: Some(request.description),
            trigger_conditions,
            outcomes: None,
        };

        let payload = RequestPayload::NarrativeEvent(NarrativeEventRequest::CreateNarrativeEvent {
            world_id: world_id.to_string(),
            data,
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::messaging::{BusMessage, PendingRequests};
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    fn create_test_command_bus() -> (CommandBus, mpsc::Receiver<BusMessage>) {
        let (tx, rx) = mpsc::channel(10);
        let pending = Arc::new(Mutex::new(PendingRequests::default()));
        (CommandBus::new(tx, pending), rx)
    }

    #[tokio::test]
    async fn list_narrative_events_sends_correct_payload() {
        let (commands, mut rx) = create_test_command_bus();
        let svc = NarrativeEventService::new(commands);

        // The request will timeout since there's no server, but we can verify a message was sent
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            svc.list_narrative_events("world-1"),
        )
        .await;

        // Verify that a request message was sent
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, BusMessage::Request { .. }));
    }
}
