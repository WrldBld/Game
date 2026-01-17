//! Regenerate staging suggestions use case.

use std::sync::Arc;

use wrldbldr_domain::RegionId;

use crate::repositories::{CharacterRepository, LlmService, Location};

use super::suggestions::generate_llm_based_suggestions;
use super::types::StagedNpc;
use super::StagingError;

/// Use case for regenerating LLM staging suggestions.
pub struct RegenerateStagingSuggestions {
    location: Arc<Location>,
    character: Arc<CharacterRepository>,
    llm: Arc<LlmService>,
}

impl RegenerateStagingSuggestions {
    pub fn new(
        location: Arc<Location>,
        character: Arc<CharacterRepository>,
        llm: Arc<LlmService>,
    ) -> Self {
        Self {
            location,
            character,
            llm,
        }
    }

    pub async fn execute(
        &self,
        region_id: RegionId,
        guidance: Option<&str>,
    ) -> Result<Vec<StagedNpc>, StagingError> {
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(StagingError::RegionNotFound)?;

        let location_name = match self.location.get(region.location_id()).await {
            Ok(Some(l)) => l.name().to_string(),
            Ok(None) => {
                tracing::warn!(location_id = %region.location_id(), "Location not found for staging regeneration");
                "Unknown Location".to_string()
            }
            Err(e) => {
                tracing::warn!(location_id = %region.location_id(), error = %e, "Failed to fetch location for staging regeneration");
                "Unknown Location".to_string()
            }
        };

        // Fetch NPCs for region once - fail fast if we can't fetch NPCs
        let npcs_for_region = self.character.get_npcs_for_region(region_id).await?;

        Ok(generate_llm_based_suggestions(
            &npcs_for_region,
            self.llm.as_ref(),
            region.name().as_str(),
            &location_name,
            guidance,
        )
        .await)
    }
}
