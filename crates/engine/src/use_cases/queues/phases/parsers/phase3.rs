//! Phase 3: Tool Intent Parser
//!
//! Parses tool intent detection from LLM response, extracting proposed tools
//! with their arguments and validating the should_act field.

use std::sync::LazyLock;

use regex_lite::Regex;

use super::common::PhaseParseError;
use crate::game_tools::ToolArgs;
use crate::queue_types::{Phase3ToolIntent, ProposedTool};
use crate::use_cases::queues::response_parser::generate_tool_id;

/// Parse Phase 3 tool intent from LLM response.
pub fn parse_phase3_tools(raw: &str) -> Result<Phase3ToolIntent, PhaseParseError> {
    const PHASE: &str = "Phase 3";

    static SHOULD_ACT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<should_act>(.*?)</should_act>").expect("valid regex"));
    static TOOL_REASONING_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<tool_analysis>.*?.*?<reasoning>(.*?)</reasoning>.*?</tool_analysis>")
            .expect("valid regex")
    });
    static TOOL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<tool\s+name="([^"]+)">(.*?)</tool>"#).expect("valid regex")
    });
    static JUSTIFICATION_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<justification>(.*?)</justification>").expect("valid regex")
    });
    static ARGUMENTS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<arguments>(.*?)</arguments>").expect("valid regex"));

    // Parse should_act FIRST - this determines how we handle tool parsing
    let should_act_value = SHOULD_ACT_RE.captures(raw).map(|c| c[1].trim().to_string());

    let should_act = match should_act_value {
        Some(ref value) => {
            let upper = value.to_ascii_uppercase();
            if upper == "YES" {
                Some(true)
            } else if upper == "NO" {
                Some(false)
            } else {
                return Err(PhaseParseError::invalid_content(
                    PHASE,
                    "<should_act>",
                    format!("expected YES or NO, got '{}'", value),
                ));
            }
        }
        None => None,
    };

    if should_act.is_none() {
        return Err(PhaseParseError::missing_element(
            PHASE,
            "<tool_analysis>/<should_act>",
        ));
    }

    let reasoning = TOOL_REASONING_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.is_empty());

    // When should_act is NO, skip tool parsing entirely - malformed tool JSON should not fail
    let mut proposed_tools = Vec::new();

    if should_act == Some(true) {
        // Strict tool parsing only when should_act == YES
        for caps in TOOL_RE.captures_iter(raw) {
            let name = caps[1].trim().to_string();
            let inner = &caps[2];

            let _justification = JUSTIFICATION_RE
                .captures(inner)
                .map(|c| c[1].trim().to_string());

            // Extract JSON arguments
            let json_str = ARGUMENTS_RE
                .captures(inner)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default();

            // Parse the JSON arguments into typed ToolArgs
            // Strict parser contract: invalid JSON is a hard parse error when should_act == YES
            let arguments: ToolArgs =
                serde_json::from_str(&json_str).map_err(|e| PhaseParseError::JsonParse {
                    phase: PHASE,
                    message: format!(
                        "Failed to parse arguments for tool '{}': {}. JSON: {}",
                        name, e, json_str
                    ),
                })?;

            // Use the original JSON string for ID generation to avoid serialization failures
            // and ensure deterministic ID derivation from the source material
            let id = generate_tool_id(&name, &json_str);
            let description = arguments.description();

            proposed_tools.push(ProposedTool {
                id,
                name,
                description,
                arguments,
            });
        }

        // Require at least one valid tool when should_act == YES
        if proposed_tools.is_empty() {
            return Err(PhaseParseError::invalid_content(
                PHASE,
                "<should_act>",
                "YES but no valid tools parsed",
            ));
        }
    }
    // When should_act == NO: skip tool parsing entirely - any malformed tool tags are ignored

    let has_tools = !proposed_tools.is_empty();
    let intent = Phase3ToolIntent {
        proposed_tools,
        has_tools,
        should_act,
        reasoning,
    };

    Ok(intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phase3_tools_reads_arguments_json() {
        let raw = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Player asked for a reward.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Offer a small reward for their help.</justification>
  <arguments>
  {
    "tool": "GiveItem",
    "item_name": "Lantern",
    "description": "An old brass lantern"
  }
  </arguments>
</tool>
"#;

        let parsed = parse_phase3_tools(raw).expect("parse phase3");
        assert_eq!(parsed.should_act, Some(true));
        assert_eq!(parsed.proposed_tools.len(), 1);

        match &parsed.proposed_tools[0].arguments {
            ToolArgs::GiveItem {
                item_name,
                description,
            } => {
                assert_eq!(item_name, "Lantern");
                assert_eq!(description, "An old brass lantern");
            }
            other => panic!("Unexpected tool args: {:?}", other),
        }
    }

    #[test]
    fn test_parse_phase3_tools_respects_should_act_no() {
        let raw = r#"
<tool_analysis>
  <should_act>NO</should_act>
  <reasoning>No tools should be used.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Should be ignored.</justification>
  <arguments>
  {
    "tool": "GiveItem",
    "item_name": "Coin",
    "description": "A single copper"
  }
  </arguments>
</tool>
"#;

        let parsed = parse_phase3_tools(raw).expect("parse phase3");
        assert_eq!(parsed.should_act, Some(false));
        assert!(parsed.proposed_tools.is_empty());
        assert!(!parsed.has_tools);
    }

    #[test]
    fn test_parse_phase3_tools_rejects_yes_without_tools() {
        let raw = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Should use tools.</reasoning>
</tool_analysis>
"#; // No <tool> tags
        let result = parse_phase3_tools(raw);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("YES but no valid tools"),
            "Error should indicate missing tools: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase3_tools_requires_should_act() {
        // Test with valid tool but no should_act tag - should fail with missing element
        let raw = r#"
<tool_analysis>
  <reasoning>No should_act tag here.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give something.</justification>
  <arguments>{"tool":"GiveItem","item_name":"Coin","description":"A coin"}</arguments>
</tool>
"#;
        let result = parse_phase3_tools(raw);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") || err_str.contains("<tool_analysis>"),
            "Error should indicate missing should_act: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase3_tools_rejects_invalid_should_act_value() {
        let raw = r#"
<tool_analysis>
  <should_act>MAYBE</should_act>
  <reasoning>Uncertain about tool usage.</reasoning>
</tool_analysis>
"#;
        let result = parse_phase3_tools(raw);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content") && err_str.contains("MAYBE"),
            "Error should indicate invalid should_act value: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase3_tools_accepts_yes_lowercase() {
        let raw = r#"
<tool_analysis>
  <should_act>yes</should_act>
  <reasoning>Should use tools.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give something.</justification>
  <arguments>{"tool":"GiveItem","item_name":"Coin","description":"A coin"}</arguments>
</tool>
"#;
        let result = parse_phase3_tools(raw).expect("parse phase3");
        assert_eq!(result.should_act, Some(true));
    }

    #[test]
    fn test_parse_phase3_tools_accepts_no_lowercase() {
        let raw = r#"
<tool_analysis>
  <should_act>no</should_act>
  <reasoning>No tools needed.</reasoning>
</tool_analysis>
"#;
        let result = parse_phase3_tools(raw).expect("parse phase3");
        assert_eq!(result.should_act, Some(false));
    }

    #[test]
    fn test_parse_phase3_tools_rejects_empty_should_act() {
        let raw = r#"
<tool_analysis>
  <should_act>   </should_act>
  <reasoning>Empty should_act.</reasoning>
</tool_analysis>
"#;
        let result = parse_phase3_tools(raw);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content"),
            "Error should indicate invalid should_act value: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase3_tools_rejects_invalid_json_arguments() {
        let raw = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Try to use tools.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give something.</justification>
  <arguments>{ invalid json }</arguments>
</tool>
"#;
        let result = parse_phase3_tools(raw);
        // Strict parser contract: invalid JSON is a hard parse error
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("JSON parse error") && err_str.contains("give_item"),
            "Error should indicate JSON parse error for tool: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase3_tool_id_uses_non_empty_argument_material() {
        // Verify that tool IDs are derived from the original JSON arguments
        // and that different arguments produce different IDs
        let raw1 = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Give a lantern.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give a lantern.</justification>
  <arguments>{"tool":"GiveItem","item_name":"Lantern","description":"An old brass lantern"}</arguments>
</tool>
"#;

        let raw2 = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Give a coin.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give a coin.</justification>
  <arguments>{"tool":"GiveItem","item_name":"Coin","description":"A single copper"}</arguments>
</tool>
"#;

        let parsed1 = parse_phase3_tools(raw1).expect("parse phase3 first");
        let parsed2 = parse_phase3_tools(raw2).expect("parse phase3 second");

        assert_eq!(parsed1.proposed_tools.len(), 1);
        assert_eq!(parsed2.proposed_tools.len(), 1);

        let id1 = &parsed1.proposed_tools[0].id;
        let id2 = &parsed2.proposed_tools[0].id;

        // Different arguments should produce different IDs
        assert_ne!(
            id1, id2,
            "Tool IDs should differ when arguments differ: got same ID '{}' for different args",
            id1
        );

        // IDs should not be empty
        assert!(!id1.is_empty(), "Tool ID should not be empty");
        assert!(!id2.is_empty(), "Tool ID should not be empty");
    }

    #[test]
    fn test_parse_phase3_tool_id_is_deterministic() {
        // Verify that the same arguments produce the same ID (deterministic)
        let raw = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Give a lantern.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give a lantern.</justification>
  <arguments>{"tool":"GiveItem","item_name":"Lantern","description":"An old brass lantern"}</arguments>
</tool>
"#;

        let parsed1 = parse_phase3_tools(raw).expect("parse phase3 first");
        let parsed2 = parse_phase3_tools(raw).expect("parse phase3 second");

        assert_eq!(
            parsed1.proposed_tools[0].id, parsed2.proposed_tools[0].id,
            "Tool ID derivation should be deterministic for identical inputs"
        );
    }

    #[test]
    fn test_parse_phase3_tool_id_includes_name_in_derivation() {
        // Verify that different tool names with same args produce different IDs
        let raw1 = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Use first tool.</reasoning>
</tool_analysis>
<tool name="tool_a">
  <justification>Use tool A.</justification>
  <arguments>{"tool":"Test","value":123}</arguments>
</tool>
"#;

        let raw2 = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Use second tool.</reasoning>
</tool_analysis>
<tool name="tool_b">
  <justification>Use tool B.</justification>
  <arguments>{"tool":"Test","value":123}</arguments>
</tool>
"#;

        let parsed1 = parse_phase3_tools(raw1).expect("parse phase3 first");
        let parsed2 = parse_phase3_tools(raw2).expect("parse phase3 second");

        let id1 = &parsed1.proposed_tools[0].id;
        let id2 = &parsed2.proposed_tools[0].id;

        assert_ne!(
            id1, id2,
            "Tool IDs should differ when tool names differ, even with same arguments"
        );
    }

    #[test]
    fn test_parse_phase3_tools_ignores_malformed_tool_json_when_should_act_no() {
        // When should_act is NO, malformed tool JSON should NOT fail parsing
        let raw = r#"
<tool_analysis>
  <should_act>NO</should_act>
  <reasoning>No tools should be used.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Should be ignored.</justification>
  <arguments>{ invalid json that would normally fail }</arguments>
</tool>
"#;

        let parsed =
            parse_phase3_tools(raw).expect("parse should succeed despite malformed tool JSON");
        assert_eq!(parsed.should_act, Some(false));
        assert!(parsed.proposed_tools.is_empty());
        assert!(!parsed.has_tools);
    }

    #[test]
    fn test_parse_phase3_tools_fails_on_malformed_json_when_should_act_yes() {
        // When should_act is YES, malformed tool JSON should fail parsing
        let raw = r#"
<tool_analysis>
  <should_act>YES</should_act>
  <reasoning>Try to use tools.</reasoning>
</tool_analysis>
<tool name="give_item">
  <justification>Give something.</justification>
  <arguments>{ invalid json }</arguments>
</tool>
"#;

        let result = parse_phase3_tools(raw);
        assert!(
            result.is_err(),
            "Should fail parsing when should_act=YES and tool JSON is malformed"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("JSON parse error") && err_str.contains("give_item"),
            "Error should indicate JSON parse error for tool: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase3_tools_skips_tool_parsing_entirely_when_should_act_no() {
        // Even with multiple malformed tools, parsing should succeed when should_act=NO
        let raw = r#"
<tool_analysis>
  <should_act>NO</should_act>
  <reasoning>Tools are not appropriate here.</reasoning>
</tool_analysis>
<tool name="tool_a">
  <justification>Malformed 1.</justification>
  <arguments>{ broken json 1 }</arguments>
</tool>
<tool name="tool_b">
  <justification>Malformed 2.</justification>
  <arguments>{ broken json 2 }</arguments>
</tool>
<tool name="tool_c">
  <justification>Also malformed.</justification>
  <arguments>not even json</arguments>
</tool>
"#;

        let parsed = parse_phase3_tools(raw).expect("parse should succeed with should_act=NO");
        assert_eq!(parsed.should_act, Some(false));
        assert!(parsed.proposed_tools.is_empty());
        assert!(!parsed.has_tools);
    }
}
