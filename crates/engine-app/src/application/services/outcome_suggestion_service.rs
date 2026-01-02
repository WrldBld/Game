//! Outcome Suggestion Service (P3.3)
//!
//! Provides LLM-powered suggestions for challenge outcome descriptions.
//! Used when DM requests alternative outcome text.

use std::sync::Arc;
use uuid::Uuid;

use crate::application::dto::{OutcomeBranchDto, OutcomeSuggestionRequest};
use wrldbldr_domain::value_objects::prompt_keys;
use wrldbldr_domain::WorldId;
use crate::application::services::internal::PromptTemplateUseCasePort;
use wrldbldr_engine_ports::outbound::LlmPort;

/// Error type for outcome suggestion operations
#[derive(Debug, thiserror::Error)]
pub enum SuggestionError {
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Service for generating LLM-powered outcome suggestions
pub struct OutcomeSuggestionService<L: LlmPort> {
    llm: Arc<L>,
    prompt_template_service: Arc<dyn PromptTemplateUseCasePort>,
}

impl<L: LlmPort> OutcomeSuggestionService<L> {
    /// Create a new outcome suggestion service
    pub fn new(llm: Arc<L>, prompt_template_service: Arc<dyn PromptTemplateUseCasePort>) -> Self {
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

    /// Parse world_id from optional string
    fn parse_world_id(world_id: Option<&String>) -> Option<WorldId> {
        world_id.and_then(|id| uuid::Uuid::parse_str(id).ok().map(WorldId::from_uuid))
    }

    /// Generate alternative outcome descriptions
    ///
    /// Returns 3 alternative descriptions for the given outcome tier.
    pub async fn generate_suggestions(
        &self,
        request: &OutcomeSuggestionRequest,
    ) -> Result<Vec<String>, SuggestionError> {
        Self::generate_suggestions_with(
            self.llm.clone(),
            self.prompt_template_service.clone(),
            request,
        )
        .await
    }

    pub async fn generate_suggestions_with(
        llm: Arc<L>,
        prompt_template_service: Arc<dyn PromptTemplateUseCasePort>,
        request: &OutcomeSuggestionRequest,
    ) -> Result<Vec<String>, SuggestionError> {
        let world_id = Self::parse_world_id(request.world_id.as_ref());
        let system_prompt =
            Self::build_system_prompt_with(&prompt_template_service, world_id).await;
        let user_prompt = Self::build_user_prompt_static(request);

        use wrldbldr_engine_ports::outbound::{ChatMessage, LlmRequest};

        let messages = vec![ChatMessage::user(user_prompt)];

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8) // Higher temperature for creativity
            .with_max_tokens(Some(500));

        let response = llm
            .generate(llm_request)
            .await
            .map_err(|e| SuggestionError::LlmError(format!("{:?}", e)))?;

        Self::parse_suggestions_content(&response.content)
    }

    /// Generate outcome branches with structured data
    ///
    /// Returns N branches based on the configured branch count, each with
    /// a unique ID, title, and description for DM selection.
    ///
    /// # Arguments
    /// * `request` - The suggestion request context
    /// * `branch_count` - Number of branches to generate
    /// * `tokens_per_branch` - Max tokens per branch for LLM (from settings, default 200)
    pub async fn generate_branches(
        &self,
        request: &OutcomeSuggestionRequest,
        branch_count: usize,
        tokens_per_branch: u32,
    ) -> Result<Vec<OutcomeBranchDto>, SuggestionError> {
        Self::generate_branches_with(
            self.llm.clone(),
            self.prompt_template_service.clone(),
            request,
            branch_count,
            tokens_per_branch,
        )
        .await
    }

    pub async fn generate_branches_with(
        llm: Arc<L>,
        prompt_template_service: Arc<dyn PromptTemplateUseCasePort>,
        request: &OutcomeSuggestionRequest,
        branch_count: usize,
        tokens_per_branch: u32,
    ) -> Result<Vec<OutcomeBranchDto>, SuggestionError> {
        let world_id = Self::parse_world_id(request.world_id.as_ref());
        let system_prompt =
            Self::build_branch_system_prompt_with(&prompt_template_service, world_id, branch_count)
                .await;
        let user_prompt = Self::build_branch_user_prompt_static(request, branch_count);

        use wrldbldr_engine_ports::outbound::{ChatMessage, LlmRequest};

        let messages = vec![ChatMessage::user(user_prompt)];

        // Calculate max tokens based on branch count and configurable tokens per branch
        let max_tokens = tokens_per_branch * branch_count as u32;

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8)
            .with_max_tokens(Some(max_tokens));

        let response = llm
            .generate(llm_request)
            .await
            .map_err(|e| SuggestionError::LlmError(format!("{:?}", e)))?;

        Self::parse_branches_content(&response.content, branch_count)
    }

    async fn build_system_prompt_with(
        prompt_template_service: &Arc<dyn PromptTemplateUseCasePort>,
        world_id: Option<WorldId>,
    ) -> String {
        Self::resolve_optional_world_template_with(
            prompt_template_service,
            world_id,
            prompt_keys::OUTCOME_SYSTEM_PROMPT,
        )
        .await
    }

