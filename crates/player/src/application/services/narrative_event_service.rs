//! Narrative Event Service - Application service for narrative event management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and managing narrative events (future story events). It abstracts
//! away the HTTP client details from the presentation layer.

use crate::application::dto::{CreateNarrativeEventRequest, NarrativeEventData};
use crate::application::ports::outbound::{ApiError, ApiPort};

/// Narrative event service for managing narrative events
///
/// This service provides methods for narrative event-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct NarrativeEventService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> NarrativeEventService<A> {
    /// Create a new NarrativeEventService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all narrative events for a world
    pub async fn list_narrative_events(
        &self,
        world_id: &str,
    ) -> Result<Vec<NarrativeEventData>, ApiError> {
        let path = format!("/api/worlds/{}/narrative-events", world_id);
        self.api.get(&path).await
    }

    /// List pending (active but not triggered) narrative events
    pub async fn list_pending_events(
        &self,
        world_id: &str,
    ) -> Result<Vec<NarrativeEventData>, ApiError> {
        let pending_path = format!("/api/worlds/{}/narrative-events/pending", world_id);

        // Try pending endpoint first
        match self.api.get::<Vec<NarrativeEventData>>(&pending_path).await {
            Ok(events) => Ok(events),
            Err(_) => {
                // Fall back to fetching all and filtering client-side
                let all: Vec<NarrativeEventData> = self.list_narrative_events(world_id).await?;
                Ok(all
                    .into_iter()
                    .filter(|e| e.is_active && !e.is_triggered)
                    .collect())
            }
        }
    }

    /// Toggle favorite status for a narrative event
    pub async fn toggle_favorite(&self, event_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/narrative-events/{}/favorite", event_id);
        self.api.post_empty(&path).await
    }

    /// Set active status for a narrative event
    pub async fn set_active(&self, event_id: &str, active: bool) -> Result<(), ApiError> {
        let path = format!("/api/narrative-events/{}/active", event_id);
        self.api.put_no_response(&path, &active).await
    }

    /// Create a new narrative event
    pub async fn create_narrative_event(
        &self,
        world_id: &str,
        request: CreateNarrativeEventRequest,
    ) -> Result<NarrativeEventData, ApiError> {
        let path = format!("/api/worlds/{}/narrative-events", world_id);
        self.api.post(&path, &request).await
    }
}

impl<A: ApiPort + Clone> Clone for NarrativeEventService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
