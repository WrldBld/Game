//! Story Event Service - Application service for story event management
//!
//! This service provides use case implementations for listing, creating,
//! and managing story events (timeline events). It abstracts away the HTTP client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};

use crate::application::dto::StoryEventData;
use crate::application::ports::outbound::{ApiError, ApiPort};

/// Paginated response wrapper from Engine API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedStoryEventsResponse {
    pub events: Vec<StoryEventData>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

/// Request to create a DM marker
#[derive(Debug, Clone, Serialize)]
pub struct CreateDmMarkerRequest {
    pub title: String,
    pub note: String,
    pub importance: String,
    pub marker_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Story event service for managing story events
///
/// This service provides methods for story event-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct StoryEventService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> StoryEventService<A> {
    /// Create a new StoryEventService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all story events for a world, optionally filtered by session
    pub async fn list_story_events(
        &self,
        world_id: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<StoryEventData>, ApiError> {
        let path = if let Some(sid) = session_id {
            format!("/api/worlds/{}/story-events?session_id={}", world_id, sid)
        } else {
            format!("/api/worlds/{}/story-events", world_id)
        };

        let paginated: PaginatedStoryEventsResponse = self.api.get(&path).await?;
        Ok(paginated.events)
    }

    /// Toggle event visibility
    pub async fn toggle_event_visibility(&self, event_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/story-events/{}/visibility", event_id);
        self.api.put_empty(&path).await
    }

    /// Create a DM marker
    pub async fn create_dm_marker(
        &self,
        world_id: &str,
        session_id: Option<&str>,
        request: &CreateDmMarkerRequest,
    ) -> Result<(), ApiError> {
        let path = if let Some(sid) = session_id {
            format!("/api/sessions/{}/story-events", sid)
        } else {
            format!("/api/worlds/{}/story-events", world_id)
        };

        self.api.post_no_response(&path, request).await
    }
}

impl<A: ApiPort + Clone> Clone for StoryEventService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
