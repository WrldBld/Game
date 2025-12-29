//! Scene relationship management for Challenge entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{ChallengeId, SceneId};

/// Scene relationship operations for Challenge entities.
///
/// This trait manages the TIED_TO_SCENE edge between Challenge and Scene nodes.
///
/// # Used By
/// - `ChallengeServiceImpl` - For tying challenges to scenes
/// - `SceneService` - For getting challenges tied to a scene
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengeScenePort: Send + Sync {
    /// Tie a challenge to a scene (creates TIED_TO_SCENE edge)
    async fn tie_to_scene(&self, challenge_id: ChallengeId, scene_id: SceneId) -> Result<()>;

    /// Get the scene a challenge is tied to
    async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>>;

    /// Remove the scene tie from a challenge
    async fn untie_from_scene(&self, challenge_id: ChallengeId) -> Result<()>;
}
