//! Core domain services for world building (Port-based abstraction)
//!
//! This module provides a grouped structure for core domain services using
//! port traits from `wrldbldr-engine-ports`. Unlike the adapter-layer
//! `CoreServices` which uses concrete app service traits, this version uses
//! the outbound port interfaces for full hexagonal architecture compliance.
//!
//! # Architecture
//!
//! This struct lives in the composition layer and depends only on port traits,
//! making it suitable for:
//! - Dependency injection at application startup
//! - Test doubles using mock implementations
//! - Clean separation between layers
//!
//! # Example
//!
//! ```ignore
//! use wrldbldr_engine_composition::CoreServices;
//!
//! let core_services = CoreServices::new(
//!     world_service,
//!     character_service,
//!     location_service,
//!     scene_service,
//!     skill_service,
//!     interaction_service,
//!     relationship_service,
//!     item_service,
//! );
//! ```

use std::sync::Arc;

// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::{
    CharacterServicePort, ItemServicePort, LocationServicePort, RelationshipServicePort,
    SkillServicePort,
};
// True outbound ports (adapter-implemented infrastructure)
use wrldbldr_engine_ports::outbound::{
    InteractionServicePort, SceneServicePort, WorldServicePort,
};

/// Core services for fundamental world-building entities using port abstractions.
///
/// This struct groups the primary domain services that handle the core
/// entities of the world-building system: worlds, characters, locations,
/// scenes, skills, interactions, relationships, and items.
///
/// All services are stored as `Arc<dyn Port>` for:
/// - Shared ownership across async tasks
/// - Flexibility to swap implementations (e.g., for testing with mocks)
/// - Consistency with the hexagonal architecture pattern
/// - Clean dependency inversion - depending on abstractions, not concretions
///
/// # Differences from adapter-layer CoreServices
///
/// The adapter-layer `CoreServices` in `engine-adapters` uses app-level service
/// traits (e.g., `WorldService`), while this composition-layer version uses
/// port traits (e.g., `WorldServicePort`). This allows the composition layer
/// to remain decoupled from the application layer's internal implementation details.
#[derive(Clone)]
pub struct CoreServices {
    /// Service for world management operations (create, read, update, delete worlds)
    pub world_service: Arc<dyn WorldServicePort>,

    /// Service for character management operations
    pub character_service: Arc<dyn CharacterServicePort>,

    /// Service for location management operations
    pub location_service: Arc<dyn LocationServicePort>,

    /// Service for scene management operations
    pub scene_service: Arc<dyn SceneServicePort>,

    /// Service for skill management operations
    pub skill_service: Arc<dyn SkillServicePort>,

    /// Service for interaction management operations
    pub interaction_service: Arc<dyn InteractionServicePort>,

    /// Service for relationship management operations
    pub relationship_service: Arc<dyn RelationshipServicePort>,

    /// Service for item management operations
    pub item_service: Arc<dyn ItemServicePort>,
}

impl CoreServices {
    /// Creates a new `CoreServices` instance with all the core domain services.
    ///
    /// # Arguments
    ///
    /// * `world_service` - Implementation of [`WorldServicePort`] for world operations
    /// * `character_service` - Implementation of [`CharacterServicePort`] for character operations
    /// * `location_service` - Implementation of [`LocationServicePort`] for location operations
    /// * `scene_service` - Implementation of [`SceneServicePort`] for scene operations
    /// * `skill_service` - Implementation of [`SkillServicePort`] for skill operations
    /// * `interaction_service` - Implementation of [`InteractionServicePort`] for interaction operations
    /// * `relationship_service` - Implementation of [`RelationshipServicePort`] for relationship operations
    /// * `item_service` - Implementation of [`ItemServicePort`] for item operations
    ///
    /// # Example
    ///
    /// ```ignore
    /// let core_services = CoreServices::new(
    ///     Arc::new(neo4j_world_service),
    ///     Arc::new(neo4j_character_service),
    ///     Arc::new(neo4j_location_service),
    ///     Arc::new(neo4j_scene_service),
    ///     Arc::new(neo4j_skill_service),
    ///     Arc::new(neo4j_interaction_service),
    ///     Arc::new(neo4j_relationship_service),
    ///     Arc::new(neo4j_item_service),
    /// );
    /// ```
    pub fn new(
        world_service: Arc<dyn WorldServicePort>,
        character_service: Arc<dyn CharacterServicePort>,
        location_service: Arc<dyn LocationServicePort>,
        scene_service: Arc<dyn SceneServicePort>,
        skill_service: Arc<dyn SkillServicePort>,
        interaction_service: Arc<dyn InteractionServicePort>,
        relationship_service: Arc<dyn RelationshipServicePort>,
        item_service: Arc<dyn ItemServicePort>,
    ) -> Self {
        Self {
            world_service,
            character_service,
            location_service,
            scene_service,
            skill_service,
            interaction_service,
            relationship_service,
            item_service,
        }
    }
}

impl std::fmt::Debug for CoreServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreServices")
            .field("world_service", &"Arc<dyn WorldServicePort>")
            .field("character_service", &"Arc<dyn CharacterServicePort>")
            .field("location_service", &"Arc<dyn LocationServicePort>")
            .field("scene_service", &"Arc<dyn SceneServicePort>")
            .field("skill_service", &"Arc<dyn SkillServicePort>")
            .field("interaction_service", &"Arc<dyn InteractionServicePort>")
            .field("relationship_service", &"Arc<dyn RelationshipServicePort>")
            .field("item_service", &"Arc<dyn ItemServicePort>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use wrldbldr_domain::SceneId;
    use wrldbldr_engine_ports::outbound::{
        MockCharacterServicePort, MockInteractionServicePort, MockItemServicePort,
        MockLocationServicePort, MockRelationshipServicePort, MockSkillServicePort,
        MockWorldServicePort, SceneWithRelations,
    };

    /// Simple mock for SceneServicePort (mockall not available for this port)
    struct MockSceneServicePort;

    #[async_trait::async_trait]
    impl SceneServicePort for MockSceneServicePort {
        async fn get_scene_with_relations(
            &self,
            _scene_id: SceneId,
        ) -> Result<Option<SceneWithRelations>> {
            Ok(None)
        }
    }

    #[test]
    fn test_core_services_construction() {
        let core_services = CoreServices::new(
            Arc::new(MockWorldServicePort::new()),
            Arc::new(MockCharacterServicePort::new()),
            Arc::new(MockLocationServicePort::new()),
            Arc::new(MockSceneServicePort),
            Arc::new(MockSkillServicePort::new()),
            Arc::new(MockInteractionServicePort::new()),
            Arc::new(MockRelationshipServicePort::new()),
            Arc::new(MockItemServicePort::new()),
        );

        // Verify debug output works
        let debug_str = format!("{:?}", core_services);
        assert!(debug_str.contains("CoreServices"));
        assert!(debug_str.contains("WorldServicePort"));
    }

    #[test]
    fn test_core_services_clone() {
        let core_services = CoreServices::new(
            Arc::new(MockWorldServicePort::new()),
            Arc::new(MockCharacterServicePort::new()),
            Arc::new(MockLocationServicePort::new()),
            Arc::new(MockSceneServicePort),
            Arc::new(MockSkillServicePort::new()),
            Arc::new(MockInteractionServicePort::new()),
            Arc::new(MockRelationshipServicePort::new()),
            Arc::new(MockItemServicePort::new()),
        );

        // Clone should work (important for sharing across async tasks)
        let _cloned = core_services.clone();
    }
}
