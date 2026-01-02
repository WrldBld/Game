//! Story event service port - Interface for story event operations
//!
//! This port abstracts story event business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::StoryEvent;
use wrldbldr_domain::{StoryEventId, WorldId};

/// Port for story event service operations
///
/// This trait defines the core operations for story event management,
/// including querying events and recording new events to the timeline.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventServicePort: Send + Sync {
    /// Get a story event by ID
    ///
    /// Returns the story event if found, or None if not found.
    async fn get_story_event(&self, id: StoryEventId) -> Result<Option<StoryEvent>>;

    /// List story events for a world with a limit
    ///
    /// Returns the most recent story events for the specified world,
    /// up to the given limit.
    async fn list_by_world(&self, world_id: WorldId, limit: usize) -> Result<Vec<StoryEvent>>;

    /// Record a new story event
    ///
    /// Creates a new story event with the given type and summary.
    /// Returns the ID of the newly created event.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world this event belongs to
    /// * `event_type` - The type of event (e.g., "dialogue", "challenge", "scene_transition")
    /// * `summary` - A human-readable summary of the event
    async fn record_event(
        &self,
        world_id: WorldId,
        event_type: &str,
        summary: &str,
    ) -> Result<StoryEventId>;
}
