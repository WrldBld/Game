//! Generation Queue Projection Service Port - Interface for projecting generation queue state
//!
//! This port abstracts the generation queue snapshot projection, allowing handlers
//! to retrieve unified queue state without depending on application service internals.
//!
//! # Architecture
//!
//! The projection service combines data from multiple sources:
//! - Active generation batches from the asset service
//! - Suggestion tasks from domain events
//! - Per-user read markers from read state storage
//!
//! # Design Notes
//!
//! This is an application-layer service port because it orchestrates multiple
//! data sources and applies user-specific filtering (read markers). It's not
//! a pure domain port.

use async_trait::async_trait;
use serde::Serialize;

use wrldbldr_domain::WorldId;

/// Snapshot of a suggestion task's current state
#[derive(Debug, Clone, Serialize)]
pub struct SuggestionTaskSnapshot {
    /// Unique request identifier
    pub request_id: String,
    /// Type of field being suggested (e.g., "name", "description")
    pub field_type: String,
    /// Entity ID if applicable
    pub entity_id: Option<String>,
    /// Current status: "queued", "processing", "ready", "failed"
    pub status: String,
    /// Generated suggestions (when status is "ready")
    pub suggestions: Option<Vec<String>>,
    /// Error message (when status is "failed")
    pub error: Option<String>,
    /// Whether the user has marked this as read
    pub is_read: bool,
}

/// Snapshot of a generation batch with read state
#[derive(Debug, Clone, Serialize)]
pub struct GenerationBatchSnapshot {
    /// Batch identifier
    pub id: String,
    /// World this batch belongs to
    pub world_id: String,
    /// Entity type being generated
    pub entity_type: String,
    /// Entity ID being generated for
    pub entity_id: Option<String>,
    /// Current status
    pub status: String,
    /// Number of items in the batch
    pub item_count: usize,
    /// Number of completed items
    pub completed_count: usize,
    /// Whether the user has marked this as read
    pub is_read: bool,
}

/// Unified snapshot of the generation queue state
///
/// Contains both image generation batches and text suggestion tasks,
/// with per-user read markers applied.
#[derive(Debug, Clone, Serialize)]
pub struct GenerationQueueSnapshot {
    /// Image generation batches
    pub batches: Vec<GenerationBatchSnapshot>,
    /// Text suggestion tasks
    pub suggestions: Vec<SuggestionTaskSnapshot>,
}

/// Port for projecting generation queue state
///
/// This trait provides a unified view of all generation tasks
/// (image batches and text suggestions) with user-specific read markers.
///
/// # Usage
///
/// HTTP handlers and WebSocket projections use this to get the current
/// queue state for a user/world combination.
///
/// # Testing
///
/// Enable the `testing` feature to get mock implementations via mockall.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait GenerationQueueProjectionServicePort: Send + Sync {
    /// Build a generation queue snapshot for a user and world
    ///
    /// # Arguments
    ///
    /// * `user_id` - Optional user ID for applying read markers.
    ///               If `None`, all items are treated as unread.
    /// * `world_id` - The world to project queue state for.
    ///
    /// # Returns
    ///
    /// A unified snapshot containing all active batches and suggestion tasks.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let snapshot = projection_service
    ///     .project_queue(Some("user123".to_string()), world_id)
    ///     .await?;
    ///
    /// for batch in snapshot.batches {
    ///     if !batch.is_read {
    ///         // Show unread notification
    ///     }
    /// }
    /// ```
    async fn project_queue(
        &self,
        user_id: Option<String>,
        world_id: WorldId,
    ) -> anyhow::Result<GenerationQueueSnapshot>;
}
