//! LLM response parser for structured dialogue output.
//!
//! Parses XML-tagged content from LLM responses:
//! - `<reasoning>` - Internal thoughts (shown to DM only)
//! - `<dialogue>` - NPC spoken response
//! - `<topics>` - Discussed topics
//! - `<challenge_suggestion>` - Suggested challenge trigger
//! - `<narrative_event_suggestion>` - Suggested narrative event trigger
//! - `<tool name="...">` - Game tool calls (give_item, change_relationship, etc.)
//!
//! See `prompt_templates.rs` for the expected output format.
//!
//! # Parse Error Policy
//!
//! The legacy parser uses **fail-fast semantics** consistent with the phased pipeline:
//!
//! - **Critical fields** (dialogue): Missing or empty dialogue is a hard parse error.
//!   The queue item will be marked failed and the error propagated.
//!
//! - **Non-critical fields** (reasoning, topics, suggestions, tools): Best-effort parsing
//!   with warnings logged. Partial output is returned for these fields.
//!
//! This ensures that unusable LLM responses (no NPC dialogue) don't silently create
//! empty approval requests, while allowing flexibility for optional content.

use regex_lite::Regex;
use serde::Deserialize;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::game_tools::ToolArgs;
use crate::queue_types::ProposedTool;

/// Generate a deterministic tool ID based on tool name and arguments.
///
/// This uses UUIDv5 with the OID namespace and a hash of the tool name
/// concatenated with the JSON-serialized arguments. This ensures that
/// identical tool calls (same name + same args) always produce the same ID,
/// which is critical for:
/// - Idempotent approval hydration
/// - Consistent behavior between legacy and phased parsers
/// - Retry safety (same tool call = same ID)
///
/// # Arguments
/// * `name` - The tool name (e.g., "give_item")
/// * `args` - The tool arguments as a JSON string
///
/// # Returns
/// A deterministic UUIDv5 string that uniquely identifies this tool call.
pub fn generate_tool_id(name: &str, args: &str) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("{}|{}", name, args).as_bytes(),
    )
    .to_string()
}

/// Errors that can occur during legacy LLM response parsing.
///
/// These errors provide structured context about what failed during parsing,
/// enabling proper error handling and queue failure semantics.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LegacyParseError {
    /// The response contains no usable dialogue content.
    ///
    /// This is a critical error because NPC dialogue is the primary output
    /// of the LLM request. Without dialogue, the approval request is unusable.
    #[error("Missing or empty <dialogue> content in LLM response")]
    MissingDialogue,

    /// The response format is unrecognizable.
    ///
    /// The response doesn't match expected XML tag format or plain text fallback.
    #[error("Unrecognized response format: {details}")]
    UnrecognizedFormat { details: String },

    /// A required XML tag is malformed or cannot be parsed.
    #[error("Malformed tag <{tag}>: {details}")]
    MalformedTag { tag: String, details: String },

    /// Generic parse error with context.
    #[error("Parse error: {message}")]
    Other { message: String },
}

impl LegacyParseError {
    /// Create a new generic parse error.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }

    /// Create an unrecognized format error.
    pub fn unrecognized_format(details: impl Into<String>) -> Self {
        Self::UnrecognizedFormat {
            details: details.into(),
        }
    }

    /// Create a malformed tag error.
    pub fn malformed_tag(tag: impl Into<String>, details: impl Into<String>) -> Self {
        Self::MalformedTag {
            tag: tag.into(),
            details: details.into(),
        }
    }
}

/// Parsed components from an LLM dialogue response.
#[derive(Debug, Clone, Default)]
pub struct ParsedLlmResponse {
    /// Internal reasoning (shown to DM, hidden from player)
    pub reasoning: String,
    /// The NPC's spoken dialogue
    pub dialogue: String,
    /// Topics discussed in this exchange
    pub topics: Vec<String>,
    /// Suggested challenge to trigger (if any) - from explicit JSON block (legacy)
    pub challenge_suggestion: Option<RawChallengeSuggestion>,
    /// Challenge names marked as "trigger: YES" in <challenge> tags
    pub triggered_challenge_names: Vec<String>,
    /// Suggested narrative event to trigger (if any) - from explicit JSON block (legacy)
    pub narrative_event_suggestion: Option<RawNarrativeEventSuggestion>,
    /// Event names marked as "trigger: YES" in <event> tags
    pub triggered_event_names: Vec<String>,
    /// Proposed game tools (give_item, change_relationship, etc.)
    pub proposed_tools: Vec<ProposedTool>,
}

