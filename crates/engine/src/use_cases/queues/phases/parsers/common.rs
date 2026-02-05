//! Common types and utilities for phase parsers.
//!
//! This module contains shared error types and helper functions used across
//! all phase-specific parsers.

use regex_lite::Regex;

/// Errors that can occur during phase output parsing.
///
/// These errors preserve structured context about what phase failed and why,
/// enabling better error handling and diagnostics in the phased pipeline.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PhaseParseError {
    /// Missing required XML tag or block in phase response.
    #[error("Missing required element in {phase}: {element}")]
    MissingElement {
        phase: &'static str,
        element: String,
    },

    /// Invalid or malformed content within a tag.
    #[error("Invalid content in {phase} <{element}>: {details}")]
    InvalidContent {
        phase: &'static str,
        element: String,
        details: String,
    },

    /// Mismatch between expected and actual count of elements.
    #[error("Count mismatch in {phase}: expected {expected} {element}, found {found}")]
    CountMismatch {
        phase: &'static str,
        element: String,
        expected: usize,
        found: usize,
    },

    /// Unexpected elements when none were expected.
    #[error("Unexpected {element} in {phase}: {details}")]
    Unexpected {
        phase: &'static str,
        element: String,
        details: String,
    },

    /// Missing required attribute on a tag.
    #[error("Missing attribute in {phase}: {attribute}")]
    MissingAttribute {
        phase: &'static str,
        attribute: String,
    },

    /// JSON parsing failure within phase content.
    #[error("JSON parse error in {phase}: {message}")]
    JsonParse {
        phase: &'static str,
        message: String,
    },

    /// Generic parse error with context.
    #[error("Parse error in {phase}: {message}")]
    Other {
        phase: &'static str,
        message: String,
    },
}

impl PhaseParseError {
    /// Create a new generic parse error for the given phase.
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self::Other {
            phase,
            message: message.into(),
        }
    }

    /// Create a missing element error for the given phase.
    pub fn missing_element(phase: &'static str, element: impl Into<String>) -> Self {
        Self::MissingElement {
            phase,
            element: element.into(),
        }
    }

    /// Create an invalid content error for the given phase.
    pub fn invalid_content(
        phase: &'static str,
        element: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::InvalidContent {
            phase,
            element: element.into(),
            details: details.into(),
        }
    }

    /// Create a count mismatch error for the given phase.
    pub fn count_mismatch(
        phase: &'static str,
        element: impl Into<String>,
        expected: usize,
        found: usize,
    ) -> Self {
        Self::CountMismatch {
            phase,
            element: element.into(),
            expected,
            found,
        }
    }

    /// Create an unexpected element error for the given phase.
    pub fn unexpected(
        phase: &'static str,
        element: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::Unexpected {
            phase,
            element: element.into(),
            details: details.into(),
        }
    }

    /// Create a missing attribute error for the given phase.
    pub fn missing_attribute(phase: &'static str, attribute: impl Into<String>) -> Self {
        Self::MissingAttribute {
            phase,
            attribute: attribute.into(),
        }
    }
}

/// Parse a count value from text with a given prefix.
///
/// Uses regex to match the specific prefix followed by optional whitespace and a number.
/// This correctly handles cases where both counts are on the same line
/// e.g., "Challenges: 2 (active), Events: 1 (pending)"
pub fn parse_count(text: &str, prefix: &str) -> usize {
    let pattern = format!(r"{}\s*(\d+)", regex_lite::escape(prefix));
    let re = Regex::new(&pattern).expect("valid regex");

    text.lines()
        .find(|l| l.contains(prefix))
        .and_then(|l| {
            re.captures(l)
                .and_then(|caps| caps.get(1))
                .and_then(|m| m.as_str().parse().ok())
        })
        .unwrap_or(0)
}

