//! Staging Service - Core business logic for NPC presence staging
//!
//! This service manages the full lifecycle of NPC staging:
//! - Checking for valid existing stagings
//! - Generating staging proposals (rule-based and LLM)
//! - Approving and persisting stagings
//! - Pre-staging regions
//! - Managing staging history

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

use crate::application::services::{
    build_staging_prompt, StagingContextProvider, StoryEventService,
};
use wrldbldr_domain::entities::{StagedNpc, Staging, StagingSource};
use wrldbldr_domain::value_objects::{prompt_keys, RuleBasedSuggestion, StagingContext};
use wrldbldr_domain::{GameTime, LocationId, RegionId, WorldId};
use crate::application::services::internal::{
    ApprovedNpc as PortApprovedNpc, PromptTemplateServicePort,
    StagedNpcProposal as PortStagedNpcProposal, StagingProposal as PortStagingProposal,
    StagingServicePort,
};
use wrldbldr_engine_ports::outbound::{
    ApprovedNpcData, ChatMessage, ClockPort, LlmPort, LlmRequest, NarrativeEventCrudPort,
    RegionCrudPort, RegionNpcPort, StagingRepositoryPort,
};

fn rule_based_suggestion_to_proposal(suggestion: &RuleBasedSuggestion) -> PortStagedNpcProposal {
    PortStagedNpcProposal {
        character_id: suggestion.character_id.to_string(),
        name: suggestion.character_name.clone(),
        sprite_asset: None,
        portrait_asset: None,
        is_present: suggestion.is_present,
        is_hidden_from_players: false,
        reasoning: suggestion.reasoning.clone(),
    }
}

/// Configuration for the staging service
#[derive(Debug, Clone)]
pub struct StagingServiceConfig {
    /// Default TTL in hours if location doesn't specify
    pub default_ttl_hours: i32,
    /// Whether to use LLM for staging suggestions
    pub use_llm: bool,
    /// Temperature for LLM queries (lower = more deterministic)
    pub llm_temperature: f32,
}

impl Default for StagingServiceConfig {
    fn default() -> Self {
        Self {
            default_ttl_hours: 3,
            use_llm: true,
            llm_temperature: 0.3,
        }
    }
}

/// Service for managing NPC staging in regions
pub struct StagingService<L, RC, RN, N, S>
where
    L: LlmPort,
    RC: RegionCrudPort,
    RN: RegionNpcPort,
    N: NarrativeEventCrudPort,
    S: StagingRepositoryPort,
{
    staging_repository: Arc<S>,
    context_provider: StagingContextProvider<RC, RN, N>,
    llm_port: Arc<L>,
    prompt_template_service: Arc<dyn PromptTemplateServicePort>,
    clock: Arc<dyn ClockPort>,
    config: StagingServiceConfig,
}

