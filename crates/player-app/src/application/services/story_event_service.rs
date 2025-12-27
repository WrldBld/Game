//! Story Event Service - Application service for story event management
//!
//! This service provides use case implementations for listing, creating,
//! and managing story events (timeline events) via WebSocket request/response pattern.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::dto::StoryEventData;
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use wrldbldr_player_ports::outbound::GameConnectionPort;
use wrldbldr_protocol::requests::CreateDmMarkerData;
use wrldbldr_protocol::RequestPayload;

/// Paginated response wrapper from Engine
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
/// using WebSocket request/response pattern via the `GameConnectionPort`.
#[derive(Clone)]
pub struct StoryEventService {
    connection: Arc<dyn GameConnectionPort>,
}

impl StoryEventService {
    /// Create a new StoryEventService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// List all story events for a world
    pub async fn list_story_events(
        &self,
        world_id: &str,
    ) -> Result<Vec<StoryEventData>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::ListStoryEvents {
                    world_id: world_id.to_string(),
                    page: None,
                    page_size: None,
                },
                get_request_timeout_ms(),
            )
            .await?;

        // The response might be paginated or just a list
        result.parse()
    }

    /// Toggle event visibility
    pub async fn toggle_event_visibility(
        &self,
        event_id: &str,
        visible: bool,
    ) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::SetStoryEventVisibility {
                    event_id: event_id.to_string(),
                    visible,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Create a DM marker
    pub async fn create_dm_marker(
        &self,
        world_id: &str,
        request: &CreateDmMarkerRequest,
    ) -> Result<(), ServiceError> {
        let data = CreateDmMarkerData {
            title: request.title.clone(),
            content: Some(request.note.clone()),
        };

        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::CreateDmMarker {
                    world_id: world_id.to_string(),
                    data,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }
}
