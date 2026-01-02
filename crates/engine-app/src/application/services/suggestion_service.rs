//! Suggestion Service - LLM-powered content suggestions for world-building
//!
//! This service provides AI-generated suggestions for:
//! - Character names, descriptions, wants, fears, backstories
//! - Location names, descriptions, atmosphere, features
//! - Plot hooks and story connections

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::value_objects::prompt_keys;
use wrldbldr_domain::WorldId;
use crate::application::services::internal::PromptTemplateUseCasePort;
use wrldbldr_engine_ports::outbound::{ChatMessage, LlmPort, LlmRequest, MessageRole};

/// Service for generating content suggestions
pub struct SuggestionService<L: LlmPort> {
    llm: L,
    prompt_template_service: Arc<dyn PromptTemplateUseCasePort>,
}

impl<L: LlmPort> SuggestionService<L> {
    /// Create a new suggestion service
    pub fn new(llm: L, prompt_template_service: Arc<dyn PromptTemplateUseCasePort>) -> Self {
        Self {
            llm,
            prompt_template_service,
        }
    }

    async fn resolve_optional_world_template_with(
        prompt_template_service: &Arc<dyn PromptTemplateUseCasePort>,
        world_id: Option<WorldId>,
        key: &str,
    ) -> String {
        match world_id {
            Some(world_id) => {
                prompt_template_service
                    .resolve_for_world_with_source(world_id, key)
                    .await
                    .value
            }
            None => prompt_template_service.resolve_with_source(key).await.value,
        }
    }

    async fn generate_list_with(
        llm: &L,
        prompt: &str,
        expected_count: usize,
    ) -> Result<Vec<String>> {
        let request = LlmRequest::new(vec![ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        }])
        .with_temperature(0.9) // High creativity for suggestions
        .with_max_tokens(Some(1000));

        let response = llm
            .generate(request)
            .await
            .map_err(|e| anyhow::anyhow!("LLM error: {}", e))?;

        // Parse the response into individual suggestions
        let suggestions: Vec<String> = response
            .content
            .split('\n')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                // Remove common list prefixes
                let s = s.trim_start_matches(|c: char| {
                    c.is_numeric() || c == '.' || c == ')' || c == '-'
                });
                s.trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .take(expected_count)
            .collect();

