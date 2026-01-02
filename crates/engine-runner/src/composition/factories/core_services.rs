//! Core Services Factory
//!
//! This module provides factory functions for creating core domain services
//! from repository ports and shared dependencies. It reduces boilerplate
//! in the composition root by encapsulating service construction logic.
//!
//! # Architecture
//!
//! Following the hexagonal architecture pattern:
//! - Input: Repository ports (from `repositories.rs` factory) and shared dependencies
//! - Output: Both `Arc<dyn *ServicePort>` and `Arc<dyn *Service>` trait objects
//!
//! # Services Created
//!
//! | Service                  | Dependencies |
//! |--------------------------|--------------|
//! | WorldServiceImpl         | WorldRepo, WorldExporter, SettingsService, Clock |
//! | CharacterServiceImpl     | WorldRepo, CharacterCrud, CharacterWant, RelationshipRepo, Settings, Clock |
//! | LocationServiceImpl      | WorldRepo, LocationCrud, LocationHierarchy, LocationConnection, LocationMap |
//! | SceneServiceImpl         | SceneRepo, LocationCrud, CharacterCrud |
//! | SkillServiceImpl         | SkillRepo, WorldRepo |
//! | InteractionServiceImpl   | InteractionRepo |
//! | RelationshipServiceImpl  | RelationshipRepo |
//! | ItemServiceImpl          | ItemRepo, PCRepo, RegionItem |
//! | PlayerCharacterServiceImpl| PCRepo, LocationCrud, WorldRepo, Clock |
//! | SceneResolutionServiceImpl| PCRepo, SceneRepo, FlagRepo, ObservationRepo |
//! | SheetTemplateService     | SheetTemplateRepo |

use std::sync::Arc;

use wrldbldr_engine_app::application::services::{
    CharacterService, CharacterServiceImpl, InteractionService, InteractionServiceImpl,
    ItemService, ItemServiceImpl, LocationService, LocationServiceImpl, PlayerCharacterService,
    PlayerCharacterServiceImpl, RelationshipService, RelationshipServiceImpl,
    SceneResolutionServiceImpl, SceneService, SceneServiceImpl, SheetTemplateService, SkillService,
    SkillServiceImpl, WorldService, WorldServiceImpl,
};
use wrldbldr_engine_ports::outbound::{
    // Repository ports (inputs)
    CharacterCrudPort,
    // Service ports (outputs)
    CharacterServicePort,
    CharacterWantPort,
    ClockPort,
    FlagRepositoryPort,
    InteractionRepositoryPort,
    InteractionServicePort,
    ItemRepositoryPort,
    ItemServicePort,
    LocationConnectionPort,
    LocationCrudPort,
    LocationHierarchyPort,
    LocationMapPort,
    LocationServicePort,
    ObservationRepositoryPort,
    // PlayerCharacter ISP ports
    PlayerCharacterCrudPort,
    PlayerCharacterInventoryPort,
    PlayerCharacterPositionPort,
    PlayerCharacterQueryPort,
    PlayerCharacterServicePort,
    RegionItemPort,
    RelationshipRepositoryPort,
    RelationshipServicePort,
    // Scene ISP ports (no god trait)
    SceneCompletionPort,
    SceneCrudPort,
    SceneFeaturedCharacterPort,
    SceneLocationPort,
    SceneQueryPort,
    SceneResolutionServicePort,
    SceneServicePort,
    SettingsServicePort,
    SheetTemplateRepositoryPort,
    SheetTemplateServicePort,
    SkillRepositoryPort,
    SkillServicePort,
    WorldExporterPort,
    WorldRepositoryPort,
    WorldServicePort,
};