/// Raw challenge suggestion as parsed from LLM JSON.
/// This needs to be enriched with challenge metadata before use.
#[derive(Debug, Clone, Deserialize)]
pub struct RawChallengeSuggestion {
    pub challenge_id: String,
    pub confidence: String,
    #[serde(default)]
    pub reasoning: String,
}

/// Raw narrative event suggestion as parsed from LLM JSON.
/// This needs to be enriched with event metadata before use.
#[derive(Debug, Clone, Deserialize)]
pub struct RawNarrativeEventSuggestion {
    pub event_id: String,
    pub confidence: String,
    #[serde(default)]
    pub reasoning: String,
    #[serde(default)]
    pub matched_triggers: Vec<String>,
}

/// A challenge analyzed by the LLM from <challenge> tags.
#[derive(Debug, Clone)]
pub struct AnalyzedChallenge {
    /// The challenge name (from the name attribute)
    pub name: String,
    /// What the player said that matched (if any)
    pub quote: Option<String>,
    /// Whether the LLM determined it should trigger
    pub should_trigger: bool,
}

/// A narrative event analyzed by the LLM from <event> tags.
#[derive(Debug, Clone)]
pub struct AnalyzedEvent {
    /// The event name (from the name attribute)
    pub name: String,
    /// What the player said that matched (if any)
    pub quote: Option<String>,
    /// Whether the LLM determined it should trigger
    pub should_trigger: bool,
}

// Compiled regexes for tag extraction
static REASONING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<reasoning>(.*?)</reasoning>").expect("valid regex"));
static DIALOGUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<dialogue>(.*?)</dialogue>").expect("valid regex"));
static TOPICS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<topics>(.*?)</topics>").expect("valid regex"));
static CHALLENGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<challenge_suggestion>(.*?)</challenge_suggestion>").expect("valid regex")
});
static NARRATIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<narrative_event_suggestion>(.*?)</narrative_event_suggestion>")
        .expect("valid regex")
});
static SUGGESTED_BEATS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<suggested_beats>(.*?)</suggested_beats>").expect("valid regex")
});
// Matches <challenge name="Challenge Name">...</challenge> with nested tags
static CHALLENGE_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<challenge\s+name="([^"]+)">(.*?)</challenge>"#).expect("valid regex")
});
// Matches <event name="Event Name">...</event> with nested tags
static EVENT_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<event\s+name="([^"]+)">(.*?)</event>"#).expect("valid regex")
});
// Extracts <quote>...</quote> content
static QUOTE_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<quote>(.*?)</quote>").expect("valid regex"));
// Extracts <trigger>...</trigger> content
static TRIGGER_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<trigger>(.*?)</trigger>").expect("valid regex"));
// Matches <tool name="tool_name">JSON arguments</tool>
static TOOL_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<tool\s+name="([^"]+)">(.*?)</tool>"#).expect("valid regex")
});

// Regex to remove model-specific special tokens (e.g., from gpt-oss, llama, etc.)
static SPECIAL_TOKENS_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Match various model special token patterns:
    // - <|...|> style tokens (common in many models)
    // - [INST], [/INST] tokens (llama)
    // - <<SYS>>, <</SYS>> tokens (llama)
    Regex::new(r"<\|[^|>]+\|>|\[/?INST\]|<</?SYS>>").expect("valid regex")
});

// Regex to extract final content from gpt-oss style responses
// Pattern: <|channel|>analysis<|message|>...thinking...<|end|><|start|>assistant<|channel|>final<|message|>ACTUAL CONTENT
static FINAL_CONTENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<\|channel\|>final<\|message\|>(.*)$").expect("valid regex"));

