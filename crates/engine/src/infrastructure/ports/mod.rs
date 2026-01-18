// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! Port traits for infrastructure boundaries.
//!
//! These are the ONLY abstractions in the engine. Everything else is concrete types.
//! Ports exist for:
//! - Database access (could swap Neo4j -> Postgres)
//! - LLM calls (could swap Ollama -> Claude/OpenAI)
//! - Image generation (could swap ComfyUI -> other)
//! - Queues (could swap SQLite -> Redis)
//! - Clock/Random (for testing)

mod error;
mod external;
mod repos;
mod testing;
mod types;

pub use error::*;
pub use external::*;
pub use repos::*;
pub use testing::*;
pub use types::*;