/// Dependencies required to create core services.
///
/// This struct groups all the repository ports and shared dependencies needed
/// to construct the core domain services. It serves as the input to the
/// `create_core_services` factory function.
///
/// All fields are `Arc<dyn Trait>` which allows cheap cloning (reference count
/// increment only). The factory function will clone these as needed internally.
///
/// # Usage
///
/// ```ignore
/// let deps = CoreServiceDependencies {
///     world_repo: repos.world.clone(),
///     world_exporter: exporter.clone(),
///     character_crud: repos.character.crud.clone(),
///     // ... other dependencies
/// };
/// let core = create_core_services(deps);
/// ```
pub struct CoreServiceDependencies {
    // World service dependencies
    pub world_repo: Arc<dyn WorldRepositoryPort>,
    pub world_exporter: Arc<dyn WorldExporterPort>,
    pub settings_service: Arc<dyn SettingsServicePort>,
    pub clock: Arc<dyn ClockPort>,

    // Character service dependencies (ISP split)
    pub character_crud: Arc<dyn CharacterCrudPort>,
    pub character_want: Arc<dyn CharacterWantPort>,
    pub relationship_repo: Arc<dyn RelationshipRepositoryPort>,

    // Location service dependencies (ISP split)
    pub location_crud: Arc<dyn LocationCrudPort>,
    pub location_hierarchy: Arc<dyn LocationHierarchyPort>,
    pub location_connection: Arc<dyn LocationConnectionPort>,
    pub location_map: Arc<dyn LocationMapPort>,

    // Scene service dependencies (ISP split)
    pub scene_crud: Arc<dyn SceneCrudPort>,
    pub scene_query: Arc<dyn SceneQueryPort>,
    pub scene_location: Arc<dyn SceneLocationPort>,
    pub scene_featured_character: Arc<dyn SceneFeaturedCharacterPort>,
    pub scene_completion: Arc<dyn SceneCompletionPort>,

    // Skill service dependencies
    pub skill_repo: Arc<dyn SkillRepositoryPort>,

    // Interaction service dependencies
    pub interaction_repo: Arc<dyn InteractionRepositoryPort>,

    // Item service dependencies
    pub item_repo: Arc<dyn ItemRepositoryPort>,
    pub region_item: Arc<dyn RegionItemPort>,

    // PlayerCharacter ISP traits (used by Item, PC, and SceneResolution services)
    pub pc_crud: Arc<dyn PlayerCharacterCrudPort>,
    pub pc_query: Arc<dyn PlayerCharacterQueryPort>,
    pub pc_position: Arc<dyn PlayerCharacterPositionPort>,
    pub pc_inventory: Arc<dyn PlayerCharacterInventoryPort>,

    // Scene resolution service dependencies
    pub flag_repo: Arc<dyn FlagRepositoryPort>,
    pub observation_repo: Arc<dyn ObservationRepositoryPort>,

    // Sheet template service dependencies
    pub sheet_template_repo: Arc<dyn SheetTemplateRepositoryPort>,
}

