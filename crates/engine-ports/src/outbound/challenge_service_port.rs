//! Challenge service port - Interface for challenge operations
//!
//! This port abstracts challenge business logic from infrastructure adapters.
//! It exposes a subset of ChallengeService methods that adapters actually need,
//! keeping the interface minimal and focused.
//!
//! # Design Notes
//!
//! This port is designed for use by infrastructure adapters (e.g., WebSocket helpers)
//! that need to query challenge information. It intentionally does NOT expose
//! mutation methods (create, update, delete) as those should go through proper
//! use case handlers, not infrastructure adapters.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{
    Challenge, ChallengeLocationAvailability, ChallengePrerequisite,
};
use wrldbldr_domain::{ChallengeId, LocationId, SceneId, SkillId, WorldId};

/// Port for challenge service operations used by infrastructure adapters.
///
/// This trait provides read-only access to challenge data for use in
/// building prompts, gathering context, and other infrastructure needs.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// `ChallengeService` directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
///
/// # Example
///
/// ```ignore
/// async fn build_challenge_context(
///     challenge_port: &dyn ChallengeServicePort,
///     scene_id: SceneId,
/// ) -> Vec<ChallengeInfo> {
///     let challenges = challenge_port.list_by_scene(scene_id).await?;
///     // ... build context
/// }
/// ```
#[async_trait]
pub trait ChallengeServicePort: Send + Sync {
    // -------------------------------------------------------------------------
    // Query Methods - Used by WebSocket helpers and prompt builders
    // -------------------------------------------------------------------------

    /// Get a challenge by ID.
    async fn get_challenge(&self, id: ChallengeId) -> Result<Option<Challenge>>;

    /// List challenges for a specific scene (via TIED_TO_SCENE edge).
    ///
    /// Used by prompt builders to provide active challenge context to the LLM.
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>>;

    /// List challenges available at a location (via AVAILABLE_AT edge).
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>>;

    /// List all challenges for a world.
    async fn list_challenges(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List active challenges for a world (for LLM context).
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    /// List favorite challenges for quick access.
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>>;

    // -------------------------------------------------------------------------
    // Skill Edge Queries (REQUIRES_SKILL)
    // -------------------------------------------------------------------------

    /// Get the required skill for a challenge.
    ///
    /// Used by prompt builders to display skill information alongside challenges.
    async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>>;

    // -------------------------------------------------------------------------
    // Scene Edge Queries (TIED_TO_SCENE)
    // -------------------------------------------------------------------------

    /// Get the scene a challenge is tied to.
    async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>>;

    // -------------------------------------------------------------------------
    // Prerequisite Edge Queries (REQUIRES_COMPLETION_OF)
    // -------------------------------------------------------------------------

    /// Get all prerequisites for a challenge.
    async fn get_prerequisites(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengePrerequisite>>;

    // -------------------------------------------------------------------------
    // Location Availability Edge Queries (AVAILABLE_AT)
    // -------------------------------------------------------------------------

    /// Get all locations where a challenge is available.
    async fn get_location_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeLocationAvailability>>;

    // -------------------------------------------------------------------------
    // Unlock Edge Queries (ON_SUCCESS_UNLOCKS)
    // -------------------------------------------------------------------------

    /// Get locations that get unlocked when this challenge succeeds.
    async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of ChallengeServicePort for testing.
    pub ChallengeServicePort {}

    #[async_trait]
    impl ChallengeServicePort for ChallengeServicePort {
        async fn get_challenge(&self, id: ChallengeId) -> Result<Option<Challenge>>;
        async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>>;
        async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>>;
        async fn list_challenges(&self, world_id: WorldId) -> Result<Vec<Challenge>>;
        async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>>;
        async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>>;
        async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>>;
        async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>>;
        async fn get_prerequisites(&self, challenge_id: ChallengeId) -> Result<Vec<ChallengePrerequisite>>;
        async fn get_location_availabilities(&self, challenge_id: ChallengeId) -> Result<Vec<ChallengeLocationAvailability>>;
        async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>>;
    }
}
