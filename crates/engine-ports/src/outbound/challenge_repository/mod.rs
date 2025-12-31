//! Split Challenge repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `ChallengeRepositoryPort` (31 methods) is split into 5 focused traits:
//!
//! 1. `ChallengeCrudPort` - Core CRUD + state management (12 methods)
//! 2. `ChallengeSkillPort` - Skill relationship management (3 methods)
//! 3. `ChallengeScenePort` - Scene relationship management (3 methods)
//! 4. `ChallengePrerequisitePort` - Prerequisite chain management (4 methods)
//! 5. `ChallengeAvailabilityPort` - Location/region availability + unlocks (9 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only CRUD operations depend on `ChallengeCrudPort`
//! - Services managing skill requirements depend on `ChallengeSkillPort`
//! - Services managing scene ties depend on `ChallengeScenePort`
//! - Services managing prerequisites depend on `ChallengePrerequisitePort`
//! - Services managing availability depend on `ChallengeAvailabilityPort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod availability_port;
mod crud_port;
mod prerequisite_port;
mod scene_port;
mod skill_port;

pub use availability_port::ChallengeAvailabilityPort;
pub use crud_port::ChallengeCrudPort;
pub use prerequisite_port::ChallengePrerequisitePort;
pub use scene_port::ChallengeScenePort;
pub use skill_port::ChallengeSkillPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::entities::{
        ChallengeLocationAvailability, ChallengePrerequisite, ChallengeRegionAvailability,
    };
    use wrldbldr_domain::{
        Challenge, ChallengeId, LocationId, RegionId, SceneId, SkillId, WorldId,
    };

    mock! {
        /// Mock implementation of all Challenge repository traits for testing.
        pub ChallengeRepository {}

        #[async_trait]
        impl ChallengeCrudPort for ChallengeRepository {
            async fn create(&self, challenge: &Challenge) -> anyhow::Result<()>;
            async fn get(&self, id: ChallengeId) -> anyhow::Result<Option<Challenge>>;
            async fn list_by_world(&self, world_id: WorldId) -> anyhow::Result<Vec<Challenge>>;
            async fn list_by_scene(&self, scene_id: SceneId) -> anyhow::Result<Vec<Challenge>>;
            async fn list_by_location(&self, location_id: LocationId) -> anyhow::Result<Vec<Challenge>>;
            async fn list_active(&self, world_id: WorldId) -> anyhow::Result<Vec<Challenge>>;
            async fn list_favorites(&self, world_id: WorldId) -> anyhow::Result<Vec<Challenge>>;
            async fn update(&self, challenge: &Challenge) -> anyhow::Result<()>;
            async fn delete(&self, id: ChallengeId) -> anyhow::Result<()>;
            async fn set_active(&self, id: ChallengeId, active: bool) -> anyhow::Result<()>;
            async fn toggle_favorite(&self, id: ChallengeId) -> anyhow::Result<bool>;
        }

        #[async_trait]
        impl ChallengeSkillPort for ChallengeRepository {
            async fn set_required_skill(&self, challenge_id: ChallengeId, skill_id: SkillId) -> anyhow::Result<()>;
            async fn get_required_skill(&self, challenge_id: ChallengeId) -> anyhow::Result<Option<SkillId>>;
            async fn remove_required_skill(&self, challenge_id: ChallengeId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl ChallengeScenePort for ChallengeRepository {
            async fn tie_to_scene(&self, challenge_id: ChallengeId, scene_id: SceneId) -> anyhow::Result<()>;
            async fn get_tied_scene(&self, challenge_id: ChallengeId) -> anyhow::Result<Option<SceneId>>;
            async fn untie_from_scene(&self, challenge_id: ChallengeId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl ChallengePrerequisitePort for ChallengeRepository {
            async fn add_prerequisite(&self, challenge_id: ChallengeId, prerequisite: ChallengePrerequisite) -> anyhow::Result<()>;
            async fn get_prerequisites(&self, challenge_id: ChallengeId) -> anyhow::Result<Vec<ChallengePrerequisite>>;
            async fn remove_prerequisite(&self, challenge_id: ChallengeId, prerequisite_id: ChallengeId) -> anyhow::Result<()>;
            async fn get_dependent_challenges(&self, challenge_id: ChallengeId) -> anyhow::Result<Vec<ChallengeId>>;
        }

        #[async_trait]
        impl ChallengeAvailabilityPort for ChallengeRepository {
            async fn add_location_availability(&self, challenge_id: ChallengeId, availability: ChallengeLocationAvailability) -> anyhow::Result<()>;
            async fn get_location_availabilities(&self, challenge_id: ChallengeId) -> anyhow::Result<Vec<ChallengeLocationAvailability>>;
            async fn remove_location_availability(&self, challenge_id: ChallengeId, location_id: LocationId) -> anyhow::Result<()>;
            async fn list_by_region(&self, region_id: RegionId) -> anyhow::Result<Vec<Challenge>>;
            async fn add_region_availability(&self, challenge_id: ChallengeId, availability: ChallengeRegionAvailability) -> anyhow::Result<()>;
            async fn get_region_availabilities(&self, challenge_id: ChallengeId) -> anyhow::Result<Vec<ChallengeRegionAvailability>>;
            async fn remove_region_availability(&self, challenge_id: ChallengeId, region_id: RegionId) -> anyhow::Result<()>;
            async fn add_unlock_location(&self, challenge_id: ChallengeId, location_id: LocationId) -> anyhow::Result<()>;
            async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> anyhow::Result<Vec<LocationId>>;
            async fn remove_unlock_location(&self, challenge_id: ChallengeId, location_id: LocationId) -> anyhow::Result<()>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockChallengeRepository;
