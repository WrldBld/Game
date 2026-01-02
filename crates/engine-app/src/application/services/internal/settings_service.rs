//! Settings service port - Re-exports from inbound port
//!
//! This module re-exports the SettingsUseCasePort trait and LlmConfig type
//! from engine-ports::inbound as the single source of truth.
//!
//! Services that need settings functionality should use SettingsUseCasePort.

// Re-export from inbound port as the single source of truth
pub use wrldbldr_engine_ports::inbound::{LlmConfig, SettingsUseCasePort};

// Re-export mock for testing
#[cfg(any(test, feature = "testing"))]
pub use wrldbldr_engine_ports::inbound::MockSettingsUseCasePort;
