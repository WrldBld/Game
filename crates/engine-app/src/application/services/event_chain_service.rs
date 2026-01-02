//! EventChain Service - Application service for event chain management
//!
//! This service provides use case implementations for creating, updating,
//! and managing event chains (story arcs) within a world.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::entities::{ChainStatus, EventChain};
use wrldbldr_domain::{EventChainId, NarrativeEventId, WorldId};
use crate::application::services::internal::EventChainServicePort;
use wrldbldr_engine_ports::outbound::{
    EventChainCrudPort, EventChainMembershipPort, EventChainQueryPort, EventChainStatePort,
};

/// EventChain service trait defining the application use cases
#[async_trait]
pub trait EventChainService: Send + Sync {
    /// Get an event chain by ID
    async fn get_event_chain(&self, id: EventChainId) -> Result<Option<EventChain>>;

    /// List all event chains for a world
    async fn list_event_chains(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List active event chains for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List favorite event chains for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// Get chains containing a specific narrative event
    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>>;

    /// Create a new event chain
    async fn create_event_chain(&self, chain: EventChain) -> Result<EventChain>;

    /// Update an existing event chain
    async fn update_event_chain(&self, chain: EventChain) -> Result<EventChain>;

    /// Delete an event chain
    async fn delete_event_chain(&self, id: EventChainId) -> Result<()>;

    /// Add an event to a chain
    async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()>;

    /// Remove an event from a chain
    async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()>;

    /// Mark an event as completed in a chain
    async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()>;

    /// Toggle favorite status for an event chain
    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool>;

    /// Set active status for an event chain
    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<()>;

    /// Reset chain progress
    async fn reset_chain(&self, id: EventChainId) -> Result<()>;

    /// Get chain status summary
    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>>;

    /// Get all chain statuses for a world
    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>>;
}

/// Default implementation of EventChainService using ISP port abstractions.
///
/// This service depends on all four ISP traits because it provides a complete
/// CRUD + query + membership + state management interface for event chains.
/// In practice, these traits are implemented by the same repository instance,
/// so the composition root passes the same Arc to all four parameters.
#[derive(Clone)]
pub struct EventChainServiceImpl {
    crud: Arc<dyn EventChainCrudPort>,
    query: Arc<dyn EventChainQueryPort>,
    membership: Arc<dyn EventChainMembershipPort>,
    state: Arc<dyn EventChainStatePort>,
}

impl EventChainServiceImpl {
    /// Create a new EventChainServiceImpl with ISP-compliant repository ports.
    ///
    /// All four ports can be the same underlying repository instance, coerced
    /// to the appropriate trait interface by the composition root.
    pub fn new(
        crud: Arc<dyn EventChainCrudPort>,
        query: Arc<dyn EventChainQueryPort>,
        membership: Arc<dyn EventChainMembershipPort>,
        state: Arc<dyn EventChainStatePort>,
    ) -> Self {
        Self {
            crud,
            query,
            membership,
            state,
        }
    }
}

#[async_trait]
impl EventChainService for EventChainServiceImpl {
    #[instrument(skip(self))]
    async fn get_event_chain(&self, id: EventChainId) -> Result<Option<EventChain>> {
        debug!(chain_id = %id, "Fetching event chain");
        self.crud
            .get(id)
            .await
            .context("Failed to get event chain from repository")
    }

    #[instrument(skip(self))]
    async fn list_event_chains(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        debug!(world_id = %world_id, "Listing all event chains for world");
        self.query
            .list_by_world(world_id)
            .await
            .context("Failed to list event chains from repository")
    }

    #[instrument(skip(self))]
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        debug!(world_id = %world_id, "Listing active event chains for world");
        self.query
            .list_active(world_id)
            .await
            .context("Failed to list active event chains from repository")
    }

