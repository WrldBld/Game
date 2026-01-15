//! Regenerate staging suggestions use case.

use std::sync::Arc;

use wrldbldr_domain::RegionId;

use crate::repositories::character::Character;
use crate::repositories::location::Location;
use crate::repositories::Llm;

use super::suggestions::generate_llm_based_suggestions;
use super::types::StagedNpc;
use super::StagingError;

/// Use case for regenerating LLM staging suggestions.
pub struct RegenerateStagingSuggestions {
    location: Arc<Location>,
    character: Arc<Character>,
    llm: Arc<Llm>,
}

impl RegenerateStagingSuggestions {
    pub fn new(location: Arc<Location>, character: Arc<Character>, llm: Arc<Llm>) -> Self {
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

        let location_name = self
            .location
            .get(region.location_id)
            .await
            .ok()
            .flatten()
            .map(|l| l.name().to_string())
            .unwrap_or_else(|| "Unknown Location".to_string());

        // Fetch NPCs for region once
        let npcs_for_region = self
            .character
            .get_npcs_for_region(region_id)
            .await
            .unwrap_or_default();

        Ok(generate_llm_based_suggestions(
            &npcs_for_region,
            self.llm.as_ref(),
            &region.name,
            &location_name,
            guidance,
        )
        .await)
    }
}
