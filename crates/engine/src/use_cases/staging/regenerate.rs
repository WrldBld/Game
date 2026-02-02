//! Regenerate staging suggestions use case.

use std::sync::Arc;

use wrldbldr_domain::{RegionId, WorldId};

use crate::infrastructure::ports::{CharacterRepo, LlmPort, LocationRepo, PromptTemplateRepo, SettingsRepo};
use crate::infrastructure::app_settings::AppSettings;

use super::suggestions::generate_llm_based_suggestions;
use super::types::StagedNpc;
use super::StagingError;

/// Use case for regenerating LLM staging suggestions.
pub struct RegenerateStagingSuggestions {
    location: Arc<dyn LocationRepo>,
    character: Arc<dyn CharacterRepo>,
    llm: Arc<dyn LlmPort>,
    settings: Arc<dyn SettingsRepo>,
    prompt_templates: Arc<dyn PromptTemplateRepo>,
}

impl RegenerateStagingSuggestions {
    pub fn new(
        location: Arc<dyn LocationRepo>,
        character: Arc<dyn CharacterRepo>,
        llm: Arc<dyn LlmPort>,
        settings: Arc<dyn SettingsRepo>,
        prompt_templates: Arc<dyn PromptTemplateRepo>,
    ) -> Self {
        Self {
            location,
            character,
            llm,
            settings,
            prompt_templates,
        }
    }

    pub async fn execute(
        &self,
        region_id: RegionId,
        world_id: WorldId,
        guidance: Option<&str>,
    ) -> Result<Vec<StagedNpc>, StagingError> {
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(StagingError::RegionNotFound(region_id))?;

        let location_name = match self.location.get_location(region.location_id()).await {
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

        // Resolve temperature from settings
        let temperature = self.resolve_temperature(world_id).await;

        // Load system prompt from template repository
        let system_prompt = self
            .prompt_templates
            .resolve_template(Some(world_id), crate::prompt_templates::keys::SYSTEM_STAGING_SUGGESTIONS)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();

        Ok(generate_llm_based_suggestions(
            &npcs_for_region,
            self.llm.as_ref(),
            region.name().as_str(),
            &location_name,
            guidance,
            temperature,
            &system_prompt,
        )
        .await)
    }

    /// Resolve temperature for staging suggestions.
    ///
    /// Falls back through: per-world setting -> global setting -> default.
    async fn resolve_temperature(&self, world_id: wrldbldr_domain::WorldId) -> f32 {
        // Try per-world setting first
        if let Ok(Some(settings)) = self.settings.get_for_world(world_id).await {
            return settings.llm_temperature_staging();
        }

        // Fall back to global setting
        if let Ok(Some(settings)) = self.settings.get_global().await {
            return settings.llm_temperature_staging();
        }

        // Final fallback to default
        AppSettings::default().llm_temperature_staging()
    }
}
