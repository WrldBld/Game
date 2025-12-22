//! Event Chain Service - Application service for event chain management
//!
//! This service provides use case implementations for fetching, creating,
//! updating, and managing event chains. It abstracts away the HTTP client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};

use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Event chain data from API
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventChainData {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub current_position: u32,
    pub completed_events: Vec<String>,
    pub act_id: Option<String>,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub is_favorite: bool,
    pub progress_percent: u32,
    pub is_complete: bool,
    pub remaining_events: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to create an event chain
#[derive(Clone, Debug, Serialize)]
pub struct CreateEventChainRequest {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_active: bool,
}

/// Request to update an event chain
#[derive(Clone, Debug, Serialize)]
pub struct UpdateEventChainRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

/// Request to add an event to a chain
#[derive(Clone, Debug, Serialize)]
pub struct AddEventRequest {
    pub event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

/// Chain status data
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ChainStatusData {
    pub chain_id: String,
    pub chain_name: String,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    pub current_event_id: Option<String>,
}

/// Event chain service for managing event chains
///
/// This service provides methods for event chain-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct EventChainService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> EventChainService<A> {
    /// Create a new EventChainService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all event chains for a world
    pub async fn list_chains(&self, world_id: &str) -> Result<Vec<EventChainData>, ApiError> {
        let path = format!("/api/worlds/{}/event-chains", world_id);
        let chains: Vec<EventChainData> = self.api.get(&path).await?;
        Ok(chains)
    }

    /// List active event chains
    pub async fn list_active(&self, world_id: &str) -> Result<Vec<EventChainData>, ApiError> {
        let path = format!("/api/worlds/{}/event-chains/active", world_id);
        let chains: Vec<EventChainData> = self.api.get(&path).await?;
        Ok(chains)
    }

    /// List favorite event chains
    pub async fn list_favorites(&self, world_id: &str) -> Result<Vec<EventChainData>, ApiError> {
        let path = format!("/api/worlds/{}/event-chains/favorites", world_id);
        let chains: Vec<EventChainData> = self.api.get(&path).await?;
        Ok(chains)
    }

    /// Get a single event chain by ID
    pub async fn get_chain(&self, chain_id: &str) -> Result<EventChainData, ApiError> {
        let path = format!("/api/event-chains/{}", chain_id);
        let chain: EventChainData = self.api.get(&path).await?;
        Ok(chain)
    }

    /// Create a new event chain
    pub async fn create_chain(
        &self,
        world_id: &str,
        request: &CreateEventChainRequest,
    ) -> Result<EventChainData, ApiError> {
        let path = format!("/api/worlds/{}/event-chains", world_id);
        let chain: EventChainData = self.api.post(&path, request).await?;
        Ok(chain)
    }

    /// Update an event chain
    pub async fn update_chain(
        &self,
        chain_id: &str,
        request: &UpdateEventChainRequest,
    ) -> Result<EventChainData, ApiError> {
        let path = format!("/api/event-chains/{}", chain_id);
        let chain: EventChainData = self.api.put(&path, request).await?;
        Ok(chain)
    }

    /// Delete an event chain
    pub async fn delete_chain(&self, chain_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/event-chains/{}", chain_id);
        self.api.delete(&path).await
    }

    /// Add an event to a chain
    pub async fn add_event(
        &self,
        chain_id: &str,
        request: &AddEventRequest,
    ) -> Result<EventChainData, ApiError> {
        let path = format!("/api/event-chains/{}/events", chain_id);
        let chain: EventChainData = self.api.post(&path, request).await?;
        Ok(chain)
    }

    /// Remove an event from a chain
    pub async fn remove_event(
        &self,
        chain_id: &str,
        event_id: &str,
    ) -> Result<(), ApiError> {
        let path = format!("/api/event-chains/{}/events/{}", chain_id, event_id);
        self.api.delete(&path).await
    }

    /// Complete an event in a chain
    pub async fn complete_event(
        &self,
        chain_id: &str,
        event_id: &str,
    ) -> Result<(), ApiError> {
        let path = format!("/api/event-chains/{}/events/{}/complete", chain_id, event_id);
        self.api.post_empty(&path).await
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(&self, chain_id: &str) -> Result<bool, ApiError> {
        let path = format!("/api/event-chains/{}/favorite", chain_id);
        // PUT with empty body returns bool
        let is_favorite: bool = self.api.put_empty_with_response(&path).await?;
        Ok(is_favorite)
    }

    /// Set active status
    pub async fn set_active(&self, chain_id: &str, is_active: bool) -> Result<(), ApiError> {
        let path = format!("/api/event-chains/{}/active", chain_id);
        #[derive(Serialize)]
        struct ActiveRequest {
            is_active: bool,
        }
        self.api.put_no_response(&path, &ActiveRequest { is_active }).await
    }

    /// Reset a chain to the beginning
    pub async fn reset_chain(&self, chain_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/event-chains/{}/reset", chain_id);
        self.api.post_empty(&path).await
    }

    /// Get chain status
    pub async fn get_status(&self, chain_id: &str) -> Result<ChainStatusData, ApiError> {
        let path = format!("/api/event-chains/{}/status", chain_id);
        let status: ChainStatusData = self.api.get(&path).await?;
        Ok(status)
    }
}

impl<A: ApiPort + Clone> Clone for EventChainService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}

