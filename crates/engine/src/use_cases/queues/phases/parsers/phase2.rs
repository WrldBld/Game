//! Phase 2: Event Trigger Parser
//!
//! Parses event trigger analysis from LLM response, validating
//! event IDs against expected order and enforcing at most one YES trigger.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex_lite::Regex;

use super::common::PhaseParseError;
use crate::queue_types::{Phase2EventTriggers, PhaseEventAnalysis};

/// Parse Phase 2 event triggers from LLM response.
pub fn parse_phase2_events(
    raw: &str,
    expected_order: &[String],
) -> Result<Phase2EventTriggers, PhaseParseError> {
    const PHASE: &str = "Phase 2";

    // Detect malformed <event> tags without attributes when no events expected
    if expected_order.is_empty() {
        static EVENT_NO_ATTR_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?s)<event\s*>(.*?)</event>").expect("valid regex"));
        if EVENT_NO_ATTR_RE.is_match(raw) {
            return Err(PhaseParseError::unexpected(
                PHASE,
                "<event> tags",
                "no active events expected (found tag without attributes)",
            ));
        }
    }

    let mut events = parse_event_tags(raw);
    if expected_order.is_empty() && !events.is_empty() {
        return Err(PhaseParseError::unexpected(
            PHASE,
            "<event> tags",
            "no active events expected",
        ));
    }
    if !expected_order.is_empty() {
        // Strict contract: IDs are required when expected_order is non-empty
        if events.iter().any(|evt| evt.event_id.is_empty()) {
            return Err(PhaseParseError::missing_attribute(
                PHASE,
                "event id attribute",
            ));
        }

        // Build frequency map of parsed IDs to detect duplicates
        let mut parsed_id_counts: HashMap<&str, usize> = HashMap::new();
        for evt in &events {
            *parsed_id_counts.entry(&evt.event_id).or_insert(0) += 1;
        }

        // Detect duplicate IDs
        for (id, count) in &parsed_id_counts {
            if *count > 1 {
                return Err(PhaseParseError::invalid_content(
                    PHASE,
                    "<event> id",
                    format!("duplicate event id: {}", id),
                ));
            }
        }

        // Validate all parsed IDs are in expected_order and detect missing IDs
        let expected_set: HashSet<&String> = expected_order.iter().collect();
        for evt in &events {
            if !expected_set.contains(&evt.event_id) {
                return Err(PhaseParseError::unexpected(
                    PHASE,
                    "<event> id",
                    format!("unknown event id: {}", evt.event_id),
                ));
            }
        }

        // Detect missing expected IDs
        let parsed_set: HashSet<&str> = parsed_id_counts.keys().copied().collect();
        for expected in expected_order {
            if !parsed_set.contains(expected.as_str()) {
                return Err(PhaseParseError::missing_element(
                    PHASE,
                    format!("<event> with id='{}'", expected),
                ));
            }
        }

        // Reorder to match expected_order (exact match guaranteed by validation above)
        let mut ordered = Vec::with_capacity(events.len());
        let mut remaining = events;

        for expected in expected_order {
            if let Some(pos) = remaining.iter().position(|evt| &evt.event_id == expected) {
                ordered.push(remaining.remove(pos));
            }
        }

        events = ordered;
    }

    let mut selected_event = None;
    let mut found_yes = false;

    // Enforce "at most one YES" - keep the first, coerce others to NO
    for event in &mut events {
        if event.should_trigger {
            if !found_yes {
                selected_event = Some(event.event_name.clone());
                found_yes = true;
            } else {
                event.should_trigger = false;
            }
        }
    }

    let expected_count = expected_order.len();
    if expected_count > 0 && events.is_empty() {
        return Err(PhaseParseError::missing_element(PHASE, "<event> tags"));
    }

    if expected_count > 0 && events.len() != expected_count {
        return Err(PhaseParseError::count_mismatch(
            PHASE,
            "<event> blocks",
            expected_count,
            events.len(),
        ));
    }

    Ok(Phase2EventTriggers {
        events,
        selected_event,
    })
}

