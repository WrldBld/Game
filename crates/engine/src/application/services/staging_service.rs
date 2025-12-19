//! Staging Service - Core business logic for NPC presence staging
//!
//! This service manages the full lifecycle of NPC staging:
//! - Checking for valid existing stagings
//! - Generating staging proposals (rule-based and LLM)
//! - Approving and persisting stagings
//! - Pre-staging regions
//! - Managing staging history

use std::sync::Arc;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{
    ChatMessage, LlmPort, LlmRequest, NarrativeEventRepositoryPort, 
    RegionRepositoryPort, StagingRepositoryPort,
};
use crate::application::services::{
    StoryEventService, StagingContextProvider, build_staging_prompt,
};
use crate::domain::entities::{Staging, StagedNpc, StagingSource};
use crate::domain::value_objects::{GameTime, RuleBasedSuggestion, StagingContext};
use wrldbldr_domain::{CharacterId, LocationId, RegionId, StagingId, WorldId};

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

/// A staging proposal with both rule-based and LLM options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingProposal {
    /// Request ID for tracking this proposal through the approval flow
    pub request_id: String,
    /// Region this staging is for
    pub region_id: String,
    /// Location containing the region
    pub location_id: String,
    /// World ID
    pub world_id: String,
    /// Rule-based NPC suggestions
    pub rule_based_npcs: Vec<StagedNpcProposal>,
    /// LLM-based NPC suggestions (may be same as rule-based if LLM agrees)
    pub llm_based_npcs: Vec<StagedNpcProposal>,
    /// Default TTL from location settings
    pub default_ttl_hours: i32,
    /// Staging context used for generation
    pub context: StagingContext,
}

/// A proposed NPC for staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedNpcProposal {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
}

impl From<RuleBasedSuggestion> for StagedNpcProposal {
    fn from(suggestion: RuleBasedSuggestion) -> Self {
        Self {
            character_id: suggestion.character_id.to_string(),
            name: suggestion.character_name,
            sprite_asset: None,
            portrait_asset: None,
            is_present: suggestion.is_present,
            reasoning: suggestion.reasoning,
        }
    }
}

/// Service for managing NPC staging in regions
pub struct StagingService<L, R, N, S>
where
    L: LlmPort,
    R: RegionRepositoryPort,
    N: NarrativeEventRepositoryPort,
    S: StagingRepositoryPort,
{
    staging_repository: Arc<S>,
    context_provider: StagingContextProvider<R, N>,
    llm_port: Arc<L>,
    config: StagingServiceConfig,
}

