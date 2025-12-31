//! Query operations for StoryEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, StoryEvent, WorldId,
};

/// Query operations for StoryEvent entities.
///
/// This trait provides read-only queries for finding story events:
/// - List by world (all, paginated, visible)
/// - Search by tags or text
/// - Find by character, location, scene
/// - Find by relationships (narrative event, challenge)
///
/// # Used By
/// - `StoryEventServiceImpl` - For listing and searching
/// - `StagingService` - For building narrative context
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventQueryPort: Send + Sync {
    /// List story events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>>;

    /// List story events for a world with pagination
    async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>>;

    /// List visible (non-hidden) story events for a world
    async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>>;

    /// Search story events by tags
    async fn search_by_tags(&self, world_id: WorldId, tags: Vec<String>)
        -> Result<Vec<StoryEvent>>;

    /// Search story events by text in summary
    async fn search_by_text(&self, world_id: WorldId, search_text: &str)
        -> Result<Vec<StoryEvent>>;

    /// List events involving a specific character
    async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>>;

    /// List events at a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>>;

    /// List events triggered by a specific narrative event
    async fn list_by_narrative_event(
        &self,
        narrative_event_id: NarrativeEventId,
    ) -> Result<Vec<StoryEvent>>;

    /// List events recording a specific challenge
    async fn list_by_challenge(&self, challenge_id: ChallengeId) -> Result<Vec<StoryEvent>>;

    /// List events that occurred in a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<StoryEvent>>;
}
