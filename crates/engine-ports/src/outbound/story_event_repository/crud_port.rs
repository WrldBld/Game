//! Core CRUD and state management for StoryEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{StoryEvent, StoryEventId, WorldId};

/// Core CRUD and state management operations for StoryEvent entities.
///
/// This trait covers:
/// - Basic entity operations (create, get, delete)
/// - State updates (summary, hidden, tags)
/// - Count operations
///
/// # Used By
/// - `StoryEventServiceImpl` - For all CRUD operations
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventCrudPort: Send + Sync {
    /// Create a new story event
    async fn create(&self, event: &StoryEvent) -> Result<()>;

    /// Get a story event by ID
    async fn get(&self, id: StoryEventId) -> Result<Option<StoryEvent>>;

    /// Update story event summary
    async fn update_summary(&self, id: StoryEventId, summary: &str) -> Result<bool>;

    /// Update event visibility
    async fn set_hidden(&self, id: StoryEventId, is_hidden: bool) -> Result<bool>;

    /// Update event tags
    async fn update_tags(&self, id: StoryEventId, tags: Vec<String>) -> Result<bool>;

    /// Delete a story event
    async fn delete(&self, id: StoryEventId) -> Result<bool>;

    /// Count events for a world
    async fn count_by_world(&self, world_id: WorldId) -> Result<u64>;
}
