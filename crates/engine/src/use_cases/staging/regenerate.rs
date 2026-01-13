//! Regenerate staging suggestions use case.

use std::sync::Arc;

use wrldbldr_domain::RegionId;

use crate::entities::character::Character;
use crate::entities::location::Location;
use crate::infrastructure::ports::LlmPort;

use super::suggestions::generate_llm_based_suggestions;
use super::types::StagedNpc;
use super::StagingError;

/// Use case for regenerating LLM staging suggestions.
pub struct RegenerateStagingSuggestions {
    location: Arc<Location>,
    character: Arc<Character>,
    llm: Arc<dyn LlmPort>,
}

impl RegenerateStagingSuggestions {
    pub fn new(location: Arc<Location>, character: Arc<Character>, llm: Arc<dyn LlmPort>) -> Self {
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
            .map(|l| l.name)
            .unwrap_or_else(|| "Unknown Location".to_string());

        Ok(generate_llm_based_suggestions(
            &self.character,
            self.llm.as_ref(),
            region_id,
            &region.name,
            &location_name,
            guidance,
        )
        .await)
    }
}
