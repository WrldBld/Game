//! Generation Service - Application service for generation queue management
//!
//! This service provides use case implementations for managing the generation queue,
//! including hydrating queue state from the Engine and syncing read state back to it
//! via WebSocket request/response pattern.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::ports::outbound::GameConnectionPort;
use wrldbldr_protocol::RequestPayload;

/// DTO for batch status information from the Engine
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchInfo {
    pub batch_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub status: String,
    pub position: Option<u32>,
    pub progress: Option<u8>,
    pub asset_count: Option<u32>,
    pub error: Option<String>,
    #[serde(default)]
    pub is_read: bool,
}

/// DTO for suggestion task information from the Engine
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SuggestionInfo {
    pub request_id: String,
    pub field_type: String,
    pub entity_id: Option<String>,
    pub status: String,
    pub suggestions: Option<Vec<String>>,
    pub error: Option<String>,
    #[serde(default)]
    pub is_read: bool,
}

/// Complete generation queue snapshot from the Engine
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GenerationQueueSnapshot {
    pub batches: Vec<BatchInfo>,
    pub suggestions: Vec<SuggestionInfo>,
}

/// Generation service for managing generation queue
///
/// This service provides methods for fetching the generation queue state
/// from the Engine and syncing read/unread markers back to it via WebSocket.
#[derive(Clone)]
pub struct GenerationService {
    connection: Arc<dyn GameConnectionPort>,
}

impl GenerationService {
    /// Create a new GenerationService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// Fetch the generation queue snapshot from the Engine
    ///
    /// # Arguments
    /// * `user_id` - Optional user ID to filter queue items by user
    /// * `world_id` - World ID to scope the queue to
    pub async fn fetch_queue(
        &self,
        user_id: Option<&str>,
        world_id: &str,
    ) -> Result<GenerationQueueSnapshot, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::GetGenerationQueue {
                    world_id: world_id.to_string(),
                    user_id: user_id.map(|s| s.to_string()),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Sync generation read state to the Engine
    ///
    /// This sends read/unread markers for batches and suggestions to persist
    /// the user's read state on the backend.
    ///
    /// # Arguments
    /// * `read_batches` - List of batch IDs marked as read
    /// * `read_suggestions` - List of suggestion request IDs marked as read
    /// * `world_id` - Optional world ID to scope read markers
    pub async fn sync_read_state(
        &self,
        read_batches: Vec<String>,
        read_suggestions: Vec<String>,
        world_id: Option<&str>,
    ) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::SyncGenerationReadState {
                    world_id: world_id.unwrap_or("GLOBAL").to_string(),
                    read_batches,
                    read_suggestions,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }
}
