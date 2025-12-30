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
//! - Output: `Arc<dyn ServicePort>` trait objects for dependency injection
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
    CharacterServiceImpl, InteractionServiceImpl, ItemServiceImpl, LocationServiceImpl,
    PlayerCharacterServiceImpl, RelationshipServiceImpl, SceneResolutionServiceImpl,
    SceneServiceImpl, SheetTemplateService, SkillServiceImpl, WorldServiceImpl,
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
    PlayerCharacterRepositoryPort,
    PlayerCharacterServicePort,
    RegionItemPort,
    RelationshipRepositoryPort,
    RelationshipServicePort,
    SceneRepositoryPort,
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

    // Scene service dependencies
    pub scene_repo: Arc<dyn SceneRepositoryPort>,

    // Skill service dependencies
    pub skill_repo: Arc<dyn SkillRepositoryPort>,

    // Interaction service dependencies
    pub interaction_repo: Arc<dyn InteractionRepositoryPort>,

    // Item service dependencies
    pub item_repo: Arc<dyn ItemRepositoryPort>,
    pub pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    pub region_item: Arc<dyn RegionItemPort>,

    // Scene resolution service dependencies
    pub flag_repo: Arc<dyn FlagRepositoryPort>,
    pub observation_repo: Arc<dyn ObservationRepositoryPort>,

    // Sheet template service dependencies
    pub sheet_template_repo: Arc<dyn SheetTemplateRepositoryPort>,
}

/// Container for all core service port trait objects.
///
/// This struct holds `Arc<dyn *ServicePort>` for each core domain service,
/// ready for injection into the composition layer's `CoreServices` struct
/// or directly into use cases and handlers.
///
/// # Services Included
///
/// - **world_service**: World management (create, update, delete worlds)
/// - **character_service**: Character management with archetype tracking
/// - **location_service**: Location hierarchy and connections
/// - **scene_service**: Scene management and character assignment
/// - **skill_service**: Skill management per world's rule system
/// - **interaction_service**: Interaction templates for scenes
/// - **relationship_service**: Character-to-character relationships
/// - **item_service**: Item management and inventory operations
/// - **player_character_service**: PC management for multiplayer sessions
/// - **scene_resolution_service**: Determines active scene based on PC locations
/// - **sheet_template_service**: Character sheet templates per world
///
/// # Example
///
/// ```ignore
/// let core_ports = create_core_services(deps);
///
/// // Use in composition layer
/// let composition_core = CoreServices::new(
///     core_ports.world_service,
///     core_ports.character_service,
///     // ...
/// );
/// ```
pub struct CoreServicePorts {
    /// World management service
    pub world_service: Arc<dyn WorldServicePort>,
    /// Character management service
    pub character_service: Arc<dyn CharacterServicePort>,
    /// Location management service
    pub location_service: Arc<dyn LocationServicePort>,
    /// Scene management service
    pub scene_service: Arc<dyn SceneServicePort>,
    /// Skill management service
    pub skill_service: Arc<dyn SkillServicePort>,
    /// Interaction management service
    pub interaction_service: Arc<dyn InteractionServicePort>,
    /// Relationship management service
    pub relationship_service: Arc<dyn RelationshipServicePort>,
    /// Item management service
    pub item_service: Arc<dyn ItemServicePort>,
    /// Player character management service
    pub player_character_service: Arc<dyn PlayerCharacterServicePort>,
    /// Scene resolution service (determines which scene to show based on PC locations)
    pub scene_resolution_service: Arc<dyn SceneResolutionServicePort>,
    /// Sheet template management service
    pub sheet_template_service: Arc<dyn SheetTemplateServicePort>,
}

