//! Game Services Factory
//!
//! This module provides factory functions for creating game-specific services
//! that depend on the event bus and operate at a higher level than core services.
//!
//! # Architecture
//!
//! Following the hexagonal architecture pattern:
//! - Input: Repository ports, event bus, and clock
//! - Output: Both `Arc<dyn *ServicePort>` and `Arc<dyn *Service>` trait objects
//!
//! # Services Created
//!
//! | Service                    | Dependencies |
//! |----------------------------|--------------|
//! | ChallengeServiceImpl       | ChallengeCrud, ChallengeSkill, ChallengeScene, etc. |
//! | EventChainServiceImpl      | EventChainCrud, EventChainQuery, EventChainMembership, etc. |
//! | StoryEventServiceImpl      | StoryEventCrud, StoryEventEdge, StoryEventQuery, etc., EventBus, Clock |
//! | NarrativeEventServiceImpl  | NarrativeEventCrud, NarrativeEventTie, etc., EventBus |
//!
//! # Dependency Level
//!
//! Game services are at Level 3b (after core services, parallel with or before queue services).
//! They require the event bus from Level 2a (event_infra).

use std::sync::Arc;

use wrldbldr_engine_app::application::services::{
    ChallengeService, ChallengeServiceImpl, EventChainService, EventChainServiceImpl,
    NarrativeEventService, NarrativeEventServiceImpl, StoryEventService, StoryEventServiceImpl,
};
use wrldbldr_engine_ports::outbound::{
    // Challenge ISP ports
    ChallengeAvailabilityPort,
    ChallengeCrudPort,
    ChallengePrerequisitePort,
    ChallengeScenePort,
    ChallengeServicePort,
    ChallengeSkillPort,
    ClockPort,
    DialogueContextServicePort,
    // Event bus
    EventBusPort,
    // EventChain ISP ports
    EventChainCrudPort,
    EventChainMembershipPort,
    EventChainQueryPort,
    EventChainServicePort,
    EventChainStatePort,
    // NarrativeEvent ISP ports
    NarrativeEventCrudPort,
    NarrativeEventNpcPort,
    NarrativeEventQueryPort,
    NarrativeEventServicePort,
    NarrativeEventTiePort,
    // StoryEvent ISP ports
    StoryEventCrudPort,
    StoryEventDialoguePort,
    StoryEventEdgePort,
    StoryEventQueryPort,
    StoryEventServicePort,
};

/// Dependencies required to create game services.
///
/// This struct groups all the repository ports and shared dependencies needed
/// to construct the game-specific services. It serves as the input to the
/// `create_game_services` factory function.
///
/// All fields are `Arc<dyn Trait>` which allows cheap cloning.
pub struct GameServiceDependencies {
    // Challenge ISP ports
    pub challenge_crud: Arc<dyn ChallengeCrudPort>,
    pub challenge_skill: Arc<dyn ChallengeSkillPort>,
    pub challenge_scene: Arc<dyn ChallengeScenePort>,
    pub challenge_prerequisite: Arc<dyn ChallengePrerequisitePort>,
    pub challenge_availability: Arc<dyn ChallengeAvailabilityPort>,

    // EventChain ISP ports
    pub event_chain_crud: Arc<dyn EventChainCrudPort>,
    pub event_chain_query: Arc<dyn EventChainQueryPort>,
    pub event_chain_membership: Arc<dyn EventChainMembershipPort>,
    pub event_chain_state: Arc<dyn EventChainStatePort>,

    // StoryEvent ISP ports
    pub story_event_crud: Arc<dyn StoryEventCrudPort>,
    pub story_event_edge: Arc<dyn StoryEventEdgePort>,
    pub story_event_query: Arc<dyn StoryEventQueryPort>,
    pub story_event_dialogue: Arc<dyn StoryEventDialoguePort>,

