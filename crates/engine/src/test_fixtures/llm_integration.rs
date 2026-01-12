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
//! # Logging LLM Interactions
//!
//! Set `LLM_TEST_LOG=1` to log all LLM requests and responses to files for analysis.
//! Logs are written to `./llm_test_logs/` by default, or set `LLM_TEST_LOG_DIR` to customize.
//!
//! ```bash
//! LLM_TEST_LOG=1 cargo test -p wrldbldr-engine --lib -- --ignored
//! ```
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
use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest, LlmResponse};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for unique log file names.
static LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);

// =============================================================================
// LLM Interaction Logging
// =============================================================================

/// Check if LLM logging is enabled via environment variable.
pub fn is_llm_logging_enabled() -> bool {
    std::env::var("LLM_TEST_LOG")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Get the directory for LLM log files.
pub fn get_llm_log_dir() -> std::path::PathBuf {
    std::env::var("LLM_TEST_LOG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./llm_test_logs"))
}

/// Log an LLM request and response to a file.
///
/// Files are named with a timestamp and counter for uniqueness.
/// Each file contains the full request (system prompt, messages) and response.
///
/// # Arguments
///
/// * `test_name` - Name of the test (e.g., "test_llm_generates_narrative")
/// * `label` - Optional label to distinguish multiple calls within a test (e.g., "generation", "validation")
/// * `request` - The LLM request that was sent
/// * `response` - The LLM response that was received
pub fn log_llm_interaction(
    test_name: &str,
    label: Option<&str>,
    request: &LlmRequest,
    response: &LlmResponse,
) {
    if !is_llm_logging_enabled() {
        return;
    }

    let log_dir = get_llm_log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create LLM log directory: {}", e);
        return;
    }

    let counter = LOG_COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let sanitized_name = test_name
        .replace("::", "_")
        .replace(" ", "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    let label_suffix = label
        .map(|l| format!("_{}", l.replace(" ", "_")))
        .unwrap_or_default();

    let filename = format!("{}_{:04}_{}{}.md", timestamp, counter, sanitized_name, label_suffix);
    let filepath = log_dir.join(&filename);

    let mut content = String::new();
    content.push_str(&format!("# LLM Test Log: {}\n\n", test_name));
    if let Some(l) = label {
        content.push_str(&format!("**Label:** {}\n\n", l));
    }
    content.push_str(&format!("**Timestamp:** {}\n\n", chrono::Utc::now().to_rfc3339()));
    content.push_str(&format!("**Sequence:** #{}\n\n", counter));

    // Request details
    content.push_str("## Request\n\n");

    if let Some(ref system) = request.system_prompt {
        content.push_str("### System Prompt\n\n");
        content.push_str("```\n");
        content.push_str(system);
        content.push_str("\n```\n\n");
    }

    content.push_str("### Messages\n\n");
    for msg in &request.messages {
        content.push_str(&format!("**{}:**\n", format!("{:?}", msg.role)));
        content.push_str("```\n");
        content.push_str(&msg.content);
        content.push_str("\n```\n\n");
    }

    content.push_str("### Parameters\n\n");
    content.push_str(&format!("- Temperature: {:?}\n", request.temperature));
    content.push_str(&format!("- Max Tokens: {:?}\n\n", request.max_tokens));

    // Response details
    content.push_str("## Response\n\n");
    content.push_str("### Content\n\n");
    content.push_str("```\n");
    content.push_str(&response.content);
    content.push_str("\n```\n\n");

    if !response.tool_calls.is_empty() {
        content.push_str("### Tool Calls\n\n");
        for (i, tool_call) in response.tool_calls.iter().enumerate() {
            content.push_str(&format!("**Tool Call #{}:**\n", i + 1));
            content.push_str(&format!("- ID: {}\n", tool_call.id));
            content.push_str(&format!("- Name: {}\n", tool_call.name));
            content.push_str("- Arguments:\n```json\n");
            if let Ok(json) = serde_json::to_string_pretty(&tool_call.arguments) {
                content.push_str(&json);
            } else {
                content.push_str(&format!("{:?}", tool_call.arguments));
            }
            content.push_str("\n```\n\n");
        }
    }

    if let Some(ref usage) = response.usage {
        content.push_str("### Usage\n\n");
        content.push_str(&format!("- Prompt Tokens: {}\n", usage.prompt_tokens));
        content.push_str(&format!("- Completion Tokens: {}\n", usage.completion_tokens));
        content.push_str(&format!("- Total Tokens: {}\n", usage.total_tokens));
    }

    if let Err(e) = std::fs::write(&filepath, content) {
        eprintln!("Failed to write LLM log file: {}", e);
    } else {
        eprintln!("LLM log written to: {}", filepath.display());
    }
}

/// Generate an LLM response and log the interaction if logging is enabled.
///
/// This is a convenience wrapper around `client.generate()` that automatically
/// logs the request and response when `LLM_TEST_LOG=1` is set.
///
/// # Arguments
///
/// * `client` - The LLM client to use
/// * `request` - The request to send
/// * `test_name` - Name of the test for the log file
/// * `label` - Optional label to distinguish multiple calls (e.g., "generation", "validation")
pub async fn generate_and_log(
    client: &dyn LlmPort,
    request: LlmRequest,
    test_name: &str,
    label: Option<&str>,
) -> Result<LlmResponse, crate::infrastructure::ports::LlmError> {
    let response = client.generate(request.clone()).await?;
    log_llm_interaction(test_name, label, &request, &response);
    Ok(response)
}

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
        .with_max_tokens(Some(500)); // Reasoning models need more tokens for validation

    match client.generate(request.clone()).await {
        Ok(validation_response) => {
            // Log the validation interaction
            log_llm_interaction("validation", Some("semantic_check"), &request, &validation_response);

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
