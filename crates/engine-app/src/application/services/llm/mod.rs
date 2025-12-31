//! LLM Service - AI-assisted game directing
//!
//! This service provides an interface for generating NPC responses and
//! game narrative content using Large Language Models. It handles:
//!
//! - Building context-aware prompts from game state
//! - Generating NPC dialogue with personality
//! - Parsing tool calls for game mechanics
//! - Providing internal reasoning for the DM to review
//!
//! # Prompt Template Integration
//!
//! The service uses configurable prompt templates that can be overridden
//! via DB (global or per-world), environment variables, or fall back to defaults.

mod prompt_builder;
mod tool_definitions;
mod tool_parser;

// Re-export public types and functions
pub use prompt_builder::{build_conversation_history, build_user_message, PromptBuilder};

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use wrldbldr_domain::value_objects::{DirectorialNotes, GamePromptRequest};
use wrldbldr_engine_ports::outbound::{
    ChatMessage, LlmPort, LlmRequest, MessageRole, PromptTemplateServicePort, ToolCall,
};

use tool_definitions::get_game_tool_definitions;
use tool_parser::parse_tool_calls_from_response;

/// Service for generating AI-powered game responses
///
/// # Example
///
/// ```ignore
/// use wrldbldr_engine::application::services::LLMService;
/// use wrldbldr_engine::infrastructure::ollama::OllamaClient;
///
/// let client = OllamaClient::new("http://localhost:11434/v1", "llama3.2");
/// let prompt_template_service = Arc::new(PromptTemplateService::new(repo));
/// let service = LLMService::new(Arc::new(client), prompt_template_service);
///
/// let request = GamePromptRequest {
///     player_action: PlayerActionContext {
///         action_type: "speak".to_string(),
///         target: Some("Bartender".to_string()),
///         dialogue: Some("What news from the capital?".to_string()),
///     },
///     scene_context: SceneContext {
///         scene_name: "The Rusty Anchor".to_string(),
///         location_name: "Port Valdris".to_string(),
///         time_context: "Late evening".to_string(),
///         present_characters: vec!["Bartender".to_string(), "Mysterious Stranger".to_string()],
///         region_items: vec![],
///     },
///     directorial_notes: "Build tension about the rebellion".to_string(),
///     conversation_history: vec![],
///     responding_character: CharacterContext {
///         name: "Gorm the Bartender".to_string(),
///         archetype: "Gruff but kind-hearted tavern keeper".to_string(),
///         current_mood: Some("Cautious".to_string()),
///         wants: vec!["Protect his establishment".to_string()],
///         relationship_to_player: Some("Acquaintance".to_string()),
///     },
/// };
///
/// let response = service.generate_npc_response(request).await?;
/// ```
pub struct LLMService<L: LlmPort> {
    ollama: Arc<L>,
    prompt_builder: PromptBuilder,
}

impl<L: LlmPort> LLMService<L> {
    /// Create a new LLM service with the provided client and prompt template service
    pub fn new(ollama: Arc<L>, prompt_template_service: Arc<dyn PromptTemplateServicePort>) -> Self {
        Self {
            ollama,
            prompt_builder: PromptBuilder::new(prompt_template_service),
        }
    }

    /// Phase 6 IoC helper: generate an NPC response without requiring callers to
    /// construct an `LLMService` instance.
    pub async fn generate_npc_response_with(
        ollama: Arc<L>,
        prompt_template_service: Arc<dyn PromptTemplateServicePort>,
        request: GamePromptRequest,
    ) -> Result<LLMGameResponse, LLMServiceError> {
        Self::new(ollama, prompt_template_service)
            .generate_npc_response(request)
            .await
    }

    /// Generate an NPC response to a player action
    ///
    /// This method builds a comprehensive prompt from the game context,
    /// sends it to the LLM, and parses the response into a structured format
    /// that includes dialogue, reasoning, and any proposed tool calls.
    pub async fn generate_npc_response(
        &self,
        request: GamePromptRequest,
    ) -> Result<LLMGameResponse, LLMServiceError> {
        // Use the enhanced version with no directorial notes
        self.generate_npc_response_with_direction(request, None)
            .await
    }

