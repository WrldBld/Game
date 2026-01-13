use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;

use wrldbldr_domain::{self as domain, NarrativeEvent, NarrativeEventId, NarrativeTrigger, WorldId};

use crate::use_cases::narrative_operations::Narrative;
use crate::infrastructure::ports::RepoError;
use crate::use_cases::narrative::{EffectExecutionContext, EffectExecutionSummary, ExecuteEffects};

// =============================================================================
// Domain Result Types
// =============================================================================

/// Summary of a narrative event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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

pub struct NarrativeEventOps {
    narrative: Arc<Narrative>,
    execute_effects: Arc<ExecuteEffects>,
}

impl NarrativeEventOps {
    pub fn new(narrative: Arc<Narrative>, execute_effects: Arc<ExecuteEffects>) -> Self {
        Self {
            narrative,
            execute_effects,
        }
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<NarrativeEventSummary>, NarrativeEventError> {
        let events = self.narrative.list_events(world_id).await?;
        Ok(events.iter().map(narrative_event_to_summary).collect())
    }

    pub async fn get(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Option<NarrativeEventSummary>, NarrativeEventError> {
        let event = self.narrative.get_event(event_id).await?;
        Ok(event.as_ref().map(narrative_event_to_summary))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        trigger_conditions: Option<Vec<NarrativeTrigger>>,
        outcomes: Option<Vec<domain::EventOutcome>>,
    ) -> Result<NarrativeEventSummary, NarrativeEventError> {
        let now = Utc::now();
        let mut event = NarrativeEvent::new(world_id, &name, now);

        if let Some(description) = description {
            event.description = description;
        }
        if let Some(triggers) = trigger_conditions {
            event.trigger_conditions = triggers;
        }
        if let Some(outcomes) = outcomes {
            event.outcomes = outcomes;
        }

        self.narrative.save_event(&event).await?;
        Ok(narrative_event_to_summary(&event))
    }

    pub async fn update(
        &self,
        event_id: NarrativeEventId,
        name: Option<String>,
        description: Option<String>,
        trigger_conditions: Option<Vec<NarrativeTrigger>>,
        outcomes: Option<Vec<domain::EventOutcome>>,
    ) -> Result<NarrativeEventSummary, NarrativeEventError> {
        let mut event = self
            .narrative
            .get_event(event_id)
            .await?
            .ok_or(NarrativeEventError::NotFound)?;

        if let Some(name) = name {
            event.name = name;
        }
        if let Some(description) = description {
            event.description = description;
        }
        if let Some(triggers) = trigger_conditions {
            event.trigger_conditions = triggers;
        }
        if let Some(outcomes) = outcomes {
            event.outcomes = outcomes;
        }

        self.narrative.save_event(&event).await?;
        Ok(narrative_event_to_summary(&event))
    }

    pub async fn delete(&self, event_id: NarrativeEventId) -> Result<(), NarrativeEventError> {
        self.narrative.delete_event(event_id).await?;
        Ok(())
    }

    pub async fn set_active(
        &self,
        event_id: NarrativeEventId,
        active: bool,
    ) -> Result<(), NarrativeEventError> {
        let mut event = self
            .narrative
            .get_event(event_id)
            .await?
            .ok_or(NarrativeEventError::NotFound)?;
        event.is_active = active;
        self.narrative.save_event(&event).await?;
        Ok(())
    }

    pub async fn set_favorite(
        &self,
        event_id: NarrativeEventId,
        favorite: bool,
    ) -> Result<(), NarrativeEventError> {
        let mut event = self
            .narrative
            .get_event(event_id)
            .await?
            .ok_or(NarrativeEventError::NotFound)?;
        event.is_favorite = favorite;
        self.narrative.save_event(&event).await?;
        Ok(())
    }

    pub async fn trigger(
        &self,
        event_id: NarrativeEventId,
        world_id: WorldId,
        pc_id: Option<domain::PlayerCharacterId>,
    ) -> Result<TriggeredNarrativeEvent, NarrativeEventError> {
        let mut event = self
            .narrative
            .get_event(event_id)
            .await?
            .ok_or(NarrativeEventError::NotFound)?;

        if event.world_id != world_id {
            return Err(NarrativeEventError::WorldMismatch);
        }

        let outcome_name = event
            .selected_outcome
            .clone()
            .or_else(|| event.default_outcome.clone())
            .or_else(|| event.outcomes.first().map(|o| o.name.clone()))
            .unwrap_or_default();

        event.is_triggered = true;
        event.selected_outcome = Some(outcome_name.clone());
        event.triggered_at = Some(Utc::now());
        event.trigger_count = event.trigger_count.saturating_add(1);
        let maybe_outcome = event.outcomes.iter().find(|o| o.name == outcome_name);
        self.narrative.save_event(&event).await?;

        let mut effects_summary = None;
        let effects_present = maybe_outcome
            .as_ref()
            .map(|outcome| !outcome.effects.is_empty())
            .unwrap_or(false);
        if let (Some(outcome), Some(pc_id)) = (maybe_outcome, pc_id) {
            if effects_present {
                let context = EffectExecutionContext {
                    pc_id,
                    world_id,
                    current_scene_id: None,
                };

                let summary = self
                    .execute_effects
                    .execute(event.id, outcome.name.clone(), &outcome.effects, &context)
                    .await;
                effects_summary = Some(summary);
            }
        }

        let outcome_description = maybe_outcome
            .map(|o| o.description.clone())
            .unwrap_or_default();

        Ok(TriggeredNarrativeEvent {
            world_id: event.world_id,
            event_id: event.id,
            event_name: event.name.clone(),
            outcome_name,
            outcome_description,
            scene_direction: event.scene_direction.clone(),
            effects_summary,
            effects_present,
        })
    }

    pub async fn reset(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<NarrativeEventSummary, NarrativeEventError> {
        let mut event = self
            .narrative
            .get_event(event_id)
            .await?
            .ok_or(NarrativeEventError::NotFound)?;
        event.is_triggered = false;
        event.selected_outcome = None;
        event.triggered_at = None;
        event.trigger_count = 0;
        self.narrative.save_event(&event).await?;
        Ok(narrative_event_to_summary(&event))
    }
}

#[derive(Debug)]
pub struct TriggeredNarrativeEvent {
    pub world_id: WorldId,
    pub event_id: NarrativeEventId,
    pub event_name: String,
    pub outcome_name: String,
    pub outcome_description: String,
    pub scene_direction: String,
    pub effects_summary: Option<EffectExecutionSummary>,
    pub effects_present: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum NarrativeEventError {
    #[error("Narrative event not found")]
    NotFound,
    #[error("Event does not belong to the requested world")]
    WorldMismatch,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn narrative_event_to_summary(event: &NarrativeEvent) -> NarrativeEventSummary {
    NarrativeEventSummary {
        id: event.id.to_string(),
        world_id: event.world_id.to_string(),
        name: event.name.clone(),
        description: event.description.clone(),
        scene_direction: event.scene_direction.clone(),
        suggested_opening: event.suggested_opening.clone(),
        trigger_count: event.trigger_count,
        is_active: event.is_active,
        is_triggered: event.is_triggered,
        triggered_at: event.triggered_at.map(|dt| dt.to_rfc3339()),
        selected_outcome: event.selected_outcome.clone(),
        is_repeatable: event.is_repeatable,
        delay_turns: event.delay_turns,
        expires_after_turns: event.expires_after_turns,
        priority: event.priority,
        is_favorite: event.is_favorite,
        tags: event.tags.clone(),
        scene_id: None,
        location_id: None,
        act_id: None,
        chain_id: None,
        chain_position: None,
        outcome_count: event.outcomes.len(),
        trigger_condition_count: event.trigger_conditions.len(),
        created_at: event.created_at.to_rfc3339(),
        updated_at: event.updated_at.to_rfc3339(),
    }
}
