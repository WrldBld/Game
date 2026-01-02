//! Factory Functions for Composition Root
//!
//! This module provides factory functions that reduce boilerplate in the composition root.
//! Each factory is responsible for creating a specific category of dependencies.
//!
//! # Factory Levels (Dependency Order)
//!
//! ```text
//! Level 0: infrastructure   - Clock, RNG, Neo4j, SQLite
//! Level 1: repositories     - ISP-compliant repository ports from Neo4j
//! Level 2a: event_infra     - Event bus, domain events, channels
//! Level 2b: queue_services  - Queue backends and queue services
//! Level 3a: core_services   - Core domain services (World, Character, Location, etc.)
//! Level 3b: game_services   - Game services (Challenge, EventChain, StoryEvent, NarrativeEvent)
//! Level 4: asset_services   - Asset services (Asset, Workflow, Generation)
//! Level 5: use_cases        - Use cases with adapters
//! ```
//!
//! # Architecture
//!
//! Factories follow the hexagonal architecture pattern:
//! - Input: Concrete adapter implementations or repository ports
//! - Output: `Arc<dyn Port>` trait objects for dependency injection

pub mod asset_services;
pub mod core_services;
pub mod event_infra;
pub mod game_services;
pub mod infrastructure;
pub mod queue_services;
pub mod repositories;
pub mod use_cases;

// Infrastructure factory (Level 0)
pub use infrastructure::create_infrastructure;

// Repository factory (Level 1) - used internally by app_state
pub use repositories::create_repository_ports;

// Event infrastructure factory (Level 2a)
pub use event_infra::create_event_infrastructure;

// Queue services factory (Level 2b)
pub use queue_services::{create_queue_services, QueueServiceDependencies};

// Core services factory (Level 3a)
pub use core_services::{create_core_services, CoreServiceDependencies};

// Game services factory (Level 3b)
pub use game_services::{create_game_services, GameServiceDependencies};

// Asset services factory (Level 4)
pub use asset_services::{create_asset_services, AssetServiceDependencies};

// Use cases factory (Level 5)
pub use use_cases::{create_use_cases, UseCaseDependencies};
