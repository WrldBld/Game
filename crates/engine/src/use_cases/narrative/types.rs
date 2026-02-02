//! Narrative use case types - DTOs for event chain and narrative event operations

use chrono::{DateTime, Utc};
use serde::Serialize;

use wrldbldr_domain::{ActId, EventChainActivation, EventChainId, NarrativeEventId, WorldId};

/// Summary of an event chain (typed DTO with domain types).
///
/// This is a use case DTO that uses typed IDs and DateTime instead of
/// presentation strings. Protocol conversion happens at the API boundary.
#[derive(Debug, Clone)]
pub struct EventChainSummary {
    pub id: EventChainId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub events: Vec<NarrativeEventId>,
    pub is_active: bool,
    pub current_position: u32,
    pub completed_events: Vec<NarrativeEventId>,
    pub act_id: Option<ActId>,
    pub tags: Vec<String>,
    /// Display color (hex or named), UI-only
    pub color: Option<String>,
    pub is_favorite: bool,
    pub progress_percent: u32,
    pub is_complete: bool,
    pub remaining_events: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EventChainSummary {
    /// Convert to protocol representation (API layer DTO).
    pub fn to_protocol(&self) -> EventChainProtocol {
        EventChainProtocol {
            id: self.id.to_string(),
            world_id: self.world_id.to_string(),
            name: self.name.clone(),
            description: self.description.clone(),
            events: self.events.iter().map(|id| id.to_string()).collect(),
            is_active: self.is_active,
            current_position: self.current_position,
            completed_events: self
                .completed_events
                .iter()
                .map(|id| id.to_string())
                .collect(),
            act_id: self.act_id.map(|id| id.to_string()),
            tags: self.tags.clone(),
            color: self.color.clone(),
            is_favorite: self.is_favorite,
            progress_percent: self.progress_percent,
            is_complete: self.is_complete,
            remaining_events: self.remaining_events,
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// Protocol representation of an event chain (for API responses).
#[derive(Debug, Clone, Serialize)]
pub struct EventChainProtocol {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub current_position: u32,
    pub completed_events: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_id: Option<String>,
    pub tags: Vec<String>,
    /// Display color (hex or named), persisted and round-tripped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub is_favorite: bool,
    pub progress_percent: u32,
    pub is_complete: bool,
    pub remaining_events: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Status summary of an event chain (typed DTO with domain types).
///
/// This is a use case DTO that uses typed IDs and DateTime instead of
/// presentation strings. Protocol conversion happens at the API boundary.
#[derive(Debug, Clone)]
pub struct ChainStatusSummary {
    pub chain_id: EventChainId,
    pub chain_name: String,
    pub activation: EventChainActivation,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    pub current_event_id: Option<NarrativeEventId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ChainStatusSummary {
    /// Convert to protocol representation (API layer DTO).
    pub fn to_protocol(&self) -> ChainStatusProtocol {
        ChainStatusProtocol {
            chain_id: self.chain_id.to_string(),
            chain_name: self.chain_name.clone(),
            activation: event_chain_activation_to_string(self.activation),
            is_active: self.is_active,
            is_complete: self.is_complete,
            total_events: self.total_events,
            completed_events: self.completed_events,
            progress_percent: self.progress_percent,
            current_event_id: self.current_event_id.map(|id| id.to_string()),
        }
    }
}

/// Convert EventChainActivation to a stable string representation.
fn event_chain_activation_to_string(activation: EventChainActivation) -> String {
    match activation {
        EventChainActivation::Active => "active".to_string(),
        EventChainActivation::Inactive => "inactive".to_string(),
        EventChainActivation::Archived => "archived".to_string(),
    }
}

/// Protocol representation of chain status (for API responses).
#[derive(Debug, Clone, Serialize)]
pub struct ChainStatusProtocol {
    pub chain_id: String,
    pub chain_name: String,
    pub activation: String,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_event_id: Option<String>,
}

/// Input for creating an event chain (domain representation).
#[derive(Debug, Clone, Default)]
pub struct CreateEventChainInput {
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub color: Option<String>,
    pub is_active: Option<bool>,
}

/// Input for updating an event chain (domain representation).
#[derive(Debug, Clone, Default)]
pub struct UpdateEventChainInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub color: Option<String>,
    pub is_active: Option<bool>,
}

/// Summary of a narrative event for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct NarrativeEventSummary {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub scene_direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_opening: Option<String>,
    pub trigger_count: u32,
    pub is_active: bool,
    pub is_triggered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_outcome: Option<String>,
    pub is_repeatable: bool,
    pub delay_turns: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after_turns: Option<u32>,
    pub priority: i32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_position: Option<u32>,
    pub outcome_count: usize,
    pub trigger_condition_count: usize,
    pub created_at: String,
    pub updated_at: String,
}
