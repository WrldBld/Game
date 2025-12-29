//! Prompt building functions for LLM requests
//!
//! # Prompt Template Integration
//!
//! The `PromptBuilder` struct provides async methods that resolve configurable
//! prompt templates from DB/env/defaults before building prompts.
//!
//! # Token Budget Enforcement
//!
//! The builder supports optional token budget enforcement via `ContextBudgetConfig`.
//! When a budget config is provided, the prompt is checked against the total budget
//! and truncated if necessary, with logging of truncation events.
//!
//! Budget enforcement can be applied via `enforce_budget()` after building a prompt.

use std::sync::Arc;

use tracing::{debug, warn};
use wrldbldr_domain::value_objects::{
    prompt_keys, ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext,
    ContextBudgetConfig, ConversationTurn, DirectorialNotes, GamePromptRequest, MotivationEntry,
    MotivationsContext, SceneContext, SecretMotivationEntry, SocialStanceContext, TokenCounter,
};
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{ChatMessage, MessageRole};

use crate::application::services::PromptTemplateService;

/// Prompt builder with configurable template support
///
/// Resolves prompt templates from the `PromptTemplateService` with priority:
/// World DB → Global DB → Env → Default
pub struct PromptBuilder {
    prompt_template_service: Arc<PromptTemplateService>,
}

impl PromptBuilder {
    /// Create a new prompt builder
    pub fn new(prompt_template_service: Arc<PromptTemplateService>) -> Self {
        Self {
            prompt_template_service,
        }
    }

    /// Resolve a template, using world-specific resolution if world_id is provided
    async fn resolve(&self, world_id: Option<WorldId>, key: &str) -> String {
        match world_id {
            Some(wid) => {
                self.prompt_template_service
                    .resolve_for_world(wid, key)
                    .await
            }
            None => self.prompt_template_service.resolve(key).await,
        }
    }

    /// Enforce token budget on a prompt, truncating if necessary
    ///
    /// This method checks the prompt against the total budget from `ContextBudgetConfig`
    /// and truncates the prompt if it exceeds the budget. Truncation events are logged.
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to enforce budget on
    /// * `budget_config` - The budget configuration containing total_budget_tokens
    ///
    /// # Returns
    /// The (possibly truncated) prompt string
    pub fn enforce_budget(&self, prompt: String, budget_config: &ContextBudgetConfig) -> String {
        let counter = TokenCounter::default();
        let token_count = counter.count(&prompt);
        let budget = budget_config.total_budget_tokens;

        if token_count <= budget {
            debug!(
                tokens = token_count,
                budget = budget,
                "Prompt within token budget"
            );
            return prompt;
        }

        // Prompt exceeds budget - truncate
        let (truncated, _) = counter.truncate_to_budget(&prompt, budget);
        let truncated_tokens = counter.count(&truncated);

        warn!(
            original_tokens = token_count,
            truncated_tokens = truncated_tokens,
            budget = budget,
            chars_removed = prompt.len() - truncated.len(),
            "Prompt exceeded token budget and was truncated"
        );

        truncated
    }

    /// Build the system prompt that establishes the NPC's personality and context
    pub async fn build_system_prompt(
        &self,
        world_id: Option<WorldId>,
        context: &SceneContext,
        character: &CharacterContext,
    ) -> String {
        self.build_system_prompt_with_notes(world_id, context, character, None, &[], &[])
            .await
    }

