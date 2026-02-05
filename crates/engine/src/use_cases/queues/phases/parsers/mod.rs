//! Phase-specific parsers for multi-phase LLM pipeline.
//!
//! Each module provides parsing for a specific phase's structured output.
//! These parsers are strict - they fail if required elements are missing.
//!
//! # Module Structure
//!
//! - `common`: Shared error types and helper functions
//! - `phase0`: Context digest parser
//! - `phase1`: Challenge trigger parser
//! - `phase2`: Event trigger parser
//! - `phase3`: Tool intent parser
//! - `phase4`: Behavior analysis parser
//! - `phase5`: Final response parser

pub mod common;
pub mod phase0;
pub mod phase1;
pub mod phase2;
pub mod phase3;
pub mod phase4;
pub mod phase5;

// Re-export commonly used items for backward compatibility
pub use common::PhaseParseError;
pub use phase0::parse_phase0_digest;
pub use phase1::parse_phase1_challenges;
pub use phase2::parse_phase2_events;
pub use phase3::parse_phase3_tools;
pub use phase4::parse_phase4_behavior;
pub use phase5::parse_phase5_response;
