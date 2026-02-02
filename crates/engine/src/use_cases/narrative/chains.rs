use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, ActId, Description, DomainError, EventChain, EventChainId,
    EventChainName, NarrativeEventId, WorldId,
};

use crate::infrastructure::ports::{ClockPort, NarrativeRepo, RepoError};
use crate::use_cases::narrative::types::{
    ChainStatusSummary, CreateEventChainInput, EventChainSummary, UpdateEventChainInput,
};

pub struct EventChainOps {
    narrative_repo: Arc<dyn NarrativeRepo>,
    clock: Arc<dyn ClockPort>,
}

impl EventChainOps {
    pub fn new(
        narrative_repo: Arc<dyn NarrativeRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            narrative_repo,
            clock,
        }
    }

    pub async fn list(&self,
        world_id: WorldId,
    ) -> Result<Vec<EventChainSummary>, EventChainError> {
        let chains = self.narrative_repo.list_chains_for_world(world_id).await?;
        Ok(chains.iter().map(event_chain_to_summary).collect())
    }

    pub async fn get(
        &self,
        chain_id: EventChainId,
    ) -> Result<Option<EventChainSummary>, EventChainError> {
        let chain = self.narrative_repo.get_chain(chain_id).await?;
        Ok(chain.as_ref().map(event_chain_to_summary))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        input: CreateEventChainInput,
        act_id: Option<ActId>,
        events: Option<Vec<NarrativeEventId>>,
    ) -> Result<EventChainSummary, EventChainError> {
        let now = self.clock.now();
        let mut chain = EventChain::new(world_id, EventChainName::new(&input.name)?, now)?;

        if let Some(description) = input.description {
            chain.set_description(Description::new(description)?, now)?;
        }
        if let Some(tags) = input.tags {
            let domain_tags: Vec<wrldbldr_domain::Tag> = tags
                .into_iter()
                .filter_map(|s| wrldbldr_domain::Tag::new(&s).ok())
                .collect();
            chain.set_tags(domain_tags, now);
        }
        if let Some(color) = input.color {
            chain.set_color(Some(color), now);
        }
        if let Some(active) = input.is_active {
            if active {
                chain.activate(now);
            } else {
                chain.deactivate(now);
            }
        }
        if let Some(act_id) = act_id {
            chain.set_act_id(Some(act_id), now);
        }
        if let Some(events) = events {
            for event_id in events {
                chain.add_event(event_id, now);
            }
        }

        self.narrative_repo.save_chain(&chain).await?;
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
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        let now = self.clock.now();
        if let Some(name) = input.name {
            chain.set_name(EventChainName::new(name)?, now)?;
        }
        if let Some(description) = input.description {
            chain.set_description(Description::new(description)?, now)?;
        }
        if let Some(tags) = input.tags {
            let domain_tags: Vec<wrldbldr_domain::Tag> = tags
                .into_iter()
                .filter_map(|s| wrldbldr_domain::Tag::new(&s).ok())
                .collect();
            chain.set_tags(domain_tags, now);
        }
        if let Some(color) = input.color {
            chain.set_color(Some(color), now);
        }
        if let Some(active) = input.is_active {
            if active {
                chain.activate(now);
            } else {
                chain.deactivate(now);
            }
        }
        if let Some(act_id) = act_id {
            chain.set_act_id(Some(act_id), now);
        }
        if let Some(events) = events {
            chain.reorder_events(events, now);
        }

        self.narrative_repo.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn delete(&self, chain_id: EventChainId) -> Result<(), EventChainError> {
        self.narrative_repo.delete_chain(chain_id).await?;
        Ok(())
    }

    pub async fn set_active(
        &self,
        chain_id: EventChainId,
        active: bool,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        let now = self.clock.now();
        if active {
            chain.activate(now);
        } else {
            chain.deactivate(now);
        }

        self.narrative_repo.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn set_favorite(
        &self,
        chain_id: EventChainId,
        favorite: bool,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        let favorite_status = if favorite {
            domain::EventChainFavorite::Favorite
        } else {
            domain::EventChainFavorite::Normal
        };

        chain.set_favorite(favorite_status, self.clock.now());
        self.narrative_repo.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn add_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
        position: Option<usize>,
    ) -> Result<EventChainSummary, EventChainError> {
        let mut chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        // Validate event existence and world ownership
        let event = self
            .narrative_repo
            .get_event(event_id)
            .await?
            .ok_or_else(|| EventChainError::EventNotFound(event_id))?;

        if event.world_id() != chain.world_id() {
            return Err(EventChainError::WorldMismatch);
        }

        let now = self.clock.now();
        if let Some(pos) = position {
            chain.insert_event(pos, event_id, now);
        } else {
            chain.add_event(event_id, now);
        }
        self.narrative_repo.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn remove_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        // Validate event existence and world ownership
        let event = self
            .narrative_repo
            .get_event(event_id)
            .await?
            .ok_or_else(|| EventChainError::EventNotFound(event_id))?;

        if event.world_id() != chain.world_id() {
            return Err(EventChainError::WorldMismatch);
        }

        let now = self.clock.now();
        chain.remove_event(&event_id, now);
        self.narrative_repo.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<(), EventChainError> {
        let mut chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        // Validate event existence
        let event = self
            .narrative_repo
            .get_event(event_id)
            .await?
            .ok_or_else(|| EventChainError::EventNotFound(event_id))?;

        // Validate event belongs to the same world as the chain
        if event.world_id() != chain.world_id() {
            return Err(EventChainError::WorldMismatch);
        }

        // Validate event is actually in this chain
        if !chain.contains_event(&event_id) {
            return Err(EventChainError::EventNotInChain);
        }

        let now = self.clock.now();
        chain.complete_event(event_id, now);
        self.narrative_repo.save_chain(&chain).await?;
        Ok(())
    }

    pub async fn reset(
        &self,
        chain_id: EventChainId,
    ) -> Result<EventChainSummary, EventChainError> {
        let mut chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;
        chain.reset(self.clock.now());
        self.narrative_repo.save_chain(&chain).await?;
        Ok(event_chain_to_summary(&chain))
    }

    pub async fn status(
        &self,
        chain_id: EventChainId,
    ) -> Result<ChainStatusSummary, EventChainError> {
        let chain = self
            .narrative_repo
            .get_chain(chain_id)
            .await?
            .ok_or(EventChainError::NotFound)?;

        Ok(ChainStatusSummary {
            chain_id: chain.id(),
            chain_name: chain.name().to_string(),
            activation: chain.activation(),
            is_active: chain.is_active(),
            is_complete: chain.is_complete(),
            total_events: chain.events().len(),
            completed_events: chain.completed_events().len(),
            progress_percent: (chain.progress() * 100.0) as u32,
            current_event_id: chain.current_event().copied(),
            created_at: chain.created_at(),
            updated_at: chain.updated_at(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventChainError {
    #[error("Event chain not found")]
    NotFound,
    #[error("Event not found: {0}")]
    EventNotFound(NarrativeEventId),
    #[error("Event does not belong to this chain")]
    EventNotInChain,
    #[error("Event does not belong to the requested world")]
    WorldMismatch,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

fn event_chain_to_summary(chain: &EventChain) -> EventChainSummary {
    EventChainSummary {
        id: chain.id(),
        world_id: chain.world_id(),
        name: chain.name().to_string(),
        description: chain.description().to_string(),
        events: chain.events().to_vec(),
        is_active: chain.is_active(),
        current_position: chain.current_position(),
        completed_events: chain.completed_events().to_vec(),
        act_id: chain.act_id(),
        tags: chain.tags().iter().map(|t| t.to_string()).collect(),
        color: chain.color().map(|s| s.to_string()),
        is_favorite: chain.is_favorite(),
        progress_percent: (chain.progress() * 100.0) as u32,
        is_complete: chain.is_complete(),
        remaining_events: chain.remaining_events(),
        created_at: chain.created_at(),
        updated_at: chain.updated_at(),
    }
}
