//! Event chain service port - Interface for event chain operations
//!
//! This port abstracts event chain (story arc) business logic from infrastructure.
//! It provides methods for creating, updating, and managing event chains within a world.
//!
//! # Design Notes
//!
//! Event chains represent story arcs that contain multiple narrative events.
//! This port exposes operations for managing chain lifecycle, event membership,
//! and chain status tracking.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{ChainStatus, EventChain};
use wrldbldr_domain::{EventChainId, NarrativeEventId, WorldId};

/// Port for event chain service operations.
///
/// This trait provides access to event chain management functionality
/// including CRUD operations, event membership management, and status tracking.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait EventChainServicePort: Send + Sync {
    /// Get an event chain by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the event chain to retrieve
    ///
    /// # Returns
    ///
    /// `Ok(Some(chain))` if found, `Ok(None)` if not found.
    async fn get_event_chain(&self, id: EventChainId) -> Result<Option<EventChain>>;

    /// List all event chains for a world.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world whose chains to list
    ///
    /// # Returns
    ///
    /// A vector of all event chains in the world.
    async fn list_event_chains(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List active event chains for a world.
    ///
    /// Returns only chains that are currently marked as active.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world whose active chains to list
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List favorite event chains for a world.
    ///
    /// Returns chains that have been marked as favorites by the DM.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world whose favorite chains to list
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// Get chains containing a specific narrative event.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The ID of the narrative event
    ///
    /// # Returns
    ///
    /// A vector of chains that contain this event.
    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>>;

    /// Create a new event chain.
    ///
    /// # Arguments
    ///
    /// * `chain` - The event chain to create
    ///
    /// # Returns
    ///
    /// The created event chain (may have updated fields like timestamps).
    async fn create_event_chain(&self, chain: EventChain) -> Result<EventChain>;

    /// Update an existing event chain.
    ///
    /// # Arguments
    ///
    /// * `chain` - The event chain with updated fields
    ///
    /// # Returns
    ///
    /// The updated event chain.
    async fn update_event_chain(&self, chain: EventChain) -> Result<EventChain>;

    /// Delete an event chain.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the chain to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the chain doesn't exist or cannot be deleted.
    async fn delete_event_chain(&self, id: EventChainId) -> Result<()>;

    /// Add an event to a chain.
    ///
    /// Creates a membership link between the chain and the narrative event.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The ID of the chain
    /// * `event_id` - The ID of the narrative event to add
    async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()>;

    /// Remove an event from a chain.
    ///
    /// Removes the membership link between the chain and the narrative event.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The ID of the chain
    /// * `event_id` - The ID of the narrative event to remove
    async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()>;

    /// Mark an event as completed in a chain.
    ///
    /// Updates the chain's progress to indicate this event has been completed.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The ID of the chain
    /// * `event_id` - The ID of the event to mark as completed
    async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<()>;

    /// Toggle favorite status for an event chain.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the chain
    ///
    /// # Returns
    ///
    /// The new favorite status (`true` if now favorited, `false` if unfavorited).
    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool>;

    /// Set active status for an event chain.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the chain
    /// * `is_active` - Whether the chain should be active
    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<()>;

    /// Reset chain progress.
    ///
    /// Clears all completed events and resets the chain to its initial state.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the chain to reset
    async fn reset_chain(&self, id: EventChainId) -> Result<()>;

    /// Get chain status summary.
    ///
    /// Returns a summary of the chain's progress including completed events.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the chain
    ///
    /// # Returns
    ///
    /// `Ok(Some(status))` if found, `Ok(None)` if the chain doesn't exist.
    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>>;

    /// Get all chain statuses for a world.
    ///
    /// Returns status summaries for all chains in the world.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world
    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of EventChainServicePort for testing.
    pub EventChainServicePort {}

    #[async_trait]
    impl EventChainServicePort for EventChainServicePort {
        async fn get_event_chain(&self, id: EventChainId) -> Result<Option<EventChain>>;
        async fn list_event_chains(&self, world_id: WorldId) -> Result<Vec<EventChain>>;
        async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>>;
        async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>>;
        async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>>;
        async fn create_event_chain(&self, chain: EventChain) -> Result<EventChain>;
        async fn update_event_chain(&self, chain: EventChain) -> Result<EventChain>;
        async fn delete_event_chain(&self, id: EventChainId) -> Result<()>;
        async fn add_event_to_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<()>;
        async fn remove_event_from_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<()>;
        async fn complete_event(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> Result<()>;
        async fn toggle_favorite(&self, id: EventChainId) -> Result<bool>;
        async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<()>;
        async fn reset_chain(&self, id: EventChainId) -> Result<()>;
        async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>>;
        async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>>;
    }
}