/// Strip all known XML tags from a response.
pub fn strip_all_tags(raw: &str) -> String {
    let mut result = raw.to_string();

    // Remove all known tags and their contents
    for tag in [
        "reasoning",
        "dialogue",
        "topics",
        "suggested_beats",
        "challenge",
        "event",
        "tool",
        "digest",
        "scene",
        "player_action",
        "npc",
        "active_elements",
        "conversation_summary",
        "behavior_analysis",
        "deflection",
        "emotional_stance",
        "mood_recommendation",
        "disposition_recommendation",
        "tool_analysis",
    ] {
        let re =
            Regex::new(&format!(r"(?s)<{}[^\u003e]*>.*?</{}>", tag, tag)).expect("valid regex");
        result = re.replace_all(&result, "").to_string();
    }

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
    fn test_phase_parse_error_display_and_source() {
        // Test MissingElement variant
        let err = PhaseParseError::missing_element("Phase 1", "<challenge> tags");
        let err_str = format!("{}", err);
        assert!(err_str.contains("Missing required element in Phase 1"));
        assert!(err_str.contains("<challenge> tags"));

        // Test CountMismatch variant
        let err = PhaseParseError::count_mismatch("Phase 1", "<challenge> blocks", 2, 1);
        let err_str = format!("{}", err);
        assert!(err_str.contains("Count mismatch in Phase 1"));
        assert!(err_str.contains("expected 2"));
        assert!(err_str.contains("found 1"));

        // Test InvalidContent variant
        let err = PhaseParseError::invalid_content(
            "Phase 3",
            "<should_act>",
            "YES but no valid tools parsed",
        );
        let err_str = format!("{}", err);
        assert!(err_str.contains("Invalid content in Phase 3"));
        assert!(err_str.contains("YES but no valid tools parsed"));

        // Test Unexpected variant
        let err = PhaseParseError::unexpected(
            "Phase 1",
            "<challenge> tags",
            "no active challenges expected",
        );
        let err_str = format!("{}", err);
        assert!(err_str.contains("Unexpected <challenge> tags in Phase 1"));

        // Test MissingAttribute variant
        let err = PhaseParseError::missing_attribute("Phase 1", "challenge id attribute");
        let err_str = format!("{}", err);
        assert!(err_str.contains("Missing attribute in Phase 1"));
        assert!(err_str.contains("challenge id attribute"));

        // Test Other variant
        let err = PhaseParseError::new("Phase 5", "Some generic error");
        let err_str = format!("{}", err);
        assert!(err_str.contains("Parse error in Phase 5"));
        assert!(err_str.contains("Some generic error"));
    }

    #[test]
    fn test_phase_parse_error_preserves_context_in_error_chain() {
        // Create a JSON parse error to test JsonParse variant
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("{ invalid }").expect_err("should fail");
        let err = PhaseParseError::JsonParse {
            phase: "Phase 3",
            message: json_err.to_string(),
        };

        // Verify the error implements std::error::Error
        let err_str = format!("{}", err);
        assert!(err_str.contains("JSON parse error in Phase 3"));
    }

    #[test]
    fn test_parse_count() {
        let text = "Challenges: 5 (active), Events: 3 (pending)";
        assert_eq!(parse_count(text, "Challenges:"), 5);
        assert_eq!(parse_count(text, "Events:"), 3);
    }

    #[test]
    fn test_parse_count_no_parentheses() {
        let text = "Challenges: 15, Events: 12";
        assert_eq!(parse_count(text, "Challenges:"), 15);
        assert_eq!(parse_count(text, "Events:"), 12);
    }

    #[test]
    fn test_parse_count_multiline() {
        let text = "Challenges: 10 (active)\nEvents: 8 (pending)";
        assert_eq!(parse_count(text, "Challenges:"), 10);
        assert_eq!(parse_count(text, "Events:"), 8);
    }

    #[test]
    fn test_strip_all_tags() {
        let raw = r#"
<reasoning>Some thoughts</reasoning>
<dialogue>Hello!</dialogue>
<topics>greeting</topics>
"#;
        let result = strip_all_tags(raw);
        assert!(!result.contains("<reasoning>"));
        assert!(!result.contains("<dialogue>"));
        assert!(!result.contains("<topics>"));
    }
}
