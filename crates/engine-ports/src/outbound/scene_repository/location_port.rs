//! Location relationship operations for Scene entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{LocationId, SceneId};

/// Location edge management for scenes.
///
/// This trait covers the AT_LOCATION relationship between
/// scenes and locations.
#[async_trait]
pub trait SceneLocationPort: Send + Sync {
    /// Set scene's location (creates AT_LOCATION edge)
    async fn set_location(&self, scene_id: SceneId, location_id: LocationId) -> Result<()>;

    /// Get scene's location
    async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>>;
}
