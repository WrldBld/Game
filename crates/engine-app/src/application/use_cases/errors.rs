//! Use case error types - Re-exported from engine-ports.

pub use wrldbldr_engine_ports::outbound::{
    ActionError, ChallengeError, InventoryError, MovementError, NarrativeEventError,
    ObservationError, SceneError, StagingError,
};

// Re-export ErrorCode and ConnectionError
pub use wrldbldr_engine_ports::outbound::{ConnectionError, ErrorCode};
