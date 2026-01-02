//! Asset generation queue service port - Re-exports from inbound port
//!
//! This module re-exports the inbound use case port for internal app-layer dependencies.
//! All consumers should use `AssetGenerationQueueUseCasePort` as the interface.

pub use wrldbldr_engine_ports::inbound::{AssetGenerationQueueItem, AssetGenerationQueueUseCasePort, AssetGenerationRequest, GenerationMetadata, GenerationResult};
#[cfg(any(test, feature = "testing"))]
pub use wrldbldr_engine_ports::inbound::MockAssetGenerationQueueUseCasePort;
