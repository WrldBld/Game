//! WrldBldr Engine library.
//!
//! This crate contains all server-side code for the WrldBldr game engine.
//!
//! ## Structure
//!
//! - `entities/` - Entity modules wrapping domain operations
//! - `use_cases/` - User story orchestration across entities  
//! - `infrastructure/` - External dependency implementations (ports + adapters)
//! - `api/` - HTTP and WebSocket entry points
//! - `app` - Application composition

pub mod api;
pub mod app;
pub mod entities;
pub mod infrastructure;
pub mod use_cases;

/// Test fixtures module for integration testing.
#[cfg(test)]
pub mod test_fixtures;

/// E2E integration tests using real Neo4j via testcontainers.
#[cfg(test)]
mod e2e_tests;

pub use app::App;
