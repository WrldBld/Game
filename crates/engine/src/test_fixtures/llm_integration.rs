//! LLM integration test helpers for Ollama.
//!
//! Provides utilities for running integration tests against a local Ollama instance.
//!
//! # Note on Reasoning Models
//!
//! The default model (gpt-oss:20b) is a reasoning/thinking model that uses tokens
//! for chain-of-thought before generating content. Token limits should be set high
//! enough to accommodate both reasoning and output (typically 500-1000 tokens).
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::test_fixtures::llm_integration::*;
//!
//! #[tokio::test]
//! #[ignore = "requires ollama"]
//! async fn test_llm_response() {
//!     let client = create_test_ollama_client();
//!     // ... test logic
//! }
//! ```

use crate::infrastructure::ollama::{OllamaClient, DEFAULT_OLLAMA_BASE_URL, DEFAULT_OLLAMA_MODEL};
use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest};

/// Creates an OllamaClient configured for integration testing.
///
/// Uses environment variables for configuration:
/// - `OLLAMA_BASE_URL`: Base URL for Ollama (default: http://localhost:11434)
/// - `OLLAMA_MODEL`: Model to use (default: gpt-oss:20b)
pub fn create_test_ollama_client() -> OllamaClient {
    OllamaClient::from_env()
}

/// Creates an OllamaClient with a custom timeout for testing timeout behavior.
pub fn create_test_ollama_client_with_timeout(timeout_secs: u64) -> OllamaClient {
    let base_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_string());
    let model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
    OllamaClient::with_timeout(&base_url, &model, timeout_secs)
}

/// Check if Ollama is available for integration tests.
///
/// Returns true if the server is reachable and responds to requests.
pub async fn ollama_available() -> bool {
    let client = create_test_ollama_client();

    // Try a minimal request to check availability
    let request = LlmRequest::new(vec![ChatMessage::user("Hi")])
        .with_temperature(0.0)
        .with_max_tokens(Some(5));

    client.generate(request).await.is_ok()
}

/// Skip test if Ollama is not available.
///
/// Use this at the start of integration tests to skip gracefully when Ollama
/// is not running.
pub async fn skip_if_ollama_unavailable() {
    if !ollama_available().await {
        panic!("Ollama is not available - skipping test");
    }
}

/// Build a simple LLM request for testing.
pub fn build_simple_request(user_message: &str) -> LlmRequest {
    LlmRequest::new(vec![ChatMessage::user(user_message)])
        .with_temperature(0.7)
        .with_max_tokens(Some(800))
}

