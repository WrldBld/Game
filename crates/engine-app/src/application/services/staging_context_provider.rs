//! Staging Context Provider - Gathers context for LLM staging decisions
//!
//! This service collects all relevant information needed for the LLM to make
//! informed decisions about NPC presence in a region:
//! - Region and location information
//! - Time of day context
//! - Active narrative events
//! - Recent NPC dialogue history (using StoryEventService)
//! - Rule-based presence suggestions

use anyhow::Result;
use std::sync::Arc;

use crate::application::services::StoryEventService;
use wrldbldr_domain::entities::Character;
use wrldbldr_domain::value_objects::{
    ActiveEventContext, NpcDialogueContext, RegionRelationshipType, RuleBasedSuggestion,
    StagingContext,
};
use wrldbldr_domain::{CharacterId, GameTime, RegionId, WorldId};
use wrldbldr_engine_ports::outbound::{NarrativeEventRepositoryPort, RegionRepositoryPort};

/// Service for gathering context needed for staging decisions
pub struct StagingContextProvider<R: RegionRepositoryPort, N: NarrativeEventRepositoryPort> {
    region_repository: Arc<R>,
    narrative_event_repository: Arc<N>,
    story_event_service: Arc<dyn StoryEventService>,
}

impl<R: RegionRepositoryPort, N: NarrativeEventRepositoryPort> StagingContextProvider<R, N> {
    pub fn new(
        region_repository: Arc<R>,
        narrative_event_repository: Arc<N>,
        story_event_service: Arc<dyn StoryEventService>,
    ) -> Self {
        Self {
            region_repository,
            narrative_event_repository,
            story_event_service,
        }
    }

