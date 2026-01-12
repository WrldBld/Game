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
