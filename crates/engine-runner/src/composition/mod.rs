//! Composition Root Module
//!
//! This module contains the dependency injection and service composition logic
//! for the engine. It is responsible for:
//! - Creating and configuring all adapters
//! - Wiring adapters to ports
//! - Building the AppState with all required services
//!
//! The composition root follows the hexagonal architecture pattern where
//! all dependencies flow inward and are assembled here at the application boundary.
//!
//! # Module Structure
//!
//! - `app_state`: Main AppState construction and wiring
//! - `factories`: Factory functions for creating port trait objects
//!   - `repositories`: ISP-compliant repository ports
//!   - `core_services`: Core domain service ports
//!   - `game_services`: Game service ports
//!   - `use_cases`: Use case construction with adapters
//! - `services`: Service container re-exports (deprecated)

pub mod app_state;
pub mod factories;
pub mod services;

pub use app_state::{new_adapter_state, AppStatePort};
pub use factories::{
    // Repository factory exports
    coerce_isp, create_repository_ports, ChallengePorts, CharacterPorts, EventChainPorts,
    LocationPorts, NarrativeEventPorts, PlayerCharacterPorts, RegionPorts, RepositoryPorts,
    ScenePorts, StoryEventPorts,
    // Game services factory exports
    create_game_services, create_story_event_ports, GameServiceDependencies, GameServicePorts,
    GameServicesResult,
    // Use case factory exports
    create_use_cases, UseCaseContext, UseCaseDependencies,
};
