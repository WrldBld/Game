//! Scene, Location, and Act relationship management for NarrativeEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{ActId, LocationId, NarrativeEventId, SceneId};

/// Scene, Location, and Act relationship management for NarrativeEvent entities.
///
/// This trait manages edges between NarrativeEvent nodes and other entities:
/// - TIED_TO_SCENE - Events tied to specific scenes
/// - TIED_TO_LOCATION - Events tied to specific locations
/// - BELONGS_TO_ACT - Events belonging to story acts
///
/// # Used By
/// - `NarrativeEventServiceImpl` - For managing event relationships
/// - `StagingService` - For checking location-based event availability
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait NarrativeEventTiePort: Send + Sync {
    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    /// Tie event to a scene (creates TIED_TO_SCENE edge)
    async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> Result<bool>;

    /// Get the scene this event is tied to (if any)
    async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>>;

    /// Remove scene tie (deletes TIED_TO_SCENE edge)
    async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    /// Tie event to a location (creates TIED_TO_LOCATION edge)
    async fn tie_to_location(
        &self,
        event_id: NarrativeEventId,
        location_id: LocationId,
    ) -> Result<bool>;

    /// Get the location this event is tied to (if any)
    async fn get_tied_location(&self, event_id: NarrativeEventId) -> Result<Option<LocationId>>;

    /// Remove location tie (deletes TIED_TO_LOCATION edge)
    async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool>;

    // =========================================================================
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    /// Assign event to an act (creates BELONGS_TO_ACT edge)
    async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool>;

    /// Get the act this event belongs to (if any)
    async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>>;

    /// Remove act assignment (deletes BELONGS_TO_ACT edge)
    async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool>;
}