/// Remove model-specific special tokens that may leak through from LLM output.
///
/// Handles patterns like:
/// - gpt-oss: `<|channel|>analysis<|message|>...<|end|><|start|>assistant<|channel|>final<|message|>CONTENT`
///   -> extracts just CONTENT after the final marker
/// - Individual tokens: `<|channel|>`, `<|message|>`, `<|end|>`, `<|start|>`
/// - Llama tokens: `[INST]`, `[/INST]`, `<<SYS>>`, `<</SYS>>`
pub fn strip_special_tokens(raw: &str) -> String {
    // First, try to extract content after <|channel|>final<|message|> marker
    // This handles the gpt-oss pattern where analysis content precedes final content
    if let Some(caps) = FINAL_CONTENT_RE.captures(raw) {
        if let Some(content) = caps.get(1) {
            let extracted = content.as_str().trim();
            // Still strip any remaining tokens from the extracted content
            return SPECIAL_TOKENS_RE.replace_all(extracted, "").to_string();
        }
    }

    // Fallback: just strip all special tokens
    SPECIAL_TOKENS_RE.replace_all(raw, "").to_string()
}

// Regex for plain headings fallback (when LLM outputs "reasoning" or "dialogue" without XML tags)
static PLAIN_REASONING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^\s*reasoning\s*[:\-]?\s*\n(.*?)\n\s*dialogue\s*[:\-]?").expect("valid regex")
});
static PLAIN_DIALOGUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^\s*dialogue\s*[:\-]?\s*\n(.*?)(?:\n\s*(?:topics|suggested_beats|challenge|event)\s*[:\-]|\z)").expect("valid regex")
});

/// Parse an LLM response into its structured components.
///
/// If the response contains explicit `<dialogue>` tags, those are used.
/// Otherwise, attempts to parse plain "reasoning"/"dialogue" headings as fallback.
/// If neither format is detected, the entire response (minus other tags) is treated as dialogue.
///
/// # Errors
///
/// Returns `LegacyParseError::MissingDialogue` if no usable dialogue content is found.
/// This is a critical error that should cause the queue item to be marked failed.
///
/// Other parsing issues (invalid JSON in suggestions, malformed tags) are logged as
/// warnings but do not cause parse failure - best-effort output is returned.
pub fn parse_llm_response(raw: &str) -> Result<ParsedLlmResponse, LegacyParseError> {
    // First, strip any model-specific special tokens that may have leaked through
    let cleaned = strip_special_tokens(raw);
    let raw = cleaned.as_str();

    let mut result = ParsedLlmResponse::default();

    // Extract reasoning - prefer XML tags, fallback to plain headings
    if let Some(caps) = REASONING_RE.captures(raw) {
        result.reasoning = caps[1].trim().to_string();
    } else if let Some(caps) = PLAIN_REASONING_RE.captures(raw) {
        // Fallback: parse plain "reasoning" heading
        result.reasoning = caps[1].trim().to_string();
    }

    // Extract dialogue - prefer XML tags, fallback to plain headings, then strip_all_tags
    if let Some(caps) = DIALOGUE_RE.captures(raw) {
        result.dialogue = caps[1].trim().to_string();
    } else if let Some(caps) = PLAIN_DIALOGUE_RE.captures(raw) {
        // Fallback: parse plain "dialogue" heading
        result.dialogue = caps[1].trim().to_string();
    } else {
        // No explicit dialogue tag - strip all other tags and use what remains
        result.dialogue = strip_all_tags(raw);
    }

    // Critical field validation: dialogue must not be empty
    // This is a hard error - without dialogue, the approval request is unusable
    if result.dialogue.is_empty() {
        return Err(LegacyParseError::MissingDialogue);
    }

    // Extract topics (one per line)
    if let Some(caps) = TOPICS_RE.captures(raw) {
        result.topics = caps[1]
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();
    }

    // Extract challenge suggestion JSON (best-effort)
    if let Some(caps) = CHALLENGE_RE.captures(raw) {
        let json_str = caps[1].trim();
        match serde_json::from_str::<RawChallengeSuggestion>(json_str) {
            Ok(suggestion) => result.challenge_suggestion = Some(suggestion),
            Err(e) => {
                tracing::warn!(
                    json = json_str,
                    error = %e,
                    "Failed to parse challenge_suggestion JSON (best-effort, continuing)"
                );
            }
        }
    }

    // Extract narrative event suggestion JSON (best-effort)
    if let Some(caps) = NARRATIVE_RE.captures(raw) {
        let json_str = caps[1].trim();
        match serde_json::from_str::<RawNarrativeEventSuggestion>(json_str) {
            Ok(suggestion) => result.narrative_event_suggestion = Some(suggestion),
            Err(e) => {
                tracing::warn!(
                    json = json_str,
                    error = %e,
                    "Failed to parse narrative_event_suggestion JSON (best-effort, continuing)"
                );
            }
        }
    }

    // Extract triggered challenges from <challenge> tags (best-effort)
    result.triggered_challenge_names = get_triggered_challenge_names(raw);

    // Extract triggered events from <event> tags (best-effort)
    result.triggered_event_names = get_triggered_event_names(raw);

    // Extract proposed tools from <tool> tags (best-effort)
    result.proposed_tools = parse_tool_tags(raw);

    Ok(result)
}