/// Parse <event> tags from response.
fn parse_event_tags(raw: &str) -> Vec<PhaseEventAnalysis> {
    static EVENT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<event\s+([^\u003e]+)>(.*?)</event>"#).expect("valid regex")
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

    for caps in EVENT_RE.captures_iter(raw) {
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

        results.push(PhaseEventAnalysis {
            event_id: id,
            event_name: name,
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
    fn test_parse_phase2_events() {
        let raw = r#"
<event id="EVT_1" name="Test Event">
  <quote>event quote</quote>
  <trigger>YES</trigger>
  <confidence>medium</confidence>
  <reasoning>Event reasoning</reasoning>
</event>
"#;

        let order = vec!["EVT_1".to_string()];
        let result = parse_phase2_events(raw, &order).expect("parse phase2");
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].event_id, "EVT_1");
        assert_eq!(result.events[0].event_name, "Test Event");
        assert!(result.events[0].should_trigger);
        assert_eq!(result.selected_event, Some("Test Event".to_string()));
    }

    #[test]
    fn test_parse_phase2_events_enforces_single_yes() {
        let raw = r#"
<event id="EVT_1" name="Marta's Knowledge">
  <quote>old mill</quote>
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Direct match.</reasoning>
</event>
<event id="EVT_2" name="The Stranger's Warning">
  <quote>old mill</quote>
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Also matches, but should be NO due to rule.</reasoning>
</event>
"#;

        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let parsed = parse_phase2_events(raw, &order).expect("parse phase2");
        let yes_count = parsed.events.iter().filter(|e| e.should_trigger).count();
        assert_eq!(yes_count, 1);
        assert_eq!(parsed.selected_event.as_deref(), Some("Marta's Knowledge"));
        assert_eq!(parsed.events.len(), 2);
        assert!(!parsed.events[1].should_trigger);
    }

    #[test]
    fn test_parse_phase2_events_reorders_by_expected_order() {
        let raw = r#"
<event id="EVT_2" name="Second Event">
  <trigger>NO</trigger>
  <confidence>medium</confidence>
  <reasoning>Second event.</reasoning>
</event>
<event id="EVT_1" name="First Event">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>First event.</reasoning>
</event>
"#;
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order).expect("parse phase2");

        assert_eq!(result.events[0].event_id, "EVT_1");
        assert_eq!(result.events[1].event_id, "EVT_2");
        assert!(result.events[0].should_trigger);
        assert!(!result.events[1].should_trigger);
    }

    #[test]
    fn test_parse_phase2_events_rejects_when_no_active_events() {
        let raw = r#"<event id="EVT_1" name="Test"><trigger>YES</trigger></event>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected"),
            "Error should indicate unexpected event: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_requires_id_when_expected_order_present() {
        let raw = r#"<event name="Test Event"><trigger>YES</trigger></event>"#;
        let order = vec!["EVT_1".to_string()];
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("event id attribute"),
            "Error should indicate missing id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_count_mismatch() {
        // When expected IDs are missing, we now get a specific missing ID error
        // This is more informative than a generic count mismatch
        let raw = r#"<event id="EVT_1" name="Test"><trigger>YES</trigger></event>"#;
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()]; // Expect 2, get 1
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        // With exact coverage validation, we detect the missing ID specifically
        assert!(
            err_str.contains("Missing") && err_str.contains("EVT_2"),
            "Error should indicate missing expected event EVT_2: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_unknown_id() {
        // LLM returns an unknown/hallucinated event ID
        let raw = r#"
<event id="EVT_1" name="Known Event">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Valid event.</reasoning>
</event>
<event id="UNKNOWN_EVT" name="Unknown Event">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Hallucinated event.</reasoning>
</event>
"#;
        // Expected order only contains EVT_1, but response has UNKNOWN_EVT
        let order = vec!["EVT_1".to_string()];
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err(), "Should reject unknown event ID");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("UNKNOWN_EVT"),
            "Error should indicate unknown event id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_unknown_id_with_matching_count() {
        // Malicious/hallucinated case: count matches but IDs don't
        let raw = r#"
<event id="FAKE_1" name="Fake Event 1">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Fake.</reasoning>
</event>
<event id="FAKE_2" name="Fake Event 2">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Also fake.</reasoning>
</event>
"#;
        // Expected order has 2 items, response has 2 items, but IDs don't match
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order);
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
    fn test_parse_phase2_events_rejects_duplicate_ids() {
        // LLM returns duplicate event IDs
        let raw = r#"
<event id="EVT_1" name="First Event">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>First occurrence.</reasoning>
</event>
<event id="EVT_1" name="Duplicate Event">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Duplicate ID - should fail.</reasoning>
</event>
"#;
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err(), "Should reject duplicate event IDs");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("duplicate") && err_str.contains("EVT_1"),
            "Error should indicate duplicate event id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_duplicate_ids_with_matching_count() {
        // Malicious case: duplicate IDs but count matches expected
        let raw = r#"
