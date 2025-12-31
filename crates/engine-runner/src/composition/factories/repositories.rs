//! Repository Port Factory
//!
//! This module provides factory functions for creating ISP-compliant repository ports
//! from concrete Neo4j repository implementations. It reduces boilerplate by using
//! the `coerce_isp!` macro to generate trait object coercions.
//!
//! # Architecture
//!
//! Following the Interface Segregation Principle (ISP), each "god" repository trait
//! has been split into focused sub-traits. A single concrete repository implements
//! all sub-traits, and this module coerces them to `Arc<dyn Trait>` for dependency injection.
//!
//! # ISP Repository Splits
//!
//! | Entity           | Traits |
//! |------------------|--------|
//! | Character        | Crud, Want, Actantial, Inventory, Location, Disposition |
//! | Location         | Crud, Hierarchy, Connection, Map |
//! | Region           | Crud, Connection, Exit, Npc, Item |
//! | Challenge        | Crud, Skill, Scene, Prerequisite, Availability |
//! | StoryEvent       | Crud, Edge, Query, Dialogue |
//! | NarrativeEvent   | Crud, Tie, Npc, Query |
//! | PlayerCharacter  | Crud, Query, Position, Inventory |
//! | Scene            | Crud, Query, Location, FeaturedCharacter, Completion |
//! | EventChain       | Crud, Query, Membership, State |

use std::sync::Arc;

use wrldbldr_engine_adapters::infrastructure::persistence::Neo4jRepository;
use wrldbldr_engine_ports::outbound::{
    // World repository (non-ISP)
    WorldRepositoryPort,
    // Character ISP ports
    CharacterActantialPort, CharacterCrudPort, CharacterDispositionPort, CharacterInventoryPort,
    CharacterLocationPort, CharacterWantPort,
    // Location ISP ports
    LocationConnectionPort, LocationCrudPort, LocationHierarchyPort, LocationMapPort,
    // Region ISP ports
    RegionConnectionPort, RegionCrudPort, RegionExitPort, RegionItemPort, RegionNpcPort,
    // Challenge ISP ports
    ChallengeAvailabilityPort, ChallengeCrudPort, ChallengePrerequisitePort, ChallengeScenePort,
    ChallengeSkillPort,
    // StoryEvent ISP ports
    StoryEventCrudPort, StoryEventDialoguePort, StoryEventEdgePort, StoryEventQueryPort,
    // NarrativeEvent ISP ports
    NarrativeEventCrudPort, NarrativeEventNpcPort, NarrativeEventQueryPort, NarrativeEventTiePort,
    // PlayerCharacter ISP ports (god trait removed - use ISP traits instead)
    PlayerCharacterCrudPort, PlayerCharacterInventoryPort, PlayerCharacterPositionPort,
    PlayerCharacterQueryPort,
    // Scene ISP ports (god trait removed)
    SceneCompletionPort, SceneCrudPort, SceneFeaturedCharacterPort, SceneLocationPort,
    SceneQueryPort,
    // EventChain ISP ports
    EventChainCrudPort, EventChainMembershipPort, EventChainQueryPort, EventChainStatePort,
    // Non-ISP repository ports (single trait per entity)
    RelationshipRepositoryPort, SkillRepositoryPort, InteractionRepositoryPort,
    AssetRepositoryPort, WorkflowRepositoryPort, SheetTemplateRepositoryPort,
    ItemRepositoryPort, GoalRepositoryPort, WantRepositoryPort, FlagRepositoryPort,
    ObservationRepositoryPort, StagingRepositoryPort,
};

