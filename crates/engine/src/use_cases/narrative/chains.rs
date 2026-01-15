use std::sync::Arc;

use serde::Serialize;

use wrldbldr_domain::{self as domain, ActId, EventChain, EventChainId, NarrativeEventId, WorldId};

use crate::infrastructure::ports::RepoError;
use crate::use_cases::narrative_operations::Narrative;

// =============================================================================
// Domain Result Types
// =============================================================================

/// Summary of an event chain.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventChainSummary {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub is_favorite: bool,
    pub progress_percent: u32,
    pub is_complete: bool,
    pub remaining_events: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Status of an event chain.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatusSummary {
    pub chain_id: String,
    pub chain_name: String,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_event_id: Option<String>,
}

// =============================================================================
// Domain Input Types
// =============================================================================

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

pub struct EventChainOps {
    narrative: Arc<Narrative>,
}

impl EventChainOps {
    pub fn new(narrative: Arc<Narrative>) -> Self {
        Self { narrative }
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<EventChainSummary>, EventChainError> {
        let chains = self.narrative.list_chains_for_world(world_id).await?;
        Ok(chains.iter().map(event_chain_to_summary).collect())
    }

    pub async fn get(
        &self,
        chain_id: EventChainId,
    ) -> Result<Option<EventChainSummary>, EventChainError> {
        let chain = self.narrative.get_chain(chain_id).await?;
        Ok(chain.as_ref().map(event_chain_to_summary))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        input: CreateEventChainInput,
        act_id: Option<ActId>,
        events: Option<Vec<NarrativeEventId>>,
    ) -> Result<EventChainSummary, EventChainError> {
        let now = self.narrative.now();
        let mut chain = EventChain::new(world_id, &input.name, now);

        if let Some(description) = input.description {
            chain.description = description;
        }
        if let Some(tags) = input.tags {
            chain.tags = tags;
        }
        if let Some(color) = input.color {
            chain.color = Some(color);
        }
        if let Some(active) = input.is_active {
            chain.is_active = active;
        }
        if let Some(act_id) = act_id {
            chain.act_id = Some(act_id);
        }
        if let Some(events) = events {
            for event_id in events {
                chain.add_event(event_id, now);
            }
        }

        self.narrative.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn update(
        &self,
        chain_id: EventChainId,
        input: UpdateEventChainInput,
        act_id: Option<ActId>,
        events: Option<Vec<NarrativeEventId>>,
    ) -> Result<EventChainSummary, EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        if let Some(name) = input.name {
            chain.name = name;
        }
        if let Some(description) = input.description {
            chain.description = description;
        }
        if let Some(tags) = input.tags {
            chain.tags = tags;
        }
        if let Some(color) = input.color {
            chain.color = Some(color);
        }
        if let Some(active) = input.is_active {
            chain.is_active = active;
        }
        if let Some(act_id) = act_id {
            chain.act_id = Some(act_id);
        }
        if let Some(events) = events {
            chain.reorder_events(events, self.narrative.now());
        }

        self.narrative.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn delete(&self, chain_id: EventChainId) -> Result<(), EventChainError> {
        self.narrative.delete_chain(chain_id).await?;
        Ok(())
    }

    pub async fn set_active(
        &self,
        chain_id: EventChainId,
        active: bool,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        let now = self.narrative.now();
        if active {
            chain.activate(now);
        } else {
            chain.deactivate(now);
        }

        self.narrative.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn set_favorite(
        &self,
        chain_id: EventChainId,
        favorite: bool,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        chain.is_favorite = favorite;
        self.narrative.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn add_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
        position: Option<usize>,
    ) -> Result<EventChainSummary, EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        let now = self.narrative.now();
        if let Some(pos) = position {
            chain.insert_event(pos, event_id, now);
        } else {
            chain.add_event(event_id, now);
        }
        self.narrative.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn remove_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        let now = self.narrative.now();
        chain.remove_event(&event_id, now);
        self.narrative.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        let now = self.narrative.now();
        chain.complete_event(event_id, now);
        self.narrative.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn reset(
        &self,
        chain_id: EventChainId,
    ) -> Result<EventChainSummary, EventChainError> {
        let mut chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        chain.reset(self.narrative.now());
        self.narrative.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn status(
        &self,
        chain_id: EventChainId,
    ) -> Result<ChainStatusSummary, EventChainError> {
        let chain = self
            .narrative
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        let status: domain::ChainStatus = (&chain).into();
        Ok(chain_status_to_summary(&status))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventChainError {
    #[error("Event chain not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn event_chain_to_summary(chain: &EventChain) -> EventChainSummary {
    EventChainSummary {
        id: chain.id.to_string(),
        world_id: chain.world_id.to_string(),
        name: chain.name.clone(),
        description: chain.description.clone(),
        events: chain.events.iter().map(|id| id.to_string()).collect(),
        is_active: chain.is_active,
        current_position: chain.current_position,
        completed_events: chain
            .completed_events
            .iter()
            .map(|id| id.to_string())
            .collect(),
        act_id: chain.act_id.map(|id| id.to_string()),
        tags: chain.tags.clone(),
        color: chain.color.clone(),
        is_favorite: chain.is_favorite,
        progress_percent: (chain.progress() * 100.0) as u32,
        is_complete: chain.is_complete(),
        remaining_events: chain.remaining_events(),
        created_at: chain.created_at.to_rfc3339(),
        updated_at: chain.updated_at.to_rfc3339(),
    }
}

fn chain_status_to_summary(status: &domain::ChainStatus) -> ChainStatusSummary {
    ChainStatusSummary {
        chain_id: status.chain_id.to_string(),
        chain_name: status.chain_name.clone(),
        is_active: status.is_active,
        is_complete: status.is_complete,
        total_events: status.total_events,
        completed_events: status.completed_events,
        progress_percent: status.progress_percent,
        current_event_id: status.current_event_id.map(|id| id.to_string()),
    }
}