    // NarrativeEvent ISP ports
    pub narrative_event_crud: Arc<dyn NarrativeEventCrudPort>,
    pub narrative_event_tie: Arc<dyn NarrativeEventTiePort>,
    pub narrative_event_npc: Arc<dyn NarrativeEventNpcPort>,
    pub narrative_event_query: Arc<dyn NarrativeEventQueryPort>,

    // Shared dependencies
    pub event_bus: Arc<dyn EventBusPort>,
    pub clock: Arc<dyn ClockPort>,
}

/// Output ports and app-layer traits from the game services factory.
///
/// Each service is available as both:
/// - `*_port`: The port trait for composition layer and adapters
/// - `*_service`: The app-layer trait for handlers
///
/// This follows the pattern established in `core_services.rs`.
pub struct GameServicePorts {
    // Challenge service
    pub challenge_service_port: Arc<dyn ChallengeServicePort>,
    pub challenge_service: Arc<dyn ChallengeService>,

    // EventChain service
    pub event_chain_service_port: Arc<dyn EventChainServicePort>,
    pub event_chain_service: Arc<dyn EventChainService>,

    // StoryEvent service
    pub story_event_service_port: Arc<dyn StoryEventServicePort>,
    pub story_event_service: Arc<dyn StoryEventService>,
    /// DialogueContextServicePort - StoryEventServiceImpl implements this
    pub dialogue_context_service: Arc<dyn DialogueContextServicePort>,

    // NarrativeEvent service
    pub narrative_event_service_port: Arc<dyn NarrativeEventServicePort>,
    pub narrative_event_service: Arc<dyn NarrativeEventService>,
}