/// Container for all core service trait objects (both port and app-layer versions).
///
/// This struct holds BOTH `Arc<dyn *ServicePort>` and `Arc<dyn *Service>` for each
/// core domain service. Each pair points to the SAME underlying implementation,
/// eliminating duplicate service instantiations.
///
/// # Architecture
///
/// Services are created once and cast to both trait types:
/// - **Port traits** (`*ServicePort`): For composition layer (CoreServices, PlayerServices)
/// - **App-layer traits** (`*Service`): For AppRequestHandler and internal use
///
/// # Services Included
///
/// - **world**: World management (create, update, delete worlds)
/// - **character**: Character management with archetype tracking
/// - **location**: Location hierarchy and connections
/// - **scene**: Scene management and character assignment
/// - **skill**: Skill management per world's rule system
/// - **interaction**: Interaction templates for scenes
/// - **relationship**: Character-to-character relationships
/// - **item**: Item management and inventory operations
/// - **player_character**: PC management for multiplayer sessions
/// - **scene_resolution**: Determines active scene based on PC locations
/// - **sheet_template**: Character sheet templates per world
///
/// # Example
///
/// ```ignore
/// let core = create_core_services(deps);
///
/// // Use port version in composition layer
/// let composition_core = CoreServices::new(
///     core.world_service_port.clone(),
///     core.character_service_port.clone(),
///     // ...
/// );
///
/// // Use app-layer version in AppRequestHandler
/// let handler = AppRequestHandler::new(
///     core.world_service.clone(),
///     core.character_service.clone(),
///     // ...
/// );
/// ```
pub struct CoreServicePorts {
    // =========================================================================
    // Port versions (for composition layer)
    // =========================================================================
    /// World management service (port)
    pub world_service_port: Arc<dyn WorldServicePort>,
    /// Character management service (port)
    pub character_service_port: Arc<dyn CharacterServicePort>,
    /// Location management service (port)
    pub location_service_port: Arc<dyn LocationServicePort>,
    /// Scene management service (port)
    pub scene_service_port: Arc<dyn SceneServicePort>,
    /// Skill management service (port)
    pub skill_service_port: Arc<dyn SkillServicePort>,
    /// Interaction management service (port)
    pub interaction_service_port: Arc<dyn InteractionServicePort>,
    /// Relationship management service (port)
    pub relationship_service_port: Arc<dyn RelationshipServicePort>,
    /// Item management service (port)
    pub item_service_port: Arc<dyn ItemServicePort>,
    /// Player character management service (port)
    pub player_character_service_port: Arc<dyn PlayerCharacterServicePort>,
    /// Scene resolution service (port)
    pub scene_resolution_service_port: Arc<dyn SceneResolutionServicePort>,
    /// Sheet template management service (port)
    pub sheet_template_service_port: Arc<dyn SheetTemplateServicePort>,

    // =========================================================================
    // App-layer versions (for AppRequestHandler)
    // =========================================================================
    /// World management service (app-layer)
    pub world_service: Arc<dyn WorldService>,
    /// Character management service (app-layer)
    pub character_service: Arc<dyn CharacterService>,
    /// Location management service (app-layer)
    pub location_service: Arc<dyn LocationService>,
    /// Scene management service (app-layer)
    pub scene_service: Arc<dyn SceneService>,
    /// Skill management service (app-layer)
    pub skill_service: Arc<dyn SkillService>,
    /// Interaction management service (app-layer)
    pub interaction_service: Arc<dyn InteractionService>,
    /// Relationship management service (app-layer)
    pub relationship_service: Arc<dyn RelationshipService>,
    /// Item management service (app-layer)
    pub item_service: Arc<dyn ItemService>,
    /// Player character management service (app-layer)
    pub player_character_service: Arc<dyn PlayerCharacterService>,
}

