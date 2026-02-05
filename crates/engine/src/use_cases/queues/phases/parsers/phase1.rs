//! Phase 1: Challenge Trigger Parser
//!
//! Parses challenge trigger analysis from LLM response, validating
//! challenge IDs against expected order and enforcing at most one YES trigger.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex_lite::Regex;

use super::common::PhaseParseError;
use crate::queue_types::{Phase1ChallengeTriggers, PhaseChallengeAnalysis};

/// Parse Phase 1 challenge triggers from LLM response.
pub fn parse_phase1_challenges(
    raw: &str,
    expected_order: &[String],
) -> Result<Phase1ChallengeTriggers, PhaseParseError> {
    const PHASE: &str = "Phase 1";

    // Detect malformed <challenge> tags without attributes when no challenges expected
    if expected_order.is_empty() {
        static CHALLENGE_NO_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?s)<challenge\s*>(.*?)</challenge>").expect("valid regex")
        });
        if CHALLENGE_NO_ATTR_RE.is_match(raw) {
            return Err(PhaseParseError::unexpected(
                PHASE,
                "<challenge> tags",
                "no active challenges expected (found tag without attributes)",
            ));
        }
    }

    let mut challenges = parse_challenge_tags(raw);
    if expected_order.is_empty() && !challenges.is_empty() {
        return Err(PhaseParseError::unexpected(
            PHASE,
            "<challenge> tags",
            "no active challenges expected",
        ));
    }
    if !expected_order.is_empty() {
        // Strict contract: IDs are required when expected_order is non-empty
        if challenges.iter().any(|ch| ch.challenge_id.is_empty()) {
            return Err(PhaseParseError::missing_attribute(
                PHASE,
                "challenge id attribute",
            ));
        }

        // Build frequency map of parsed IDs to detect duplicates
        let mut parsed_id_counts: HashMap<&str, usize> = HashMap::new();
        for ch in &challenges {
            *parsed_id_counts.entry(&ch.challenge_id).or_insert(0) += 1;
        }

        // Detect duplicate IDs
        for (id, count) in &parsed_id_counts {
            if *count > 1 {
                return Err(PhaseParseError::invalid_content(
                    PHASE,
                    "<challenge> id",
                    format!("duplicate challenge id: {}", id),
                ));
            }
        }

        // Validate all parsed IDs are in expected_order and detect missing IDs
        let expected_set: HashSet<&String> = expected_order.iter().collect();
        for ch in &challenges {
            if !expected_set.contains(&ch.challenge_id) {
                return Err(PhaseParseError::unexpected(
                    PHASE,
                    "<challenge> id",
                    format!("unknown challenge id: {}", ch.challenge_id),
                ));
            }
        }

        // Detect missing expected IDs
        let parsed_set: HashSet<&str> = parsed_id_counts.keys().copied().collect();
        for expected in expected_order {
            if !parsed_set.contains(expected.as_str()) {
                return Err(PhaseParseError::missing_element(
                    PHASE,
                    format!("<challenge> with id='{}'", expected),
                ));
            }
        }

        // Reorder to match expected_order (exact match guaranteed by validation above)
        let mut ordered = Vec::with_capacity(challenges.len());
        let mut remaining = challenges;

        for expected in expected_order {
            if let Some(pos) = remaining.iter().position(|ch| &ch.challenge_id == expected) {
                ordered.push(remaining.remove(pos));
            }
        }

        challenges = ordered;
    }

    let mut found_yes = false;

    // Enforce "at most one YES" to match approval handling
    for challenge in &mut challenges {
        if challenge.should_trigger {
            if !found_yes {
                found_yes = true;
            } else {
                challenge.should_trigger = false;
            }
        }
    }
    let expected_count = expected_order.len();
    if expected_count > 0 && challenges.is_empty() {
        return Err(PhaseParseError::missing_element(PHASE, "<challenge> tags"));
    }

    if expected_count > 0 && challenges.len() != expected_count {
        return Err(PhaseParseError::count_mismatch(
            PHASE,
            "<challenge> blocks",
            expected_count,
            challenges.len(),
        ));
    }

    Ok(Phase1ChallengeTriggers { challenges })
}

