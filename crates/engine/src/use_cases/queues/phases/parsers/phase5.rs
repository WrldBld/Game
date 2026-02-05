//! Phase 5: Final Response Parser
//!
//! Parses the final NPC dialogue response from LLM output,
//! extracting dialogue, reasoning, topics, and suggested narrative beats.

use std::sync::LazyLock;

use regex_lite::Regex;

use super::common::PhaseParseError;
use crate::queue_types::Phase5FinalResponse;

/// Parse Phase 5 final response from LLM response.
pub fn parse_phase5_response(raw: &str) -> Result<Phase5FinalResponse, PhaseParseError> {
    const PHASE: &str = "Phase 5";

    static REASONING_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<reasoning>(.*?)</reasoning>").expect("valid regex"));
    static DIALOGUE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<dialogue>(.*?)</dialogue>").expect("valid regex"));
    static TOPICS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<topics>(.*?)</topics>").expect("valid regex"));
    static BEATS_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<suggested_beats>(.*?)</suggested_beats>").expect("valid regex")
    });

    let reasoning = REASONING_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let dialogue = match DIALOGUE_RE.captures(raw).map(|c| c[1].trim().to_string()) {
        Some(value) if !value.is_empty() => value,
        _ => return Err(PhaseParseError::missing_element(PHASE, "<dialogue>")),
    };

    let topics: Vec<String> = TOPICS_RE
        .captures(raw)
        .map(|c| {
            c[1].lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let suggested_beats: Vec<String> = BEATS_RE
        .captures(raw)
        .map(|c| {
            c[1].lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    Ok(Phase5FinalResponse {
        dialogue,
        reasoning,
        topics,
        suggested_beats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phase5_response() {
        let raw = r#"
<reasoning>Internal thoughts</reasoning>
<dialogue>Hello there!</dialogue>
<topics>
greeting
</topics>
"#;

        let result = parse_phase5_response(raw).expect("parse phase5");
        assert_eq!(result.dialogue, "Hello there!");
        assert_eq!(result.reasoning, "Internal thoughts");
        assert_eq!(result.topics, vec!["greeting"]);
    }

    #[test]
    fn test_parse_phase5_response_requires_dialogue() {
        let raw = r#"<reasoning>Some thoughts</reasoning>"#; // No dialogue
        let result = parse_phase5_response(raw);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("dialogue"),
            "Error should indicate missing dialogue: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase5_response_rejects_empty_dialogue() {
        let raw = r#"<dialogue>   </dialogue>"#; // Empty/whitespace dialogue
        let result = parse_phase5_response(raw);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("dialogue"),
            "Error should indicate missing dialogue: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase5_response_with_empty_optional_fields() {
        let raw = r#"
<reasoning>Simple response.</reasoning>
<dialogue>Hello!</dialogue>
<topics></topics>
"#;
        let result = parse_phase5_response(raw).expect("parse phase5");

        assert_eq!(result.dialogue, "Hello!");
        assert!(result.topics.is_empty());
        assert!(result.suggested_beats.is_empty());
    }
}
