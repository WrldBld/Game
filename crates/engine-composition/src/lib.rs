//! WrldBldr Engine Composition - Service container types and dependency injection
//!
//! This crate is the **composition layer** that defines how services are grouped and
//! wired together. It provides type definitions for service containers that use ONLY
//! `Arc<dyn Trait>` - no concrete adapter types are allowed here.
//!
//! ## Architecture Role
//!
//! The composition layer sits between the application and runner layers:
//!
//! ```text
//! domain → ports → adapters → app → composition → runner
//! ```
//!
//! - **Defines** service container structures (AppState, CoreServices, etc.)
//! - **Uses** port traits from `engine-ports` and service traits from `engine-app`
//! - **Does NOT** reference any concrete adapter implementations
//!
//! ## Key Design Principles
//!
//! 1. **Trait Objects Only**: All service references use `Arc<dyn Trait>` to maintain
//!    hexagonal architecture boundaries and enable testing with mock implementations.
//!
//! 2. **No Concrete Types**: This crate never imports from `engine-adapters`. The runner
//!    crate is responsible for instantiating concrete implementations and wiring them.
//!
//! 3. **Service Grouping**: Related services are grouped into container structs for
//!    better organization and easier dependency management.
//!
//! ## Main Types
//!
//! - [`AppState`]: The central composition root holding all services
//! - [`AppConfig`]: Application configuration (server addresses, URLs)
//! - [`CoreServices`]: Fundamental domain services (worlds, characters, locations)
//! - [`GameServices`]: Game mechanics (challenges, narrative events, dispositions)
//! - [`QueueServices`]: Async processing (player actions, LLM requests)
//! - [`AssetServices`]: Asset management and generation
//! - [`PlayerServices`]: Player-facing operations (character sheets, scenes)
//! - [`EventInfra`]: Domain event infrastructure
//! - [`UseCases`]: High-level operations coordinating multiple services

mod app_state;
mod asset_services;
mod core_services;
mod event_infra;
mod game_services;
mod player_services;
mod queue_services;
mod use_case_adapters;
mod use_cases;

// Re-export all public types from submodules
pub use app_state::{AppConfig, AppState, LlmPortDyn};
pub use asset_services::AssetServices;
pub use core_services::CoreServices;
pub use event_infra::EventInfra;
pub use game_services::GameServices;
pub use player_services::PlayerServices;
pub use queue_services::QueueServices;
pub use use_case_adapters::{
    AssetGenerationQueueUseCaseAdapter, AssetUseCaseAdapter, DmApprovalQueueUseCaseAdapter,
    GenerationQueueProjectionUseCaseAdapter, GenerationUseCaseAdapter, LlmQueueUseCaseAdapter,
    PlayerActionQueueUseCaseAdapter, PromptTemplateUseCaseAdapter, SettingsUseCaseAdapter,
    WorkflowUseCaseAdapter, WorldUseCaseAdapter,
};
pub use use_cases::UseCases;
