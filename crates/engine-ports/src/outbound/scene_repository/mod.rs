//! Split Scene repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `SceneRepositoryPort` (16 methods) is split into 5 focused traits:
//!
//! 1. `SceneCrudPort` - Core CRUD + directorial notes (6 methods)
//! 2. `SceneQueryPort` - Query/lookup by act/location (2 methods)
//! 3. `SceneLocationPort` - AT_LOCATION edge management (2 methods)
//! 4. `SceneFeaturedCharacterPort` - FEATURES_CHARACTER edges (5 methods)
//! 5. `SceneCompletionPort` - COMPLETED_SCENE tracking (3 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only basic CRUD depend on `SceneCrudPort`
//! - Query services depend on `SceneQueryPort`
//! - Location management depends on `SceneLocationPort`
//! - Character featuring depends on `SceneFeaturedCharacterPort`
//! - Progress tracking depends on `SceneCompletionPort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod completion_port;
mod crud_port;
mod featured_character_port;
mod location_port;
mod query_port;

pub use completion_port::SceneCompletionPort;
pub use crud_port::SceneCrudPort;
pub use featured_character_port::SceneFeaturedCharacterPort;
pub use location_port::SceneLocationPort;
pub use query_port::SceneQueryPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::{
        ActId, CharacterId, LocationId, PlayerCharacterId, Scene, SceneCharacter, SceneId,
    };

    mock! {
        /// Mock implementation of all Scene repository traits for testing.
        pub SceneRepository {}

        #[async_trait]
        impl SceneCrudPort for SceneRepository {
            async fn create(&self, scene: &Scene) -> anyhow::Result<()>;
            async fn get(&self, id: SceneId) -> anyhow::Result<Option<Scene>>;
            async fn update(&self, scene: &Scene) -> anyhow::Result<()>;
            async fn delete(&self, id: SceneId) -> anyhow::Result<()>;
            async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl SceneQueryPort for SceneRepository {
            async fn list_by_act(&self, act_id: ActId) -> anyhow::Result<Vec<Scene>>;
            async fn list_by_location(&self, location_id: LocationId) -> anyhow::Result<Vec<Scene>>;
        }

        #[async_trait]
        impl SceneLocationPort for SceneRepository {
            async fn set_location(&self, scene_id: SceneId, location_id: LocationId) -> anyhow::Result<()>;
            async fn get_location(&self, scene_id: SceneId) -> anyhow::Result<Option<LocationId>>;
        }

        #[async_trait]
        impl SceneFeaturedCharacterPort for SceneRepository {
            async fn add_featured_character(&self, scene_id: SceneId, character_id: CharacterId, scene_char: &SceneCharacter) -> anyhow::Result<()>;
            async fn get_featured_characters(&self, scene_id: SceneId) -> anyhow::Result<Vec<(CharacterId, SceneCharacter)>>;
            async fn update_featured_character(&self, scene_id: SceneId, character_id: CharacterId, scene_char: &SceneCharacter) -> anyhow::Result<()>;
            async fn remove_featured_character(&self, scene_id: SceneId, character_id: CharacterId) -> anyhow::Result<()>;
            async fn get_scenes_for_character(&self, character_id: CharacterId) -> anyhow::Result<Vec<Scene>>;
        }

        #[async_trait]
        impl SceneCompletionPort for SceneRepository {
            async fn mark_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId) -> anyhow::Result<()>;
            async fn is_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId) -> anyhow::Result<bool>;
            async fn get_completed_scenes(&self, pc_id: PlayerCharacterId) -> anyhow::Result<Vec<SceneId>>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockSceneRepository;