/// Macro to reduce boilerplate when coercing a concrete repository to multiple ISP trait objects.
///
/// # Example
///
/// ```ignore
/// coerce_isp!(
///     character_concrete,
///     dyn CharacterCrudPort => character_crud,
///     dyn CharacterWantPort => character_want,
///     dyn CharacterActantialPort => character_actantial,
/// );
/// ```
///
/// Expands to:
/// ```ignore
/// let character_crud: Arc<dyn CharacterCrudPort> = character_concrete.clone();
/// let character_want: Arc<dyn CharacterWantPort> = character_concrete.clone();
/// let character_actantial: Arc<dyn CharacterActantialPort> = character_concrete.clone();
/// ```
///
/// # Note
///
/// You must prefix each trait with `dyn` to ensure proper parsing.
#[macro_export]
macro_rules! coerce_isp {
    ($concrete:expr, $(dyn $trait_name:path => $var_name:ident),+ $(,)?) => {
        $(
            let $var_name: std::sync::Arc<dyn $trait_name> = $concrete.clone();
        )+
    };
}



// ============================================================================
// ISP Port Container Structs
// ============================================================================
// These structs are preparatory infrastructure for gradual ISP migration.
// Not all fields are used yet, but they enable services to incrementally
// adopt ISP traits instead of "god traits".

/// Container for all Character repository ISP ports.
///
/// These 6 traits cover the full `CharacterRepositoryPort` interface:
/// - `CharacterCrudPort`: Core CRUD operations (6 methods)
/// - `CharacterWantPort`: Want management (7 methods)
/// - `CharacterActantialPort`: Actantial view management (5 methods)
/// - `CharacterInventoryPort`: Inventory management (5 methods)
/// - `CharacterLocationPort`: Location relationships (13 methods)
/// - `CharacterDispositionPort`: NPC disposition tracking (6 methods)
#[allow(dead_code)]
pub struct CharacterPorts {
    pub crud: Arc<dyn CharacterCrudPort>,
    pub want: Arc<dyn CharacterWantPort>,
    pub actantial: Arc<dyn CharacterActantialPort>,
    pub inventory: Arc<dyn CharacterInventoryPort>,
    pub location: Arc<dyn CharacterLocationPort>,
    pub disposition: Arc<dyn CharacterDispositionPort>,
}

/// Container for all Location repository ISP ports.
///
/// These 4 traits cover the full `LocationRepositoryPort` interface:
/// - `LocationCrudPort`: Core CRUD operations (5 methods)
/// - `LocationHierarchyPort`: Parent-child relationships (4 methods)
/// - `LocationConnectionPort`: Navigation connections (5 methods)
/// - `LocationMapPort`: Grid maps and regions (5 methods)
#[allow(dead_code)]
pub struct LocationPorts {
    pub crud: Arc<dyn LocationCrudPort>,
    pub hierarchy: Arc<dyn LocationHierarchyPort>,
    pub connection: Arc<dyn LocationConnectionPort>,
    pub map: Arc<dyn LocationMapPort>,
}

/// Container for all Region repository ISP ports.
///
/// These 5 traits cover the full `RegionRepositoryPort` interface:
/// - `RegionCrudPort`: Core CRUD operations (5 methods)
/// - `RegionConnectionPort`: Region-to-region connections (4 methods)
/// - `RegionExitPort`: Region-to-location exits (3 methods)
/// - `RegionNpcPort`: NPC relationship queries (1 method)
/// - `RegionItemPort`: Item placement in regions (3 methods)
#[allow(dead_code)]
pub struct RegionPorts {
    pub crud: Arc<dyn RegionCrudPort>,
    pub connection: Arc<dyn RegionConnectionPort>,
    pub exit: Arc<dyn RegionExitPort>,
    pub npc: Arc<dyn RegionNpcPort>,
    pub item: Arc<dyn RegionItemPort>,
}

/// Container for all Challenge repository ISP ports.
///
/// These 5 traits cover the full `ChallengeRepositoryPort` interface:
/// - `ChallengeCrudPort`: Core CRUD + state management (12 methods)
/// - `ChallengeSkillPort`: Skill relationship management (3 methods)
/// - `ChallengeScenePort`: Scene relationship management (3 methods)
/// - `ChallengePrerequisitePort`: Prerequisite chain management (4 methods)
/// - `ChallengeAvailabilityPort`: Location/region availability + unlocks (9 methods)
#[allow(dead_code)]
pub struct ChallengePorts {
    pub crud: Arc<dyn ChallengeCrudPort>,
    pub skill: Arc<dyn ChallengeSkillPort>,
    pub scene: Arc<dyn ChallengeScenePort>,
    pub prerequisite: Arc<dyn ChallengePrerequisitePort>,
    pub availability: Arc<dyn ChallengeAvailabilityPort>,
}