/// Creates all core domain services from their dependencies.
///
/// This factory function constructs each service implementation with the
/// required repository ports and shared dependencies, then casts each to
/// its corresponding port trait object.
///
/// # Arguments
///
/// * `deps` - The [`CoreServiceDependencies`] containing all required inputs
///
/// # Returns
///
/// A [`CoreServicePorts`] struct containing all service trait objects.
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
/// let deps = CoreServiceDependencies {
///     world_repo: repos.world.clone(),
///     world_exporter: world_exporter.clone(),
///     settings_service: settings_service.clone(),
///     clock: clock.clone(),
///     character_crud: repos.character.crud.clone(),
///     character_want: repos.character.want.clone(),
///     relationship_repo: repos.relationship.clone(),
///     location_crud: repos.location.crud.clone(),
///     location_hierarchy: repos.location.hierarchy.clone(),
///     location_connection: repos.location.connection.clone(),
///     location_map: repos.location.map.clone(),
///     scene_repo: Arc::new(repository.scenes()),
///     skill_repo: repos.skill.clone(),
///     interaction_repo: repos.interaction.clone(),
///     item_repo: repos.item.clone(),
///     pc_repo: Arc::new(repository.player_characters()),
///     region_item: repos.region.item.clone(),
///     flag_repo: repos.flag.clone(),
///     observation_repo: repos.observation.clone(),
///     sheet_template_repo: repos.sheet_template.clone(),
/// };
///
/// let core_services = create_core_services(deps);
///
/// // Use the services
/// let world = core_services.world_service.get_world(world_id).await?;
/// ```
pub fn create_core_services(deps: CoreServiceDependencies) -> CoreServicePorts {
    // Clone shared dependencies that are used by multiple services
    let world_repo = deps.world_repo;
    let clock = deps.clock;
    let location_crud = deps.location_crud;
    let character_crud = deps.character_crud;
    let pc_repo = deps.pc_repo;
    let scene_repo = deps.scene_repo;

    // World Service
    // Dependencies: WorldRepo, WorldExporter, SettingsService, Clock
    let world_service_impl = WorldServiceImpl::new(
        world_repo.clone(),
        deps.world_exporter,
        deps.settings_service.clone(),
        clock.clone(),
    );
    let world_service: Arc<dyn WorldServicePort> = Arc::new(world_service_impl);

    // Character Service
    // Dependencies: WorldRepo, CharacterCrud, CharacterWant, RelationshipRepo, Settings, Clock
    let character_service_impl = CharacterServiceImpl::new(
        world_repo.clone(),
        character_crud.clone(),
        deps.character_want,
        deps.relationship_repo.clone(),
        deps.settings_service,
        clock.clone(),
    );
    let character_service: Arc<dyn CharacterServicePort> = Arc::new(character_service_impl);

    // Location Service
    // Dependencies: WorldRepo, LocationCrud, LocationHierarchy, LocationConnection, LocationMap
    let location_service_impl = LocationServiceImpl::new(
        world_repo.clone(),
        location_crud.clone(),
        deps.location_hierarchy,
        deps.location_connection,
        deps.location_map,
    );
    let location_service: Arc<dyn LocationServicePort> = Arc::new(location_service_impl);

    // Scene Service
    // Dependencies: SceneRepo, LocationCrud, CharacterCrud
    let scene_service_impl =
        SceneServiceImpl::new(scene_repo.clone(), location_crud.clone(), character_crud);
    let scene_service: Arc<dyn SceneServicePort> = Arc::new(scene_service_impl);

    // Skill Service
    // Dependencies: SkillRepo, WorldRepo
    let skill_service_impl = SkillServiceImpl::new(deps.skill_repo, world_repo.clone());
    let skill_service: Arc<dyn SkillServicePort> = Arc::new(skill_service_impl);

    // Interaction Service
    // Dependencies: InteractionRepo
    let interaction_service_impl = InteractionServiceImpl::new(deps.interaction_repo);
    let interaction_service: Arc<dyn InteractionServicePort> = Arc::new(interaction_service_impl);

    // Relationship Service
    // Dependencies: RelationshipRepo
    let relationship_service_impl = RelationshipServiceImpl::new(deps.relationship_repo);
    let relationship_service: Arc<dyn RelationshipServicePort> =
        Arc::new(relationship_service_impl);

    // Item Service
    // Dependencies: ItemRepo, PCRepo, RegionItem
    let item_service_impl = ItemServiceImpl::new(deps.item_repo, pc_repo.clone(), deps.region_item);
    let item_service: Arc<dyn ItemServicePort> = Arc::new(item_service_impl);

    // Player Character Service
    // Dependencies: PCRepo, LocationCrud, WorldRepo, Clock
    let player_character_service_impl =
        PlayerCharacterServiceImpl::new(pc_repo.clone(), location_crud, world_repo, clock);
    let player_character_service: Arc<dyn PlayerCharacterServicePort> =
        Arc::new(player_character_service_impl);

    // Scene Resolution Service
    // Dependencies: PCRepo, SceneRepo, FlagRepo, ObservationRepo
    let scene_resolution_service_impl =
        SceneResolutionServiceImpl::new(pc_repo, scene_repo, deps.flag_repo, deps.observation_repo);
    let scene_resolution_service: Arc<dyn SceneResolutionServicePort> =
        Arc::new(scene_resolution_service_impl);

    // Sheet Template Service
    // Dependencies: SheetTemplateRepo
    let sheet_template_service = Arc::new(SheetTemplateService::new(deps.sheet_template_repo));
    let sheet_template_service: Arc<dyn SheetTemplateServicePort> = sheet_template_service;

    CoreServicePorts {
        world_service,
        character_service,
        location_service,
        scene_service,
        skill_service,
        interaction_service,
        relationship_service,
        item_service,
        player_character_service,
        scene_resolution_service,
        sheet_template_service,
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

            // Scene service deps
            let _: &Arc<dyn SceneRepositoryPort> = &deps.scene_repo;

            // Skill service deps
            let _: &Arc<dyn SkillRepositoryPort> = &deps.skill_repo;

            // Interaction service deps
            let _: &Arc<dyn InteractionRepositoryPort> = &deps.interaction_repo;

            // Item service deps
            let _: &Arc<dyn ItemRepositoryPort> = &deps.item_repo;
            let _: &Arc<dyn PlayerCharacterRepositoryPort> = &deps.pc_repo;
            let _: &Arc<dyn RegionItemPort> = &deps.region_item;

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
            let _: &Arc<dyn WorldServicePort> = &ports.world_service;
            let _: &Arc<dyn CharacterServicePort> = &ports.character_service;
            let _: &Arc<dyn LocationServicePort> = &ports.location_service;
            let _: &Arc<dyn SceneServicePort> = &ports.scene_service;
            let _: &Arc<dyn SkillServicePort> = &ports.skill_service;
            let _: &Arc<dyn InteractionServicePort> = &ports.interaction_service;
            let _: &Arc<dyn RelationshipServicePort> = &ports.relationship_service;
            let _: &Arc<dyn ItemServicePort> = &ports.item_service;
            let _: &Arc<dyn PlayerCharacterServicePort> = &ports.player_character_service;
            let _: &Arc<dyn SceneResolutionServicePort> = &ports.scene_resolution_service;
            let _: &Arc<dyn SheetTemplateServicePort> = &ports.sheet_template_service;
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