/// Creates all core domain services from their dependencies.
///
/// This factory function constructs each service implementation ONCE, then casts
/// each to BOTH its port trait and app-layer trait. This eliminates duplicate
/// service instantiations while providing both trait versions for different use cases.
///
/// # Architecture
///
/// Each service impl implements both:
/// - `*ServicePort` (outbound port trait for composition layer)
/// - `*Service` (app-layer trait for handlers)
///
/// We create one Arc<Impl> and clone it for casting to both trait types.
///
/// # Arguments
///
/// * `deps` - The [`CoreServiceDependencies`] containing all required inputs
///
/// # Returns
///
/// A [`CoreServicePorts`] struct containing both port and app-layer trait objects.
///
/// # Arc Cloning
///
/// The function clones `Arc` references internally as needed. Since `Arc::clone()`
/// only increments a reference count (no deep copy), this is efficient. Callers
/// should provide Arcs that can be consumed or cloned as needed.
///
/// # Example
///
/// ```ignore
/// let repos = create_repository_ports(&repository);
/// let deps = CoreServiceDependencies { ... };
///
/// let core = create_core_services(deps);
///
/// // Use port versions for composition layer
/// let composition_core = CoreServices::new(
///     core.world_service_port.clone(),
///     core.character_service_port.clone(),
///     // ...
/// );
///
/// // Use app-layer versions for AppRequestHandler
/// let handler = AppRequestHandler::new(
///     core.world_service.clone(),
///     core.character_service.clone(),
///     // ...
/// );
/// ```
pub fn create_core_services(deps: CoreServiceDependencies) -> CoreServicePorts {
    // Clone shared dependencies that are used by multiple services
    let world_repo = deps.world_repo;
    let clock = deps.clock;
    let location_crud = deps.location_crud;
    let character_crud = deps.character_crud;
    // PC ISP ports
    let pc_crud = deps.pc_crud;
    let pc_query = deps.pc_query;
    let pc_position = deps.pc_position;
    let pc_inventory = deps.pc_inventory;
    // Scene ISP ports
    let scene_crud = deps.scene_crud;
    let scene_query = deps.scene_query;
    let scene_location = deps.scene_location;
    let scene_featured_character = deps.scene_featured_character;
    let scene_completion = deps.scene_completion;

    // =========================================================================
    // World Service - single instance, cast to both traits
    // =========================================================================
    let world_service_impl = Arc::new(WorldServiceImpl::new(
        world_repo.clone(),
        deps.world_exporter,
        deps.settings_service.clone(),
        clock.clone(),
    ));
    let world_service_port: Arc<dyn WorldServicePort> = world_service_impl.clone();
    let world_service: Arc<dyn WorldService> = world_service_impl;

    // =========================================================================
    // Character Service - single instance, cast to both traits
    // =========================================================================
    let character_service_impl = Arc::new(CharacterServiceImpl::new(
        world_repo.clone(),
        character_crud.clone(),
        deps.character_want,
        deps.relationship_repo.clone(),
        deps.settings_service,
        clock.clone(),
    ));
    let character_service_port: Arc<dyn CharacterServicePort> = character_service_impl.clone();
    let character_service: Arc<dyn CharacterService> = character_service_impl;

    // =========================================================================
    // Location Service - single instance, cast to both traits
    // =========================================================================
    let location_service_impl = Arc::new(LocationServiceImpl::new(
        world_repo.clone(),
        location_crud.clone(),
        deps.location_hierarchy,
        deps.location_connection,
        deps.location_map,
    ));
    let location_service_port: Arc<dyn LocationServicePort> = location_service_impl.clone();
    let location_service: Arc<dyn LocationService> = location_service_impl;

    // =========================================================================
    // Scene Service - single instance, cast to both traits
    // =========================================================================
    let scene_service_impl = Arc::new(SceneServiceImpl::new(
        scene_crud,
        scene_query.clone(),
        scene_location,
        scene_featured_character,
        location_crud.clone(),
        character_crud,
    ));
    let scene_service_port: Arc<dyn SceneServicePort> = scene_service_impl.clone();
    let scene_service: Arc<dyn SceneService> = scene_service_impl;

    // =========================================================================
    // Skill Service - single instance, cast to both traits
    // =========================================================================
    let skill_service_impl = Arc::new(SkillServiceImpl::new(deps.skill_repo, world_repo.clone()));
    let skill_service_port: Arc<dyn SkillServicePort> = skill_service_impl.clone();
    let skill_service: Arc<dyn SkillService> = skill_service_impl;

    // =========================================================================
    // Interaction Service - single instance, cast to both traits
    // =========================================================================
    let interaction_service_impl = Arc::new(InteractionServiceImpl::new(deps.interaction_repo));
    let interaction_service_port: Arc<dyn InteractionServicePort> =
        interaction_service_impl.clone();
    let interaction_service: Arc<dyn InteractionService> = interaction_service_impl;

    // =========================================================================
    // Relationship Service - single instance, cast to both traits
    // =========================================================================
    let relationship_service_impl = Arc::new(RelationshipServiceImpl::new(deps.relationship_repo));
    let relationship_service_port: Arc<dyn RelationshipServicePort> =
        relationship_service_impl.clone();
    let relationship_service: Arc<dyn RelationshipService> = relationship_service_impl;

    // =========================================================================
    // Item Service - single instance, cast to both traits
    // =========================================================================
    let item_service_impl = Arc::new(ItemServiceImpl::new(
        deps.item_repo,
        pc_inventory.clone(),
        deps.region_item,
    ));
    let item_service_port: Arc<dyn ItemServicePort> = item_service_impl.clone();
    let item_service: Arc<dyn ItemService> = item_service_impl;

    // =========================================================================
    // Player Character Service - single instance, cast to both traits
    // =========================================================================
    let player_character_service_impl = Arc::new(PlayerCharacterServiceImpl::new(
        pc_crud.clone(),
        pc_query.clone(),
        pc_position,
        location_crud,
        world_repo,
        clock,
    ));
    let player_character_service_port: Arc<dyn PlayerCharacterServicePort> =
        player_character_service_impl.clone();
    let player_character_service: Arc<dyn PlayerCharacterService> = player_character_service_impl;

    // =========================================================================
    // Scene Resolution Service - port only (no app-layer trait)
    // =========================================================================
    let scene_resolution_service_impl = Arc::new(SceneResolutionServiceImpl::new(
        pc_crud,
        pc_query,
        pc_inventory,
        scene_query,
        scene_completion,
        deps.flag_repo,
        deps.observation_repo,
    ));
    let scene_resolution_service_port: Arc<dyn SceneResolutionServicePort> =
        scene_resolution_service_impl;

    // =========================================================================
    // Sheet Template Service - port only (no app-layer trait)
    // =========================================================================
    let sheet_template_service_impl = Arc::new(SheetTemplateService::new(deps.sheet_template_repo));
    let sheet_template_service_port: Arc<dyn SheetTemplateServicePort> =
        sheet_template_service_impl;

    CoreServicePorts {
        // Port versions
        world_service_port,
        character_service_port,
        location_service_port,
        scene_service_port,
        skill_service_port,
        interaction_service_port,
        relationship_service_port,
        item_service_port,
        player_character_service_port,
        scene_resolution_service_port,
        sheet_template_service_port,
        // App-layer versions
        world_service,
        character_service,
        location_service,
        scene_service,
        skill_service,
        interaction_service,
        relationship_service,
        item_service,
        player_character_service,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify CoreServiceDependencies struct has all expected fields.
    ///
    /// This is a compile-time verification test - it ensures the struct
    /// has the correct field types. If any field is missing or has a wrong
    /// type, this test won't compile.
    #[test]
    fn test_core_service_dependencies_fields() {
        fn _verify_deps(deps: &CoreServiceDependencies) {
            // World service deps
            let _: &Arc<dyn WorldRepositoryPort> = &deps.world_repo;
            let _: &Arc<dyn WorldExporterPort> = &deps.world_exporter;
            let _: &Arc<dyn SettingsServicePort> = &deps.settings_service;
            let _: &Arc<dyn ClockPort> = &deps.clock;

            // Character service deps
            let _: &Arc<dyn CharacterCrudPort> = &deps.character_crud;
            let _: &Arc<dyn CharacterWantPort> = &deps.character_want;
            let _: &Arc<dyn RelationshipRepositoryPort> = &deps.relationship_repo;

            // Location service deps
            let _: &Arc<dyn LocationCrudPort> = &deps.location_crud;
            let _: &Arc<dyn LocationHierarchyPort> = &deps.location_hierarchy;
            let _: &Arc<dyn LocationConnectionPort> = &deps.location_connection;
            let _: &Arc<dyn LocationMapPort> = &deps.location_map;

            // Scene service deps (ISP split)
            let _: &Arc<dyn SceneCrudPort> = &deps.scene_crud;
            let _: &Arc<dyn SceneQueryPort> = &deps.scene_query;
            let _: &Arc<dyn SceneLocationPort> = &deps.scene_location;
            let _: &Arc<dyn SceneFeaturedCharacterPort> = &deps.scene_featured_character;
            let _: &Arc<dyn SceneCompletionPort> = &deps.scene_completion;

            // Skill service deps
            let _: &Arc<dyn SkillRepositoryPort> = &deps.skill_repo;

            // Interaction service deps
            let _: &Arc<dyn InteractionRepositoryPort> = &deps.interaction_repo;

            // Item service deps
            let _: &Arc<dyn ItemRepositoryPort> = &deps.item_repo;
            let _: &Arc<dyn RegionItemPort> = &deps.region_item;

            // PC ISP ports (used by Item, PC, and SceneResolution services)
            let _: &Arc<dyn PlayerCharacterCrudPort> = &deps.pc_crud;
            let _: &Arc<dyn PlayerCharacterQueryPort> = &deps.pc_query;
            let _: &Arc<dyn PlayerCharacterPositionPort> = &deps.pc_position;
            let _: &Arc<dyn PlayerCharacterInventoryPort> = &deps.pc_inventory;

            // Scene resolution deps
            let _: &Arc<dyn FlagRepositoryPort> = &deps.flag_repo;
            let _: &Arc<dyn ObservationRepositoryPort> = &deps.observation_repo;

            // Sheet template deps
            let _: &Arc<dyn SheetTemplateRepositoryPort> = &deps.sheet_template_repo;
        }

        // Existence of this function proves the types are correct at compile time
        let _ = _verify_deps;
    }

    /// Verify CoreServicePorts struct has all expected fields with correct types.
    #[test]
    fn test_core_service_ports_fields() {
        fn _verify_ports(ports: &CoreServicePorts) {
            // Port versions
            let _: &Arc<dyn WorldServicePort> = &ports.world_service_port;
            let _: &Arc<dyn CharacterServicePort> = &ports.character_service_port;
            let _: &Arc<dyn LocationServicePort> = &ports.location_service_port;
            let _: &Arc<dyn SceneServicePort> = &ports.scene_service_port;
            let _: &Arc<dyn SkillServicePort> = &ports.skill_service_port;
            let _: &Arc<dyn InteractionServicePort> = &ports.interaction_service_port;
            let _: &Arc<dyn RelationshipServicePort> = &ports.relationship_service_port;
            let _: &Arc<dyn ItemServicePort> = &ports.item_service_port;
            let _: &Arc<dyn PlayerCharacterServicePort> = &ports.player_character_service_port;
            let _: &Arc<dyn SceneResolutionServicePort> = &ports.scene_resolution_service_port;
            let _: &Arc<dyn SheetTemplateServicePort> = &ports.sheet_template_service_port;
            // App-layer versions
            let _: &Arc<dyn WorldService> = &ports.world_service;
            let _: &Arc<dyn CharacterService> = &ports.character_service;
            let _: &Arc<dyn LocationService> = &ports.location_service;
            let _: &Arc<dyn SceneService> = &ports.scene_service;
            let _: &Arc<dyn SkillService> = &ports.skill_service;
            let _: &Arc<dyn InteractionService> = &ports.interaction_service;
            let _: &Arc<dyn RelationshipService> = &ports.relationship_service;
            let _: &Arc<dyn ItemService> = &ports.item_service;
            let _: &Arc<dyn PlayerCharacterService> = &ports.player_character_service;
        }

        let _ = _verify_ports;
    }

    /// Verify the factory function signature.
    ///
    /// This test ensures the factory function has the expected signature
    /// at compile time. It doesn't actually call the function (which would
    /// require real dependencies) but verifies the types are correct.
    #[test]
    fn test_create_core_services_signature() {
        // Verify the function exists and has the correct signature
        fn _verify_signature(_f: fn(CoreServiceDependencies) -> CoreServicePorts) {}
        _verify_signature(create_core_services);
    }
}