    /// Build system prompt with optional directorial notes
    ///
    /// This enhanced version integrates DirectorialNotes for better LLM guidance
    /// on tone, pacing, and scene-specific guidance.
    pub async fn build_system_prompt_with_notes(
        &self,
        world_id: Option<WorldId>,
        context: &SceneContext,
        character: &CharacterContext,
        directorial_notes: Option<&DirectorialNotes>,
        active_challenges: &[ActiveChallengeContext],
        active_narrative_events: &[ActiveNarrativeEventContext],
    ) -> String {
        // Resolve templates
        let response_format = self
            .resolve(world_id, prompt_keys::DIALOGUE_RESPONSE_FORMAT)
            .await;
        let challenge_format = self
            .resolve(world_id, prompt_keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT)
            .await;
        let narrative_event_format = self
            .resolve(world_id, prompt_keys::DIALOGUE_NARRATIVE_EVENT_FORMAT)
            .await;

        let mut prompt = String::new();

        // Role establishment
        prompt.push_str(&format!(
            "You are roleplaying as {}, a {}.\n\n",
            character.name, character.archetype
        ));

        // Scene context
        prompt.push_str(&format!("CURRENT SCENE: {}\n", context.scene_name));
        prompt.push_str(&format!("LOCATION: {}\n", context.location_name));
        prompt.push_str(&format!("TIME: {}\n", context.time_context));

        if !context.present_characters.is_empty() {
            prompt.push_str(&format!(
                "OTHERS PRESENT: {}\n",
                context.present_characters.join(", ")
            ));
        }

        // Region items - visible objects in the area
        if !context.region_items.is_empty() {
            prompt.push_str("\nVISIBLE ITEMS IN AREA:\n");
            for item in &context.region_items {
                let type_suffix = item
                    .item_type
                    .as_ref()
                    .map(|t| format!(" [{}]", t))
                    .unwrap_or_default();
                let desc_suffix = item
                    .description
                    .as_ref()
                    .map(|d| format!(" - {}", d))
                    .unwrap_or_default();
                prompt.push_str(&format!("- {}{}{}\n", item.name, type_suffix, desc_suffix));
            }
        }

        prompt.push_str("\n");

        // Directorial notes - tone and pacing guidance
        if let Some(notes) = directorial_notes {
            prompt.push_str("=== DIRECTOR'S SCENE GUIDANCE ===\n");
            prompt.push_str(&format!("Tone: {}\n", notes.tone.description()));
            prompt.push_str(&format!("Pacing: {}\n", notes.pacing.description()));

            if !notes.general_notes.is_empty() {
                prompt.push_str(&format!("General Notes: {}\n", notes.general_notes));
            }

            if !notes.forbidden_topics.is_empty() {
                prompt.push_str(&format!(
                    "Avoid discussing: {}\n",
                    notes.forbidden_topics.join(", ")
                ));
            }

            if !notes.suggested_beats.is_empty() {
                prompt.push_str("Suggested narrative beats to work toward:\n");
                for beat in &notes.suggested_beats {
                    prompt.push_str(&format!("  - {}\n", beat));
                }
            }
            prompt.push_str("\n");
        }

        // Character details
        if let Some(mood) = &character.current_mood {
            prompt.push_str(&format!("YOUR CURRENT MOOD: {}\n", mood));
        }

        // Motivations from actantial model
        if let Some(motivations) = &character.motivations {
            Self::format_motivations(&mut prompt, motivations);
        }

        // Social stance (allies/enemies)
        if let Some(social) = &character.social_stance {
            Self::format_social_stance(&mut prompt, social);
        }

        if let Some(relationship) = &character.relationship_to_player {
            prompt.push_str(&format!(
                "\nYOUR RELATIONSHIP TO THE PLAYER: {}\n",
                relationship
            ));
        }

        // Active challenges - potential things that might be triggered
        if !active_challenges.is_empty() {
            prompt.push_str("## Active Challenges\n");
            prompt
                .push_str("The following challenges may be triggered based on player actions:\n\n");
            for (idx, challenge) in active_challenges.iter().enumerate() {
                prompt.push_str(&format!(
                    "{}. \"{}\" ({} {})\n",
                    idx + 1,
                    challenge.name,
                    challenge.skill_name,
                    challenge.difficulty_display
                ));
                prompt.push_str(&format!(
                    "   Triggers: {}\n",
                    challenge.trigger_hints.join(", ")
                ));
                prompt.push_str(&format!("   Description: {}\n\n", challenge.description));
            }

            // Use configurable challenge suggestion format
            prompt.push_str(&challenge_format);
            prompt.push_str("\n\n");
        }

        // Active narrative events - DM-designed story beats that can be triggered
        if !active_narrative_events.is_empty() {
            prompt.push_str("## Active Narrative Events\n");
            prompt.push_str("The following story events may be triggered based on player actions or conversation:\n\n");
            for (idx, event) in active_narrative_events.iter().enumerate() {
                prompt.push_str(&format!(
                    "{}. \"{}\" (Priority: {})\n",
                    idx + 1,
                    event.name,
                    event.priority
                ));
                prompt.push_str(&format!("   Description: {}\n", event.description));
                if !event.trigger_hints.is_empty() {
                    prompt.push_str(&format!(
                        "   Triggers when: {}\n",
                        event.trigger_hints.join(", ")
                    ));
                }
                if !event.featured_npc_names.is_empty() {
                    prompt.push_str(&format!(
                        "   Featured NPCs: {}\n",
                        event.featured_npc_names.join(", ")
                    ));
                }
                prompt.push_str("\n");
            }

            // Use configurable narrative event suggestion format
            prompt.push_str(&narrative_event_format);
            prompt.push_str("\n\n");
        }

        // Response format instructions (configurable)
        prompt.push_str(&response_format);

        prompt
    }

