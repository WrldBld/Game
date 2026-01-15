//! WrldBldr Engine library.
//!
//! This crate contains all server-side code for the WrldBldr game engine.
//!
//! ## Structure
//!
//! - `repositories/` - Repository modules wrapping port traits for data access
//! - `use_cases/` - User story orchestration across repositories
//! - `infrastructure/` - External dependency implementations (ports + adapters)
//! - `api/` - HTTP and WebSocket entry points
//! - `app` - Application composition

pub mod api;
pub mod app;
pub mod infrastructure;
pub mod llm_context;
pub mod queue_types;
pub mod repositories;
pub mod use_cases;

/// Test fixtures module for integration testing.
#[cfg(test)]
pub mod test_fixtures;

/// E2E integration tests using real Neo4j via testcontainers.
#[cfg(test)]
pub mod e2e_tests;

pub use app::App;
