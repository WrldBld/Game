//! Narrative event approval service port - Interface for DM approval of narrative events
//!
//! This port abstracts the DM approval workflow for narrative event suggestions.
//! When an event is suggested (by LLM or trigger evaluation), it goes through
//! DM approval before being triggered.
//!
//! # Architecture Note
//!
//! This port handles:
//! - Processing DM decisions on narrative event suggestions
//! - Triggering approved events
//! - Recording triggered events in the story timeline
//!
//! The service returns domain result types for the use case layer to broadcast.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::{NarrativeEventId, WorldId};

/// Result of a successful narrative event trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Port for narrative event approval service operations
///
/// This trait defines the application use cases for DM approval of narrative events.
/// It handles processing approval/rejection decisions and triggering approved events.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait NarrativeEventApprovalServicePort: Send + Sync {
    /// Handle DM's decision on a narrative event suggestion
    ///
    /// Returns `Ok(Some(result))` if approved and triggered successfully,
    /// `Ok(None)` if rejected, or `Err` on failure.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world where the event exists
    /// * `request_id` - The request ID that initiated this suggestion
    /// * `event_id` - The narrative event to approve/reject
    /// * `approved` - Whether the DM approved the event
    /// * `selected_outcome` - Optional specific outcome to use (defaults to first)
    async fn handle_decision(
        &self,
        world_id: WorldId,
        request_id: String,
        event_id: String,
        approved: bool,
        selected_outcome: Option<String>,
    ) -> Result<Option<NarrativeEventTriggerResult>>;
}
