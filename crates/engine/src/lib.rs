//! WrldBldr Engine library.
//!
//! This crate contains all server-side code for the WrldBldr game engine.
//!
//! ## Structure
//!
//! - `use_cases/` - User story orchestration injecting port traits directly (ADR-009)
//! - `infrastructure/` - External dependency implementations (ports + adapters)
//! - `api/` - HTTP and WebSocket entry points
//! - `app` - Application composition

pub mod api;
pub mod app;
pub mod game_tools;
pub mod infrastructure;
pub mod llm_context;
pub mod prompt_templates;
pub mod queue_types;
pub mod stores;
pub mod use_cases;

/// Test fixtures module for integration testing.
#[cfg(test)]
pub mod test_fixtures;

/// E2E integration tests using real Neo4j via testcontainers.
#[cfg(test)]
pub mod e2e_tests;

pub use app::App;
pub use prompt_templates::{
    all_keys as prompt_template_keys, defaults as prompt_defaults, prompt_template_metadata,
    PromptTemplateCategory, PromptTemplateMetadata,
};
