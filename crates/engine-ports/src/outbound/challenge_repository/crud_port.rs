//! Core CRUD and state management for Challenge entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{Challenge, ChallengeId, LocationId, SceneId, WorldId};

/// Core CRUD and state management operations for Challenge entities.
///
/// This trait covers:
/// - Basic entity operations (create, get, update, delete)
/// - List operations by world, scene, location (all, active, favorites)
/// - State toggles (favorite, active)
///
/// # Used By
/// - `ChallengeServiceImpl` - For all CRUD operations
/// - `ChallengeResolutionService` - For getting and updating challenges
/// - `TriggerEvaluationService` - For listing active challenges
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengeCrudPort: Send + Sync {
    /// Create a new challenge
    async fn create(&self, challenge: &Challenge) -> Result<()>;

    /// Get a challenge by ID
    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>>;

    /// List all challenges for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List challenges for a specific scene (via TIED_TO_SCENE edge)
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>>;

    /// List challenges available at a location (via AVAILABLE_AT edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>>;

    /// List active challenges for a world (for LLM context)
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List favorite challenges for quick access
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// Update a challenge
    async fn update(&self, challenge: &Challenge) -> Result<()>;

    /// Delete a challenge
    async fn delete(&self, id: ChallengeId) -> Result<()>;

    /// Set active status for a challenge
    async fn set_active(&self, id: ChallengeId, active: bool) -> Result<()>;

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: ChallengeId) -> Result<bool>;
}
