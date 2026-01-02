//! Workflow service port - Re-exports from inbound port
//!
//! This module re-exports the inbound use case port for internal app-layer dependencies.
//! All consumers should use `WorkflowUseCasePort` as the interface.

#[cfg(any(test, feature = "testing"))]
pub use wrldbldr_engine_ports::inbound::MockWorkflowUseCasePort;
pub use wrldbldr_engine_ports::inbound::WorkflowUseCasePort;