/// Parse <challenge> tags from response.
fn parse_challenge_tags(raw: &str) -> Vec<PhaseChallengeAnalysis> {
    static CHALLENGE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<challenge\s+([^\u003e]+)>(.*?)</challenge>"#).expect("valid regex")
    });
    static NAME_ATTR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new("name=\"([^\"]+)\"").expect("valid regex"));
    static ID_ATTR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new("id=\"([^\"]+)\"").expect("valid regex"));
    static QUOTE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<quote>(.*?)</quote>").expect("valid regex"));
    static TRIGGER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<trigger>(.*?)</trigger>").expect("valid regex"));
    static CONFIDENCE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<confidence>(.*?)</confidence>").expect("valid regex"));
    static REASONING_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<reasoning>(.*?)</reasoning>").expect("valid regex"));

    let mut results = Vec::new();

    for caps in CHALLENGE_RE.captures_iter(raw) {
        let attrs = &caps[1];
        let inner = &caps[2];

        let name = NAME_ATTR_RE
            .captures(attrs)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();
        let id = ID_ATTR_RE
            .captures(attrs)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();

        let quote = QUOTE_RE
            .captures(inner)
            .map(|c| c[1].trim().to_string())
            .filter(|q| !q.eq_ignore_ascii_case("none") && !q.is_empty());

        let should_trigger = TRIGGER_RE
            .captures(inner)
            .map(|c| c[1].trim().eq_ignore_ascii_case("YES"))
            .unwrap_or(false);

        let confidence = CONFIDENCE_RE
            .captures(inner)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_else(|| "low".to_string());

        let reasoning = REASONING_RE
            .captures(inner)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();

        results.push(PhaseChallengeAnalysis {
            challenge_id: id,
            challenge_name: name,
            trigger_quote: quote,
            should_trigger,
            confidence,
            reasoning,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phase1_challenges() {
        let raw = r#"
<challenge id="CHAL_1" name="Test Challenge">
  <quote>test quote</quote>
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Test reasoning</reasoning>
</challenge>
"#;

        let order = vec!["CHAL_1".to_string()];
        let result = parse_phase1_challenges(raw, &order).expect("parse phase1");
        assert_eq!(result.challenges.len(), 1);
        assert_eq!(result.challenges[0].challenge_id, "CHAL_1");
        assert_eq!(result.challenges[0].challenge_name, "Test Challenge");
        assert!(result.challenges[0].should_trigger);
        assert_eq!(
            result.challenges[0].trigger_quote.as_deref(),
            Some("test quote")
        );
    }

    #[test]
    fn test_parse_phase1_challenges_enforces_single_yes() {
        let raw = r#"
<challenge id="CHAL_1" name="First Challenge">
  <quote>ask</quote>
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Direct match.</reasoning>
</challenge>
<challenge id="CHAL_2" name="Second Challenge">
  <quote>ask</quote>
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Also matches.</reasoning>
</challenge>
"#;

        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let parsed = parse_phase1_challenges(raw, &order).expect("parse phase1");
        let yes_count = parsed
            .challenges
            .iter()
            .filter(|c| c.should_trigger)
            .count();
        assert_eq!(yes_count, 1);
        assert_eq!(parsed.challenges.len(), 2);
        assert!(!parsed.challenges[1].should_trigger);
    }

    #[test]
    fn test_parse_phase1_challenges_reorders_by_expected_order() {
        let raw = r#"
<challenge id="CHAL_2" name="Second">
  <trigger>NO</trigger>
  <confidence>medium</confidence>
  <reasoning>Second challenge.</reasoning>
</challenge>
<challenge id="CHAL_1" name="First">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>First challenge.</reasoning>
</challenge>
"#;
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let result = parse_phase1_challenges(raw, &order).expect("parse phase1");

        assert_eq!(result.challenges[0].challenge_id, "CHAL_1");
        assert_eq!(result.challenges[1].challenge_id, "CHAL_2");
        assert!(result.challenges[0].should_trigger);
        assert!(!result.challenges[1].should_trigger);
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_when_no_active_challenges() {
        let raw = r#"<challenge id="CHAL_1" name="Test"><trigger>YES</trigger></challenge>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected"),
            "Error should indicate unexpected challenge: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_requires_id_when_expected_order_present() {
        let raw = r#"<challenge name="Test Challenge"><trigger>YES</trigger></challenge>"#;
        let order = vec!["CHAL_1".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("challenge id attribute"),
            "Error should indicate missing id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_count_mismatch() {
        // When expected IDs are missing, we now get a specific missing ID error
        // This is more informative than a generic count mismatch
        let raw = r#"<challenge id="CHAL_1" name="Test"><trigger>YES</trigger></challenge>"#;
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()]; // Expect 2, get 1
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        // With exact coverage validation, we detect the missing ID specifically
        assert!(
            err_str.contains("Missing") && err_str.contains("CHAL_2"),
            "Error should indicate missing expected challenge CHAL_2: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_unknown_id() {
        // LLM returns an unknown/hallucinated challenge ID
        let raw = r#"
<challenge id="CHAL_1" name="Known Challenge">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Valid challenge.</reasoning>
</challenge>
<challenge id="UNKNOWN_CHAL" name="Unknown Challenge">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Hallucinated challenge.</reasoning>
</challenge>
"#;
        // Expected order only contains CHAL_1, but response has UNKNOWN_CHAL
        let order = vec!["CHAL_1".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err(), "Should reject unknown challenge ID");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("UNKNOWN_CHAL"),
            "Error should indicate unknown challenge id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_unknown_id_with_matching_count() {
        // Malicious/hallucinated case: count matches but IDs don't
        let raw = r#"
<challenge id="FAKE_1" name="Fake Challenge 1">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Fake.</reasoning>
</challenge>
<challenge id="FAKE_2" name="Fake Challenge 2">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Also fake.</reasoning>
</challenge>
"#;
        // Expected order has 2 items, response has 2 items, but IDs don't match
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(
            result.is_err(),
            "Should reject when count matches but IDs are unknown"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("FAKE_1"),
            "Error should indicate first unknown id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_duplicate_ids() {
        // LLM returns duplicate challenge IDs
        let raw = r#"
<challenge id="CHAL_1" name="First Challenge">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>First occurrence.</reasoning>
</challenge>
<challenge id="CHAL_1" name="Duplicate Challenge">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Duplicate ID - should fail.</reasoning>
</challenge>
"#;
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err(), "Should reject duplicate challenge IDs");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("duplicate") && err_str.contains("CHAL_1"),
            "Error should indicate duplicate challenge id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_duplicate_ids_with_matching_count() {
        // Malicious case: duplicate IDs but count matches expected
        // This could mask a missing expected challenge
        let raw = r#"
<challenge id="CHAL_1" name="First Challenge">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>First occurrence.</reasoning>
</challenge>
<challenge id="CHAL_1" name="Duplicate Challenge">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Duplicate ID - CHAL_2 is missing.</reasoning>
</challenge>
"#;
        // Expected 2 challenges, got 2 challenges, but both have same ID
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(
            result.is_err(),
            "Should reject duplicate IDs even when count matches"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("duplicate"),
            "Error should indicate duplicate: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_missing_expected_id() {
        // LLM returns only one challenge when two are expected
        let raw = r#"
<challenge id="CHAL_1" name="First Challenge">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Only challenge returned.</reasoning>
</challenge>
"#;
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(
            result.is_err(),
            "Should reject when expected challenge is missing"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") && err_str.contains("CHAL_2"),
            "Error should indicate missing challenge CHAL_2: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_all_missing_expected_ids() {
        // LLM returns no challenges when some are expected
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(
            result.is_err(),
            "Should reject when all expected challenges are missing"
        );
        // This should fail at the "no challenge tags" check before missing ID check
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("challenge") || err_str.contains("Missing"),
            "Error should indicate missing challenges: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_no_tags_when_expected() {
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let order = vec!["CHAL_1".to_string()];
        let result = parse_phase1_challenges(raw, &order);

        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("challenge"),
            "Error should indicate missing challenge: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_malformed_tag_without_attributes() {
        // When expected_order is empty, <challenge> tags without attributes should be rejected
        let raw = r#"<challenge><trigger>YES</trigger></challenge>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase1_challenges(raw, &order);
        assert!(
            result.is_err(),
            "Should reject <challenge> tags without attributes when no challenges expected"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("challenge"),
            "Error should indicate unexpected challenge tag: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase1_challenges_rejects_malformed_tag_without_id() {
        // When expected_order is empty, <challenge> tags with name but no id should be rejected
        let raw = r#"<challenge name="Test"><trigger>YES</trigger></challenge>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase1_challenges(raw, &order);
        assert!(
            result.is_err(),
            "Should reject <challenge> tags without id when no challenges expected"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("challenge"),
            "Error should indicate unexpected challenge tag: {}",
            err_str
        );
    }
}