    async fn build_branch_system_prompt_with(
        prompt_template_service: &Arc<dyn PromptTemplateUseCasePort>,
        world_id: Option<WorldId>,
        branch_count: usize,
    ) -> String {
        let template = Self::resolve_optional_world_template_with(
            prompt_template_service,
            world_id,
            prompt_keys::OUTCOME_BRANCH_SYSTEM_PROMPT,
        )
        .await;

        template.replace("{branch_count}", &branch_count.to_string())
    }

    fn build_user_prompt_static(request: &OutcomeSuggestionRequest) -> String {
        let mut prompt = format!(
            "Generate 3 alternative {} outcome descriptions for:\n\n\
            Challenge: {}\n\
            Description: {}\n\
            Skill: {}\n\
            Roll Context: {}",
            request.outcome_type,
            request.challenge_name,
            request.challenge_description,
            request.skill_name,
            request.roll_context,
        );

        if let Some(guidance) = &request.guidance {
            prompt.push_str(&format!("\n\nDM Guidance: {}", guidance));
        }

        if let Some(context) = &request.narrative_context {
            prompt.push_str(&format!("\n\nNarrative Context: {}", context));
        }

        prompt.push_str("\n\nProvide 3 distinct suggestions, one per line:");

        prompt
    }

    fn build_branch_user_prompt_static(
        request: &OutcomeSuggestionRequest,
        branch_count: usize,
    ) -> String {
        let mut prompt = format!(
            "Generate {} distinct {} outcome branches for:\n\n\
            Challenge: {}\n\
            Description: {}\n\
            Skill: {}\n\
            Roll Context: {}",
            branch_count,
            request.outcome_type,
            request.challenge_name,
            request.challenge_description,
            request.skill_name,
            request.roll_context,
        );

        if let Some(guidance) = &request.guidance {
            prompt.push_str(&format!("\n\nDM Guidance: {}", guidance));
        }

        if let Some(context) = &request.narrative_context {
            prompt.push_str(&format!("\n\nNarrative Context: {}", context));
        }

        prompt.push_str(&format!(
            "\n\nProvide {} branches using the TITLE:/DESCRIPTION: format:",
            branch_count
        ));

        prompt
    }

    fn parse_suggestions_content(content: &str) -> Result<Vec<String>, SuggestionError> {
        let suggestions: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            // Filter out numbered prefixes like "1." or "1)"
            .map(|line| {
                let trimmed = line.trim_start_matches(|c: char| {
                    c.is_numeric() || c == '.' || c == ')' || c == ' '
                });
                trimmed.trim().to_string()
            })
            .filter(|line| !line.is_empty())
            .take(3)
            .collect();

        if suggestions.is_empty() {
            return Err(SuggestionError::ParseError(
                "No valid suggestions in LLM response".to_string(),
            ));
        }

        Ok(suggestions)
    }

    fn parse_branches_content(
        content: &str,
        expected_count: usize,
    ) -> Result<Vec<OutcomeBranchDto>, SuggestionError> {
        let mut branches = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_description: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                // End of a branch block - save if we have both parts
                if let (Some(title), Some(desc)) =
                    (current_title.take(), current_description.take())
                {
                    branches.push(OutcomeBranchDto::new(
                        Uuid::new_v4().to_string(),
                        title,
                        desc,
                    ));
                }
                continue;
            }