    /// Gather complete staging context for a region
    ///
    /// This collects all information needed for both rule-based and LLM-based
    /// staging decisions.
    pub async fn gather_context(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_name: &str,
        game_time: &GameTime,
    ) -> Result<StagingContext> {
        // Get region information
        let region = self
            .region_repository
            .get(region_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", region_id))?;

        let time_of_day = game_time.time_of_day();

        // Get active narrative events for this region/world
        let active_events = self.get_active_events(world_id, region_id).await?;

        // Get NPC relationships to this region (for gathering dialogue context)
        let npc_relationships = self
            .region_repository
            .get_npcs_related_to_region(region_id)
            .await?;

        // Gather dialogue context for each NPC
        let mut npc_dialogues = Vec::new();
        for (character, _rel_type) in &npc_relationships {
            if let Some(dialogue_ctx) = self
                .get_npc_dialogue_context(world_id, character.id, &character.name)
                .await?
            {
                npc_dialogues.push(dialogue_ctx);
            }
        }

        Ok(StagingContext::new(
            region.name,
            region.description,
            location_name,
            time_of_day.to_string(),
            game_time.display_time(),
        )
        .with_active_events(active_events)
        .with_npc_dialogues(npc_dialogues))
    }

    /// Generate rule-based NPC presence suggestions
    ///
    /// Uses the canonical domain logic from RegionRelationshipType to determine
    /// which NPCs should be present based on their relationship to the region
    /// and the current time of day.
    pub async fn generate_rule_based_suggestions(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Vec<RuleBasedSuggestion>> {
        let npc_relationships = self
            .region_repository
            .get_npcs_related_to_region(region_id)
            .await?;

        let time_of_day = game_time.time_of_day();

        let suggestions = npc_relationships
            .into_iter()
            .map(|(character, rel_type)| {
                let is_present = rel_type.is_npc_present(time_of_day);
                let reasoning = rel_type.presence_reasoning(time_of_day);

                // For frequents relationships, we could add probabilistic logic
                // For now, use deterministic rules
                RuleBasedSuggestion {
                    character_id: character.id.into(),
                    character_name: character.name,
                    is_present,
                    reasoning,
                    roll_result: None,
                }
            })
            .collect();

        Ok(suggestions)
    }

    /// Get NPCs related to a region with their relationships
    ///
    /// This is used by the staging service to build the full NPC list.
    pub async fn get_npcs_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        self.region_repository
            .get_npcs_related_to_region(region_id)
            .await
    }

    /// Get active narrative events relevant to staging
    async fn get_active_events(
        &self,
        world_id: WorldId,
        _region_id: RegionId,
    ) -> Result<Vec<ActiveEventContext>> {
        // Get active narrative events for this world
        // TODO: Filter by region relevance when we have region-specific events
        let events = self
            .narrative_event_repository
            .list_by_world(world_id)
            .await?;

        let active_events: Vec<ActiveEventContext> = events
            .into_iter()
            .filter(|e| e.is_active)
            .filter_map(|event| {
                // Only include events that might affect NPC presence
                if event.outcomes.is_empty() {
                    return None;
                }

                Some(ActiveEventContext::new(
                    event.name,
                    event.description,
                    "May affect NPC availability".to_string(),
                ))
            })
            .take(5) // Limit to 5 most relevant events
            .collect();

        Ok(active_events)
    }

    /// Get dialogue context for an NPC
    async fn get_npc_dialogue_context(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        npc_name: &str,
    ) -> Result<Option<NpcDialogueContext>> {
        // Use story event service to get dialogue summary
        let summary = self
            .story_event_service
            .get_dialogue_summary_for_npc(world_id, npc_id, 3)
            .await?;

        match summary {
            Some(summary_text) => {
                Ok(Some(NpcDialogueContext::new(
                    npc_id.into(),
                    npc_name,
                    summary_text,
                    "Recent".to_string(), // TODO: Add actual timestamp
                )))
            }
            None => Ok(None),
        }
    }
}

/// Build the LLM prompt for staging decisions
///
/// This function formats the staging context into a prompt suitable for
/// sending to the LLM for NPC presence reasoning.
///
/// # Arguments
/// * `context` - The staging context with location/time info
/// * `rule_suggestions` - Rule-based NPC presence suggestions
/// * `dm_guidance` - Optional DM guidance text
/// * `role_instructions` - Configurable role instructions from prompt template
/// * `response_format` - Configurable response format from prompt template
pub fn build_staging_prompt(
    context: &StagingContext,
    rule_suggestions: &[RuleBasedSuggestion],
    dm_guidance: Option<&str>,
    role_instructions: &str,
    response_format: &str,
) -> String {
    let mut prompt = format!(
        r#"You are helping determine which NPCs are present in a location for a TTRPG game.

## Location
{} ({})
{}
Time: {} ({})

"#,
        context.region_name,
        context.location_name,
        context.region_description,
        context.time_of_day,
        context.time_display,
    );

    // Add rule-based suggestions
    if !rule_suggestions.is_empty() {
        prompt.push_str("## Rule-Based Suggestions\n");
        for suggestion in rule_suggestions {
            prompt.push_str(&format!(
                "- {} ({}): {}\n",
                suggestion.character_name,
                if suggestion.is_present {
                    "present"
                } else {
                    "absent"
                },
                suggestion.reasoning,
            ));
        }
        prompt.push('\n');
    }

    // Add role instructions (configurable)
    prompt.push_str(role_instructions);
    prompt.push('\n');

    // Add active events
    if !context.active_events.is_empty() {
        prompt.push_str("## Active Story Elements\n");
        for event in &context.active_events {
            prompt.push_str(&format!(
                "- {}: {} ({})\n",
                event.event_name, event.description, event.relevance,
            ));
        }
        prompt.push('\n');
    }

    // Add NPC dialogues
    if !context.npc_dialogues.is_empty() {
        prompt.push_str("## Recent NPC Interactions\n");
        for dialogue in &context.npc_dialogues {
            prompt.push_str(&format!(
                "- {} ({}): {}\n",
                dialogue.character_name,
                dialogue.game_time_of_dialogue,
                dialogue.last_dialogue_summary,
            ));
        }
        prompt.push('\n');
    }

    // Add DM guidance if provided
    if let Some(guidance) = dm_guidance {
        prompt.push_str(&format!("## DM Guidance\n{}\n\n", guidance));
    }

    // Add response format (configurable)
    prompt.push_str(response_format);

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use wrldbldr_domain::value_objects::prompt_defaults;
    use wrldbldr_domain::TimeOfDay;

    #[test]
    fn test_build_staging_prompt_basic() {
        let context = StagingContext::new(
            "Town Square",
            "A busy marketplace in the center of town",
            "Riverside Village",
            TimeOfDay::Morning.to_string(),
            "8:00 AM",
        );

        let suggestions = vec![RuleBasedSuggestion::present(
            Uuid::new_v4(),
            "Bob the Baker",
            "Works here during the day",
        )];

        let prompt = build_staging_prompt(
            &context,
            &suggestions,
            None,
            prompt_defaults::STAGING_ROLE_INSTRUCTIONS,
            prompt_defaults::STAGING_RESPONSE_FORMAT,
        );

        assert!(prompt.contains("Town Square"));
        assert!(prompt.contains("Riverside Village"));
        assert!(prompt.contains("Bob the Baker"));
        assert!(prompt.contains("Morning"));
    }

    #[test]
    fn test_build_staging_prompt_with_guidance() {
        let context = StagingContext::new(
            "Dark Alley",
            "A shadowy back alley",
            "City of Shadows",
            TimeOfDay::Night.to_string(),
            "11:00 PM",
        );

        let prompt = build_staging_prompt(
            &context,
            &[],
            Some("The thief guild is having a secret meeting tonight"),
            prompt_defaults::STAGING_ROLE_INSTRUCTIONS,
            prompt_defaults::STAGING_RESPONSE_FORMAT,
        );

        assert!(prompt.contains("DM Guidance"));
        assert!(prompt.contains("thief guild"));
    }
}
