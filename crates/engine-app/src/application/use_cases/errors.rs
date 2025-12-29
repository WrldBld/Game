//! Use case error types - Re-exported from engine-ports
//!
//! The canonical definitions are in engine-ports/src/inbound/use_case_errors.rs
//! as part of the hexagonal architecture (errors are part of the port contract).

pub use wrldbldr_engine_ports::inbound::{
    ActionError, ChallengeError, InventoryError, MovementError, NarrativeEventError,
    ObservationError, SceneError, StagingError,
};

// Re-export ErrorCode and ConnectionError
pub use wrldbldr_engine_ports::outbound::{ConnectionError, ErrorCode};
