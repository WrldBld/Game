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

// =============================================================================
// Scene Context Builder
// =============================================================================

use wrldbldr_domain::{
    CharacterContext, ConversationTurn, GamePromptRequest, MotivationEntry, MotivationsContext,
    PlayerActionContext, RegionItemContext, SceneContext, SecretMotivationEntry,
    SocialStanceContext,
};

/// Builder for constructing `SceneContext` for tests.
///
/// Provides preset configurations for common test scenarios.
#[derive(Debug, Clone)]
pub struct SceneContextBuilder {
    scene_name: String,
    location_name: String,
    time_context: String,
    present_characters: Vec<String>,
    region_items: Vec<RegionItemContext>,
}

impl Default for SceneContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneContextBuilder {
    pub fn new() -> Self {
        Self {
            scene_name: "Current Scene".to_string(),
            location_name: "Current Location".to_string(),
            time_context: "Present".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    // -------------------------------------------------------------------------
    // Presets
    // -------------------------------------------------------------------------

    /// Tavern scene in the evening with typical atmosphere.
    pub fn tavern_evening() -> Self {
        Self {
            scene_name: "Evening at the Inn".to_string(),
            location_name: "The Drowsy Dragon Inn".to_string(),
            time_context: "Evening".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    /// Tavern scene in the morning with quieter atmosphere.
    pub fn tavern_morning() -> Self {
        Self {
            scene_name: "Morning at the Inn".to_string(),
            location_name: "The Drowsy Dragon Inn".to_string(),
            time_context: "Morning".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    /// Village marketplace during busy hours.
    pub fn marketplace_morning() -> Self {
        Self {
            scene_name: "Market Day".to_string(),
            location_name: "Thornhaven Square".to_string(),
            time_context: "Morning".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    /// Temple scene during morning prayers.
    pub fn temple_morning() -> Self {
        Self {
            scene_name: "Morning Prayers".to_string(),
            location_name: "Temple of the Dawn".to_string(),
            time_context: "Morning".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    /// Abandoned mill at night, eerie atmosphere.
    pub fn mill_night() -> Self {
        Self {
            scene_name: "The Haunted Mill".to_string(),
            location_name: "The Old Mill".to_string(),
            time_context: "Night".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    /// Generic dungeon scene for exploration.
    pub fn dungeon_exploration() -> Self {
        Self {
            scene_name: "Dungeon Depths".to_string(),
            location_name: "Ancient Ruins".to_string(),
            time_context: "Timeless".to_string(),
            present_characters: vec![],
            region_items: vec![],
        }
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    pub fn with_scene_name(mut self, name: &str) -> Self {
        self.scene_name = name.to_string();
        self
    }

    pub fn with_location_name(mut self, name: &str) -> Self {
        self.location_name = name.to_string();
        self
    }

    pub fn with_time_context(mut self, time: &str) -> Self {
        self.time_context = time.to_string();
        self
    }

    pub fn with_present_character(mut self, name: &str) -> Self {
        self.present_characters.push(name.to_string());
        self
    }

    pub fn with_present_characters(mut self, names: Vec<&str>) -> Self {
        self.present_characters
            .extend(names.into_iter().map(String::from));
        self
    }

    pub fn with_item(mut self, name: &str, description: Option<&str>, item_type: Option<&str>) -> Self {
        self.region_items.push(RegionItemContext {
            name: name.to_string(),
            description: description.map(String::from),
            item_type: item_type.map(String::from),
        });
        self
    }

    pub fn build(self) -> SceneContext {
        SceneContext {
            scene_name: self.scene_name,
            location_name: self.location_name,
            time_context: self.time_context,
            present_characters: self.present_characters,
            region_items: self.region_items,
        }
    }
}

// =============================================================================
// Character Context Builder
// =============================================================================

/// Builder for constructing `CharacterContext` for tests.
///
/// Provides preset configurations for common NPC archetypes.
#[derive(Debug, Clone)]
pub struct CharacterContextBuilder {
    character_id: Option<String>,
    name: String,
    archetype: String,
    current_mood: Option<String>,
    disposition_toward_player: Option<String>,
    motivations: Option<MotivationsContext>,
    social_stance: Option<SocialStanceContext>,
    relationship_to_player: Option<String>,
    available_expressions: Option<Vec<String>>,
    available_actions: Option<Vec<String>>,
}

impl Default for CharacterContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterContextBuilder {
    pub fn new() -> Self {
        Self {
            character_id: None,
            name: "NPC".to_string(),
            archetype: "NPC".to_string(),
            current_mood: None,
            disposition_toward_player: None,
            motivations: None,
            social_stance: None,
            relationship_to_player: None,
            available_expressions: None,
            available_actions: None,
        }
    }

    // -------------------------------------------------------------------------
    // Presets
    // -------------------------------------------------------------------------

    /// Friendly tavern keeper who knows local gossip (Mentor archetype).
    pub fn friendly_innkeeper() -> Self {
        Self {
            character_id: None,
            name: "Marta Hearthwood".to_string(),
            archetype: "Mentor".to_string(),
            current_mood: Some("Calm".to_string()),
            disposition_toward_player: Some("Friendly".to_string()),
            motivations: Some(MotivationsContext {
                known: vec![MotivationEntry {
                    description: "Keep her inn prosperous and welcoming".to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                }],
                suspected: vec![],
                secret: vec![],
            }),
            social_stance: None,
            relationship_to_player: Some("Acquaintance".to_string()),
            available_expressions: Some(vec![
                "neutral".to_string(),
                "happy".to_string(),
                "concerned".to_string(),
            ]),
            available_actions: Some(vec![
                "wipes the counter".to_string(),
                "leans in conspiratorially".to_string(),
            ]),
        }
    }

    /// Gruff blacksmith with a hidden past (Threshold Guardian archetype).
    pub fn gruff_blacksmith() -> Self {
        Self {
            character_id: None,
            name: "Grom Ironhand".to_string(),
            archetype: "Threshold Guardian".to_string(),
            current_mood: Some("Calm".to_string()),
            disposition_toward_player: Some("Neutral".to_string()),
            motivations: Some(MotivationsContext {
                known: vec![],
                suspected: vec![MotivationEntry {
                    description: "Hiding something about his past".to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                }],
                secret: vec![SecretMotivationEntry {
                    description: "Atone for failing to protect his adventuring party".to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                    sender: None,
                    receiver: None,
                    deflection_behavior: "Changes subject, focuses on work".to_string(),
                    tells: vec!["Grips hammer tighter when past is mentioned".to_string()],
                }],
            }),
            social_stance: None,
            relationship_to_player: Some("Stranger".to_string()),
            available_expressions: Some(vec![
                "neutral".to_string(),
                "suspicious".to_string(),
                "sad".to_string(),
            ]),
            available_actions: Some(vec![
                "hammers on anvil".to_string(),
                "looks away".to_string(),
            ]),
        }
    }

    /// Mysterious merchant with hidden agenda (Shapeshifter archetype).
    pub fn mysterious_merchant() -> Self {
        Self {
            character_id: None,
            name: "Vera Nightshade".to_string(),
            archetype: "Shapeshifter".to_string(),
            current_mood: Some("Calm".to_string()),
            disposition_toward_player: Some("Friendly".to_string()),
            motivations: Some(MotivationsContext {
                known: vec![MotivationEntry {
                    description: "Sell exotic goods to travelers".to_string(),
                    priority: 2,
                    intensity: "Moderate".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                }],
                suspected: vec![],
                secret: vec![SecretMotivationEntry {
                    description: "Locate and acquire the Shadowheart Stone for her employers"
                        .to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: Some("The Old Mill".to_string()),
                    helpers: vec![],
                    opponents: vec![],
                    sender: None,
                    receiver: None,
                    deflection_behavior: "Redirects conversation to merchandise".to_string(),
                    tells: vec!["Eyes flick toward the mill when artifacts are mentioned".to_string()],
                }],
            }),
            social_stance: None,
            relationship_to_player: Some("Stranger".to_string()),
            available_expressions: Some(vec![
                "neutral".to_string(),
                "charming".to_string(),
                "calculating".to_string(),
            ]),
            available_actions: Some(vec![
                "adjusts her wares".to_string(),
                "smiles knowingly".to_string(),
            ]),
        }
    }

    /// Hostile guard suspicious of strangers.
    pub fn hostile_guard() -> Self {
        Self {
            character_id: None,
            name: "Guard Thorne".to_string(),
            archetype: "Threshold Guardian".to_string(),
            current_mood: Some("Angry".to_string()),
            disposition_toward_player: Some("Hostile".to_string()),
            motivations: Some(MotivationsContext {
                known: vec![MotivationEntry {
                    description: "Protect the town from outsiders".to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                }],
                suspected: vec![],
                secret: vec![],
            }),
            social_stance: None,
            relationship_to_player: Some("Stranger".to_string()),
            available_expressions: Some(vec![
                "neutral".to_string(),
                "suspicious".to_string(),
                "angry".to_string(),
            ]),
            available_actions: Some(vec![
                "crosses arms".to_string(),
                "blocks the path".to_string(),
            ]),
        }
    }

    /// Traumatized witness with crucial information (Shadow archetype).
    pub fn traumatized_witness() -> Self {
        Self {
            character_id: None,
            name: "Old Tom".to_string(),
            archetype: "Shadow".to_string(),
            current_mood: Some("Anxious".to_string()),
            disposition_toward_player: Some("Dismissive".to_string()),
            motivations: Some(MotivationsContext {
                known: vec![],
                suspected: vec![MotivationEntry {
                    description: "Haunted by something that happened at the mill".to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                }],
                secret: vec![SecretMotivationEntry {
                    description: "Witnessed a dark ritual that claimed his family".to_string(),
                    priority: 1,
                    intensity: "Overwhelming".to_string(),
                    target: Some("The Old Mill".to_string()),
                    helpers: vec![],
                    opponents: vec![],
                    sender: None,
                    receiver: None,
                    deflection_behavior: "Mutters incoherently, avoids eye contact".to_string(),
                    tells: vec!["Trembles at mention of the mill".to_string()],
                }],
            }),
            social_stance: None,
            relationship_to_player: Some("Stranger".to_string()),
            available_expressions: Some(vec![
                "fearful".to_string(),
                "distant".to_string(),
                "haunted".to_string(),
            ]),
            available_actions: Some(vec![
                "stares into distance".to_string(),
                "wrings hands".to_string(),
            ]),
        }
    }

    /// Helpful priest who gives quests (Herald archetype).
    pub fn quest_giving_priest() -> Self {
        Self {
            character_id: None,
            name: "Brother Aldric".to_string(),
            archetype: "Herald".to_string(),
            current_mood: Some("Calm".to_string()),
            disposition_toward_player: Some("Respectful".to_string()),
            motivations: Some(MotivationsContext {
                known: vec![MotivationEntry {
                    description: "Protect the village from the growing darkness".to_string(),
                    priority: 1,
                    intensity: "Strong".to_string(),
                    target: None,
                    helpers: vec![],
                    opponents: vec![],
                }],
                suspected: vec![],
                secret: vec![],
            }),
            social_stance: None,
            relationship_to_player: Some("Acquaintance".to_string()),
            available_expressions: Some(vec![
                "serene".to_string(),
                "concerned".to_string(),
                "hopeful".to_string(),
            ]),
            available_actions: Some(vec![
                "clasps hands in prayer".to_string(),
                "makes holy symbol".to_string(),
            ]),
        }
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    pub fn with_character_id(mut self, id: &str) -> Self {
        self.character_id = Some(id.to_string());
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_archetype(mut self, archetype: &str) -> Self {
        self.archetype = archetype.to_string();
        self
    }

    pub fn with_mood(mut self, mood: &str) -> Self {
        self.current_mood = Some(mood.to_string());
        self
    }

    pub fn with_disposition(mut self, disposition: &str) -> Self {
        self.disposition_toward_player = Some(disposition.to_string());
        self
    }

    pub fn with_relationship(mut self, relationship: &str) -> Self {
        self.relationship_to_player = Some(relationship.to_string());
        self
    }

    pub fn with_expressions(mut self, expressions: Vec<&str>) -> Self {
        self.available_expressions = Some(expressions.into_iter().map(String::from).collect());
        self
    }

    pub fn with_actions(mut self, actions: Vec<&str>) -> Self {
        self.available_actions = Some(actions.into_iter().map(String::from).collect());
        self
    }

    pub fn with_known_motivation(mut self, description: &str, priority: u32, intensity: &str) -> Self {
        let entry = MotivationEntry {
            description: description.to_string(),
            priority,
            intensity: intensity.to_string(),
            target: None,
            helpers: vec![],
            opponents: vec![],
        };
        match &mut self.motivations {
            Some(m) => m.known.push(entry),
            None => {
                self.motivations = Some(MotivationsContext {
                    known: vec![entry],
                    suspected: vec![],
                    secret: vec![],
                });
            }
        }
        self
    }

    pub fn with_secret_motivation(
        mut self,
        description: &str,
        priority: u32,
        intensity: &str,
        deflection: &str,
        tells: Vec<&str>,
    ) -> Self {
        let entry = SecretMotivationEntry {
            description: description.to_string(),
            priority,
            intensity: intensity.to_string(),
            target: None,
            helpers: vec![],
            opponents: vec![],
            sender: None,
            receiver: None,
            deflection_behavior: deflection.to_string(),
            tells: tells.into_iter().map(String::from).collect(),
        };
        match &mut self.motivations {
            Some(m) => m.secret.push(entry),
            None => {
                self.motivations = Some(MotivationsContext {
                    known: vec![],
                    suspected: vec![],
                    secret: vec![entry],
                });
            }
        }
        self
    }

    pub fn build(self) -> CharacterContext {
        CharacterContext {
            character_id: self.character_id,
            name: self.name,
            archetype: self.archetype,
            current_mood: self.current_mood,
            disposition_toward_player: self.disposition_toward_player,
            motivations: self.motivations,
            social_stance: self.social_stance,
            relationship_to_player: self.relationship_to_player,
            available_expressions: self.available_expressions,
            available_actions: self.available_actions,
        }
    }
}

// =============================================================================
// Game Prompt Builder
// =============================================================================

/// Builder for constructing `GamePromptRequest` for tests.
///
/// Combines scene context, character context, and player action for dialogue testing.
#[derive(Debug, Clone)]
pub struct GamePromptBuilder {
    world_id: Option<String>,
    scene_context: SceneContext,
    responding_character: CharacterContext,
    player_action: PlayerActionContext,
    directorial_notes: String,
    conversation_history: Vec<ConversationTurn>,
}

impl Default for GamePromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GamePromptBuilder {
    pub fn new() -> Self {
        Self {
            world_id: None,
            scene_context: SceneContextBuilder::new().build(),
            responding_character: CharacterContextBuilder::new().build(),
            player_action: PlayerActionContext {
                action_type: "talk".to_string(),
                target: None,
                dialogue: None,
            },
            directorial_notes: String::new(),
            conversation_history: vec![],
        }
    }

    pub fn with_world_id(mut self, id: &str) -> Self {
        self.world_id = Some(id.to_string());
        self
    }

    pub fn with_scene(mut self, scene: SceneContext) -> Self {
        self.scene_context = scene;
        self
    }

    pub fn with_scene_builder(mut self, builder: SceneContextBuilder) -> Self {
        self.scene_context = builder.build();
        self
    }

    pub fn with_character(mut self, character: CharacterContext) -> Self {
        self.responding_character = character;
        self
    }

    pub fn with_character_builder(mut self, builder: CharacterContextBuilder) -> Self {
        self.responding_character = builder.build();
        self
    }

    pub fn with_player_dialogue(mut self, target: &str, dialogue: &str) -> Self {
        self.player_action = PlayerActionContext {
            action_type: "talk".to_string(),
            target: Some(target.to_string()),
            dialogue: Some(dialogue.to_string()),
        };
        self
    }

    pub fn with_player_action(mut self, action_type: &str, target: Option<&str>) -> Self {
        self.player_action = PlayerActionContext {
            action_type: action_type.to_string(),
            target: target.map(String::from),
            dialogue: None,
        };
        self
    }

    pub fn with_directorial_notes(mut self, notes: &str) -> Self {
        self.directorial_notes = notes.to_string();
        self
    }

    pub fn with_conversation_turn(mut self, speaker: &str, text: &str) -> Self {
        self.conversation_history.push(ConversationTurn {
            speaker: speaker.to_string(),
            text: text.to_string(),
        });
        self
    }

    pub fn with_conversation_history(mut self, history: Vec<(&str, &str)>) -> Self {
        self.conversation_history = history
            .into_iter()
            .map(|(speaker, text)| ConversationTurn {
                speaker: speaker.to_string(),
                text: text.to_string(),
            })
            .collect();
        self
    }

    pub fn build(self) -> GamePromptRequest {
        GamePromptRequest {
            world_id: self.world_id,
            player_action: self.player_action,
            scene_context: self.scene_context,
            directorial_notes: self.directorial_notes,
            conversation_history: self.conversation_history,
            responding_character: self.responding_character,
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        }
    }

    /// Build the prompt and convert to an LLM request.
    ///
    /// This creates a properly formatted LlmRequest from the GamePromptRequest,
    /// suitable for sending to the LLM for dialogue generation.
    pub fn build_llm_request(self) -> crate::infrastructure::ports::LlmRequest {
        let prompt = self.build();

        let system_prompt = format!(
            "You are roleplaying as {} ({}) in a fantasy TTRPG.\n\n\
            Scene: {} at {} ({})\n\
            Present characters: {}\n\n\
            Your current mood: {}\n\
            Your disposition toward the player: {}\n\
            {}\n\n\
            Respond in character. Keep responses concise (1-3 sentences).\n\
            You may include expression markers like *happy* or action markers like *crosses arms*.",
            prompt.responding_character.name,
            prompt.responding_character.archetype,
            prompt.scene_context.scene_name,
            prompt.scene_context.location_name,
            prompt.scene_context.time_context,
            prompt.scene_context.present_characters.join(", "),
            prompt
                .responding_character
                .current_mood
                .as_deref()
                .unwrap_or("Neutral"),
            prompt
                .responding_character
                .disposition_toward_player
                .as_deref()
                .unwrap_or("Neutral"),
            if prompt.directorial_notes.is_empty() {
                String::new()
            } else {
                format!("\nDirectorial notes: {}", prompt.directorial_notes)
            }
        );

        let user_message = if let Some(ref dialogue) = prompt.player_action.dialogue {
            format!(
                "The player character says to {}: \"{}\"",
                prompt.player_action.target.as_deref().unwrap_or("you"),
                dialogue
            )
        } else {
            format!(
                "The player character performs action '{}' targeting {}",
                prompt.player_action.action_type,
                prompt.player_action.target.as_deref().unwrap_or("you")
            )
        };

        crate::infrastructure::ports::LlmRequest::new(vec![
            crate::infrastructure::ports::ChatMessage::user(&user_message),
        ])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7)
        .with_max_tokens(Some(500))
    }
}

// =============================================================================
// Tests
// =============================================================================

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

    #[test]
    fn test_scene_context_builder_presets() {
        let tavern = SceneContextBuilder::tavern_evening().build();
        assert_eq!(tavern.time_context, "Evening");
        assert!(tavern.location_name.contains("Drowsy Dragon"));

        let market = SceneContextBuilder::marketplace_morning().build();
        assert_eq!(market.time_context, "Morning");
        assert!(market.location_name.contains("Square"));
    }

    #[test]
    fn test_scene_context_builder_customization() {
        let scene = SceneContextBuilder::tavern_evening()
            .with_present_character("Player")
            .with_present_character("Marta")
            .with_item("Mysterious Scroll", Some("A sealed scroll"), Some("Quest"))
            .build();

        assert_eq!(scene.present_characters.len(), 2);
        assert_eq!(scene.region_items.len(), 1);
        assert_eq!(scene.region_items[0].name, "Mysterious Scroll");
    }

    #[test]
    fn test_character_context_builder_presets() {
        let innkeeper = CharacterContextBuilder::friendly_innkeeper().build();
        assert_eq!(innkeeper.name, "Marta Hearthwood");
        assert_eq!(innkeeper.archetype, "Mentor");
        assert_eq!(
            innkeeper.disposition_toward_player,
            Some("Friendly".to_string())
        );

        let guard = CharacterContextBuilder::hostile_guard().build();
        assert_eq!(guard.disposition_toward_player, Some("Hostile".to_string()));
        assert_eq!(guard.current_mood, Some("Angry".to_string()));
    }

    #[test]
    fn test_character_context_builder_customization() {
        let character = CharacterContextBuilder::new()
            .with_name("Custom NPC")
            .with_archetype("Trickster")
            .with_mood("Mischievous")
            .with_disposition("Playful")
            .with_known_motivation("Cause harmless chaos", 1, "Moderate")
            .build();

        assert_eq!(character.name, "Custom NPC");
        assert_eq!(character.archetype, "Trickster");
        assert!(character.motivations.is_some());
        assert_eq!(character.motivations.unwrap().known.len(), 1);
    }

    #[test]
    fn test_game_prompt_builder() {
        let prompt = GamePromptBuilder::new()
            .with_scene_builder(
                SceneContextBuilder::tavern_evening().with_present_character("Player"),
            )
            .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
            .with_player_dialogue("Marta", "What news do you have?")
            .with_directorial_notes("Be welcoming but hint at danger")
            .build();

        assert_eq!(prompt.responding_character.name, "Marta Hearthwood");
        assert_eq!(
            prompt.player_action.dialogue,
            Some("What news do you have?".to_string())
        );
        assert!(prompt.directorial_notes.contains("welcoming"));
    }

    #[test]
    fn test_game_prompt_builder_to_llm_request() {
        let request = GamePromptBuilder::new()
            .with_scene_builder(SceneContextBuilder::tavern_evening())
            .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
            .with_player_dialogue("Marta", "Hello!")
            .build_llm_request();

        assert!(request.system_prompt.is_some());
        assert_eq!(request.messages.len(), 1);
        let system = request.system_prompt.unwrap();
        assert!(system.contains("Marta Hearthwood"));
        assert!(system.contains("Mentor"));
    }
}
