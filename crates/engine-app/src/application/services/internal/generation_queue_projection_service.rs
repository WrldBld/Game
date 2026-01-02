//! Generation queue projection service port - Re-exports from inbound port
//!
//! This module re-exports the inbound use case port for internal app-layer dependencies.
//! All consumers should use `GenerationQueueProjectionUseCasePort` as the interface.

pub use wrldbldr_engine_ports::inbound::{GenerationBatchSnapshot, GenerationQueueProjectionUseCasePort, GenerationQueueSnapshot, SuggestionTaskSnapshot};
#[cfg(any(test, feature = "testing"))]
pub use wrldbldr_engine_ports::inbound::MockGenerationQueueProjectionUseCasePort;