    #[instrument(skip(self))]
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        debug!(world_id = %world_id, "Listing favorite event chains for world");
        self.query
            .list_favorites(world_id)
            .await
            .context("Failed to list favorite event chains from repository")
    }

    #[instrument(skip(self))]
    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>> {
        debug!(event_id = %event_id, "Getting chains containing event");
        self.query
            .get_chains_for_event(event_id)
            .await
            .context("Failed to get chains for event from repository")
    }

    #[instrument(skip(self, chain))]
    async fn create_event_chain(&self, chain: EventChain) -> Result<EventChain> {
        info!(chain_id = %chain.id, world_id = %chain.world_id, "Creating event chain");
        self.crud
            .create(&chain)
            .await
            .context("Failed to create event chain in repository")?;
        Ok(chain)
    }

    #[instrument(skip(self, chain))]
    async fn update_event_chain(&self, chain: EventChain) -> Result<EventChain> {
        info!(chain_id = %chain.id, "Updating event chain");
        self.crud
            .update(&chain)
            .await
            .context("Failed to update event chain in repository")?;
        Ok(chain)
    }

    #[instrument(skip(self))]
    async fn delete_event_chain(&self, id: EventChainId) -> Result<()> {
        info!(chain_id = %id, "Deleting event chain");
        self.crud
            .delete(id)
            .await
            .context("Failed to delete event chain from repository")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()> {
        info!(chain_id = %chain_id, event_id = %event_id, "Adding event to chain");
        self.membership
            .add_event_to_chain(chain_id, event_id)
            .await
            .context("Failed to add event to chain in repository")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()> {
        info!(chain_id = %chain_id, event_id = %event_id, "Removing event from chain");
        self.membership
            .remove_event_from_chain(chain_id, event_id)
            .await
            .context("Failed to remove event from chain in repository")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()> {
        info!(chain_id = %chain_id, event_id = %event_id, "Completing event in chain");
        self.membership
            .complete_event(chain_id, event_id)
            .await
            .context("Failed to complete event in chain in repository")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool> {
        info!(chain_id = %id, "Toggling favorite status for event chain");
        self.state
            .toggle_favorite(id)
            .await
            .context("Failed to toggle favorite status in repository")
    }

    #[instrument(skip(self))]
    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<()> {
        info!(chain_id = %id, is_active = is_active, "Setting active status for event chain");
        self.state
            .set_active(id, is_active)
            .await
            .context("Failed to set active status in repository")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn reset_chain(&self, id: EventChainId) -> Result<()> {
        info!(chain_id = %id, "Resetting chain progress");
        self.state
            .reset(id)
            .await
            .context("Failed to reset chain progress in repository")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>> {
        debug!(chain_id = %id, "Getting chain status");
        self.state
            .get_status(id)
            .await
            .context("Failed to get chain status from repository")
    }

    #[instrument(skip(self))]
    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>> {
        debug!(world_id = %world_id, "Listing chain statuses for world");
        self.state
            .list_statuses(world_id)
            .await
            .context("Failed to list chain statuses from repository")
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

/// Implementation of the `EventChainServicePort` for `EventChainServiceImpl`.
///
/// This exposes the event chain service methods to infrastructure adapters.
#[async_trait]
impl EventChainServicePort for EventChainServiceImpl {
    async fn get_event_chain(&self, id: EventChainId) -> Result<Option<EventChain>> {
        EventChainService::get_event_chain(self, id).await
    }

    async fn list_event_chains(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        EventChainService::list_event_chains(self, world_id).await
    }

    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        EventChainService::list_active(self, world_id).await
    }

    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>> {
        EventChainService::list_favorites(self, world_id).await
    }

    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>> {
        EventChainService::get_chains_for_event(self, event_id).await
    }

    async fn create_event_chain(&self, chain: EventChain) -> Result<EventChain> {
        EventChainService::create_event_chain(self, chain).await
    }

    async fn update_event_chain(&self, chain: EventChain) -> Result<EventChain> {
        EventChainService::update_event_chain(self, chain).await
    }

    async fn delete_event_chain(&self, id: EventChainId) -> Result<()> {
        EventChainService::delete_event_chain(self, id).await
    }

    async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()> {
        EventChainService::add_event_to_chain(self, chain_id, event_id).await
    }

    async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()> {
        EventChainService::remove_event_from_chain(self, chain_id, event_id).await
    }

    async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()> {
        EventChainService::complete_event(self, chain_id, event_id).await
    }

    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool> {
        EventChainService::toggle_favorite(self, id).await
    }

    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<()> {
        EventChainService::set_active(self, id, is_active).await
    }

    async fn reset_chain(&self, id: EventChainId) -> Result<()> {
        EventChainService::reset_chain(self, id).await
    }

    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>> {
        EventChainService::get_status(self, id).await
    }

    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>> {
        EventChainService::list_statuses(self, world_id).await
    }
}
