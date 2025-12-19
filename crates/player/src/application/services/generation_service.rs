//! Generation Service - Application service for generation queue management
//!
//! This service provides use case implementations for managing the generation queue,
//! including hydrating queue state from the Engine and syncing read state back to it.

use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{ApiError, ApiPort};

/// DTO for batch status information from the Engine
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
pub struct GenerationQueueSnapshot {
    pub batches: Vec<BatchInfo>,
    pub suggestions: Vec<SuggestionInfo>,
}

/// Request to sync read state to the Engine
#[derive(Clone, Debug, Serialize)]
pub struct SyncReadStateRequest {
    pub read_batches: Vec<String>,
    pub read_suggestions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_id: Option<String>,
}

/// Generation service for managing generation queue
///
/// This service provides methods for fetching the generation queue state
/// from the Engine and syncing read/unread markers back to it.
pub struct GenerationService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> GenerationService<A> {
    /// Create a new GenerationService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
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
    ) -> Result<GenerationQueueSnapshot, ApiError> {
        let path = if let Some(uid) = user_id {
            format!("/api/generation/queue?user_id={}&world_id={}", uid, world_id)
        } else {
            format!("/api/generation/queue?world_id={}", world_id)
        };
        self.api.get(&path).await
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
    ) -> Result<(), ApiError> {
        let request = SyncReadStateRequest {
            read_batches,
            read_suggestions,
            world_id: world_id.map(|s| s.to_string()),
        };
        self.api.post_no_response("/api/generation/read-state", &request).await
    }
}

impl<A: ApiPort + Clone> Clone for GenerationService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