impl<L, RC, RN, N, S> StagingService<L, RC, RN, N, S>
where
    L: LlmPort,
    RC: RegionCrudPort,
    RN: RegionNpcPort,
    N: NarrativeEventCrudPort,
    S: StagingRepositoryPort,
{
    pub fn new(
        staging_repository: Arc<S>,
        region_crud: Arc<RC>,
        region_npc: Arc<RN>,
        narrative_event_repository: Arc<N>,
        story_event_service: Arc<dyn StoryEventService>,
        llm_port: Arc<L>,
        prompt_template_service: Arc<dyn PromptTemplateServicePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        let context_provider = StagingContextProvider::new(
            region_crud,
            region_npc,
            narrative_event_repository,
            story_event_service,
        );

        Self {
            staging_repository,
            context_provider,
            llm_port,
            prompt_template_service,
            clock,
            config: StagingServiceConfig::default(),
        }
    }

    pub fn with_config(mut self, config: StagingServiceConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the current valid staging for a region, if one exists
    ///
    /// Returns None if no staging exists or the current staging has expired.
    pub async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Option<Staging>> {
        let staging = self.staging_repository.get_current(region_id).await?;

        match staging {
            Some(s) if !s.is_expired(&game_time.current()) => Ok(Some(s)),
            _ => Ok(None),
        }
    }

    /// Generate a staging proposal for a region
    ///
    /// This creates both rule-based and LLM-based suggestions for DM approval.
    pub async fn generate_proposal(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_id: LocationId,
        location_name: &str,
        game_time: &GameTime,
        ttl_hours: i32,
        dm_guidance: Option<&str>,
    ) -> Result<PortStagingProposal> {
        // Generate request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        // Gather context for staging decisions
        let context = self
            .context_provider
            .gather_context(world_id, region_id, location_name, game_time)
            .await?;

        // Generate rule-based suggestions
        let rule_suggestions = self
            .context_provider
            .generate_rule_based_suggestions(region_id, game_time)
            .await?;

        // Get NPC details for sprites/portraits
        let npcs_with_relationships = self.context_provider.get_npcs_for_region(region_id).await?;

        // Convert rule suggestions to proposals with NPC details
        let rule_based_npcs: Vec<PortStagedNpcProposal> = rule_suggestions
            .iter()
            .map(|s| {
                let mut proposal = rule_based_suggestion_to_proposal(s);

                // Enrich with sprite/portrait from character data
                if let Some((character, _)) = npcs_with_relationships
                    .iter()
                    .find(|(c, _)| c.id == s.character_id.into())
                {
                    proposal.sprite_asset = character.sprite_asset.clone();
                    proposal.portrait_asset = character.portrait_asset.clone();
                }

                proposal
            })
            .collect();

        // Generate LLM suggestions if enabled
        let llm_based_npcs = if self.config.use_llm && !rule_suggestions.is_empty() {
            match self
                .generate_llm_suggestions(
                    world_id,
                    &context,
                    &rule_suggestions,
                    &npcs_with_relationships,
                    dm_guidance,
                )
                .await
            {
                Ok(npcs) => npcs,
                Err(e) => {
                    tracing::warn!("LLM staging suggestions failed: {}. Using rules only.", e);
                    rule_based_npcs.clone()
                }
            }
        } else {
            // If LLM disabled or no NPCs, just use rule-based
            rule_based_npcs.clone()
        };

        Ok(PortStagingProposal {
            request_id,
            region_id: region_id.to_string(),
            location_id: location_id.to_string(),
            world_id: world_id.to_string(),
            rule_based_npcs,
            llm_based_npcs,
            default_ttl_hours: ttl_hours,
            context,
        })
    }

    /// Approve a staging proposal and persist it
    ///
    /// Called when DM approves a staging with their chosen NPCs.
    pub async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        approved_npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        source: StagingSource,
        approved_by: &str,
        dm_guidance: Option<String>,
    ) -> Result<Staging> {
        // Invalidate any existing stagings for this region
        self.staging_repository.invalidate_all(region_id).await?;

        // Create the staging entity
        let mut staging = Staging::new(
            region_id,
            location_id,
            world_id,
            game_time.current(),
            approved_by,
            source,
            ttl_hours,
            self.clock.now(),
        );

        if let Some(guidance) = dm_guidance {
            staging = staging.with_guidance(guidance);
        }

        // Convert approved NPCs to StagedNpc entities
        let staged_npcs: Vec<StagedNpc> = approved_npcs
            .into_iter()
            .map(|npc| {
                let mut staged =
                    StagedNpc::new(npc.character_id, npc.name, npc.is_present, npc.reasoning);
                staged.is_hidden_from_players = npc.is_hidden_from_players;
                if let Some(sprite) = npc.sprite_asset {
                    staged = staged.with_sprite(sprite);
                }
                if let Some(portrait) = npc.portrait_asset {
                    staged = staged.with_portrait(portrait);
                }
                staged
            })
            .collect();

        staging = staging.with_npcs(staged_npcs);

        // Persist the staging
        let staging_id = self.staging_repository.save(&staging).await?;
        staging.id = staging_id;

        // Set as current staging for the region
        self.staging_repository.set_current(staging.id).await?;

        tracing::info!(
            "Approved staging {} for region {} with {} NPCs (TTL: {}h)",
            staging.id,
            region_id,
            staging.npcs.len(),
            ttl_hours
        );

        Ok(staging)
    }

    /// Pre-stage a region before player arrival
    ///
    /// Used by DM to set up NPCs ahead of time.
    pub async fn pre_stage_region(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        dm_id: &str,
    ) -> Result<Staging> {
        self.approve_staging(
            region_id,
            location_id,
            world_id,
            game_time,
            npcs,
            ttl_hours,
            StagingSource::PreStaged,
            dm_id,
            None,
        )
        .await
    }

    /// Regenerate LLM suggestions with new guidance
    ///
    /// Called when DM requests regeneration with additional context.
    pub async fn regenerate_suggestions(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_name: &str,
        game_time: &GameTime,
        guidance: &str,
    ) -> Result<Vec<PortStagedNpcProposal>> {
        // Gather fresh context
        let context = self
            .context_provider
            .gather_context(world_id, region_id, location_name, game_time)
            .await?;

        // Get rule suggestions and NPC data
        let rule_suggestions = self
            .context_provider
            .generate_rule_based_suggestions(region_id, game_time)
            .await?;

        let npcs_with_relationships = self.context_provider.get_npcs_for_region(region_id).await?;

        // Regenerate with guidance
        self.generate_llm_suggestions(
            world_id,
            &context,
            &rule_suggestions,
            &npcs_with_relationships,
            Some(guidance),
        )
        .await
    }

    /// Get previous staging for a region (even if expired)
    ///
    /// Used to show DM what was there before.
    pub async fn get_previous_staging(&self, region_id: RegionId) -> Result<Option<Staging>> {
        self.staging_repository.get_current(region_id).await
    }

    /// Get staging history for a region
    pub async fn get_history(&self, region_id: RegionId, limit: u32) -> Result<Vec<Staging>> {
        self.staging_repository.get_history(region_id, limit).await
    }

    /// Generate LLM-based staging suggestions
    async fn generate_llm_suggestions(
        &self,
        world_id: WorldId,
        context: &StagingContext,
        rule_suggestions: &[RuleBasedSuggestion],
        npcs_with_relationships: &[(
            wrldbldr_domain::entities::Character,
            wrldbldr_domain::value_objects::RegionRelationshipType,
        )],
        dm_guidance: Option<&str>,
    ) -> Result<Vec<PortStagedNpcProposal>> {
        // Resolve prompt templates
        let system_prompt = self
            .prompt_template_service
            .resolve_for_world_with_source(world_id, prompt_keys::STAGING_SYSTEM_PROMPT)
            .await
            .value;
        let role_instructions = self
            .prompt_template_service
            .resolve_for_world_with_source(world_id, prompt_keys::STAGING_ROLE_INSTRUCTIONS)
            .await
            .value;
        let response_format = self
            .prompt_template_service
            .resolve_for_world_with_source(world_id, prompt_keys::STAGING_RESPONSE_FORMAT)
            .await
            .value;

        // Build the prompt with configurable templates
        let prompt = build_staging_prompt(
            context,
            rule_suggestions,
            dm_guidance,
            &role_instructions,
            &response_format,
        );

        let request = LlmRequest::new(vec![ChatMessage::user(prompt)])
            .with_system_prompt(system_prompt)
            .with_temperature(self.config.llm_temperature);

        // Query the LLM
        let response = self
            .llm_port
            .generate(request)
            .await
            .map_err(|e| anyhow::anyhow!("LLM staging query failed: {}", e))?;

        // Parse the response
        self.parse_llm_response(&response.content, rule_suggestions, npcs_with_relationships)
    }

    /// Parse LLM response into staged NPC proposals
    fn parse_llm_response(
        &self,
        response: &str,
        rule_suggestions: &[RuleBasedSuggestion],
        npcs_with_relationships: &[(
            wrldbldr_domain::entities::Character,
            wrldbldr_domain::value_objects::RegionRelationshipType,
        )],
    ) -> Result<Vec<PortStagedNpcProposal>> {
        // Extract JSON from response
        let json_str = extract_json_array(response)
            .ok_or_else(|| anyhow::anyhow!("Could not parse LLM response as JSON"))?;

        #[derive(Deserialize)]
        struct LlmNpcResult {
            name: String,
            is_present: bool,
            #[serde(default)]
            is_hidden_from_players: bool,
            reasoning: String,
        }

        let llm_results: Vec<LlmNpcResult> = serde_json::from_str(&json_str)?;

        // Map LLM results back to full proposals
        let mut proposals = Vec::new();

        for suggestion in rule_suggestions {
            let llm_result = llm_results
                .iter()
                .find(|r| r.name.to_lowercase() == suggestion.character_name.to_lowercase());

            let (is_present, is_hidden_from_players, reasoning) = if let Some(r) = llm_result {
                (r.is_present, r.is_hidden_from_players, r.reasoning.clone())
            } else {
                // Default to rule-based if LLM didn't mention this NPC
                (suggestion.is_present, false, suggestion.reasoning.clone())
            };

            let mut proposal = PortStagedNpcProposal {
                character_id: suggestion.character_id.to_string(),
                name: suggestion.character_name.clone(),
                sprite_asset: None,
                portrait_asset: None,
                is_present,
                is_hidden_from_players,
                reasoning,
            };

            // Enrich with sprite/portrait
            if let Some((character, _)) = npcs_with_relationships
                .iter()
                .find(|(c, _)| c.id == suggestion.character_id.into())
            {
                proposal.sprite_asset = character.sprite_asset.clone();
                proposal.portrait_asset = character.portrait_asset.clone();
            }

            proposals.push(proposal);
        }

        Ok(proposals)
    }
}

