//! Workflow service port - Interface for workflow configuration operations
//!
//! This port abstracts workflow configuration business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{WorkflowConfiguration, WorkflowSlot};
use wrldbldr_domain::{WorkflowConfigId, WorldId};

/// Port for workflow service operations
///
/// This trait defines the application use cases for workflow configuration
/// management, including listing, retrieving, and finding active workflows.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait WorkflowServicePort: Send + Sync {
    /// Get a workflow configuration by ID
    async fn get_workflow(&self, id: WorkflowConfigId) -> Result<Option<WorkflowConfiguration>>;

    /// List all workflow configurations for a slot
    async fn list_by_slot(&self, slot: WorkflowSlot) -> Result<Vec<WorkflowConfiguration>>;

    /// Get the active workflow configuration for a world and slot
    ///
    /// Returns the configured workflow for the given slot, falling back to
    /// a default configuration if none is explicitly set for the world.
    async fn get_active_for_slot(
        &self,
        world_id: WorldId,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>>;
}
