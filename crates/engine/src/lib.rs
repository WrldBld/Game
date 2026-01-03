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

pub use app::App;
