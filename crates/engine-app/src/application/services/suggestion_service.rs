//! Suggestion Service - LLM-powered content suggestions for world-building
//!
//! This service provides AI-generated suggestions for:
//! - Character names, descriptions, wants, fears, backstories
//! - Location names, descriptions, atmosphere, features
//! - Plot hooks and story connections

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::application::services::PromptTemplateService;
use wrldbldr_domain::value_objects::prompt_keys;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{ChatMessage, LlmPort, LlmRequest, MessageRole};

/// Service for generating content suggestions
pub struct SuggestionService<L: LlmPort> {
    llm: L,
    prompt_template_service: Arc<PromptTemplateService>,
}

impl<L: LlmPort> SuggestionService<L> {
    /// Create a new suggestion service
    pub fn new(llm: L, prompt_template_service: Arc<PromptTemplateService>) -> Self {
        Self { llm, prompt_template_service }
    }
    
    /// Parse world_id from optional string
    fn parse_world_id(world_id: Option<&String>) -> Option<WorldId> {
        world_id.and_then(|id| {
            uuid::Uuid::parse_str(id).ok().map(WorldId::from_uuid)
        })
    }
    
    /// Apply placeholder substitutions to a template
    fn apply_placeholders(template: &str, context: &SuggestionContext) -> String {
        template
            .replace("{entity_type}", context.entity_type.as_deref().unwrap_or("fantasy"))
            .replace("{entity_name}", context.entity_name.as_deref().unwrap_or("this entity"))
            .replace("{world_setting}", context.world_setting.as_deref().unwrap_or("fantasy"))
            .replace("{hints}", context.hints.as_deref().unwrap_or(""))
            .replace("{additional_context}", context.additional_context.as_deref().unwrap_or(""))
    }

    /// Generate character name suggestions
    pub async fn suggest_character_names(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_CHARACTER_NAME)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_CHARACTER_DESCRIPTION)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Generate character wants/desires suggestions
    pub async fn suggest_character_wants(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_CHARACTER_WANTS)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 4).await
    }

    /// Generate character fears suggestions
    pub async fn suggest_character_fears(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_CHARACTER_FEARS)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_CHARACTER_BACKSTORY)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 2).await
    }

    /// Generate location name suggestions
    pub async fn suggest_location_names(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let world_id = Self::parse_world_id(context.world_id.as_ref());
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_LOCATION_NAME)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_LOCATION_DESCRIPTION)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_LOCATION_ATMOSPHERE)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_LOCATION_FEATURES)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_LOCATION_SECRETS)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_DEFLECTION_BEHAVIOR)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_BEHAVIORAL_TELLS)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_WANT_DESCRIPTION)
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
        let template = self.prompt_template_service
            .resolve_optional_world(world_id.as_ref(), prompt_keys::SUGGESTION_ACTANTIAL_REASON)
            .await;
        let prompt = Self::apply_placeholders(&template, context);
        self.generate_list(&prompt, 3).await
    }

    /// Internal method to generate a list of suggestions
    async fn generate_list(&self, prompt: &str, expected_count: usize) -> Result<Vec<String>> {
        let request = LlmRequest::new(vec![ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        }])
        .with_temperature(0.9) // High creativity for suggestions
        .with_max_tokens(Some(1000));

        let response = self
            .llm
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
                let s = s
                    .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == '-');
                s.trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .take(expected_count)
            .collect();

        Ok(suggestions)
    }
}

/// Context for generating suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionContext {
    /// Type of entity (e.g., "character", "location", "tavern", "forest")
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    pub entity_name: Option<String>,
    /// World/setting name or type
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    pub hints: Option<String>,
    /// Additional context from other fields
    pub additional_context: Option<String>,
    /// World ID for per-world template resolution
    #[serde(default)]
    pub world_id: Option<String>,
}

impl Default for SuggestionContext {
    fn default() -> Self {
        Self {
            entity_type: None,
            entity_name: None,
            world_setting: Some("fantasy".to_string()),
            hints: None,
            additional_context: None,
            world_id: None,
        }
    }
}

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
