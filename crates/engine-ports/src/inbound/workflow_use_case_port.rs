//! Workflow use case port - Inbound interface for workflow configuration operations
//!
//! This port is called by HTTP handlers to manage ComfyUI workflow configurations.
//! The implementation lives in engine-app.
//!
//! # Architecture Note
//!
//! Workflow utility functions (analyze_workflow, validate_workflow, prepare_workflow, etc.)
//! are implemented in `engine-app::application::services::WorkflowService`.
//! Adapters should import from there, not from this port.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{InputDefault, PromptMapping, WorkflowConfiguration, WorkflowSlot};
use wrldbldr_domain::{WorkflowConfigId, WorldId};

/// Port for workflow use case operations
///
/// This trait defines the application use cases for workflow configuration
/// management, including listing, retrieving, saving, deleting, and finding
/// active workflows.
///
/// Called by: HTTP handlers in workflow_routes.rs
/// Implemented by: WorkflowConfigService in engine-app
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait WorkflowUseCasePort: Send + Sync {
    /// Get a workflow configuration by ID
    async fn get_workflow(&self, id: WorkflowConfigId) -> Result<Option<WorkflowConfiguration>>;

    /// List all workflow configurations
    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>>;

    /// List all workflow configurations for a slot
    async fn list_by_slot(&self, slot: WorkflowSlot) -> Result<Vec<WorkflowConfiguration>>;

    /// Get a workflow configuration by slot
    ///
    /// Returns the workflow configuration for the given slot, if one exists.
    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>>;

    /// Save a workflow configuration
    ///
    /// Creates a new configuration or updates an existing one based on the slot.
    async fn save(&self, config: &WorkflowConfiguration) -> Result<()>;

    /// Delete a workflow configuration by slot
    ///
    /// Returns true if a configuration was deleted, false if none existed.
    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool>;

    /// Get the active workflow configuration for a world and slot
    ///
    /// Returns the configured workflow for the given slot, falling back to
    /// a default configuration if none is explicitly set for the world.
    async fn get_active_for_slot(
        &self,
        world_id: WorldId,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>>;

    /// Create a new workflow configuration or update an existing one
    ///
    /// Returns the created/updated configuration with is_update flag.
    async fn create_or_update(
        &self,
        slot: WorkflowSlot,
        name: String,
        workflow_json: serde_json::Value,
        prompt_mappings: Vec<PromptMapping>,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Vec<String>,
    ) -> Result<(WorkflowConfiguration, bool)>;

    /// Update just the defaults for an existing workflow configuration
    ///
    /// Returns the updated configuration, or an error if no configuration exists for the slot.
    async fn update_defaults(
        &self,
        slot: WorkflowSlot,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Option<Vec<String>>,
    ) -> Result<WorkflowConfiguration>;

    /// Import workflow configurations, optionally replacing existing ones
    ///
    /// Returns (imported_count, skipped_count).
    async fn import_configs(
        &self,
        configs: Vec<WorkflowConfiguration>,
        replace_existing: bool,
    ) -> Result<(usize, usize)>;
}