/// Extract a JSON array from a potentially mixed response
fn extract_json_array(text: &str) -> Option<String> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if end > start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl<L, RC, RN, N, S> StagingServicePort for StagingService<L, RC, RN, N, S>
where
    L: LlmPort + Send + Sync + 'static,
    RC: RegionCrudPort + Send + Sync + 'static,
    RN: RegionNpcPort + Send + Sync + 'static,
    N: NarrativeEventCrudPort + Send + Sync + 'static,
    S: StagingRepositoryPort + Send + Sync + 'static,
{
    async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: GameTime,
    ) -> Result<Option<Staging>> {
        self.get_current_staging(region_id, &game_time).await
    }

    async fn generate_proposal(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_id: LocationId,
        location_name: String,
        game_time: GameTime,
        ttl_hours: i32,
        dm_guidance: Option<String>,
    ) -> Result<PortStagingProposal> {
        self.generate_proposal(
            world_id,
            region_id,
            location_id,
            &location_name,
            &game_time,
            ttl_hours,
            dm_guidance.as_deref(),
        )
        .await
    }

    async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: GameTime,
        approved_npcs: Vec<PortApprovedNpc>,
        ttl_hours: i32,
        source: StagingSource,
        approved_by: String,
        dm_guidance: Option<String>,
    ) -> Result<Staging> {
        // Convert PortApprovedNpc to ApprovedNpcData
        let approved_npc_data: Vec<ApprovedNpcData> = approved_npcs
            .into_iter()
            .map(|n| ApprovedNpcData {
                character_id: n.character_id,
                name: n.name,
                sprite_asset: n.sprite_asset,
                portrait_asset: n.portrait_asset,
                is_present: n.is_present,
                is_hidden_from_players: n.is_hidden_from_players,
                reasoning: n.reasoning,
            })
            .collect();

        self.approve_staging(
            region_id,
            location_id,
            world_id,
            &game_time,
            approved_npc_data,
            ttl_hours,
            source,
            &approved_by,
            dm_guidance,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_extract_json_array() {
        let response = r#"Here are the results:
[
  {"name": "Bob", "is_present": true, "reasoning": "Works here"}
]
That's all!"#;

        let json = extract_json_array(response).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[test]
    fn test_staged_npc_proposal_from_suggestion() {
        let suggestion = RuleBasedSuggestion::present(Uuid::new_v4(), "Test NPC", "Test reasoning");

        let proposal = rule_based_suggestion_to_proposal(&suggestion);
        assert_eq!(proposal.name, "Test NPC");
        assert!(proposal.is_present);
        assert_eq!(proposal.reasoning, "Test reasoning");
    }
}
