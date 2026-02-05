//! Phase 0: Context Digest Parser
//!
//! Parses the deterministic context digest from LLM response.
//! This is typically deterministic (non-LLM), but the parser handles
//! both LLM-generated and deterministic digests.

use std::sync::LazyLock;

use regex_lite::Regex;

use crate::queue_types::Phase0ContextDigest;

static DIGEST_SCENE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<scene>(.*?)</scene>").expect("valid regex"));
static DIGEST_PLAYER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<player_action>(.*?)</player_action>").expect("valid regex"));
static DIGEST_NPC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<npc>(.*?)</npc>").expect("valid regex"));
static DIGEST_ACTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<active_elements>(.*?)</active_elements>").expect("valid regex")
});
static DIGEST_CONV_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<conversation_summary>(.*?)</conversation_summary>").expect("valid regex")
});

/// Parse Phase 0 context digest from LLM response.
///
/// This is typically deterministic (non-LLM), but the parser handles
/// both LLM-generated and deterministic digests.
///
/// Returns `None` if required tags are missing or all key fields are empty,
/// signaling that the deterministic fallback (`build_context_digest`) should be used.
pub fn parse_phase0_digest(raw: &str) -> Option<Phase0ContextDigest> {
    // Extract fields from XML tags
    let scene = DIGEST_SCENE_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let player_action = DIGEST_PLAYER_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let npc = DIGEST_NPC_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let active = DIGEST_ACTIVE_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let conversation = DIGEST_CONV_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    // Check if all required tags are missing or all key fields are empty
    // Required fields: scene, player_action, npc, active_elements, conversation_summary
    let has_scene = DIGEST_SCENE_RE.is_match(raw);
    let has_player_action = DIGEST_PLAYER_RE.is_match(raw);
    let has_npc = DIGEST_NPC_RE.is_match(raw);
    let has_active = DIGEST_ACTIVE_RE.is_match(raw);
    let has_conversation = DIGEST_CONV_RE.is_match(raw);

    // If none of the required tags are present, return None to trigger fallback
    if !has_scene && !has_player_action && !has_npc && !has_active && !has_conversation {
        return None;
    }

    // If all key fields are empty, return None to trigger fallback
    if scene.is_empty()
        && player_action.is_empty()
        && npc.is_empty()
        && active.is_empty()
        && conversation.is_empty()
    {
        return None;
    }

    // Parse challenge/event counts from active elements
    let challenge_count = super::common::parse_count(&active, "Challenges:");
    let event_count = super::common::parse_count(&active, "Events:");

    // Extract recent topics
    let recent_topics: Vec<String> = conversation
        .lines()
        .filter(|l| l.contains("Recent Topics:"))
        .flat_map(|l| {
            l.split(':').nth(1).map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .filter(|t| !t.is_empty())
        .collect();

    Some(Phase0ContextDigest {
        scene_summary: scene,
        player_action_digest: player_action,
        npc_summary: npc,
        active_challenge_count: challenge_count,
        active_event_count: event_count,
        recent_topics,
        directorial_excerpt: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phase0_digest_extracts_all_fields() {
        let raw = r#"
<scene>Tavern scene</scene>
<player_action>speak to bartender</player_action>
<npc>Marta</npc>
<active_elements>Challenges: 2 (active), Events: 1 (pending)</active_elements>
<conversation_summary>Recent Topics: gossip, rumors</conversation_summary>
"#;
        let result = parse_phase0_digest(raw);
        assert!(result.is_some());
        let digest = result.expect("result should be Some");
        assert_eq!(digest.scene_summary, "Tavern scene");
        assert_eq!(digest.player_action_digest, "speak to bartender");
        assert_eq!(digest.npc_summary, "Marta");
        assert_eq!(digest.active_challenge_count, 2);
        assert_eq!(digest.active_event_count, 1);
        assert_eq!(digest.recent_topics, vec!["gossip", "rumors"]);
    }

    #[test]
    fn test_parse_phase0_digest_same_line_counts_parsed_correctly() {
        // Regression test: both counts on same line should parse correctly
        // Previously, splitting on first ':' would cause event count to be parsed as 0
        let raw = r#"
<scene>Tavern</scene>
<player_action>ask</player_action>
<npc>Barkeep</npc>
<active_elements>Challenges: 5 (active), Events: 3 (pending)</active_elements>
<conversation_summary>Recent Topics:</conversation_summary>
"#;
        let result = parse_phase0_digest(raw).expect("should parse");
        assert_eq!(result.active_challenge_count, 5);
        assert_eq!(result.active_event_count, 3);
    }

    #[test]
    fn test_parse_phase0_digest_events_before_challenges() {
        // Ensure parsing works regardless of order
        let raw = r#"
<scene>Tavern</scene>
<player_action>ask</player_action>
<npc>Barkeep</npc>
<active_elements>Events: 7 (active), Challenges: 4 (pending)</active_elements>
<conversation_summary>Recent Topics:</conversation_summary>
"#;
        let result = parse_phase0_digest(raw).expect("should parse");
        assert_eq!(result.active_challenge_count, 4);
        assert_eq!(result.active_event_count, 7);
    }

    #[test]
    fn test_parse_phase0_digest_multiline_format() {
        // Multi-line format should also work
        let raw = r#"
<scene>Tavern</scene>
<player_action>ask</player_action>
<npc>Barkeep</npc>
<active_elements>
Challenges: 10 (active)
Events: 8 (pending)
</active_elements>
<conversation_summary>Recent Topics:</conversation_summary>
"#;
        let result = parse_phase0_digest(raw).expect("should parse");
        assert_eq!(result.active_challenge_count, 10);
        assert_eq!(result.active_event_count, 8);
    }

    #[test]
    fn test_parse_phase0_digest_no_parentheses() {
        // Format without parentheses should also parse
        let raw = r#"
<scene>Tavern</scene>
<player_action>ask</player_action>
<npc>Barkeep</npc>
<active_elements>Challenges: 15, Events: 12</active_elements>
<conversation_summary>Recent Topics:</conversation_summary>
"#;
        let result = parse_phase0_digest(raw).expect("should parse");
        assert_eq!(result.active_challenge_count, 15);
        assert_eq!(result.active_event_count, 12);
    }

    #[test]
    fn test_parse_phase0_digest_returns_none_for_empty() {
        let raw = r#""#;
        let result = parse_phase0_digest(raw);
        // Empty input should return None to trigger deterministic fallback
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_phase0_digest_returns_none_for_malformed() {
        // Malformed input with no required tags should return None
        let raw = r#"Some random text without proper tags"#;
        let result = parse_phase0_digest(raw);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_phase0_digest_returns_none_for_all_empty_tags() {
        // Input with all empty tags should return None
        let raw = r#"
<scene></scene>
<player_action></player_action>
<npc></npc>
<active_elements></active_elements>
<conversation_summary></conversation_summary>
"#;
        let result = parse_phase0_digest(raw);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_phase0_digest_returns_some_for_partial_content() {
        // Input with at least one non-empty field should return Some
        let raw = r#"
<scene>Tavern</scene>
<player_action></player_action>
<npc></npc>
<active_elements></active_elements>
<conversation_summary></conversation_summary>
"#;
        let result = parse_phase0_digest(raw);
        assert!(result.is_some());
        let digest = result.expect("result should be Some");
        assert_eq!(digest.scene_summary, "Tavern");
        assert_eq!(digest.player_action_digest, "");
        assert_eq!(digest.npc_summary, "");
    }
}