/// Parse all <challenge name="...">...</challenge> tags from the response.
///
/// Returns a list of analyzed challenges with their trigger status.
pub fn parse_challenge_tags(raw: &str) -> Vec<AnalyzedChallenge> {
    let mut results = Vec::new();

    for caps in CHALLENGE_TAG_RE.captures_iter(raw) {
        let name = caps[1].trim().to_string();
        let inner_content = &caps[2];

        // Extract <quote>...</quote>
        let quote = QUOTE_TAG_RE
            .captures(inner_content)
            .map(|c| c[1].trim().to_string())
            .filter(|q| !q.eq_ignore_ascii_case("none") && !q.is_empty());

        // Extract <trigger>...</trigger>
        let should_trigger = TRIGGER_TAG_RE
            .captures(inner_content)
            .map(|c| c[1].trim().eq_ignore_ascii_case("YES"))
            .unwrap_or(false);

        results.push(AnalyzedChallenge {
            name,
            quote,
            should_trigger,
        });
    }

    results
}

/// Get the names of challenges that should be triggered.
pub fn get_triggered_challenge_names(raw: &str) -> Vec<String> {
    parse_challenge_tags(raw)
        .into_iter()
        .filter(|c| c.should_trigger)
        .map(|c| c.name)
        .collect()
}

/// Parse all <event name="...">...</event> tags from the response.
///
/// Returns a list of analyzed events with their trigger status.
pub fn parse_event_tags(raw: &str) -> Vec<AnalyzedEvent> {
    let mut results = Vec::new();

    for caps in EVENT_TAG_RE.captures_iter(raw) {
        let name = caps[1].trim().to_string();
        let inner_content = &caps[2];

        // Extract <quote>...</quote>
        let quote = QUOTE_TAG_RE
            .captures(inner_content)
            .map(|c| c[1].trim().to_string())
            .filter(|q| !q.eq_ignore_ascii_case("none") && !q.is_empty());

        // Extract <trigger>...</trigger>
        let should_trigger = TRIGGER_TAG_RE
            .captures(inner_content)
            .map(|c| c[1].trim().eq_ignore_ascii_case("YES"))
            .unwrap_or(false);

        results.push(AnalyzedEvent {
            name,
            quote,
            should_trigger,
        });
    }

    results
}

/// Get the names of events that should be triggered.
pub fn get_triggered_event_names(raw: &str) -> Vec<String> {
    parse_event_tags(raw)
        .into_iter()
        .filter(|e| e.should_trigger)
        .map(|e| e.name)
        .collect()
}

/// Validate that a quote is a substring of the player dialogue.
///
/// Returns true if the quote is "none" (case-insensitive) or if the quote
/// is found as a substring within the player dialogue.
pub fn is_valid_quote(quote: &str, player_dialogue: &str) -> bool {
    if quote.eq_ignore_ascii_case("none") {
        return true;
    }
    let quote_lower = quote.to_lowercase();
    let player_lower = player_dialogue.to_lowercase();
    player_lower.contains(&quote_lower)
}

/// Filter triggered challenges to only those with valid quotes.
///
/// If a challenge is marked as triggered but its quote is not a substring
/// of the player dialogue, it is demoted to not triggered.
pub fn filter_valid_challenge_triggers(
    challenges: Vec<AnalyzedChallenge>,
    player_dialogue: &str,
) -> Vec<AnalyzedChallenge> {
    challenges
        .into_iter()
        .map(|mut c| {
            if c.should_trigger {
                if let Some(ref quote) = c.quote {
                    if !is_valid_quote(quote, player_dialogue) {
                        tracing::debug!(
                            challenge = %c.name,
                            quote = %quote,
                            "Demoting challenge trigger to NO: quote not found in player dialogue"
                        );
                        c.should_trigger = false;
                    }
                } else {
                    // No quote provided but trigger is YES - demote to NO
                    tracing::debug!(
                        challenge = %c.name,
                        "Demoting challenge trigger to NO: no quote provided"
                    );
                    c.should_trigger = false;
                }
            }
            c
        })
        .collect()
}

