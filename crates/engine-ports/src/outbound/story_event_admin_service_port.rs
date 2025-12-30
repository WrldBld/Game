//! Story event admin service port - Interface for DM/admin story event operations
//!
//! This port abstracts admin-level story event mutations from infrastructure,
//! providing operations for updating, hiding, tagging, and deleting story events.
//! These operations are typically used by the DM/admin interface rather than
//! regular gameplay.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{StoryEventId, WorldId};

/// Port for DM/admin story event operations
///
/// This trait defines administrative operations for story event management,
/// including updating summaries, managing visibility, updating tags, and deletion.
/// These operations are separated from the core `StoryEventServicePort` to follow
/// Interface Segregation Principle (ISP).
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventAdminServicePort: Send + Sync {
    // =========================================================================
    // Core Admin Methods
    // =========================================================================

    /// Update story event summary
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the story event to update
    /// * `summary` - The new summary text
    ///
    /// # Returns
    ///
    /// `true` if the event was found and updated, `false` if not found
    async fn update_summary(&self, event_id: StoryEventId, summary: &str) -> Result<bool>;

    /// Update event visibility (hidden/shown)
    ///
    /// Hidden events are not displayed to players but remain in the timeline.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the story event to update
    /// * `is_hidden` - Whether the event should be hidden from players
    ///
    /// # Returns
    ///
    /// `true` if the event was found and updated, `false` if not found
    async fn set_hidden(&self, event_id: StoryEventId, is_hidden: bool) -> Result<bool>;

    /// Update event tags
    ///
    /// Tags are used for categorization and filtering of story events.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the story event to update
    /// * `tags` - The new list of tags (replaces existing tags)
    ///
    /// # Returns
    ///
    /// `true` if the event was found and updated, `false` if not found
    async fn update_tags(&self, event_id: StoryEventId, tags: Vec<String>) -> Result<bool>;

    /// Delete a story event
    ///
    /// Permanently removes the story event from the timeline.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the story event to delete
    ///
    /// # Returns
    ///
    /// `true` if the event was found and deleted, `false` if not found
    async fn delete(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // Handler Convenience Methods
    // =========================================================================

    /// Update a story event with optional summary and visibility
    ///
    /// This is a convenience method for handlers that need to update
    /// multiple fields at once.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the story event to update
    /// * `summary` - Optional new summary text
    /// * `player_visible` - Optional visibility setting (true = visible, false = hidden)
    async fn update_event(
        &self,
        id: StoryEventId,
        summary: Option<String>,
        player_visible: Option<bool>,
    ) -> Result<()>;

    /// Set visibility of a story event
    ///
    /// This is a convenience wrapper around `set_hidden` with inverted semantics
    /// (visible vs hidden).
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the story event to update
    /// * `visible` - Whether the event should be visible to players
    async fn set_visibility(&self, id: StoryEventId, visible: bool) -> Result<()>;

    /// Create a DM marker event
    ///
    /// DM markers are special story events used to annotate the timeline
    /// with notes, reminders, or other administrative information.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world this marker belongs to
    /// * `title` - The marker title
    /// * `content` - Optional content/description for the marker
    ///
    /// # Returns
    ///
    /// The ID of the newly created marker event
    async fn create_dm_marker(
        &self,
        world_id: WorldId,
        title: String,
        content: Option<String>,
    ) -> Result<StoryEventId>;
}