/// Build an LLM request with a system prompt.
pub fn build_request_with_system(system_prompt: &str, user_message: &str) -> LlmRequest {
    LlmRequest::new(vec![ChatMessage::user(user_message)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7)
        .with_max_tokens(Some(800))
}

/// Build an LLM request with conversation history.
pub fn build_request_with_history(
    system_prompt: Option<&str>,
    messages: Vec<(&str, &str)>, // (role, content) - role is "user" or "assistant"
) -> LlmRequest {
    use crate::infrastructure::ports::ChatMessage;

    let chat_messages: Vec<ChatMessage> = messages
        .into_iter()
        .map(|(role, content)| match role {
            "user" => ChatMessage::user(content),
            "assistant" => ChatMessage::assistant(content),
            "system" => ChatMessage::system(content),
            _ => ChatMessage::user(content),
        })
        .collect();

    let mut request = LlmRequest::new(chat_messages)
        .with_temperature(0.7)
        .with_max_tokens(Some(800));

    if let Some(sys) = system_prompt {
        request = request.with_system_prompt(sys);
    }

    request
}

/// DM-style system prompt for narrative generation.
pub const DM_SYSTEM_PROMPT: &str = r#"You are a Dungeon Master for a fantasy roleplaying game.
Generate vivid, immersive narrative descriptions. Keep responses concise but evocative.
Always stay in character as the narrator."#;

/// System prompt for suggesting game mechanics.
pub const MECHANICS_SYSTEM_PROMPT: &str = r#"You are a game rules assistant.
When players describe actions, suggest appropriate skill checks or mechanics.
Be concise and reference D&D 5e rules when applicable."#;

// =============================================================================
// LLM-based Output Validation
// =============================================================================

/// Result of LLM-based validation.
#[derive(Debug, Clone)]
pub struct LlmValidation {
    /// Whether the response passed validation.
    pub passed: bool,
    /// Explanation from the validator.
    pub explanation: String,
}

/// Validate an LLM response using semantic analysis instead of keyword matching.
///
/// This function asks an LLM to judge whether a response meets the specified criteria.
/// It's more robust than keyword matching because it understands context and meaning.
///
/// # Arguments
///
/// * `client` - The LLM client to use for validation
/// * `task` - Description of what the original task was asking for
/// * `response` - The response to validate
/// * `criteria` - What makes a valid response (e.g., "should be dramatic and describe a critical hit")
///
/// # Example
///
/// ```rust,ignore
/// let validation = validate_response(
///     &client,
///     "Generate a dramatic description of a critical hit",
///     &response.content,
///     "The response should be exciting and describe a powerful attack with vivid imagery"
/// ).await;
/// assert!(validation.passed, "Validation failed: {}", validation.explanation);
/// ```
pub async fn validate_response(
    client: &dyn LlmPort,
    task: &str,
    response: &str,
    criteria: &str,
) -> LlmValidation {
    let system_prompt = r#"You are a test validator. Your job is to determine if a response meets the specified criteria.

IMPORTANT: You must respond with EXACTLY this format:
PASS or FAIL
[Your brief explanation]

Example responses:
PASS
The response describes a tavern scene with atmospheric details.

FAIL
The response is about cooking, not about the tavern as requested."#;

    let user_message = format!(
        "TASK: {}\n\nRESPONSE TO VALIDATE:\n{}\n\nCRITERIA: {}\n\nDoes this response meet the criteria? Answer PASS or FAIL with a brief explanation.",
        task, response, criteria
    );

    let request = LlmRequest::new(vec![ChatMessage::user(&user_message)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.0) // Use deterministic responses for validation
        .with_max_tokens(Some(300));

    match client.generate(request).await {
        Ok(validation_response) => {
            let content = validation_response.content.trim();
            let passed = content.to_uppercase().starts_with("PASS");
            LlmValidation {
                passed,
                explanation: content.to_string(),
            }
        }
        Err(e) => LlmValidation {
            passed: false,
            explanation: format!("Validation request failed: {}", e),
        },
    }
}

/// Assert that an LLM response passes semantic validation.
///
/// This is a convenience macro/function for use in tests. It validates the response
/// and panics with a helpful message if validation fails.
///
/// # Example
///
/// ```rust,ignore
/// assert_llm_valid(
///     &client,
///     "Describe a tavern scene",
///     &response.content,
///     "Should describe a tavern with atmospheric details like lighting, sounds, or smells"
/// ).await;
/// ```
pub async fn assert_llm_valid(
    client: &dyn LlmPort,
    task: &str,
    response: &str,
    criteria: &str,
) {
    let validation = validate_response(client, task, response, criteria).await;
    assert!(
        validation.passed,
        "LLM validation failed.\n\nTask: {}\n\nResponse: {}\n\nCriteria: {}\n\nValidator said: {}",
        task,
        response,
        criteria,
        validation.explanation
    );
}

/// Validate multiple criteria for a single response.
///
/// Returns true only if ALL criteria pass. Useful for complex validations.
pub async fn validate_all_criteria(
    client: &dyn LlmPort,
    task: &str,
    response: &str,
    criteria_list: &[&str],
) -> (bool, Vec<LlmValidation>) {
    let mut results = Vec::new();
    let mut all_passed = true;

    for criteria in criteria_list {
        let validation = validate_response(client, task, response, criteria).await;
        if !validation.passed {
            all_passed = false;
        }
        results.push(validation);
    }

    (all_passed, results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::MessageRole;

    #[test]
    fn test_build_simple_request() {
        let request = build_simple_request("Hello world");
        assert!(request.system_prompt.is_none());
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].content, "Hello world");
    }

    #[test]
    fn test_build_request_with_system() {
        let request = build_request_with_system("Be helpful", "Hello");
        assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_build_request_with_history() {
        let request = build_request_with_history(
            Some("System prompt"),
            vec![
                ("user", "Hello"),
                ("assistant", "Hi there!"),
                ("user", "How are you?"),
            ],
        );
        assert_eq!(request.messages.len(), 3);
        assert!(matches!(request.messages[0].role, MessageRole::User));
        assert!(matches!(request.messages[1].role, MessageRole::Assistant));
    }
}
