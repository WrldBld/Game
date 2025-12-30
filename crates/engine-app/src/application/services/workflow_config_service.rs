//! Workflow configuration CRUD service
//!
//! This service handles persistence operations for workflow configurations.
//! It provides a clean interface between the HTTP layer and the repository.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use wrldbldr_domain::entities::{InputDefault, PromptMapping, WorkflowConfiguration, WorkflowSlot};
use wrldbldr_domain::{WorkflowConfigId, WorldId};
use wrldbldr_engine_ports::outbound::{ClockPort, WorkflowRepositoryPort, WorkflowServicePort};

/// Service for managing workflow configuration persistence
pub struct WorkflowConfigService {
    repository: Arc<dyn WorkflowRepositoryPort>,
    clock: Arc<dyn ClockPort>,
}

impl WorkflowConfigService {
    /// Create a new workflow configuration service
    pub fn new(repository: Arc<dyn WorkflowRepositoryPort>, clock: Arc<dyn ClockPort>) -> Self {
        Self { repository, clock }
    }

    /// Get a workflow configuration by slot
    pub async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        self.repository.get_by_slot(slot).await
    }

    /// Save a workflow configuration (create or update)
    pub async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        self.repository.save(config).await
    }

    /// Delete a workflow configuration by slot
    pub async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        self.repository.delete_by_slot(slot).await
    }

    /// List all workflow configurations
    pub async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        self.repository.list_all().await
    }

    /// List all workflow configurations for a slot
    pub async fn list_by_slot(&self, slot: WorkflowSlot) -> Result<Vec<WorkflowConfiguration>> {
        // For now, this just returns the single config for the slot, wrapped in a vec
        // This could be extended in the future to support multiple workflows per slot
        match self.repository.get_by_slot(slot).await? {
            Some(config) => Ok(vec![config]),
            None => Ok(vec![]),
        }
    }

    /// Get a workflow configuration by ID
    ///
    /// Note: The repository currently only supports slot-based lookup.
    /// This method iterates through all configs to find the matching ID.
    pub async fn get_workflow(
        &self,
        id: WorkflowConfigId,
    ) -> Result<Option<WorkflowConfiguration>> {
        let all = self.repository.list_all().await?;
        Ok(all.into_iter().find(|c| c.id == id))
    }

    /// Get the active workflow configuration for a world and slot
    ///
    /// Returns the configured workflow for the given slot, falling back to
    /// a default configuration if none is explicitly set for the world.
    pub async fn get_active_for_slot(
        &self,
        _world_id: WorldId,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>> {
        // For now, world-specific configurations are not implemented
        // Just return the global slot configuration
        self.get_by_slot(slot).await
    }

    /// Create a new workflow configuration or update an existing one
    ///
    /// Returns the created/updated configuration along with a flag indicating if it was an update.
    pub async fn create_or_update(
        &self,
        slot: WorkflowSlot,
        name: String,
        workflow_json: serde_json::Value,
        prompt_mappings: Vec<PromptMapping>,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Vec<String>,
    ) -> Result<(WorkflowConfiguration, bool)> {
        let existing = self.repository.get_by_slot(slot).await?;
        let is_update = existing.is_some();
        let now = self.clock.now();

        let config = if let Some(mut existing_config) = existing {
            // Update existing
            existing_config.name = name;
            existing_config.update_workflow(workflow_json, now);
            existing_config.set_prompt_mappings(prompt_mappings, now);
            existing_config.set_input_defaults(input_defaults, now);
            existing_config.set_locked_inputs(locked_inputs, now);
            existing_config
        } else {
            // Create new
            let mut config = WorkflowConfiguration::new(slot, name, workflow_json, now);
            config.set_prompt_mappings(prompt_mappings, now);
            config.set_input_defaults(input_defaults, now);
            config.set_locked_inputs(locked_inputs, now);
            config
        };

        self.repository.save(&config).await?;
        Ok((config, is_update))
    }

    /// Update just the defaults for an existing workflow configuration
    ///
    /// Returns the updated configuration, or an error if no configuration exists for the slot.
    pub async fn update_defaults(
        &self,
        slot: WorkflowSlot,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Option<Vec<String>>,
    ) -> Result<WorkflowConfiguration> {
        let mut config = self
            .repository
            .get_by_slot(slot)
            .await?
            .ok_or_else(|| anyhow!("No workflow configured for slot: {}", slot.as_str()))?;

        let now = self.clock.now();
        config.set_input_defaults(input_defaults, now);

        if let Some(locked) = locked_inputs {
            config.set_locked_inputs(locked, now);
        }

        self.repository.save(&config).await?;
        Ok(config)
    }

    /// Import workflow configurations, optionally replacing existing ones
    ///
    /// Returns (imported_count, skipped_count).
    pub async fn import_configs(
        &self,
        configs: Vec<WorkflowConfiguration>,
        replace_existing: bool,
    ) -> Result<(usize, usize)> {
        let mut imported = 0;
        let mut skipped = 0;

        for config in configs {
            let existing = self.repository.get_by_slot(config.slot).await?;

            if existing.is_some() && !replace_existing {
                skipped += 1;
                continue;
            }

            self.repository.save(&config).await?;
            imported += 1;
        }

        Ok((imported, skipped))
    }
}

// Implementation of the port trait for hexagonal architecture compliance
#[async_trait]
impl WorkflowServicePort for WorkflowConfigService {
    async fn get_workflow(&self, id: WorkflowConfigId) -> Result<Option<WorkflowConfiguration>> {
        WorkflowConfigService::get_workflow(self, id).await
    }

    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        WorkflowConfigService::list_all(self).await
    }

    async fn list_by_slot(&self, slot: WorkflowSlot) -> Result<Vec<WorkflowConfiguration>> {
        WorkflowConfigService::list_by_slot(self, slot).await
    }

    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        WorkflowConfigService::get_by_slot(self, slot).await
    }

    async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        WorkflowConfigService::save(self, config).await
    }

    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        WorkflowConfigService::delete_by_slot(self, slot).await
    }

    async fn get_active_for_slot(
        &self,
        world_id: WorldId,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>> {
        WorkflowConfigService::get_active_for_slot(self, world_id, slot).await
    }

    async fn create_or_update(
        &self,
        slot: WorkflowSlot,
        name: String,
        workflow_json: serde_json::Value,
        prompt_mappings: Vec<PromptMapping>,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Vec<String>,
    ) -> Result<(WorkflowConfiguration, bool)> {
        WorkflowConfigService::create_or_update(
            self,
            slot,
            name,
            workflow_json,
            prompt_mappings,
            input_defaults,
            locked_inputs,
        )
        .await
    }

    async fn update_defaults(
        &self,
        slot: WorkflowSlot,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Option<Vec<String>>,
    ) -> Result<WorkflowConfiguration> {
        WorkflowConfigService::update_defaults(self, slot, input_defaults, locked_inputs).await
    }

    async fn import_configs(
        &self,
        configs: Vec<WorkflowConfiguration>,
        replace_existing: bool,
    ) -> Result<(usize, usize)> {
        WorkflowConfigService::import_configs(self, configs, replace_existing).await
    }
}