/// Creates all game services from their dependencies.
///
/// This factory function constructs each service implementation ONCE, then casts
/// each to BOTH its port trait and app-layer trait. This eliminates duplicate
/// service instantiations while providing both trait versions for different use cases.
///
/// # Arguments
///
/// * `deps` - The [`GameServiceDependencies`] containing all required inputs
///
/// # Returns
///
/// A [`GameServicePorts`] struct containing both port and app-layer trait objects.
///
/// # Example
///
/// ```ignore
/// let deps = GameServiceDependencies {
///     challenge_crud: repos.challenge.crud.clone(),
///     event_bus: event_infra.event_bus.clone(),
///     clock: clock.clone(),
///     // ...
/// };
///
/// let game = create_game_services(deps);
///
/// // Use port versions for composition layer
/// let challenge_port = game.challenge_service_port.clone();
///
/// // Use app-layer versions for AppRequestHandler
/// let challenge = game.challenge_service.clone();
/// ```
pub fn create_game_services(deps: GameServiceDependencies) -> GameServicePorts {
    // =========================================================================
    // Challenge Service - single instance, cast to both traits
    // =========================================================================
    let challenge_service_impl = Arc::new(ChallengeServiceImpl::new(
        deps.challenge_crud,
        deps.challenge_skill,
        deps.challenge_scene,
        deps.challenge_prerequisite,
        deps.challenge_availability,
    ));
    let challenge_service_port: Arc<dyn ChallengeServicePort> = challenge_service_impl.clone();
    let challenge_service: Arc<dyn ChallengeService> = challenge_service_impl;

    // =========================================================================
    // EventChain Service - single instance, cast to both traits
    // =========================================================================
    let event_chain_service_impl = Arc::new(EventChainServiceImpl::new(
        deps.event_chain_crud,
        deps.event_chain_query,
        deps.event_chain_membership,
        deps.event_chain_state,
    ));
    let event_chain_service_port: Arc<dyn EventChainServicePort> = event_chain_service_impl.clone();
    let event_chain_service: Arc<dyn EventChainService> = event_chain_service_impl;

    // =========================================================================
    // StoryEvent Service - single instance, cast to multiple traits
    // StoryEventServiceImpl implements: StoryEventServicePort, StoryEventService, DialogueContextServicePort
    // =========================================================================
    let story_event_service_impl = Arc::new(StoryEventServiceImpl::new(
        deps.story_event_crud,
        deps.story_event_edge,
        deps.story_event_query,
        deps.story_event_dialogue,
        deps.event_bus.clone(),
        deps.clock,
    ));
    let story_event_service_port: Arc<dyn StoryEventServicePort> = story_event_service_impl.clone();
    let story_event_service: Arc<dyn StoryEventService> = story_event_service_impl.clone();
    let dialogue_context_service: Arc<dyn DialogueContextServicePort> = story_event_service_impl;

    // =========================================================================
    // NarrativeEvent Service - single instance, cast to both traits
    // =========================================================================
    let narrative_event_service_impl = Arc::new(NarrativeEventServiceImpl::new(
        deps.narrative_event_crud,
        deps.narrative_event_tie,
        deps.narrative_event_npc,
        deps.narrative_event_query,
        deps.event_bus,
    ));
    let narrative_event_service_port: Arc<dyn NarrativeEventServicePort> =
        narrative_event_service_impl.clone();
    let narrative_event_service: Arc<dyn NarrativeEventService> = narrative_event_service_impl;

    GameServicePorts {
        challenge_service_port,
        challenge_service,
        event_chain_service_port,
        event_chain_service,
        story_event_service_port,
        story_event_service,
        dialogue_context_service,
        narrative_event_service_port,
        narrative_event_service,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_service_ports_structure() {
        // Verify the struct has all expected fields
        // This is a compile-time check that the struct layout is correct
        fn _assert_fields(ports: GameServicePorts) {
            let _: Arc<dyn ChallengeServicePort> = ports.challenge_service_port;
            let _: Arc<dyn ChallengeService> = ports.challenge_service;
            let _: Arc<dyn EventChainServicePort> = ports.event_chain_service_port;
            let _: Arc<dyn EventChainService> = ports.event_chain_service;
            let _: Arc<dyn StoryEventServicePort> = ports.story_event_service_port;
            let _: Arc<dyn StoryEventService> = ports.story_event_service;
            let _: Arc<dyn DialogueContextServicePort> = ports.dialogue_context_service;
            let _: Arc<dyn NarrativeEventServicePort> = ports.narrative_event_service_port;
            let _: Arc<dyn NarrativeEventService> = ports.narrative_event_service;
        }
    }

    #[test]
    fn test_game_service_dependencies_structure() {
        // Verify the struct has all expected fields
        fn _assert_fields(deps: GameServiceDependencies) {
            let _: Arc<dyn ChallengeCrudPort> = deps.challenge_crud;
            let _: Arc<dyn ChallengeSkillPort> = deps.challenge_skill;
            let _: Arc<dyn ChallengeScenePort> = deps.challenge_scene;
            let _: Arc<dyn ChallengePrerequisitePort> = deps.challenge_prerequisite;
            let _: Arc<dyn ChallengeAvailabilityPort> = deps.challenge_availability;
            let _: Arc<dyn EventChainCrudPort> = deps.event_chain_crud;
            let _: Arc<dyn EventChainQueryPort> = deps.event_chain_query;
            let _: Arc<dyn EventChainMembershipPort> = deps.event_chain_membership;
            let _: Arc<dyn EventChainStatePort> = deps.event_chain_state;
            let _: Arc<dyn StoryEventCrudPort> = deps.story_event_crud;
            let _: Arc<dyn StoryEventEdgePort> = deps.story_event_edge;
            let _: Arc<dyn StoryEventQueryPort> = deps.story_event_query;
            let _: Arc<dyn StoryEventDialoguePort> = deps.story_event_dialogue;
            let _: Arc<dyn NarrativeEventCrudPort> = deps.narrative_event_crud;
            let _: Arc<dyn NarrativeEventTiePort> = deps.narrative_event_tie;
            let _: Arc<dyn NarrativeEventNpcPort> = deps.narrative_event_npc;
            let _: Arc<dyn NarrativeEventQueryPort> = deps.narrative_event_query;
            let _: Arc<dyn EventBusPort> = deps.event_bus;
            let _: Arc<dyn ClockPort> = deps.clock;
        }
    }
}
