//! Staging DTOs for engine-adapters layer
//!
//! These DTOs are used for staging proposal/approval workflows between
//! adapters and the engine-app layer.

use serde::{Deserialize, Serialize};
use wrldbldr_domain::value_objects::{RuleBasedSuggestion, StagingContext};

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
    #[serde(default)]
    pub is_hidden_from_players: bool,
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
            is_hidden_from_players: false,
            reasoning: suggestion.reasoning,
        }
    }
}