impl<L, R, N, S> StagingService<L, R, N, S>
where
    L: LlmPort,
    R: RegionRepositoryPort,
    N: NarrativeEventRepositoryPort,
    S: StagingRepositoryPort,
{
    pub fn new(
        staging_repository: Arc<S>,
        region_repository: Arc<R>,
        narrative_event_repository: Arc<N>,
        story_event_service: StoryEventService,
        llm_port: Arc<L>,
    ) -> Self {
        let context_provider = StagingContextProvider::new(
            region_repository,
            narrative_event_repository,
            story_event_service,
        );

        Self {
            staging_repository,
            context_provider,
            llm_port,
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
    ) -> Result<StagingProposal> {
        // Generate request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        // Gather context for staging decisions
        let context = self.context_provider.gather_context(
            world_id,
            region_id,
            location_name,
            game_time,
        ).await?;

        // Generate rule-based suggestions
        let rule_suggestions = self.context_provider
            .generate_rule_based_suggestions(region_id, game_time)
            .await?;

        // Get NPC details for sprites/portraits
        let npcs_with_relationships = self.context_provider
            .get_npcs_for_region(region_id)
            .await?;

        // Convert rule suggestions to proposals with NPC details
        let rule_based_npcs: Vec<StagedNpcProposal> = rule_suggestions
            .iter()
            .map(|s| {
                let mut proposal = StagedNpcProposal::from(s.clone());
                
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
            match self.generate_llm_suggestions(
                &context,
                &rule_suggestions,
                &npcs_with_relationships,
                dm_guidance,
            ).await {
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

        Ok(StagingProposal {
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
        );

        if let Some(guidance) = dm_guidance {
            staging = staging.with_guidance(guidance);
        }

        // Convert approved NPCs to StagedNpc entities
        let staged_npcs: Vec<StagedNpc> = approved_npcs
            .into_iter()
            .map(|npc| {
                let mut staged = StagedNpc::new(
                    npc.character_id,
                    npc.name,
                    npc.is_present,
                    npc.reasoning,
                );
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
        ).await
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
    ) -> Result<Vec<StagedNpcProposal>> {
        // Gather fresh context
        let context = self.context_provider.gather_context(
            world_id,
            region_id,
            location_name,
            game_time,
        ).await?;

        // Get rule suggestions and NPC data
        let rule_suggestions = self.context_provider
            .generate_rule_based_suggestions(region_id, game_time)
            .await?;

        let npcs_with_relationships = self.context_provider
            .get_npcs_for_region(region_id)
            .await?;

        // Regenerate with guidance
        self.generate_llm_suggestions(
            &context,
            &rule_suggestions,
            &npcs_with_relationships,
            Some(guidance),
        ).await
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
        context: &StagingContext,
        rule_suggestions: &[RuleBasedSuggestion],
        npcs_with_relationships: &[(crate::domain::entities::Character, crate::domain::value_objects::RegionRelationshipType)],
        dm_guidance: Option<&str>,
    ) -> Result<Vec<StagedNpcProposal>> {
        // Build the prompt
        let prompt = build_staging_prompt(context, rule_suggestions, dm_guidance);

        let request = LlmRequest::new(vec![ChatMessage::user(prompt)])
            .with_system_prompt("You are a game master assistant helping determine NPC presence.")
            .with_temperature(self.config.llm_temperature);

        // Query the LLM
        let response = self.llm_port.generate(request).await
            .map_err(|e| anyhow::anyhow!("LLM staging query failed: {}", e))?;

        // Parse the response
        self.parse_llm_response(&response.content, rule_suggestions, npcs_with_relationships)
    }

    /// Parse LLM response into staged NPC proposals
    fn parse_llm_response(
        &self,
        response: &str,
        rule_suggestions: &[RuleBasedSuggestion],
        npcs_with_relationships: &[(crate::domain::entities::Character, crate::domain::value_objects::RegionRelationshipType)],
    ) -> Result<Vec<StagedNpcProposal>> {
        // Extract JSON from response
        let json_str = extract_json_array(response)
            .ok_or_else(|| anyhow::anyhow!("Could not parse LLM response as JSON"))?;

        #[derive(Deserialize)]
        struct LlmNpcResult {
            name: String,
            is_present: bool,
            reasoning: String,
        }

        let llm_results: Vec<LlmNpcResult> = serde_json::from_str(&json_str)?;

        // Map LLM results back to full proposals
        let mut proposals = Vec::new();
        
        for suggestion in rule_suggestions {
            let llm_result = llm_results
                .iter()
                .find(|r| r.name.to_lowercase() == suggestion.character_name.to_lowercase());

            let (is_present, reasoning) = if let Some(r) = llm_result {
                (r.is_present, r.reasoning.clone())
            } else {
                // Default to rule-based if LLM didn't mention this NPC
                (suggestion.is_present, suggestion.reasoning.clone())
            };

            let mut proposal = StagedNpcProposal {
                character_id: suggestion.character_id.to_string(),
                name: suggestion.character_name.clone(),
                sprite_asset: None,
                portrait_asset: None,
                is_present,
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

/// Data for an approved NPC
#[derive(Debug, Clone)]
pub struct ApprovedNpcData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let suggestion = RuleBasedSuggestion::present(
            CharacterId::new(),
            "Test NPC",
            "Test reasoning",
        );

        let proposal = StagedNpcProposal::from(suggestion);
        assert_eq!(proposal.name, "Test NPC");
        assert!(proposal.is_present);
        assert_eq!(proposal.reasoning, "Test reasoning");
    }
}
