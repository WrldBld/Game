//! Query operations for Scene entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{ActId, LocationId, Scene};

/// Query operations for finding scenes.
///
/// This trait covers lookup operations that return collections
/// of scenes based on act or location.
#[async_trait]
pub trait SceneQueryPort: Send + Sync {
    /// List scenes by act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<Scene>>;

    /// List scenes by location (via AT_LOCATION edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>>;
}