    /// Generate an NPC response with comprehensive directorial guidance
    ///
    /// This is the enhanced version that integrates DirectorialNotes for fuller
    /// scene context and more tailored LLM responses. Recommended for complex
    /// scene interactions where pacing, tone, and narrative beats are important.
    ///
    /// # Arguments
    ///
    /// * `request` - The core interaction request
    /// * `directorial_notes` - Optional structured guidance for the LLM about
    ///                         tone, pacing, and narrative direction
    ///
    /// # Example
    ///
    /// ```ignore
    /// use wrldbldr_engine::domain::value_objects::{DirectorialNotes, ToneGuidance, PacingGuidance};
    ///
    /// let notes = DirectorialNotes::new()
    ///     .with_tone(ToneGuidance::Mysterious)
    ///     .with_pacing(PacingGuidance::Slow)
    ///     .with_general_notes("Build suspicion about the stranger");
    ///
    /// let response = service.generate_npc_response_with_direction(request, Some(&notes)).await?;
    /// ```
    pub async fn generate_npc_response_with_direction(
        &self,
        request: GamePromptRequest,
        directorial_notes: Option<&DirectorialNotes>,
    ) -> Result<LLMGameResponse, LLMServiceError> {
        // Build prompt using configurable templates
        // Convert world_id string to WorldId if present
        let world_id = request.world_id.as_ref().and_then(|id| {
            uuid::Uuid::parse_str(id)
                .ok()
                .map(wrldbldr_domain::WorldId::from_uuid)
        });

        let system_prompt = self
            .prompt_builder
            .build_system_prompt_with_notes(
                world_id,
                &request.scene_context,
                &request.responding_character,
                directorial_notes,
                &request.active_challenges,
                &request.active_narrative_events,
            )
            .await;

        // Apply token budget enforcement if configured
        let system_prompt = match &request.context_budget {
            Some(budget_config) => self
                .prompt_builder
                .enforce_budget(system_prompt, budget_config),
            None => system_prompt,
        };

        let user_message = build_user_message(&request);

        let mut messages = build_conversation_history(&request.conversation_history);
        messages.push(ChatMessage {
            role: MessageRole::User,
            content: user_message,
        });

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8); // Slightly creative for roleplay

        let tools = get_game_tool_definitions();

        let response = self
            .ollama
            .generate_with_tools(llm_request, tools)
            .await
            .map_err(|e| LLMServiceError::LlmError(e.to_string()))?;

        self.parse_response(&response.content, &response.tool_calls)
    }

    /// Parse the LLM response into structured components
    fn parse_response(
        &self,
        content: &str,
        tool_calls: &[ToolCall],
    ) -> Result<LLMGameResponse, LLMServiceError> {
        let reasoning = self
            .extract_tag_content(content, "reasoning")
            .unwrap_or_else(|| "No internal reasoning provided.".to_string());

        let dialogue = self
            .extract_tag_content(content, "dialogue")
            .unwrap_or_else(|| {
                // Fallback: if no tags, treat the whole content as dialogue
                content.to_string()
            });

        let suggested_beats = self
            .extract_tag_content(content, "suggested_beats")
            .map(|beats| {
                beats
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let proposed_tool_calls = parse_tool_calls_from_response(tool_calls);

        // Parse challenge suggestion if present
        let challenge_suggestion = self
            .extract_tag_content(content, "challenge_suggestion")
            .and_then(|suggestion_text| {
                serde_json::from_str::<ChallengeSuggestion>(&suggestion_text).ok()
            });

        // Parse narrative event suggestion if present
        let narrative_event_suggestion = self
            .extract_tag_content(content, "narrative_event_suggestion")
            .and_then(|suggestion_text| {
                serde_json::from_str::<NarrativeEventSuggestion>(&suggestion_text).ok()
            });

        // Parse topics from dialogue for persistence/search
        let topics = self
            .extract_tag_content(content, "topics")
            .map(|topics_text| {
                topics_text
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .map(|line| line.trim_start_matches('-').trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(LLMGameResponse {
            npc_dialogue: dialogue.trim().to_string(),
            internal_reasoning: reasoning.trim().to_string(),
            proposed_tool_calls,
            suggested_beats,
            challenge_suggestion,
            narrative_event_suggestion,
            topics,
        })
    }

    /// Extract content between XML-style tags
    fn extract_tag_content(&self, text: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        let start = text.find(&open_tag)?;
        let end = text.find(&close_tag)?;

        if start >= end {
            return None;
        }

        let content_start = start + open_tag.len();
        Some(text[content_start..end].to_string())
    }
}

/// Suggested challenge from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestion {
    /// ID of the suggested challenge
    pub challenge_id: String,
    /// Confidence level of the suggestion
    pub confidence: SuggestionConfidence,
    /// Why the LLM suggests this challenge
    pub reasoning: String,
}

/// Suggested narrative event trigger from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestion {
    /// ID of the suggested narrative event
    pub event_id: String,
    /// Confidence level of the suggestion
    pub confidence: SuggestionConfidence,
    /// Why the LLM suggests triggering this event
    pub reasoning: String,
    /// Which triggers matched based on context
    pub matched_triggers: Vec<String>,
}

/// Confidence level for a challenge suggestion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionConfidence {
    High,
    Medium,
    Low,
}

/// Response from the LLM service for a game prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMGameResponse {
    /// The NPC's dialogue to show to the player
    pub npc_dialogue: String,
    /// Internal reasoning (shown to DM, hidden from player)
    pub internal_reasoning: String,
    /// Proposed game mechanic changes (require DM approval)
    pub proposed_tool_calls: Vec<ProposedToolCall>,
    /// Narrative suggestions for the DM
    pub suggested_beats: Vec<String>,
    /// Optional suggested challenge from the LLM
    pub challenge_suggestion: Option<ChallengeSuggestion>,
    /// Optional suggested narrative event trigger from the LLM
    pub narrative_event_suggestion: Option<NarrativeEventSuggestion>,
    /// Topics discussed in this dialogue exchange (for persistence/search)
    pub topics: Vec<String>,
}

