//! Session-related DTOs for player-app
//!
//! These types are re-exported from player-ports for use by the application layer.
//! The ports layer owns the canonical definitions to avoid circular dependencies.

// Re-export all session types from player-ports
pub use wrldbldr_player_ports::session_types::*;
