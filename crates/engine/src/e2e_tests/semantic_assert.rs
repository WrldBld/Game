//! LLM-based semantic assertions for E2E tests.
//!
//! Provides helpers to validate LLM outputs using another LLM call,
//! enabling semantic validation that goes beyond string matching.
//!
//! # Example
//!
//! ```ignore
//! use crate::e2e_tests::semantic_assert::SemanticAssert;
//!
//! let asserter = SemanticAssert::new(vcr.clone());
//!
//! // Assert that dialogue is about a specific topic
//! asserter.assert_dialogue_about(
//!     &final_dialogue,
//!     "the old mill and strange happenings",
//!     "NPC should discuss the mill when asked about it"
//! ).await?;
//!
//! // Assert that response is in character
//! asserter.assert_in_character(
//!     &final_dialogue,
//!     "Marta Hearthwood, a friendly innkeeper",
//!     "Response should match Marta's warm personality"
//! ).await?;
//! ```
//!
//! # VCR Integration
//!
//! Assertion LLM calls are recorded in the same cassette as the main test calls.
//! Each assertion has a unique fingerprint based on its prompt, allowing them
//! to be replayed deterministically.

use std::sync::Arc;

use crate::infrastructure::ports::{ChatMessage, LlmError, LlmPort, LlmRequest};
use crate::use_cases::queues::response_parser::strip_special_tokens;

/// Semantic assertion helper that uses LLM to validate outputs.
///
/// Wraps an `LlmPort` (typically `VcrLlm`) to make assertion calls that
/// validate semantic properties of dialogue and other LLM outputs.
pub struct SemanticAssert {
    llm: Arc<dyn LlmPort>,
}

impl SemanticAssert {
    /// Create a new semantic assertion helper.
    ///
    /// # Arguments
    ///
    /// * `llm` - The LLM port to use for assertions (typically VcrLlm for recording/playback)
    pub fn new(llm: Arc<dyn LlmPort>) -> Self {
        Self { llm }
    }

    /// Ask the LLM a yes/no question and return the boolean result.
    ///
    /// The LLM is prompted to respond with only "YES" or "NO".
    /// Any response starting with "YES" (case-insensitive) returns true.
    ///
    /// # Arguments
    ///
    /// * `context` - The text to evaluate (e.g., dialogue)
    /// * `question` - A yes/no question about the context
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if the LLM answers YES
    /// * `Ok(false)` if the LLM answers NO or gives an unclear response
    /// * `Err` if the LLM call fails
    pub async fn ask_yes_no(&self, context: &str, question: &str) -> Result<bool, LlmError> {
        let prompt = format!(
            "You are evaluating text for a test assertion. Answer ONLY with 'YES' or 'NO'.\n\n\
             TEXT TO EVALUATE:\n\
             \"\"\"\n{}\n\"\"\"\n\n\
             QUESTION: {}\n\n\
             Answer (YES or NO only):",
            context, question
        );

        let request = LlmRequest {
            system_prompt: Some(
                "You are a precise evaluator. Answer only YES or NO, nothing else.".to_string(),
            ),
            messages: vec![ChatMessage::user(&prompt)],
            temperature: Some(0.0), // Deterministic
            max_tokens: Some(512),  // Allow room for model's internal reasoning tokens
            images: Vec::new(),
        };

        let response = self.llm.generate(request).await?;
        // Strip model-specific special tokens (e.g., <|channel|>, <|message|>)
        let cleaned = strip_special_tokens(&response.content);
        let answer = cleaned.trim().to_uppercase();

        tracing::debug!(
            question = %question,
            raw_response = %response.content,
            cleaned_answer = %answer,
            "Semantic assertion LLM response"
        );

        Ok(answer.starts_with("YES"))
    }

