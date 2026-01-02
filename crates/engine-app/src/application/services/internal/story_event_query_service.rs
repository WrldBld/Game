//! Story event query service port - Interface for read-only story event query operations
//!
//! This port abstracts story event query/read operations from infrastructure adapters.
//! It provides methods for retrieving and searching story events without mutation.
//!
//! # Design Notes
//!
//! This port follows the Interface Segregation Principle (ISP) by separating
//! read-only query operations from mutation operations. Services that only need
//! to query story events can depend on this trait rather than the full service.
//!
//! # Usage
//!
//! Infrastructure adapters and use cases that need to query story event data
//! should depend on this trait rather than importing the service directly,
//! maintaining proper hexagonal architecture boundaries.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::StoryEvent;
use wrldbldr_domain::{CharacterId, LocationId, StoryEventId, WorldId};

/// Port for read-only story event query operations.
///
/// This trait provides query access to story event data for use in
/// building prompts, gathering context, timeline display, and search.
///
/// # Methods
///
/// - `get_event` - Get a single event by ID
/// - `list_by_world` - List all events for a world
/// - `list_by_world_paginated` - List events with offset/limit pagination
/// - `list_visible` - List non-hidden events for player display
/// - `search_by_tags` - Search events by tag filter
/// - `search_by_text` - Full-text search in summaries
/// - `list_by_character` - List events involving a character
/// - `list_by_location` - List events at a location
/// - `count_by_world` - Count total events for a world
/// - `list_events` - Handler convenience method with page/page_size pagination
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventQueryServicePort: Send + Sync {
    /// Get a single story event by ID.
    ///
    /// Returns `Ok(Some(event))` if found, `Ok(None)` if not found.
    async fn get_event(&self, event_id: StoryEventId) -> Result<Option<StoryEvent>>;

    /// List story events for a world.
    ///
    /// Returns all story events belonging to the specified world,
    /// typically ordered by timestamp descending (most recent first).
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>>;

    /// List story events for a world with pagination.
    ///
    /// Returns a page of story events using offset/limit pagination.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to query events from
    /// * `limit` - Maximum number of events to return
    /// * `offset` - Number of events to skip
    async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>>;

    /// List visible (non-hidden) story events for a world.
    ///
    /// Returns events that should be displayed to players (not marked as hidden),
    /// limited to the specified count.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to query events from
    /// * `limit` - Maximum number of events to return
    async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>>;

    /// Search story events by tags.
    ///
    /// Returns events that match any of the provided tags.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to search within
    /// * `tags` - List of tags to match (OR logic)
    async fn search_by_tags(&self, world_id: WorldId, tags: Vec<String>)
        -> Result<Vec<StoryEvent>>;

    /// Search story events by text in summary.
    ///
    /// Performs a case-insensitive search in event summaries.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to search within
    /// * `search_text` - Text to search for in summaries
    async fn search_by_text(&self, world_id: WorldId, search_text: &str)
        -> Result<Vec<StoryEvent>>;

    /// List events involving a specific character.
    ///
    /// Returns events where the character is a participant,
    /// connected via INVOLVES_CHARACTER edges.
    async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>>;

    /// List events at a specific location.
    ///
    /// Returns events that occurred at the specified location,
    /// connected via AT_LOCATION edges.
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>>;

    /// Count events for a world.
    ///
    /// Returns the total number of story events in the specified world.
    async fn count_by_world(&self, world_id: WorldId) -> Result<u64>;

    /// List story events with optional pagination (page/page_size style).
    ///
    /// This is a handler convenience method using page-based pagination
    /// instead of offset/limit. If page or page_size is None, returns
    /// all events (up to a reasonable default limit).
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to query events from
    /// * `page` - Optional page number (1-indexed)
    /// * `page_size` - Optional number of events per page
    async fn list_events(
        &self,
        world_id: WorldId,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Vec<StoryEvent>>;
}