    /// Format motivations context for LLM prompt
    fn format_motivations(prompt: &mut String, motivations: &MotivationsContext) {
        // Known motivations
        if !motivations.known.is_empty() {
            prompt.push_str("\n=== KNOWN MOTIVATIONS ===\n");
            for m in &motivations.known {
                Self::format_motivation_entry(prompt, m, false);
            }
        }

        // Suspected motivations
        if !motivations.suspected.is_empty() {
            prompt.push_str("\n=== SUSPECTED MOTIVATIONS (player senses something) ===\n");
            for m in &motivations.suspected {
                Self::format_motivation_entry(prompt, m, false);
            }
            prompt.push_str(
                "The player has noticed your interest but doesn't know why. You may be evasive.\n",
            );
        }

        // Secret motivations with behavioral guidance
        if !motivations.secret.is_empty() {
            prompt.push_str("\n=== SECRET MOTIVATIONS (player does not know) ===\n");
            for s in &motivations.secret {
                Self::format_secret_motivation(prompt, s);
            }
        }
    }

    /// Format a single motivation entry
    fn format_motivation_entry(prompt: &mut String, m: &MotivationEntry, _is_secret: bool) {
        prompt.push_str(&format!(
            "- {} (Priority: {}, Intensity: {})\n",
            m.description, m.priority, m.intensity
        ));
        if let Some(target) = &m.target {
            prompt.push_str(&format!("  Target: {}\n", target));
        }
        if !m.helpers.is_empty() {
            let helpers: Vec<String> = m
                .helpers
                .iter()
                .map(|a| format!("{} ({}): {}", a.name, a.actor_type, a.reason))
                .collect();
            prompt.push_str(&format!("  Helpers: {}\n", helpers.join("; ")));
        }
        if !m.opponents.is_empty() {
            let opponents: Vec<String> = m
                .opponents
                .iter()
                .map(|a| format!("{} ({}): {}", a.name, a.actor_type, a.reason))
                .collect();
            prompt.push_str(&format!("  Opponents: {}\n", opponents.join("; ")));
        }
    }

    /// Format a secret motivation with behavioral guidance
    fn format_secret_motivation(prompt: &mut String, s: &SecretMotivationEntry) {
        prompt.push_str(&format!(
            "- {} (Priority: {}, Intensity: {})\n",
            s.description, s.priority, s.intensity
        ));
        if let Some(target) = &s.target {
            prompt.push_str(&format!("  Target: {}\n", target));
        }
        if !s.helpers.is_empty() {
            let helpers: Vec<String> = s
                .helpers
                .iter()
                .map(|a| format!("{} ({}): {}", a.name, a.actor_type, a.reason))
                .collect();
            prompt.push_str(&format!("  Helpers: {}\n", helpers.join("; ")));
        }
        if !s.opponents.is_empty() {
            let opponents: Vec<String> = s
                .opponents
                .iter()
                .map(|a| format!("{} ({}): {}", a.name, a.actor_type, a.reason))
                .collect();
            prompt.push_str(&format!("  Opponents: {}\n", opponents.join("; ")));
        }
        if let Some(sender) = &s.sender {
            prompt.push_str(&format!(
                "  Sender/Motivator: {} - {}\n",
                sender.name, sender.reason
            ));
        }
        if let Some(receiver) = &s.receiver {
            prompt.push_str(&format!(
                "  Beneficiary: {} - {}\n",
                receiver.name, receiver.reason
            ));
        }
        prompt.push_str("\n  BEHAVIORAL GUIDANCE:\n");
        prompt.push_str(&format!("  - When probed: {}\n", s.deflection_behavior));
        if !s.tells.is_empty() {
            prompt.push_str("  - Tells (subtle signs you may show):\n");
            for tell in &s.tells {
                prompt.push_str(&format!("    * {}\n", tell));
            }
        }
        prompt.push_str(
            "  DO NOT directly reveal this motivation. Use the behavioral guidance above.\n\n",
        );
    }