    /// Assert that dialogue is about a specific topic.
    ///
    /// # Arguments
    ///
    /// * `dialogue` - The dialogue text to check
    /// * `topic` - Description of what the dialogue should be about
    /// * `failure_message` - Message to include if assertion fails
    ///
    /// # Panics
    ///
    /// Panics if the dialogue is not about the specified topic, or if the LLM call fails.
    pub async fn assert_dialogue_about(
        &self,
        dialogue: &str,
        topic: &str,
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let question = format!(
            "Is this dialogue primarily about or related to '{}'?",
            topic
        );
        let result = self.ask_yes_no(dialogue, &question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: format!("Dialogue should be about: {}", topic),
                actual: dialogue.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!(
            topic = %topic,
            "Semantic assertion passed: dialogue is about expected topic"
        );
        Ok(())
    }

    /// Assert that dialogue is in character for an NPC.
    ///
    /// # Arguments
    ///
    /// * `dialogue` - The dialogue text to check
    /// * `character_description` - Description of the character (name, traits, role)
    /// * `failure_message` - Message to include if assertion fails
    ///
    /// # Panics
    ///
    /// Panics if the dialogue doesn't match the character, or if the LLM call fails.
    pub async fn assert_in_character(
        &self,
        dialogue: &str,
        character_description: &str,
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let question = format!(
            "Would this dialogue be appropriate and in-character for {}?",
            character_description
        );
        let result = self.ask_yes_no(dialogue, &question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: format!(
                    "Dialogue should be in character for: {}",
                    character_description
                ),
                actual: dialogue.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!(
            character = %character_description,
            "Semantic assertion passed: dialogue is in character"
        );
        Ok(())
    }

    /// Assert that dialogue contains a greeting or welcoming response.
    ///
    /// # Arguments
    ///
    /// * `dialogue` - The dialogue text to check
    /// * `failure_message` - Message to include if assertion fails
    pub async fn assert_is_greeting(
        &self,
        dialogue: &str,
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let question = "Is this a greeting, welcome, or friendly acknowledgment of someone's arrival or hello?";
        let result = self.ask_yes_no(dialogue, question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: "Dialogue should be a greeting".to_string(),
                actual: dialogue.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!("Semantic assertion passed: dialogue is a greeting");
        Ok(())
    }

    /// Assert that dialogue mentions or discusses specific subject matter.
    ///
    /// # Arguments
    ///
    /// * `dialogue` - The dialogue text to check
    /// * `subjects` - List of subjects that should be mentioned (at least one)
    /// * `failure_message` - Message to include if assertion fails
    pub async fn assert_mentions_any(
        &self,
        dialogue: &str,
        subjects: &[&str],
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let subjects_list = subjects.join(", ");
        let question = format!(
            "Does this dialogue mention, discuss, or relate to ANY of these subjects: {}?",
            subjects_list
        );
        let result = self.ask_yes_no(dialogue, &question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: format!("Dialogue should mention one of: {}", subjects_list),
                actual: dialogue.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!(
            subjects = %subjects_list,
            "Semantic assertion passed: dialogue mentions expected subjects"
        );
        Ok(())
    }

    /// Assert that dialogue has a specific emotional tone.
    ///
    /// # Arguments
    ///
    /// * `dialogue` - The dialogue text to check
    /// * `tone` - Expected emotional tone (e.g., "friendly", "cautious", "mysterious")
    /// * `failure_message` - Message to include if assertion fails
    pub async fn assert_tone(
        &self,
        dialogue: &str,
        tone: &str,
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let question = format!(
            "Does this dialogue have a {} tone or emotional quality?",
            tone
        );
        let result = self.ask_yes_no(dialogue, &question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: format!("Dialogue should have {} tone", tone),
                actual: dialogue.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!(
            tone = %tone,
            "Semantic assertion passed: dialogue has expected tone"
        );
        Ok(())
    }

    /// Assert that the NPC appropriately responded to a player question.
    ///
    /// # Arguments
    ///
    /// * `player_question` - What the player asked
    /// * `npc_response` - The NPC's response dialogue
    /// * `failure_message` - Message to include if assertion fails
    pub async fn assert_responds_to_question(
        &self,
        player_question: &str,
        npc_response: &str,
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let context = format!(
            "PLAYER ASKED: {}\n\nNPC RESPONDED: {}",
            player_question, npc_response
        );
        let question = "Is the NPC's response relevant and appropriate to what the player asked?";
        let result = self.ask_yes_no(&context, question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: format!("NPC should respond appropriately to: {}", player_question),
                actual: npc_response.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!("Semantic assertion passed: NPC responded appropriately to player question");
        Ok(())
    }

    /// Custom semantic assertion with arbitrary question.
    ///
    /// # Arguments
    ///
    /// * `context` - The text to evaluate
    /// * `question` - A yes/no question to ask about the context
    /// * `failure_message` - Message to include if assertion fails
    pub async fn assert_custom(
        &self,
        context: &str,
        question: &str,
        failure_message: &str,
    ) -> Result<(), AssertionError> {
        let result = self.ask_yes_no(context, question).await?;

        if !result {
            return Err(AssertionError::Failed {
                assertion: question.to_string(),
                actual: context.to_string(),
                message: failure_message.to_string(),
            });
        }

        tracing::info!(
            question = %question,
            "Semantic assertion passed"
        );
        Ok(())
    }
}

/// Error type for semantic assertion failures.
#[derive(Debug, thiserror::Error)]
pub enum AssertionError {
    /// The semantic assertion failed (LLM answered NO)
    #[error("Semantic assertion failed: {assertion}\nMessage: {message}\nActual: {actual}")]
    Failed {
        assertion: String,
        actual: String,
        message: String,
    },

    /// LLM error during assertion
    #[error("LLM error during assertion: {0}")]
    LlmError(#[from] LlmError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{FinishReason, LlmResponse};
    use async_trait::async_trait;

    /// Mock LLM that returns predetermined responses.
    struct MockLlm {
        response: String,
    }

    impl MockLlm {
        fn yes() -> Self {
            Self {
                response: "YES".to_string(),
            }
        }

        fn no() -> Self {
            Self {
                response: "NO".to_string(),
            }
        }
    }

    #[async_trait]
    impl LlmPort for MockLlm {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                content: self.response.clone(),
                tool_calls: vec![],
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }

        async fn generate_with_tools(
            &self,
            _request: LlmRequest,
            _tools: Vec<crate::infrastructure::ports::ToolDefinition>,
        ) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                content: self.response.clone(),
                tool_calls: vec![],
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }
    }

    #[tokio::test]
    async fn test_ask_yes_no_returns_true_for_yes() {
        let asserter = SemanticAssert::new(Arc::new(MockLlm::yes()));
        let result = asserter.ask_yes_no("test", "question?").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_ask_yes_no_returns_false_for_no() {
        let asserter = SemanticAssert::new(Arc::new(MockLlm::no()));
        let result = asserter.ask_yes_no("test", "question?").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_assert_dialogue_about_passes_when_yes() {
        let asserter = SemanticAssert::new(Arc::new(MockLlm::yes()));
        let result = asserter
            .assert_dialogue_about("Hello world", "greetings", "should pass")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_assert_dialogue_about_fails_when_no() {
        let asserter = SemanticAssert::new(Arc::new(MockLlm::no()));
        let result = asserter
            .assert_dialogue_about("Hello world", "greetings", "test failure")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AssertionError::Failed { .. }));
    }
}
