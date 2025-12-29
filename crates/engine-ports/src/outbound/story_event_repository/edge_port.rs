//! Edge relationship management for StoryEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{
    ChallengeId, CharacterId, InvolvedCharacter, LocationId, NarrativeEventId, SceneId,
    StoryEventId,
};

/// Edge relationship management for StoryEvent entities.
///
/// This trait manages edges between StoryEvent nodes and other entities:
/// - OCCURRED_AT (Location) - Where the event happened
/// - OCCURRED_IN_SCENE (Scene) - Which scene context
/// - INVOLVES (Character) - Characters involved with roles
/// - TRIGGERED_BY_NARRATIVE (NarrativeEvent) - Narrative event trigger
/// - RECORDS_CHALLENGE (Challenge) - Challenge outcomes recorded
///
/// # Used By
/// - `StoryEventServiceImpl` - For managing event relationships
/// - `ChallengeResolutionService` - For recording challenge outcomes
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventEdgePort: Send + Sync {
    // =========================================================================
    // OCCURRED_AT Edge Methods (Location)
    // =========================================================================

    /// Set the location where event occurred (creates OCCURRED_AT edge)
    async fn set_location(&self, event_id: StoryEventId, location_id: LocationId) -> Result<bool>;

    /// Get the location where event occurred
    async fn get_location(&self, event_id: StoryEventId) -> Result<Option<LocationId>>;

    /// Remove location association (deletes OCCURRED_AT edge)
    async fn remove_location(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // OCCURRED_IN_SCENE Edge Methods
    // =========================================================================

    /// Set the scene where event occurred (creates OCCURRED_IN_SCENE edge)
    async fn set_scene(&self, event_id: StoryEventId, scene_id: SceneId) -> Result<bool>;

    /// Get the scene where event occurred
    async fn get_scene(&self, event_id: StoryEventId) -> Result<Option<SceneId>>;

    /// Remove scene association (deletes OCCURRED_IN_SCENE edge)
    async fn remove_scene(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // INVOLVES Edge Methods
    // =========================================================================

    /// Add an involved character (creates INVOLVES edge with role)
    async fn add_involved_character(
        &self,
        event_id: StoryEventId,
        involved: InvolvedCharacter,
    ) -> Result<bool>;

    /// Get all involved characters for an event
    async fn get_involved_characters(
        &self,
        event_id: StoryEventId,
    ) -> Result<Vec<InvolvedCharacter>>;

    /// Remove an involved character (deletes INVOLVES edge)
    async fn remove_involved_character(
        &self,
        event_id: StoryEventId,
        character_id: CharacterId,
    ) -> Result<bool>;

    // =========================================================================
    // TRIGGERED_BY_NARRATIVE Edge Methods
    // =========================================================================

    /// Set the narrative event that triggered this story event
    async fn set_triggered_by(
        &self,
        event_id: StoryEventId,
        narrative_event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Get the narrative event that triggered this story event
    async fn get_triggered_by(&self, event_id: StoryEventId) -> Result<Option<NarrativeEventId>>;

    /// Remove the triggered_by association
    async fn remove_triggered_by(&self, event_id: StoryEventId) -> Result<bool>;

    // =========================================================================
    // RECORDS_CHALLENGE Edge Methods
    // =========================================================================

    /// Set the challenge this event records (creates RECORDS_CHALLENGE edge)
    async fn set_recorded_challenge(
        &self,
        event_id: StoryEventId,
        challenge_id: ChallengeId,
    ) -> Result<bool>;

    /// Get the challenge this event records
    async fn get_recorded_challenge(&self, event_id: StoryEventId) -> Result<Option<ChallengeId>>;

    /// Remove the recorded challenge association
    async fn remove_recorded_challenge(&self, event_id: StoryEventId) -> Result<bool>;
}