            // Check for TITLE: prefix (case insensitive)
            if let Some(title) = line
                .strip_prefix("TITLE:")
                .or_else(|| line.strip_prefix("Title:"))
            {
                current_title = Some(title.trim().to_string());
            }
            // Check for DESCRIPTION: prefix (case insensitive)
            else if let Some(desc) = line
                .strip_prefix("DESCRIPTION:")
                .or_else(|| line.strip_prefix("Description:"))
            {
                current_description = Some(desc.trim().to_string());
            }
            // If we have a title but no description, and this line doesn't have a prefix,
            // treat it as a continuation of the description
            else if current_title.is_some() && current_description.is_none() {
                // Might be a description without prefix
                current_description = Some(line.to_string());
            }
            // Continuation of description
            else if current_description.is_some() {
                if let Some(ref mut desc) = current_description {
                    desc.push(' ');
                    desc.push_str(line);
                }
            }
        }

        // Don't forget the last branch if content doesn't end with blank line
        if let (Some(title), Some(desc)) = (current_title.take(), current_description.take()) {
            branches.push(OutcomeBranchDto::new(
                Uuid::new_v4().to_string(),
                title,
                desc,
            ));
        }

        // If parsing failed, fall back to line-by-line and create generic titles
        if branches.is_empty() {
            let lines: Vec<&str> = content
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();

            for (i, line) in lines.into_iter().take(expected_count).enumerate() {
                // Strip any leading numbering
                let clean = line
                    .trim_start_matches(|c: char| {
                        c.is_numeric() || c == '.' || c == ')' || c == ' '
                    })
                    .trim();

                // Generate a title from first few words
                let words: Vec<&str> = clean.split_whitespace().take(5).collect();
                let title = words.join(" ");

                branches.push(OutcomeBranchDto::new(
                    Uuid::new_v4().to_string(),
                    format!("Option {}: {}", i + 1, title),
                    clean.to_string(),
                ));
            }
        }

        if branches.is_empty() {
            return Err(SuggestionError::ParseError(
                "No valid branches in LLM response".to_string(),
            ));
        }

        Ok(branches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::PromptTemplateService;
use crate::application::services::internal::PromptTemplateUseCasePort;
    use wrldbldr_engine_ports::outbound::{
        EnvironmentPort, FinishReason, PromptTemplateCachePort, PromptTemplateError,
        PromptTemplateRepositoryPort, ResolvedPromptTemplate, ToolDefinition,
    };

    struct NoopPromptTemplateCache;

    #[async_trait::async_trait]
    impl PromptTemplateCachePort for NoopPromptTemplateCache {
        async fn get_global(&self, _key: &str) -> Option<ResolvedPromptTemplate> {
            None
        }

        async fn set_global(&self, _key: String, _resolved: ResolvedPromptTemplate) {}

        async fn remove_global(&self, _key: &str) {}

        async fn clear_global(&self) {}

        async fn get_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
            _key: &str,
        ) -> Option<ResolvedPromptTemplate> {
            None
        }

        async fn set_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
            _key: String,
            _resolved: ResolvedPromptTemplate,
        ) {
        }

        async fn remove_for_world(&self, _world_id: wrldbldr_domain::WorldId, _key: &str) {}

        async fn clear_world(&self) {}

        async fn remove_world(&self, _world_id: wrldbldr_domain::WorldId) {}
    }

    /// Mock environment for tests
    struct MockEnvironmentPort;

    impl EnvironmentPort for MockEnvironmentPort {
        fn get_var(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn create_test_service() -> OutcomeSuggestionService<MockLlm> {
        let mock_repo: Arc<dyn PromptTemplateRepositoryPort> =
            Arc::new(MockPromptTemplateRepository);
        let mock_env: Arc<dyn EnvironmentPort> = Arc::new(MockEnvironmentPort);
        let cache: Arc<dyn PromptTemplateCachePort> = Arc::new(NoopPromptTemplateCache);
        let prompt_template_service: Arc<dyn PromptTemplateUseCasePort> =
            Arc::new(PromptTemplateService::new(mock_repo, mock_env, cache));
        OutcomeSuggestionService {
            llm: Arc::new(MockLlm),
            prompt_template_service,
        }
    }

    #[test]
    fn test_parse_suggestions_simple() {
        let content = "You succeed with flying colors!\nThe guard barely notices you slip past.\nYour skills prove more than adequate.";
        let result =
            OutcomeSuggestionService::<MockLlm>::parse_suggestions_content(content).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "You succeed with flying colors!");
    }

    #[test]
    fn test_parse_suggestions_numbered() {
        let content = "1. You succeed with flying colors!\n2. The guard barely notices you slip past.\n3. Your skills prove more than adequate.";
        let result =
            OutcomeSuggestionService::<MockLlm>::parse_suggestions_content(content).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "You succeed with flying colors!");
    }

    #[test]
    fn test_parse_branches_structured() {
        let content = "TITLE: Swift Success\nDESCRIPTION: You move with precision and grace.\n\nTITLE: Lucky Break\nDESCRIPTION: Fortune favors you today.";
        let result =
            OutcomeSuggestionService::<MockLlm>::parse_branches_content(content, 2).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "Swift Success");
        assert_eq!(result[0].description, "You move with precision and grace.");
        assert_eq!(result[1].title, "Lucky Break");
    }

    #[test]
    fn test_parse_branches_fallback() {
        // Unstructured input - should fall back to line-by-line
        let content = "You succeed brilliantly!\nThe task is completed with ease.";
        let result =
            OutcomeSuggestionService::<MockLlm>::parse_branches_content(content, 2).unwrap();

        assert_eq!(result.len(), 2);
        // Should have generated titles from content
        assert!(result[0].title.starts_with("Option 1:"));
    }

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

    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmPort for MockLlm {
        type Error = std::convert::Infallible;

        async fn generate(
            &self,
            _request: wrldbldr_engine_ports::outbound::LlmRequest,
        ) -> Result<wrldbldr_engine_ports::outbound::LlmResponse, Self::Error> {
            Ok(wrldbldr_engine_ports::outbound::LlmResponse {
                content: "Mock response".to_string(),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }

        async fn generate_with_tools(
            &self,
            _request: wrldbldr_engine_ports::outbound::LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<wrldbldr_engine_ports::outbound::LlmResponse, Self::Error> {
            Ok(wrldbldr_engine_ports::outbound::LlmResponse {
                content: "Mock response".to_string(),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }
    }
}
