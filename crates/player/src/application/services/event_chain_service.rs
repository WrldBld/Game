//! Event Chain Service - Application service for event chain management
//!
//! This service provides use case implementations for fetching, creating,
//! updating, and managing event chains via WebSocket request/response pattern.

use serde::{Deserialize, Serialize};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_protocol::{EventChainRequest, RequestPayload};

/// Event chain data from engine
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

// From impls for protocol conversion at the boundary
impl From<&CreateEventChainRequest> for wrldbldr_protocol::requests::CreateEventChainData {
    fn from(req: &CreateEventChainRequest) -> Self {
        Self {
            name: req.name.clone(),
            description: if req.description.is_empty() {
                None
            } else {
                Some(req.description.clone())
            },
            events: if req.events.is_empty() {
                None
            } else {
                Some(req.events.clone())
            },
            act_id: req.act_id.clone(),
            tags: if req.tags.is_empty() {
                None
            } else {
                Some(req.tags.clone())
            },
            color: req.color.clone(),
            is_active: if req.is_active { None } else { Some(false) },
        }
    }
}

impl From<&UpdateEventChainRequest> for wrldbldr_protocol::requests::UpdateEventChainData {
    fn from(req: &UpdateEventChainRequest) -> Self {
        Self {
            name: req.name.clone(),
            description: req.description.clone(),
            events: req.events.clone(),
            act_id: req.act_id.clone(),
            tags: req.tags.clone(),
            color: req.color.clone(),
            is_active: req.is_active,
        }
    }
}

/// Event chain service for managing event chains
///
/// This service provides methods for event chain-related operations
/// using WebSocket request/response pattern via the `CommandBus`.
#[derive(Clone)]
pub struct EventChainService {
    commands: CommandBus,
}

impl EventChainService {
    /// Create a new EventChainService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// List all event chains for a world
    pub async fn list_chains(&self, world_id: &str) -> Result<Vec<EventChainData>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::ListEventChains {
                    world_id: world_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Get a single event chain by ID
    pub async fn get_chain(&self, chain_id: &str) -> Result<EventChainData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::GetEventChain {
                    chain_id: chain_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Create a new event chain
    pub async fn create_chain(
        &self,
        world_id: &str,
        request: &CreateEventChainRequest,
    ) -> Result<EventChainData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::CreateEventChain {
                    world_id: world_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update an event chain
    pub async fn update_chain(
        &self,
        chain_id: &str,
        request: &UpdateEventChainRequest,
    ) -> Result<EventChainData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::UpdateEventChain {
                    chain_id: chain_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Delete an event chain
    pub async fn delete_chain(&self, chain_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::DeleteEventChain {
                    chain_id: chain_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Add an event to a chain
    pub async fn add_event(
        &self,
        chain_id: &str,
        request: &AddEventRequest,
    ) -> Result<EventChainData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::AddEventToChain {
                    chain_id: chain_id.to_string(),
                    event_id: request.event_id.clone(),
                    position: request.position.map(|p| p as u32),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Remove an event from a chain
    pub async fn remove_event(&self, chain_id: &str, event_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::RemoveEventFromChain {
                    chain_id: chain_id.to_string(),
                    event_id: event_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Complete an event in a chain
    pub async fn complete_event(&self, chain_id: &str, event_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::CompleteChainEvent {
                    chain_id: chain_id.to_string(),
                    event_id: event_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Toggle favorite status
    pub async fn toggle_favorite(
        &self,
        chain_id: &str,
        favorite: bool,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::SetEventChainFavorite {
                    chain_id: chain_id.to_string(),
                    favorite,
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Set active status
    pub async fn set_active(&self, chain_id: &str, active: bool) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::SetEventChainActive {
                    chain_id: chain_id.to_string(),
                    active,
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Reset a chain to the beginning
    pub async fn reset_chain(&self, chain_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::ResetEventChain {
                    chain_id: chain_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Get chain status
    pub async fn get_status(&self, chain_id: &str) -> Result<ChainStatusData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::EventChain(EventChainRequest::GetEventChainStatus {
                    chain_id: chain_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }
}
