//! Outcome trigger service port
//!
//! Abstracts execution of static outcome triggers (from challenge definitions)
//! so application services can depend on a trait rather than a concrete service.

use async_trait::async_trait;

use wrldbldr_domain::entities::OutcomeTrigger;
use wrldbldr_domain::WorldId;

use wrldbldr_engine_ports::outbound::StateChange;

/// Result of executing outcome triggers.
#[derive(Debug, Clone)]
pub struct OutcomeTriggerExecutionResult {
    /// Number of triggers executed.
    pub trigger_count: usize,
    /// State changes produced by trigger execution.
    pub state_changes: Vec<StateChange>,
    /// Any non-fatal warnings that occurred.
    pub warnings: Vec<String>,
}

/// Port for executing outcome triggers.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait OutcomeTriggerServicePort: Send + Sync {
    /// Execute a list of outcome triggers.
    async fn execute_triggers(
        &self,
        triggers: Vec<OutcomeTrigger>,
        world_id: WorldId,
    ) -> OutcomeTriggerExecutionResult;
}