/// Filter triggered events to only those with valid quotes.
///
/// If an event is marked as triggered but its quote is not a substring
/// of the player dialogue, it is demoted to not triggered.
pub fn filter_valid_event_triggers(
    events: Vec<AnalyzedEvent>,
    player_dialogue: &str,
) -> Vec<AnalyzedEvent> {
    events
        .into_iter()
        .map(|mut e| {
            if e.should_trigger {
                if let Some(ref quote) = e.quote {
                    if !is_valid_quote(quote, player_dialogue) {
                        tracing::debug!(
                            event = %e.name,
                            quote = %quote,
                            "Demoting event trigger to NO: quote not found in player dialogue"
                        );
                        e.should_trigger = false;
                    }
                } else {
                    // No quote provided but trigger is YES - demote to NO
                    tracing::debug!(
                        event = %e.name,
                        "Demoting event trigger to NO: no quote provided"
                    );
                    e.should_trigger = false;
                }
            }
            e
        })
        .collect()
}

/// Parse all `<tool name="...">JSON</tool>` tags from the response.
///
/// Returns a list of ProposedTool structs ready for DM approval.
/// Each tool tag should contain valid JSON arguments.
///
/// Example:
/// ```xml
/// <tool name="give_item">
/// {"item_name": "Healing Potion", "description": "A red potion"}
/// </tool>
/// ```
pub fn parse_tool_tags(raw: &str) -> Vec<ProposedTool> {
    let mut results = Vec::new();

    for caps in TOOL_TAG_RE.captures_iter(raw) {
        let name = caps[1].trim().to_string();
        let json_str = caps[2].trim();

        // Parse the JSON arguments into typed ToolArgs
        let arguments: ToolArgs = match serde_json::from_str(json_str) {
            Ok(args) => args,
            Err(e) => {
                tracing::error!(
                    tool_name = %name,
                    json = json_str,
                    error = %e,
                    "Failed to parse tool arguments JSON, skipping tool"
                );
                continue;
            }
        };

        // Generate a deterministic ID for this tool call using the original JSON string
        // to avoid serialization failures and ensure stable ID derivation
        let id = generate_tool_id(&name, json_str);

        // Generate human-readable description using ToolArgs
        let description = arguments.description();

        results.push(ProposedTool {
            id,
            name,
            description,
            arguments,
        });
    }

    results
}