/// Container for all StoryEvent repository ISP ports.
///
/// These 4 traits cover the full `StoryEventRepositoryPort` interface:
/// - `StoryEventCrudPort`: Core CRUD + state management (7 methods)
/// - `StoryEventEdgePort`: Edge relationship management (15 methods)
/// - `StoryEventQueryPort`: Query operations (10 methods)
/// - `StoryEventDialoguePort`: Dialogue-specific operations (2 methods)
#[allow(dead_code)]
pub struct StoryEventPorts {
    pub crud: Arc<dyn StoryEventCrudPort>,
    pub edge: Arc<dyn StoryEventEdgePort>,
    pub query: Arc<dyn StoryEventQueryPort>,
    pub dialogue: Arc<dyn StoryEventDialoguePort>,
}

/// Container for all NarrativeEvent repository ISP ports.
///
/// These 4 traits cover the full `NarrativeEventRepositoryPort` interface:
/// - `NarrativeEventCrudPort`: Core CRUD + state management (12 methods)
/// - `NarrativeEventTiePort`: Scene/Location/Act relationships (9 methods)
/// - `NarrativeEventNpcPort`: Featured NPC management (5 methods)
/// - `NarrativeEventQueryPort`: Query by relationships (4 methods)
#[allow(dead_code)]
pub struct NarrativeEventPorts {
    pub crud: Arc<dyn NarrativeEventCrudPort>,
    pub tie: Arc<dyn NarrativeEventTiePort>,
    pub npc: Arc<dyn NarrativeEventNpcPort>,
    pub query: Arc<dyn NarrativeEventQueryPort>,
}

/// Container for all PlayerCharacter repository ISP ports.
///
/// These 4 traits cover the full `PlayerCharacterRepositoryPort` interface (god trait removed):
/// - `PlayerCharacterCrudPort`: Core CRUD operations (5 methods)
/// - `PlayerCharacterQueryPort`: Query/lookup operations (4 methods)
/// - `PlayerCharacterPositionPort`: Position/movement operations (3 methods)
/// - `PlayerCharacterInventoryPort`: Inventory management (5 methods)
#[allow(dead_code)]
pub struct PlayerCharacterPorts {
    pub crud: Arc<dyn PlayerCharacterCrudPort>,
    pub query: Arc<dyn PlayerCharacterQueryPort>,
    pub position: Arc<dyn PlayerCharacterPositionPort>,
    pub inventory: Arc<dyn PlayerCharacterInventoryPort>,
}

/// Container for all Scene repository ISP ports.
///
/// These 5 traits cover the full Scene repository interface (god trait removed):
/// - `SceneCrudPort`: Core CRUD operations (5 methods)
/// - `SceneQueryPort`: Query by act/location (2 methods)
/// - `SceneLocationPort`: AT_LOCATION edge management (2 methods)
/// - `SceneFeaturedCharacterPort`: FEATURES_CHARACTER edges (5 methods)
/// - `SceneCompletionPort`: COMPLETED_SCENE tracking (3 methods)
pub struct ScenePorts {
    pub crud: Arc<dyn SceneCrudPort>,
    pub query: Arc<dyn SceneQueryPort>,
    pub location: Arc<dyn SceneLocationPort>,
    pub featured_character: Arc<dyn SceneFeaturedCharacterPort>,
    pub completion: Arc<dyn SceneCompletionPort>,
}

