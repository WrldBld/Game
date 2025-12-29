//! WrldBldr Engine DTOs - Shared data types for engine internals
//!
//! This crate contains DTOs used internally within the engine layer:
//! - LLM request/response types
//! - Queue item types (payloads for player actions, DM actions, LLM requests, etc.)
//! - Request context for handlers
//!
//! # Design Principles
//!
//! 1. **Engine-internal only** - Not shared with Player (use `protocol` for that)
//! 2. **Behavior allowed** - Unlike `protocol`, these types can have constructors/methods
//! 3. **Domain types allowed** - Can depend on domain value objects for queue payloads

pub mod llm;
pub mod persistence;
pub mod queue;
pub mod request_context;
pub mod staging;

pub use llm::*;
pub use persistence::*;
pub use queue::*;
pub use request_context::*;
pub use staging::*;
