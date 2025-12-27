//! WrldBldr Engine DTOs - Shared data types for engine internals
//!
//! This crate contains DTOs used internally within the engine layer:
//! - LLM request/response types
//! - Queue item types
//! - Request context for handlers
//!
//! # Design Principles
//!
//! 1. **Engine-internal only** - Not shared with Player (use `protocol` for that)
//! 2. **Behavior allowed** - Unlike `protocol`, these types can have constructors/methods
//! 3. **No domain dependency** - Uses only primitive types and protocol types

pub mod llm;
pub mod queue;
pub mod request_context;

pub use llm::*;
pub use queue::*;
pub use request_context::*;
