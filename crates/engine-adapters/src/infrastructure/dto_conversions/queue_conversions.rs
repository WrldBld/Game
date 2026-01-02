//! Queue DTO conversion functions.
//!
//! Re-exports conversion functions from engine-dto for use at the boundary
//! between domain types and protocol types in queue workers.

pub use wrldbldr_engine_dto::{
    challenge_suggestion_to_info, info_to_challenge_suggestion, info_to_narrative_event_suggestion,
    info_to_proposed_tool, narrative_event_suggestion_to_info, proposed_tool_to_info,
};