<event id="EVT_1" name="First Event">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>First occurrence.</reasoning>
</event>
<event id="EVT_1" name="Duplicate Event">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Duplicate ID - EVT_2 is missing.</reasoning>
</event>
"#;
        // Expected 2 events, got 2 events, but both have same ID
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order);
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
    fn test_parse_phase2_events_rejects_missing_expected_id() {
        // LLM returns only one event when two are expected
        let raw = r#"
<event id="EVT_1" name="First Event">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Only event returned.</reasoning>
</event>
"#;
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order);
        assert!(
            result.is_err(),
            "Should reject when expected event is missing"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") && err_str.contains("EVT_2"),
            "Error should indicate missing event EVT_2: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_all_missing_expected_ids() {
        // LLM returns no events when some are expected
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order);
        assert!(
            result.is_err(),
            "Should reject when all expected events are missing"
        );
        // This should fail at the "no event tags" check before missing ID check
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("event") || err_str.contains("Missing"),
            "Error should indicate missing events: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_no_tags_when_expected() {
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let order = vec!["EVT_1".to_string()];
        let result = parse_phase2_events(raw, &order);

        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("event"),
            "Error should indicate missing event: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_sets_selected_event_correctly() {
        let raw = r#"
<event id="EVT_1" name="First Event">
  <trigger>NO</trigger>
  <confidence>low</confidence>
  <reasoning>Not triggered.</reasoning>
</event>
<event id="EVT_2" name="Second Event">
  <trigger>YES</trigger>
  <confidence>high</confidence>
  <reasoning>Triggered!</reasoning>
</event>
"#;
        let order = vec!["EVT_1".to_string(), "EVT_2".to_string()];
        let result = parse_phase2_events(raw, &order).expect("parse phase2");

        assert_eq!(result.selected_event, Some("Second Event".to_string()));
    }

    #[test]
    fn test_parse_phase2_events_rejects_malformed_tag_without_attributes() {
        // When expected_order is empty, <event> tags without attributes should be rejected
        let raw = r#"
<event><trigger>YES</trigger></event>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase2_events(raw, &order);
        assert!(
            result.is_err(),
            "Should reject <event> tags without attributes when no events expected"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("event"),
            "Error should indicate unexpected event tag: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_malformed_tag_without_id() {
        // When expected_order is empty, <event> tags with name but no id should be rejected
        let raw = r#"
<event name="Test"><trigger>YES</trigger></event>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase2_events(raw, &order);
        assert!(
            result.is_err(),
            "Should reject <event> tags without id when no events expected"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("event"),
            "Error should indicate unexpected event tag: {}",
            err_str
        );
    }
}