    /// Format social stance for LLM prompt
    fn format_social_stance(prompt: &mut String, social: &SocialStanceContext) {
        if social.allies.is_empty() && social.enemies.is_empty() {
            return;
        }

        prompt.push_str("\n=== SOCIAL STANCE ===\n");

        if !social.allies.is_empty() {
            prompt.push_str("ALLIES (characters you trust/appreciate):\n");
            for ally in &social.allies {
                prompt.push_str(&format!(
                    "- {} ({}): {}\n",
                    ally.name,
                    ally.character_type,
                    ally.reasons.join("; ")
                ));
            }
        }

        if !social.enemies.is_empty() {
            prompt.push_str("ENEMIES (characters you distrust/oppose):\n");
            for enemy in &social.enemies {
                prompt.push_str(&format!(
                    "- {} ({}): {}\n",
                    enemy.name,
                    enemy.character_type,
                    enemy.reasons.join("; ")
                ));
            }
        }
    }
}

// ============================================================================
// Standalone utility functions (don't need template resolution)
// ============================================================================

/// Build the user message containing the player's action and directorial notes
pub fn build_user_message(request: &GamePromptRequest) -> String {
    let mut message = String::new();

    // Directorial notes (for the AI, not visible to player)
    if !request.directorial_notes.is_empty() {
        message.push_str(&format!(
            "[DIRECTOR'S NOTES: {}]\n\n",
            request.directorial_notes
        ));
    }

    // Player action
    match request.player_action.action_type.as_str() {
        "speak" => {
            if let Some(dialogue) = &request.player_action.dialogue {
                if let Some(target) = &request.player_action.target {
                    message.push_str(&format!(
                        "The player says to {}: \"{}\"\n",
                        target, dialogue
                    ));
                } else {
                    message.push_str(&format!("The player says: \"{}\"\n", dialogue));
                }
            }
        }
        "examine" => {
            if let Some(target) = &request.player_action.target {
                message.push_str(&format!("The player examines {}.\n", target));
            }
        }
        "use_item" => {
            if let Some(target) = &request.player_action.target {
                message.push_str(&format!("The player uses an item on {}.\n", target));
            }
        }
        other => {
            message.push_str(&format!("The player performs action: {}\n", other));
            if let Some(target) = &request.player_action.target {
                message.push_str(&format!("Target: {}\n", target));
            }
        }
    }

    message.push_str(&format!(
        "\nRespond as {}.",
        request.responding_character.name
    ));

    message
}

