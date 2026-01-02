//! Asset service port - Re-exports from inbound port
//!
//! This module re-exports the inbound use case port for internal app-layer dependencies.
//! All consumers should use `AssetUseCasePort` as the interface.

#[cfg(any(test, feature = "testing"))]
pub use wrldbldr_engine_ports::inbound::MockAssetUseCasePort;
pub use wrldbldr_engine_ports::inbound::{AssetUseCasePort, CreateAssetRequest};