/// A proposed tool call that requires DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolCall {
    /// Name of the tool to call
    pub tool_name: String,
    /// Arguments for the tool call
    pub arguments: serde_json::Value,
    /// Human-readable description of what this will do
    pub description: String,
}

/// Errors that can occur in the LLM service
#[derive(Debug, thiserror::Error)]
pub enum LLMServiceError {
    /// Error from the underlying LLM client
    #[error("LLM error: {0}")]
    LlmError(String),
    /// Error parsing the LLM response
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::PromptTemplateService;
    use wrldbldr_domain::WorldId;
    use wrldbldr_engine_dto::FinishReason;
    use wrldbldr_engine_ports::outbound::{
        EnvironmentPort, LlmResponse, PromptTemplateError, PromptTemplateRepositoryPort,
        PromptTemplateServicePort, ToolDefinition,
    };

    /// Mock environment for tests
    struct MockEnvironmentPort;

    impl EnvironmentPort for MockEnvironmentPort {
        fn get_var(&self, _key: &str) -> Option<String> {
            None
        }
    }

    /// Mock prompt template repository for tests
    struct MockPromptTemplateRepository;

    #[async_trait::async_trait]
    impl PromptTemplateRepositoryPort for MockPromptTemplateRepository {
        async fn get_global(&self, _key: &str) -> Result<Option<String>, PromptTemplateError> {
            Ok(None)
        }
        async fn get_all_global(&self) -> Result<Vec<(String, String)>, PromptTemplateError> {
            Ok(vec![])
        }
        async fn set_global(&self, _key: &str, _value: &str) -> Result<(), PromptTemplateError> {
            Ok(())
        }
        async fn delete_global(&self, _key: &str) -> Result<(), PromptTemplateError> {
            Ok(())
        }
        async fn delete_all_global(&self) -> Result<(), PromptTemplateError> {
            Ok(())
        }
        async fn get_for_world(
            &self,
            _world_id: WorldId,
            _key: &str,
        ) -> Result<Option<String>, PromptTemplateError> {
            Ok(None)
        }
        async fn get_all_for_world(
            &self,
            _world_id: WorldId,
        ) -> Result<Vec<(String, String)>, PromptTemplateError> {
            Ok(vec![])
        }
        async fn set_for_world(
            &self,
            _world_id: WorldId,
            _key: &str,
            _value: &str,
        ) -> Result<(), PromptTemplateError> {
            Ok(())
        }
        async fn delete_for_world(
            &self,
            _world_id: WorldId,
            _key: &str,
        ) -> Result<(), PromptTemplateError> {
            Ok(())
        }
        async fn delete_all_for_world(
            &self,
            _world_id: WorldId,
        ) -> Result<(), PromptTemplateError> {
            Ok(())
        }
    }

    /// Shared mock LLM for tests that don't need actual LLM calls
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmPort for MockLlm {
        type Error = std::io::Error;

        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, Self::Error> {
            Ok(LlmResponse {
                content: String::new(),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }

        async fn generate_with_tools(
            &self,
            _request: LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<LlmResponse, Self::Error> {
            self.generate(_request).await
        }
    }

    fn create_test_service() -> LLMService<MockLlm> {
        let repo: Arc<dyn PromptTemplateRepositoryPort> = Arc::new(MockPromptTemplateRepository);
        let env: Arc<dyn EnvironmentPort> = Arc::new(MockEnvironmentPort);
        let prompt_service: Arc<dyn PromptTemplateServicePort> =
            Arc::new(PromptTemplateService::new(repo, env));
        LLMService::new(Arc::new(MockLlm), prompt_service)
    }

    #[test]
    fn test_extract_tag_content() {
        let service = create_test_service();

        let text = r#"
<reasoning>
This is the reasoning section.
It has multiple lines.
</reasoning>

<dialogue>
Hello, traveler! What brings you here?
</dialogue>
"#;

        let reasoning = service.extract_tag_content(text, "reasoning");
        assert!(reasoning.is_some());
        assert!(reasoning.unwrap().contains("This is the reasoning section"));

        let dialogue = service.extract_tag_content(text, "dialogue");
        assert!(dialogue.is_some());
        assert!(dialogue.unwrap().contains("Hello, traveler"));

        let missing = service.extract_tag_content(text, "missing");
        assert!(missing.is_none());
    }
}