/// Convert conversation history to ChatMessage format
pub fn build_conversation_history(history: &[ConversationTurn]) -> Vec<ChatMessage> {
    history
        .iter()
        .map(|turn| {
            // Determine role based on speaker name
            // If it matches the player, it's a user message; otherwise assistant
            let role = if turn.speaker.to_lowercase() == "player" {
                MessageRole::User
            } else {
                MessageRole::Assistant
            };

            ChatMessage {
                role,
                content: format!("{}: {}", turn.speaker, turn.text),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wrldbldr_engine_ports::outbound::PromptTemplateRepositoryPort;

    // Mock repository for testing
    struct MockPromptTemplateRepository;

    #[async_trait::async_trait]
    impl PromptTemplateRepositoryPort for MockPromptTemplateRepository {
        async fn get_global(
            &self,
            _key: &str,
        ) -> Result<Option<String>, wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(None) // Always return default
        }
        async fn get_all_global(
            &self,
        ) -> Result<Vec<(String, String)>, wrldbldr_engine_ports::outbound::PromptTemplateError>
        {
            Ok(vec![])
        }
        async fn set_global(
            &self,
            _key: &str,
            _value: &str,
        ) -> Result<(), wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(())
        }
        async fn delete_global(
            &self,
            _key: &str,
        ) -> Result<(), wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(())
        }
        async fn delete_all_global(
            &self,
        ) -> Result<(), wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(())
        }
        async fn get_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
            _key: &str,
        ) -> Result<Option<String>, wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(None)
        }
        async fn get_all_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
        ) -> Result<Vec<(String, String)>, wrldbldr_engine_ports::outbound::PromptTemplateError>
        {
            Ok(vec![])
        }
        async fn set_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
            _key: &str,
            _value: &str,
        ) -> Result<(), wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(())
        }
        async fn delete_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
            _key: &str,
        ) -> Result<(), wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(())
        }
        async fn delete_all_for_world(
            &self,
            _world_id: wrldbldr_domain::WorldId,
        ) -> Result<(), wrldbldr_engine_ports::outbound::PromptTemplateError> {
            Ok(())
        }
    }

    fn create_test_prompt_builder() -> PromptBuilder {
        let repo: Arc<dyn PromptTemplateRepositoryPort> = Arc::new(MockPromptTemplateRepository);
        let service = Arc::new(PromptTemplateService::new(repo));
        PromptBuilder::new(service)
    }

    #[tokio::test]
    async fn test_build_system_prompt() {
        let builder = create_test_prompt_builder();

        let context = SceneContext {
            scene_name: "The Rusty Anchor".to_string(),
            location_name: "Port Valdris".to_string(),
            time_context: "Late evening".to_string(),
            present_characters: vec!["Bartender".to_string()],
            region_items: vec![],
        };

        let character = CharacterContext {
            character_id: None,
            name: "Gorm".to_string(),
            archetype: "Gruff tavern keeper".to_string(),
            current_mood: Some("Suspicious".to_string()),
            motivations: None,
            social_stance: None,
            relationship_to_player: Some("Acquaintance".to_string()),
        };

        let prompt = builder
            .build_system_prompt(None, &context, &character)
            .await;

        assert!(prompt.contains("Gorm"));
        assert!(prompt.contains("Gruff tavern keeper"));
        assert!(prompt.contains("The Rusty Anchor"));
        assert!(prompt.contains("Suspicious"));
        assert!(prompt.contains("<reasoning>"));
        assert!(prompt.contains("<dialogue>"));
    }

    #[test]
    fn test_enforce_budget_within_limit() {
        let builder = create_test_prompt_builder();
        let prompt = "This is a short prompt.".to_string();
        let budget = ContextBudgetConfig::default(); // 4000 tokens

        let result = builder.enforce_budget(prompt.clone(), &budget);
        assert_eq!(result, prompt);
    }

    #[test]
    fn test_enforce_budget_truncates_when_over() {
        let builder = create_test_prompt_builder();
        // Create a very long prompt that exceeds the minimal budget
        let long_prompt = "word ".repeat(500); // ~500 words = ~665 tokens
        let mut budget = ContextBudgetConfig::minimal();
        budget.total_budget_tokens = 50; // Force truncation

        let result = builder.enforce_budget(long_prompt.clone(), &budget);
        assert!(result.len() < long_prompt.len());
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_enforce_budget_logs_truncation() {
        // This test just verifies that the truncation path works
        // Actual log verification would require a more complex test setup
        let builder = create_test_prompt_builder();
        let long_prompt = "word ".repeat(200);
        let mut budget = ContextBudgetConfig::default();
        budget.total_budget_tokens = 10;

        let result = builder.enforce_budget(long_prompt, &budget);
        assert!(result.ends_with("..."));
    }
}