/// Container for all EventChain repository ISP ports.
///
/// These 4 traits cover the full `EventChainRepositoryPort` interface:
/// - `EventChainCrudPort`: Core CRUD operations (4 methods)
/// - `EventChainQueryPort`: Query/lookup operations (4 methods)
/// - `EventChainMembershipPort`: Event membership management (3 methods)
/// - `EventChainStatePort`: Status and state management (5 methods)
#[allow(dead_code)]
pub struct EventChainPorts {
    pub crud: Arc<dyn EventChainCrudPort>,
    pub query: Arc<dyn EventChainQueryPort>,
    pub membership: Arc<dyn EventChainMembershipPort>,
    pub state: Arc<dyn EventChainStatePort>,
}

/// Container for all repository ports (both ISP-split and non-ISP).
///
/// This struct holds all repository trait objects needed by the application.
/// ISP-split repositories are grouped into sub-structs, while non-ISP
/// repositories are held as individual `Arc<dyn Trait>` fields.
///
/// # Usage
///
/// ```ignore
/// let repos = create_repository_ports(&repository);
///
/// // Access ISP-split ports
/// let char_crud = repos.character.crud.clone();
/// let char_want = repos.character.want.clone();
///
/// // Access non-ISP ports
/// let world_repo = repos.world.clone();
/// ```
#[allow(dead_code)]
pub struct RepositoryPorts {
    // ISP-split repository port groups
    pub character: CharacterPorts,
    pub location: LocationPorts,
    pub region: RegionPorts,
    pub challenge: ChallengePorts,
    pub story_event: StoryEventPorts,
    pub narrative_event: NarrativeEventPorts,
    pub player_character: PlayerCharacterPorts,
    pub scene: ScenePorts,
    pub event_chain: EventChainPorts,

    // Non-ISP repository ports (single trait per entity)
    pub world: Arc<dyn WorldRepositoryPort>,
    pub relationship: Arc<dyn RelationshipRepositoryPort>,
    pub skill: Arc<dyn SkillRepositoryPort>,
    pub interaction: Arc<dyn InteractionRepositoryPort>,
    pub asset: Arc<dyn AssetRepositoryPort>,
    pub workflow: Arc<dyn WorkflowRepositoryPort>,
    pub sheet_template: Arc<dyn SheetTemplateRepositoryPort>,
    pub item: Arc<dyn ItemRepositoryPort>,
    pub goal: Arc<dyn GoalRepositoryPort>,
    pub want: Arc<dyn WantRepositoryPort>,
    pub flag: Arc<dyn FlagRepositoryPort>,
    pub observation: Arc<dyn ObservationRepositoryPort>,
    pub staging: Arc<dyn StagingRepositoryPort>,
}