        Ok(suggestions)
    }

    /// Generate suggestions for a given field type without constructing a SuggestionService.
    ///
    /// This is intended for orchestrators (e.g., queue workers) that shouldn't call
    /// `SuggestionService::new()` internally (Phase 6 IoC cleanup).
    pub async fn suggest_field_with(
        llm: L,
        prompt_template_service: Arc<dyn PromptTemplateUseCasePort>,
        field_type: &str,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let (prompt_key, expected_count) = match field_type {
            "character_name" => (prompt_keys::SUGGESTION_CHARACTER_NAME, 5),
            "character_description" => (prompt_keys::SUGGESTION_CHARACTER_DESCRIPTION, 3),
            "character_wants" => (prompt_keys::SUGGESTION_CHARACTER_WANTS, 4),
            "character_fears" => (prompt_keys::SUGGESTION_CHARACTER_FEARS, 4),
            "character_backstory" => (prompt_keys::SUGGESTION_CHARACTER_BACKSTORY, 2),
            "location_name" => (prompt_keys::SUGGESTION_LOCATION_NAME, 5),
            "location_description" => (prompt_keys::SUGGESTION_LOCATION_DESCRIPTION, 3),
            "location_atmosphere" => (prompt_keys::SUGGESTION_LOCATION_ATMOSPHERE, 4),
            "location_features" => (prompt_keys::SUGGESTION_LOCATION_FEATURES, 5),
            "location_secrets" => (prompt_keys::SUGGESTION_LOCATION_SECRETS, 3),
            // Actantial Model suggestions
            "deflection_behavior" => (prompt_keys::SUGGESTION_DEFLECTION_BEHAVIOR, 3),
            "behavioral_tells" => (prompt_keys::SUGGESTION_BEHAVIORAL_TELLS, 3),
            "want_description" => (prompt_keys::SUGGESTION_WANT_DESCRIPTION, 3),
            "actantial_reason" => (prompt_keys::SUGGESTION_ACTANTIAL_REASON, 3),
            other => {
                return Err(anyhow::anyhow!("Unknown suggestion field type: {}", other));
            }
        };

        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = Self::resolve_optional_world_template_with(
            &prompt_template_service,
            world_id,
            prompt_key,
        )
        .await;
        let prompt = Self::apply_placeholders(&template, context);

        Self::generate_list_with(&llm, &prompt, expected_count).await
    }

    async fn resolve_optional_world_template(
        &self,
        world_id: Option<WorldId>,
        key: &str,
    ) -> String {
        Self::resolve_optional_world_template_with(&self.prompt_template_service, world_id, key)
            .await
    }

    /// Parse world_id from optional string
    fn parse_world_id(world_id: Option<&String>) -> Option<WorldId> {
        world_id.and_then(|id| uuid::Uuid::parse_str(id).ok().map(WorldId::from_uuid))
    }

    /// Apply placeholder substitutions to a template
    fn apply_placeholders(template: &str, context: &SuggestionContext) -> String {
        template
            .replace(
                "{entity_type}",
                context.entity_type.as_deref().unwrap_or("fantasy"),
            )
            .replace(
                "{entity_name}",
                context.entity_name.as_deref().unwrap_or("this entity"),
            )
            .replace(
                "{world_setting}",
                context.world_setting.as_deref().unwrap_or("fantasy"),
            )
            .replace("{hints}", context.hints.as_deref().unwrap_or(""))
            .replace(
                "{additional_context}",
                context.additional_context.as_deref().unwrap_or(""),
            )
    }

    /// Generate character name suggestions
    pub async fn suggest_character_names(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_CHARACTER_NAME)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 5).await
    }

    /// Generate character description suggestions
    pub async fn suggest_character_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(
                world_id,
                prompt_keys::SUGGESTION_CHARACTER_DESCRIPTION,
            )
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Generate character wants/desires suggestions
    pub async fn suggest_character_wants(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_CHARACTER_WANTS)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 4).await
    }

    /// Generate character fears suggestions
    pub async fn suggest_character_fears(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_CHARACTER_FEARS)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 4).await
    }

    /// Generate character backstory suggestions
    pub async fn suggest_character_backstory(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_CHARACTER_BACKSTORY)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 2).await
    }

    /// Generate location name suggestions
    pub async fn suggest_location_names(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_LOCATION_NAME)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 5).await
    }

    /// Generate location description suggestions
    pub async fn suggest_location_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_LOCATION_DESCRIPTION)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Generate location atmosphere suggestions
    pub async fn suggest_location_atmosphere(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_LOCATION_ATMOSPHERE)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 4).await
    }

    /// Generate location notable features
    pub async fn suggest_location_features(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_LOCATION_FEATURES)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 5).await
    }

    /// Generate location hidden secrets
    pub async fn suggest_location_secrets(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_LOCATION_SECRETS)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    // === Actantial Model Suggestion Methods ===

    /// Generate deflection behavior suggestions for an NPC hiding their wants
    ///
    /// Context expectations:
    /// - entity_name: NPC name
    /// - hints: The want being hidden
    /// - world_setting: Campaign setting
    /// - additional_context: Character description/archetype
    pub async fn suggest_deflection_behavior(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_DEFLECTION_BEHAVIOR)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Generate behavioral tells that reveal a hidden want
    ///
    /// Context expectations:
    /// - entity_name: NPC name
    /// - hints: The want being hidden
    /// - world_setting: Campaign setting
    /// - additional_context: Character description/archetype
    pub async fn suggest_behavioral_tells(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_BEHAVIORAL_TELLS)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Generate want description suggestions (actantial-aware)
    ///
    /// Context expectations:
    /// - entity_name: NPC name
    /// - hints: Character archetype
    /// - world_setting: Campaign setting
    /// - additional_context: Other relevant character info
    pub async fn suggest_want_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_WANT_DESCRIPTION)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Generate reasons for actantial relationships
    ///
    /// Context expectations:
    /// - entity_name: NPC who holds this view
    /// - hints: Target of the actantial relationship (the actor)
    /// - additional_context: The actantial role (e.g., "a helper", "an opponent")
    /// - world_setting: Campaign setting
    pub async fn suggest_actantial_reason(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self
            .resolve_optional_world_template(world_id, prompt_keys::SUGGESTION_ACTANTIAL_REASON)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Internal method to generate a list of suggestions
    async fn generate_list(&self, prompt: &str, expected_count: usize) -> Result<Vec<String>> {
        Self::generate_list_with(&self.llm, prompt, expected_count).await
    }
}

// Re-export SuggestionContext from engine-dto (single source of truth)
pub use wrldbldr_engine_dto::SuggestionContext;

/// Request for a suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionRequest {
    /// Type of suggestion to generate
    pub suggestion_type: SuggestionType,
    /// Context for the suggestion
    pub context: SuggestionContext,
}

/// Types of suggestions available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    CharacterName,
    CharacterDescription,
    CharacterWants,
    CharacterFears,
    CharacterBackstory,
    LocationName,
    LocationDescription,
    LocationAtmosphere,
    LocationFeatures,
    LocationSecrets,
    // Actantial Model suggestions
    DeflectionBehavior,
    BehavioralTells,
    WantDescription,
    ActantialReason,
}

impl SuggestionType {
    /// Convert to the field type string used in LLM queue routing
    pub fn to_field_type(&self) -> &'static str {
        match self {
            SuggestionType::CharacterName => "character_name",
            SuggestionType::CharacterDescription => "character_description",
            SuggestionType::CharacterWants => "character_wants",
            SuggestionType::CharacterFears => "character_fears",
            SuggestionType::CharacterBackstory => "character_backstory",
            SuggestionType::LocationName => "location_name",
            SuggestionType::LocationDescription => "location_description",
            SuggestionType::LocationAtmosphere => "location_atmosphere",
            SuggestionType::LocationFeatures => "location_features",
            SuggestionType::LocationSecrets => "location_secrets",
            // Actantial Model suggestions
            SuggestionType::DeflectionBehavior => "deflection_behavior",
            SuggestionType::BehavioralTells => "behavioral_tells",
            SuggestionType::WantDescription => "want_description",
            SuggestionType::ActantialReason => "actantial_reason",
        }
    }
}

/// Response containing suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResponse {
    /// The type of suggestion that was generated
    pub suggestion_type: SuggestionType,
    /// The generated suggestions
    pub suggestions: Vec<String>,
}
