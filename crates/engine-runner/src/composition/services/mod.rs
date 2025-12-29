//! Service Composition Modules
//!
//! This module organizes the service initialization into logical groups:
//! - Core services (database connections, event bus)
//! - Domain services (characters, locations, items, etc.)
//! - Infrastructure services (LLM, asset generation, etc.)
//!
//! Each submodule is responsible for creating and configuring a group of
//! related adapters and returning them ready to be wired into the AppState.

// TODO: Add service group modules as composition is migrated:
// pub mod core;
// pub mod domain;
// pub mod infrastructure;
