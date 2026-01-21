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

use regex_lite::Regex;
use serde::Deserialize;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::game_tools::GameTool;
use crate::queue_types::ProposedTool;

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

/// Parse an LLM response into its structured components.
///
/// If the response contains explicit `<dialogue>` tags, those are used.
/// Otherwise, the entire response (minus other tags) is treated as dialogue.
pub fn parse_llm_response(raw: &str) -> ParsedLlmResponse {
    // First, strip any model-specific special tokens that may have leaked through
    let cleaned = strip_special_tokens(raw);
    let raw = cleaned.as_str();

    let mut result = ParsedLlmResponse::default();

    // Extract reasoning
    if let Some(caps) = REASONING_RE.captures(raw) {
        result.reasoning = caps[1].trim().to_string();
    }

    // Extract dialogue - if no explicit tag, use the cleaned response
    if let Some(caps) = DIALOGUE_RE.captures(raw) {
        result.dialogue = caps[1].trim().to_string();
    } else {
        // No explicit dialogue tag - strip all other tags and use what remains
        result.dialogue = strip_all_tags(raw);
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

    // Extract challenge suggestion JSON
    if let Some(caps) = CHALLENGE_RE.captures(raw) {
        let json_str = caps[1].trim();
        match serde_json::from_str::<RawChallengeSuggestion>(json_str) {
            Ok(suggestion) => result.challenge_suggestion = Some(suggestion),
            Err(e) => {
                tracing::warn!(
                    json = json_str,
                    error = %e,
                    "Failed to parse challenge_suggestion JSON"
                );
            }
        }
    }

    // Extract narrative event suggestion JSON
    if let Some(caps) = NARRATIVE_RE.captures(raw) {
        let json_str = caps[1].trim();
        match serde_json::from_str::<RawNarrativeEventSuggestion>(json_str) {
            Ok(suggestion) => result.narrative_event_suggestion = Some(suggestion),
            Err(e) => {
                tracing::warn!(
                    json = json_str,
                    error = %e,
                    "Failed to parse narrative_event_suggestion JSON"
                );
            }
        }
    }

    // Extract triggered challenges from <challenge> tags
    result.triggered_challenge_names = get_triggered_challenge_names(raw);

    // Extract triggered events from <event> tags
    result.triggered_event_names = get_triggered_event_names(raw);

    // Extract proposed tools from <tool> tags
    result.proposed_tools = parse_tool_tags(raw);

    result
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

        // Parse the JSON arguments
        let arguments: serde_json::Value = match serde_json::from_str(json_str) {
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

        // Generate a unique ID for this tool call
        let id = Uuid::new_v4().to_string();

        // Generate human-readable description using GameTool
        let description = GameTool::describe_from_json(&name, &arguments);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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
{"item_name": "Healing Potion", "description": "A red potion that restores health"}
</tool>
"#;

        let parsed = parse_llm_response(raw);

        assert!(parsed.dialogue.contains("Take this"));
        assert_eq!(parsed.proposed_tools.len(), 1);

        let tool = &parsed.proposed_tools[0];
        assert_eq!(tool.name, "give_item");
        assert_eq!(tool.arguments["item_name"], "Healing Potion");
        assert!(tool.description.contains("Healing Potion"));
    }

    #[test]
    fn test_parse_multiple_tools() {
        let raw = r#"
<dialogue>
"Your help means everything to me."
</dialogue>

<tool name="change_relationship">
{"change": "improve", "amount": "moderate", "reason": "helped with quest"}
</tool>

<tool name="reveal_info">
{"info_type": "quest", "content": "The temple is hidden in the mountains", "importance": "major"}
</tool>
"#;

        let parsed = parse_llm_response(raw);

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

        let parsed = parse_llm_response(raw);

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
{"item_name": "Gold Coins", "description": "100 shiny gold coins"}
</tool>
"#;

        let parsed = parse_llm_response(raw);

        // Tool content should be stripped from dialogue
        assert_eq!(parsed.dialogue, "Here's a gift for you, brave adventurer!");
        // But tool should still be parsed
        assert_eq!(parsed.proposed_tools.len(), 1);
        assert_eq!(parsed.proposed_tools[0].name, "give_item");
    }
}
