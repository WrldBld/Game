//! Workflow configuration CRUD service
//!
//! This service handles persistence operations for workflow configurations.
//! It provides a clean interface between the HTTP layer and the repository.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{WorkflowConfiguration, WorkflowSlot};
use wrldbldr_domain::{WorkflowConfigId, WorldId};
use wrldbldr_engine_ports::outbound::{WorkflowRepositoryPort, WorkflowServicePort};

/// Service for managing workflow configuration persistence
pub struct WorkflowConfigService {
    repository: Arc<dyn WorkflowRepositoryPort>,
}

impl WorkflowConfigService {
    /// Create a new workflow configuration service
    pub fn new(repository: Arc<dyn WorkflowRepositoryPort>) -> Self {
        Self { repository }
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
}
