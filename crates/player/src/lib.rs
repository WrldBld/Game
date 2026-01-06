//! Unified Player crate.
//!
//! This crate contains UI, application logic, and infrastructure adapters.
//! Multi-platform support is provided via compile-time `cfg` selection.

pub mod application;
pub mod infrastructure;
pub mod ports;
pub mod session_types;
pub mod state;
pub mod ui;

// Transitional root-level aliases for moved modules.
// These allow `crate::outbound::...` and `crate::presentation::...` paths to keep compiling.
pub mod outbound {
    pub use crate::ports::outbound::*;
}

pub use ui::presentation;
pub use ui::routes;

// Re-export commonly used entrypoints
pub use ui::app;
pub use ui::{use_platform, Platform, Route, ShellKind};