/// Creates all repository ports from a Neo4j repository instance.
///
/// This function instantiates concrete repository implementations and coerces them
/// to their respective ISP trait objects. The same concrete instance is used for
/// all ISP sub-traits of a given entity type.
///
/// # Arguments
///
/// * `repository` - The Neo4j repository providing access to all entity repositories
///
/// # Returns
///
/// A `RepositoryPorts` struct containing all trait objects ready for dependency injection.
///
/// # Example
///
/// ```ignore
/// let repository = Neo4jRepository::new(&uri, &user, &password, &database, clock.clone()).await?;
/// let ports = create_repository_ports(&repository);
///
/// // Use ports in service construction
/// let character_service = CharacterServiceImpl::new(
///     ports.world.clone(),
///     ports.character.crud.clone(),
///     ports.character.want.clone(),
///     ports.relationship.clone(),
///     settings_service.clone(),
///     clock.clone(),
/// );
/// ```
pub fn create_repository_ports(repository: &Neo4jRepository) -> RepositoryPorts {
    // Character repository - ISP split into 6 traits
    let character_concrete = Arc::new(repository.characters());
    coerce_isp!(
        character_concrete,
        dyn CharacterCrudPort => character_crud,
        dyn CharacterWantPort => character_want,
        dyn CharacterActantialPort => character_actantial,
        dyn CharacterInventoryPort => character_inventory,
        dyn CharacterLocationPort => character_location,
        dyn CharacterDispositionPort => character_disposition,
    );

    // Location repository - ISP split into 4 traits
    let location_concrete = Arc::new(repository.locations());
    coerce_isp!(
        location_concrete,
        dyn LocationCrudPort => location_crud,
        dyn LocationHierarchyPort => location_hierarchy,
        dyn LocationConnectionPort => location_connection,
        dyn LocationMapPort => location_map,
    );

    // Region repository - ISP split into 5 traits
    let region_concrete = Arc::new(repository.regions());
    coerce_isp!(
        region_concrete,
        dyn RegionCrudPort => region_crud,
        dyn RegionConnectionPort => region_connection,
        dyn RegionExitPort => region_exit,
        dyn RegionNpcPort => region_npc,
        dyn RegionItemPort => region_item,
    );

    // Challenge repository - ISP split into 5 traits
    let challenge_concrete = Arc::new(repository.challenges());
    coerce_isp!(
        challenge_concrete,
        dyn ChallengeCrudPort => challenge_crud,
        dyn ChallengeSkillPort => challenge_skill,
        dyn ChallengeScenePort => challenge_scene,
        dyn ChallengePrerequisitePort => challenge_prerequisite,
        dyn ChallengeAvailabilityPort => challenge_availability,
    );

    // StoryEvent repository - ISP split into 4 traits
    let story_event_concrete = Arc::new(repository.story_events());
    coerce_isp!(
        story_event_concrete,
        dyn StoryEventCrudPort => story_event_crud,
        dyn StoryEventEdgePort => story_event_edge,
        dyn StoryEventQueryPort => story_event_query,
        dyn StoryEventDialoguePort => story_event_dialogue,
    );

    // NarrativeEvent repository - ISP split into 4 traits
    let narrative_event_concrete = Arc::new(repository.narrative_events());
    coerce_isp!(
        narrative_event_concrete,
        dyn NarrativeEventCrudPort => narrative_event_crud,
        dyn NarrativeEventTiePort => narrative_event_tie,
        dyn NarrativeEventNpcPort => narrative_event_npc,
        dyn NarrativeEventQueryPort => narrative_event_query,
    );

    // PlayerCharacter repository - ISP split into 4 traits (god trait removed)
    let player_character_concrete = Arc::new(repository.player_characters());
    coerce_isp!(
        player_character_concrete,
        dyn PlayerCharacterCrudPort => player_character_crud,
        dyn PlayerCharacterQueryPort => player_character_query,
        dyn PlayerCharacterPositionPort => player_character_position,
        dyn PlayerCharacterInventoryPort => player_character_inventory,
    );

    // Scene repository - ISP split into 5 traits (god trait removed)
    let scene_concrete = Arc::new(repository.scenes());
    coerce_isp!(
        scene_concrete,
        dyn SceneCrudPort => scene_crud,
        dyn SceneQueryPort => scene_query,
        dyn SceneLocationPort => scene_location,
        dyn SceneFeaturedCharacterPort => scene_featured_character,
        dyn SceneCompletionPort => scene_completion,
    );

    // EventChain repository - ISP split into 4 traits
    let event_chain_concrete = Arc::new(repository.event_chains());
    coerce_isp!(
        event_chain_concrete,
        dyn EventChainCrudPort => event_chain_crud,
        dyn EventChainQueryPort => event_chain_query,
        dyn EventChainMembershipPort => event_chain_membership,
        dyn EventChainStatePort => event_chain_state,
    );

    // Non-ISP repositories (single trait per entity)
    let world: Arc<dyn WorldRepositoryPort> = Arc::new(repository.worlds());
    let relationship: Arc<dyn RelationshipRepositoryPort> = Arc::new(repository.relationships());
    let skill: Arc<dyn SkillRepositoryPort> = Arc::new(repository.skills());
    let interaction: Arc<dyn InteractionRepositoryPort> = Arc::new(repository.interactions());
    let asset: Arc<dyn AssetRepositoryPort> = Arc::new(repository.assets());
    let workflow: Arc<dyn WorkflowRepositoryPort> = Arc::new(repository.workflows());
    let sheet_template: Arc<dyn SheetTemplateRepositoryPort> = Arc::new(repository.sheet_templates());
    let item: Arc<dyn ItemRepositoryPort> = Arc::new(repository.items());
    let goal: Arc<dyn GoalRepositoryPort> = Arc::new(repository.goals());
    let want: Arc<dyn WantRepositoryPort> = Arc::new(repository.wants());
    let flag: Arc<dyn FlagRepositoryPort> = Arc::new(repository.flags());
    let observation: Arc<dyn ObservationRepositoryPort> = Arc::new(repository.observations());
    let staging: Arc<dyn StagingRepositoryPort> = Arc::new(repository.stagings());

    RepositoryPorts {
        character: CharacterPorts {
            crud: character_crud,
            want: character_want,
            actantial: character_actantial,
            inventory: character_inventory,
            location: character_location,
            disposition: character_disposition,
        },
        location: LocationPorts {
            crud: location_crud,
            hierarchy: location_hierarchy,
            connection: location_connection,
            map: location_map,
        },
        region: RegionPorts {
            crud: region_crud,
            connection: region_connection,
            exit: region_exit,
            npc: region_npc,
            item: region_item,
        },
        challenge: ChallengePorts {
            crud: challenge_crud,
            skill: challenge_skill,
            scene: challenge_scene,
            prerequisite: challenge_prerequisite,
            availability: challenge_availability,
        },
        story_event: StoryEventPorts {
            crud: story_event_crud,
            edge: story_event_edge,
            query: story_event_query,
            dialogue: story_event_dialogue,
        },
        narrative_event: NarrativeEventPorts {
            crud: narrative_event_crud,
            tie: narrative_event_tie,
            npc: narrative_event_npc,
            query: narrative_event_query,
        },
        player_character: PlayerCharacterPorts {
            crud: player_character_crud,
            query: player_character_query,
            position: player_character_position,
            inventory: player_character_inventory,
        },
        scene: ScenePorts {
            crud: scene_crud,
            query: scene_query,
            location: scene_location,
            featured_character: scene_featured_character,
            completion: scene_completion,
        },
        event_chain: EventChainPorts {
            crud: event_chain_crud,
            query: event_chain_query,
            membership: event_chain_membership,
            state: event_chain_state,
        },
        world,
        relationship,
        skill,
        interaction,
        asset,
        workflow,
        sheet_template,
        item,
        goal,
        want,
        flag,
        observation,
        staging,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the coerce_isp! macro correctly generates variable bindings.
    ///
    /// This test verifies the macro expansion by using mock trait objects.
    /// In production, the concrete Neo4j repositories implement all ISP traits,
    /// so the coercion is valid. Here we just verify the syntax works.
    #[test]
    fn test_coerce_isp_macro_syntax() {
        // Create a mock struct that implements multiple traits
        struct MockMultiTrait;
        
        trait TraitA: Send + Sync {}
        trait TraitB: Send + Sync {}
        trait TraitC: Send + Sync {}
        
        impl TraitA for MockMultiTrait {}
        impl TraitB for MockMultiTrait {}
        impl TraitC for MockMultiTrait {}
        
        let concrete = Arc::new(MockMultiTrait);
        
        // Use the macro to coerce to multiple trait objects
        coerce_isp!(
            concrete,
            dyn TraitA => trait_a,
            dyn TraitB => trait_b,
            dyn TraitC => trait_c,
        );
        
        // Verify the variables were created with correct types
        let _: Arc<dyn TraitA> = trait_a;
        let _: Arc<dyn TraitB> = trait_b;
        let _: Arc<dyn TraitC> = trait_c;
    }

    /// Test that the macro handles single trait coercion.
    #[test]
    fn test_coerce_isp_macro_single_trait() {
        struct MockSingle;
        trait SingleTrait: Send + Sync {}
        impl SingleTrait for MockSingle {}
        
        let concrete = Arc::new(MockSingle);
        
        coerce_isp!(concrete, dyn SingleTrait => single);
        
        let _: Arc<dyn SingleTrait> = single;
    }

    /// Test that the macro handles trailing comma correctly.
    #[test]
    fn test_coerce_isp_macro_trailing_comma() {
        struct MockTrailing;
        trait TrailingTrait: Send + Sync {}
        impl TrailingTrait for MockTrailing {}
        
        let concrete = Arc::new(MockTrailing);
        
        // With trailing comma
        coerce_isp!(concrete, dyn TrailingTrait => with_comma,);
        
        let _: Arc<dyn TrailingTrait> = with_comma;
    }

    /// Test that RepositoryPorts struct has all expected fields.
    ///
    /// This is a compile-time test - if the struct fields don't match,
    /// the code won't compile. We just verify the field structure here.
    #[test]
    fn test_repository_ports_structure() {
        // This test verifies the struct field names at compile time.
        // We can't actually instantiate RepositoryPorts without a real
        // Neo4j connection, but we can verify the type structure exists.
        
        fn _verify_character_ports(ports: &CharacterPorts) {
            let _ = &ports.crud;
            let _ = &ports.want;
            let _ = &ports.actantial;
            let _ = &ports.inventory;
            let _ = &ports.location;
            let _ = &ports.disposition;
        }
        
        fn _verify_location_ports(ports: &LocationPorts) {
            let _ = &ports.crud;
            let _ = &ports.hierarchy;
            let _ = &ports.connection;
            let _ = &ports.map;
        }
        
        fn _verify_region_ports(ports: &RegionPorts) {
            let _ = &ports.crud;
            let _ = &ports.connection;
            let _ = &ports.exit;
            let _ = &ports.npc;
            let _ = &ports.item;
        }
        
        fn _verify_challenge_ports(ports: &ChallengePorts) {
            let _ = &ports.crud;
            let _ = &ports.skill;
            let _ = &ports.scene;
            let _ = &ports.prerequisite;
            let _ = &ports.availability;
        }
        
        fn _verify_story_event_ports(ports: &StoryEventPorts) {
            let _ = &ports.crud;
            let _ = &ports.edge;
            let _ = &ports.query;
            let _ = &ports.dialogue;
        }
        
        fn _verify_narrative_event_ports(ports: &NarrativeEventPorts) {
            let _ = &ports.crud;
            let _ = &ports.tie;
            let _ = &ports.npc;
            let _ = &ports.query;
        }
        
        fn _verify_player_character_ports(ports: &PlayerCharacterPorts) {
            let _ = &ports.crud;
            let _ = &ports.query;
            let _ = &ports.position;
            let _ = &ports.inventory;
        }
        
        fn _verify_scene_ports(ports: &ScenePorts) {
            let _ = &ports.crud;
            let _ = &ports.query;
            let _ = &ports.location;
            let _ = &ports.featured_character;
            let _ = &ports.completion;
        }
        
        fn _verify_event_chain_ports(ports: &EventChainPorts) {
            let _ = &ports.crud;
            let _ = &ports.query;
            let _ = &ports.membership;
            let _ = &ports.state;
        }
        
        fn _verify_repository_ports(ports: &RepositoryPorts) {
            // ISP-split groups
            _verify_character_ports(&ports.character);
            _verify_location_ports(&ports.location);
            _verify_region_ports(&ports.region);
            _verify_challenge_ports(&ports.challenge);
            _verify_story_event_ports(&ports.story_event);
            _verify_narrative_event_ports(&ports.narrative_event);
            _verify_player_character_ports(&ports.player_character);
            _verify_scene_ports(&ports.scene);
            _verify_event_chain_ports(&ports.event_chain);
            
            // Non-ISP ports
            let _ = &ports.world;
            let _ = &ports.relationship;
            let _ = &ports.skill;
            let _ = &ports.interaction;
            let _ = &ports.asset;
            let _ = &ports.workflow;
            let _ = &ports.sheet_template;
            let _ = &ports.item;
            let _ = &ports.goal;
            let _ = &ports.want;
            let _ = &ports.flag;
            let _ = &ports.observation;
            let _ = &ports.staging;
        }
        
        // The existence of this function proves the types are correct at compile time
        let _ = _verify_repository_ports;
    }
}
