//! Core domain services for world building
//!
//! This module provides a grouped structure for core domain services,
//! using trait objects for flexibility and testability.

use std::sync::Arc;

use wrldbldr_engine_app::application::services::{
    CharacterService, InteractionService, ItemService, LocationService,
    RelationshipService, SceneService, SkillService, WorldService,
};

/// Core services for fundamental world-building entities
///
/// This struct groups the primary domain services that handle the core
/// entities of the world-building system: worlds, characters, locations,
/// scenes, skills, interactions, relationships, and items.
///
/// All services are stored as `Arc<dyn Trait>` for:
/// - Shared ownership across async tasks
/// - Flexibility to swap implementations (e.g., for testing)
/// - Consistency with the hexagonal architecture pattern
pub struct CoreServices {
    pub world_service: Arc<dyn WorldService>,
    pub character_service: Arc<dyn CharacterService>,
    pub location_service: Arc<dyn LocationService>,
    pub scene_service: Arc<dyn SceneService>,
    pub skill_service: Arc<dyn SkillService>,
    pub interaction_service: Arc<dyn InteractionService>,
    pub relationship_service: Arc<dyn RelationshipService>,
    pub item_service: Arc<dyn ItemService>,
}

impl CoreServices {
    /// Creates a new CoreServices instance with all the core domain services
    pub fn new(
        world_service: Arc<dyn WorldService>,
        character_service: Arc<dyn CharacterService>,
        location_service: Arc<dyn LocationService>,
        scene_service: Arc<dyn SceneService>,
        skill_service: Arc<dyn SkillService>,
        interaction_service: Arc<dyn InteractionService>,
        relationship_service: Arc<dyn RelationshipService>,
        item_service: Arc<dyn ItemService>,
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
