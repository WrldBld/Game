//! Query operations for NarrativeEvent entities by relationships.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{ActId, CharacterId, LocationId, NarrativeEvent, SceneId};

/// Query operations for NarrativeEvent entities by edge relationships.
///
/// This trait provides read-only queries that find events by their relationships:
/// - Events tied to a specific scene
/// - Events tied to a specific location
/// - Events belonging to a specific act
/// - Events featuring a specific NPC
///
/// # Used By
/// - `StagingService` - For finding relevant events for a location/scene
/// - `ActantialContextService` - For building narrative context
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait NarrativeEventQueryPort: Send + Sync {
    /// List events tied to a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>>;

    /// List events tied to a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>>;

    /// List events belonging to a specific act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>>;

    /// List events featuring a specific NPC
    async fn list_by_featured_npc(&self, character_id: CharacterId) -> Result<Vec<NarrativeEvent>>;
}
