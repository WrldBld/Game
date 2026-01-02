//! Generation service port - Re-exports from inbound port
//!
//! This module re-exports the inbound use case port for internal app-layer dependencies.
//! All consumers should use `GenerationUseCasePort` as the interface.

#[cfg(any(test, feature = "testing"))]
pub use wrldbldr_engine_ports::inbound::MockGenerationUseCasePort;
pub use wrldbldr_engine_ports::inbound::{GenerationRequest, GenerationUseCasePort};