/// Strip all known XML tags from a response, leaving just plain dialogue.
fn strip_all_tags(raw: &str) -> String {
    let mut result = raw.to_string();

    // Remove all known tags and their contents (except dialogue)
    result = REASONING_RE.replace_all(&result, "").to_string();
    result = TOPICS_RE.replace_all(&result, "").to_string();
    result = CHALLENGE_RE.replace_all(&result, "").to_string();
    result = NARRATIVE_RE.replace_all(&result, "").to_string();
    result = SUGGESTED_BEATS_RE.replace_all(&result, "").to_string();
    result = CHALLENGE_TAG_RE.replace_all(&result, "").to_string();
    result = EVENT_TAG_RE.replace_all(&result, "").to_string();
    result = TOOL_TAG_RE.replace_all(&result, "").to_string();

    // Clean up whitespace
    result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_response() {
        let raw = r#"
<reasoning>
The player is asking about the artifact. I should be mysterious but helpful.
This matches the hidden_artifact challenge trigger.
</reasoning>

<dialogue>
*narrows eyes* "You seek the Heartstone?" *suspicious* "Many have asked, few have survived the asking."
</dialogue>

<topics>
heartstone
quest_information
warning
</topics>

<challenge_suggestion>
{"challenge_id": "hidden_artifact", "confidence": "high", "reasoning": "Player explicitly asked about artifact"}
</challenge_suggestion>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.reasoning.contains("asking about the artifact"));
        assert!(parsed.dialogue.contains("Heartstone"));
        assert!(parsed.dialogue.contains("*narrows eyes*"));
        assert_eq!(parsed.topics.len(), 3);
        assert!(parsed.topics.contains(&"heartstone".to_string()));

        let challenge = parsed.challenge_suggestion.unwrap();
        assert_eq!(challenge.challenge_id, "hidden_artifact");
        assert_eq!(challenge.confidence, "high");
    }

    #[test]
    fn test_parse_no_tags() {
        let raw = r#"*smiles warmly* "Welcome to my shop, traveler. What brings you here today?""#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.reasoning.is_empty());
        assert_eq!(parsed.dialogue, raw);
        assert!(parsed.topics.is_empty());
        assert!(parsed.challenge_suggestion.is_none());
        assert!(parsed.narrative_event_suggestion.is_none());
    }

    #[test]
    fn test_parse_narrative_event_suggestion() {
        let raw = r#"
<dialogue>
"The curse has been broken at last!"
</dialogue>

<narrative_event_suggestion>
{"event_id": "curse_lifted", "confidence": "high", "reasoning": "Player completed the ritual", "matched_triggers": ["ritual_complete", "has_artifact"]}
</narrative_event_suggestion>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.dialogue.contains("curse has been broken"));

        let event = parsed.narrative_event_suggestion.unwrap();
        assert_eq!(event.event_id, "curse_lifted");
        assert_eq!(event.matched_triggers.len(), 2);
    }

    #[test]
    fn test_strip_tags_fallback() {
        let raw = r#"
<reasoning>
Internal thoughts here
</reasoning>

Hello traveler, welcome to the inn!

<topics>
greeting
</topics>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        // Should extract just the dialogue text
        assert_eq!(parsed.dialogue, "Hello traveler, welcome to the inn!");
    }

    #[test]
    fn test_invalid_json_gracefully_handled() {
        let raw = r#"
<dialogue>
"Hello there!"
</dialogue>

<challenge_suggestion>
{not valid json}
</challenge_suggestion>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.dialogue.contains("Hello there"));
        // Invalid JSON should be None, not panic
        assert!(parsed.challenge_suggestion.is_none());
    }

    #[test]
    fn test_empty_topics() {
        let raw = r#"
<dialogue>
"Just passing through."
</dialogue>

<topics>
</topics>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.topics.is_empty());
    }

    #[test]
    fn test_strips_special_tokens() {
        // This simulates output from models that leak special tokens
        let raw = r#"<|end|><|start|>assistant<|channel|>final<|message|><reasoning>
I should be helpful here.
</reasoning>

<dialogue>
*friendly* "Hello there, welcome!"
</dialogue>"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.reasoning.contains("I should be helpful"));
        assert!(parsed.dialogue.contains("Hello there"));
        // No special tokens should be in the output
        assert!(!parsed.dialogue.contains("<|"));
        assert!(!parsed.reasoning.contains("<|"));
    }

    #[test]
    fn test_strips_llama_tokens() {
        let raw = r#"[INST] Some instruction [/INST]
<<SYS>> System stuff <</SYS>>
<reasoning>
Thinking about this.
</reasoning>

<dialogue>
"Greetings!"
</dialogue>"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.reasoning.contains("Thinking about this"));
        assert!(parsed.dialogue.contains("Greetings"));
        // No special tokens should be in the output
        assert!(!parsed.dialogue.contains("[INST]"));
        assert!(!parsed.dialogue.contains("<<SYS>>"));
    }

    #[test]
    fn test_parse_tool_tags() {
        let raw = r#"
<dialogue>
*hands over a potion* "Take this, you'll need it for your journey."
</dialogue>

<tool name="give_item">
{"tool": "GiveItem", "item_name": "Healing Potion", "description": "A red potion that restores health"}
</tool>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.dialogue.contains("Take this"));
        assert_eq!(parsed.proposed_tools.len(), 1);

        let tool = &parsed.proposed_tools[0];
        assert_eq!(tool.name, "give_item");
        match &tool.arguments {
            crate::game_tools::ToolArgs::GiveItem { item_name, .. } => {
                assert_eq!(item_name, "Healing Potion");
            }
            _ => panic!("Expected GiveItem tool args"),
        }
        assert!(tool.description.contains("Healing Potion"));
    }

    #[test]
    fn test_parse_multiple_tools() {
        let raw = r#"
<dialogue>
"Your help means everything to me."
</dialogue>

<tool name="change_relationship">
{"tool": "ChangeRelationship", "change": "improve", "amount": "moderate", "reason": "helped with quest"}
</tool>

<tool name="reveal_info">
{"tool": "RevealInfo", "info_type": "quest", "content": "The temple is hidden in the mountains", "importance": "major"}
</tool>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert_eq!(parsed.proposed_tools.len(), 2);
        assert_eq!(parsed.proposed_tools[0].name, "change_relationship");
        assert_eq!(parsed.proposed_tools[1].name, "reveal_info");
    }

    #[test]
    fn test_invalid_tool_json_skipped() {
        let raw = r#"
<dialogue>
"Hello there!"
</dialogue>

<tool name="give_item">
{not valid json}
</tool>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        assert!(parsed.dialogue.contains("Hello there"));
        // Invalid JSON tool should be skipped, not panic
        assert!(parsed.proposed_tools.is_empty());
    }

    #[test]
    fn test_tools_stripped_from_fallback_dialogue() {
        let raw = r#"
<reasoning>
The player deserves a reward.
</reasoning>

Here's a gift for you, brave adventurer!

<tool name="give_item">
{"tool": "GiveItem", "item_name": "Gold Coins", "description": "100 shiny gold coins"}
</tool>
"#;

        let parsed = parse_llm_response(raw).expect("parse should succeed");

        // Tool content should be stripped from dialogue
        assert_eq!(parsed.dialogue, "Here's a gift for you, brave adventurer!");
        // But tool should still be parsed
        assert_eq!(parsed.proposed_tools.len(), 1);
        assert_eq!(parsed.proposed_tools[0].name, "give_item");
    }

    // =============================================================================
    // Legacy Parse Error Tests
    // =============================================================================

    #[test]
    fn test_parse_error_missing_dialogue() {
        // Response with no dialogue content should fail
        let raw = r#"
<reasoning>
Some internal thoughts.
</reasoning>

<topics>
topic1
</topics>
"#;

        let result = parse_llm_response(raw);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            LegacyParseError::MissingDialogue => {
                // Expected error type
            }
            _ => panic!("Expected MissingDialogue error, got: {:?}", err),
        }
    }

    #[test]
    fn test_parse_error_empty_dialogue_tag() {
        // Response with empty dialogue tag should fail
        let raw = r#"
<reasoning>
Some thoughts.
</reasoning>

<dialogue>
   
</dialogue>
"#;

        let result = parse_llm_response(raw);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, LegacyParseError::MissingDialogue),
            "Expected MissingDialogue for empty dialogue tag, got: {:?}",
            err
        );
    }

    #[test]
    fn test_parse_error_empty_response() {
        // Completely empty response should fail
        let raw = "";

        let result = parse_llm_response(raw);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, LegacyParseError::MissingDialogue),
            "Expected MissingDialogue for empty response, got: {:?}",
            err
        );
    }

    #[test]
    fn test_parse_error_whitespace_only_response() {
        // Whitespace-only response should fail
        let raw = "   \n\t   \n   ";

        let result = parse_llm_response(raw);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, LegacyParseError::MissingDialogue),
            "Expected MissingDialogue for whitespace-only response, got: {:?}",
            err
        );
    }

    #[test]
    fn test_parse_error_display_format() {
        // Test that error messages are informative
        let err = LegacyParseError::MissingDialogue;
        let err_str = format!("{}", err);
        assert!(err_str.contains("dialogue"));

        let err = LegacyParseError::unrecognized_format("test details");
        let err_str = format!("{}", err);
        assert!(err_str.contains("test details"));

        let err = LegacyParseError::malformed_tag("dialogue", "unclosed");
        let err_str = format!("{}", err);
        assert!(err_str.contains("dialogue"));
        assert!(err_str.contains("unclosed"));

        let err = LegacyParseError::other("custom message");
        let err_str = format!("{}", err);
        assert!(err_str.contains("custom message"));
    }

    // =============================================================================
    // Tool ID Determinism Tests
    // =============================================================================

    #[test]
    fn test_generate_tool_id_is_deterministic() {
        // Identical tool inputs should produce identical IDs
        let name = "give_item";
        let args =
            r#"{"tool": "GiveItem", "item_name": "Healing Potion", "description": "A red potion"}"#;

        let id1 = generate_tool_id(name, args);
        let id2 = generate_tool_id(name, args);
        let id3 = generate_tool_id(name, args);

        assert_eq!(id1, id2, "Tool ID should be deterministic");
        assert_eq!(
            id2, id3,
            "Tool ID should be deterministic across multiple calls"
        );
    }

    #[test]
    fn test_generate_tool_id_different_inputs_produce_different_ids() {
        // Different tool names or args should produce different IDs
        let id1 = generate_tool_id("give_item", r#"{"item_name": "Potion"}"#);
        let id2 = generate_tool_id("give_item", r#"{"item_name": "Sword"}"#); // Different args
        let id3 = generate_tool_id("change_relationship", r#"{"item_name": "Potion"}"#); // Different name

        assert_ne!(id1, id2, "Different args should produce different IDs");
        assert_ne!(
            id1, id3,
            "Different tool names should produce different IDs"
        );
        assert_ne!(
            id2, id3,
            "Different name and args should produce different IDs"
        );
    }

    #[test]
    fn test_parsed_tools_have_deterministic_ids() {
        // Parse the same tool twice and verify IDs match
        let raw = r#"
<dialogue>
"Take this potion."
</dialogue>

<tool name="give_item">
{"tool": "GiveItem", "item_name": "Healing Potion", "description": "A red potion"}
</tool>
"#;

        let parsed1 = parse_llm_response(raw).expect("parse should succeed");
        let parsed2 = parse_llm_response(raw).expect("parse should succeed");

        assert_eq!(parsed1.proposed_tools.len(), 1);
        assert_eq!(parsed2.proposed_tools.len(), 1);

        // IDs should be identical because tool name + args are identical
        assert_eq!(
            parsed1.proposed_tools[0].id, parsed2.proposed_tools[0].id,
            "Parsed tool IDs should be deterministic for identical inputs"
        );
    }

    #[test]
    fn test_legacy_and_phased_parsers_produce_deterministic_tool_ids() {
        // Verify that both parsers produce deterministic IDs for their respective formats.
        // Both parsers now use the original JSON string for ID generation (not re-serialized)
        // to avoid serialization failures (unwrap_or_default).
        use crate::use_cases::queues::phases::parsers::parse_phase3_tools;

        // Phased parser format: JSON within <arguments> tags
        let tool_xml = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Player needs healing.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give a healing potion.</justification>
  <arguments>
  {"tool": "GiveItem", "item_name": "Healing Potion", "description": "A red potion"}
  </arguments>
</tool>
"#;

        // Parse with phased parser
        let phased_result = parse_phase3_tools(tool_xml).expect("phase3 parse should succeed");
        assert_eq!(phased_result.proposed_tools.len(), 1);
        let phased_id = &phased_result.proposed_tools[0].id;

        // Legacy parser format: JSON directly within <tool> tags
        let legacy_raw = r#"
<dialogue>"Take this."</dialogue>
<tool name="give_item">
{"tool": "GiveItem", "item_name": "Healing Potion", "description": "A red potion"}
</tool>
"#;
        let legacy_result = parse_llm_response(legacy_raw).expect("legacy parse should succeed");
        assert_eq!(legacy_result.proposed_tools.len(), 1);
        let legacy_id = &legacy_result.proposed_tools[0].id;

        // Both parsers should produce non-empty, valid IDs
        assert!(
            !phased_id.is_empty(),
            "Phased parser should produce non-empty tool ID"
        );
        assert!(
            !legacy_id.is_empty(),
            "Legacy parser should produce non-empty tool ID"
        );

        // When the raw JSON content is identical, both parsers produce the same ID
        // (the XML wrapper formatting doesn't affect the extracted JSON)
        assert_eq!(
            phased_id, legacy_id,
            "Parsers should produce identical IDs when JSON content is identical"
        );

        // Both parsers should be deterministic - same input produces same ID
        let phased_result2 = parse_phase3_tools(tool_xml).expect("phase3 parse should succeed");
        let legacy_result2 = parse_llm_response(legacy_raw).expect("legacy parse should succeed");
        assert_eq!(
            phased_result.proposed_tools[0].id, phased_result2.proposed_tools[0].id,
            "Phased parser should be deterministic"
        );
        assert_eq!(
            legacy_result.proposed_tools[0].id, legacy_result2.proposed_tools[0].id,
            "Legacy parser should be deterministic"
        );
    }
}
