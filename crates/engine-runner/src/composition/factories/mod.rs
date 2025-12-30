//! Factory Functions for Composition Root
//!
//! This module provides factory functions that reduce boilerplate in the composition root.
//! Each factory is responsible for creating a specific category of dependencies.
//!
//! # Factory Levels (Dependency Order)
//!
//! ```text
//! Level 0: infrastructure   - Clock, RNG, Neo4j, Ollama, ComfyUI, SQLite
//! Level 1: repositories     - ISP-compliant repository ports from Neo4j
//! Level 2: event_infra      - Event bus, domain events, channels (parallel with queues)
//! Level 2: queue_services   - Queue backends and queue services (parallel with events)
//! Level 3: core_services    - Core domain services (World, Character, Location, etc.)
//! Level 4: game_services    - Game services (Challenge, Narrative, Staging, etc.) [NOT YET WIRED]
//! Level 4: asset_services   - Asset services (Asset, Workflow, Generation) [parallel with game_services]
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
pub use infrastructure::{create_infrastructure, InfrastructureContext};

// Repository factory (Level 1) - used internally by app_state
pub use repositories::create_repository_ports;

// Event infrastructure factory (Level 2a)
pub use event_infra::create_event_infrastructure;

// Queue services factory (Level 2b)
pub use queue_services::{create_queue_services, QueueServiceDependencies};

// Core services factory (Level 3)
pub use core_services::{create_core_services, CoreServiceDependencies};

// Asset services factory (Level 4b)
pub use asset_services::{create_asset_services, AssetServiceDependencies};

// Use cases factory (Level 5)
pub use use_cases::{create_use_cases, UseCaseDependencies};
