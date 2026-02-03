//! Phase-specific parsers for multi-phase LLM pipeline.
//!
//! Each module provides parsing for a specific phase's structured output.
//! These parsers are strict - they fail if required elements are missing.

use regex_lite::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::queue_types::{
    DispositionChange, Phase0ContextDigest, Phase1ChallengeTriggers, Phase2EventTriggers,
    Phase3ToolIntent, Phase4BehaviorAnalysis, Phase5FinalResponse, PhaseChallengeAnalysis,
    PhaseEventAnalysis,
};

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
    fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self::Other {
            phase,
            message: message.into(),
        }
    }

    /// Create a missing element error for the given phase.
    fn missing_element(phase: &'static str, element: impl Into<String>) -> Self {
        Self::MissingElement {
            phase,
            element: element.into(),
        }
    }

    /// Create an invalid content error for the given phase.
    fn invalid_content(
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
    fn count_mismatch(
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
    fn unexpected(
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
    fn missing_attribute(phase: &'static str, attribute: impl Into<String>) -> Self {
        Self::MissingAttribute {
            phase,
            attribute: attribute.into(),
        }
    }
}

// =============================================================================
// Shared Regex Patterns
// =============================================================================

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

// =============================================================================
// Phase 0: Context Digest Parser
// =============================================================================

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
    let challenge_count = parse_count(&active, "Challenges:");
    let event_count = parse_count(&active, "Events:");

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

fn parse_count(text: &str, prefix: &str) -> usize {
    // Use regex to match the specific prefix followed by optional whitespace and a number
    // This correctly handles cases where both counts are on the same line
    // e.g., "Challenges: 2 (active), Events: 1 (pending)"
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

// =============================================================================
// Phase 1: Challenge Trigger Parser
// =============================================================================

/// Parse Phase 1 challenge triggers from LLM response.
pub fn parse_phase1_challenges(
    raw: &str,
    expected_order: &[String],
) -> Result<Phase1ChallengeTriggers, PhaseParseError> {
    const PHASE: &str = "Phase 1";
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
        Regex::new(r#"(?s)<challenge\s+([^>]+)>(.*?)</challenge>"#).expect("valid regex")
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

// =============================================================================
// Phase 2: Event Trigger Parser
// =============================================================================

/// Parse Phase 2 event triggers from LLM response.
pub fn parse_phase2_events(
    raw: &str,
    expected_order: &[String],
) -> Result<Phase2EventTriggers, PhaseParseError> {
    const PHASE: &str = "Phase 2";
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
    static EVENT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)<event\s+([^>]+)>(.*?)</event>"#).expect("valid regex"));
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

// =============================================================================
// Phase 3: Tool Intent Parser
// =============================================================================

/// Parse Phase 3 tool intent from LLM response.
pub fn parse_phase3_tools(raw: &str) -> Result<Phase3ToolIntent, PhaseParseError> {
    use crate::game_tools::ToolArgs;
    use crate::queue_types::ProposedTool;
    use crate::use_cases::queues::response_parser::generate_tool_id;

    const PHASE: &str = "Phase 3";

    static SHOULD_ACT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<should_act>(.*?)</should_act>").expect("valid regex"));
    static TOOL_REASONING_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<tool_analysis>.*?<reasoning>(.*?)</reasoning>.*?</tool_analysis>")
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

    let mut proposed_tools = Vec::new();

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
    let reasoning = TOOL_REASONING_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.is_empty());

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
        // Strict parser contract: invalid JSON is a hard parse error
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

    if should_act.is_none() {
        return Err(PhaseParseError::missing_element(
            PHASE,
            "<tool_analysis>/<should_act>",
        ));
    }

    let has_tools = !proposed_tools.is_empty();
    let mut intent = Phase3ToolIntent {
        proposed_tools,
        has_tools,
        should_act,
        reasoning,
    };

    if intent.should_act == Some(false) {
        intent.proposed_tools.clear();
        intent.has_tools = false;
    }

    if intent.should_act == Some(true) && intent.proposed_tools.is_empty() {
        return Err(PhaseParseError::invalid_content(
            PHASE,
            "<should_act>",
            "YES but no valid tools parsed",
        ));
    }

    Ok(intent)
}

// =============================================================================
// Phase 4: Behavior Analysis Parser
// =============================================================================

/// Parse Phase 4 behavior analysis from LLM response.
pub fn parse_phase4_behavior(raw: &str) -> Result<Phase4BehaviorAnalysis, PhaseParseError> {
    const PHASE: &str = "Phase 4";

    static DEFLECT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<should_deflect>(.*?)</should_deflect>").expect("valid regex")
    });
    static DEFLECT_BEHAVIOR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<deflection_behavior>(.*?)</deflection_behavior>").expect("valid regex")
    });
    static STANCE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<stance>(.*?)</stance>").expect("valid regex"));
    static DECEPTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<is_deceptive>(.*?)</is_deceptive>").expect("valid regex")
    });
    static MOOD_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<recommended_mood>(.*?)</recommended_mood>").expect("valid regex")
    });
    static DISP_CHANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<should_change>(.*?)</should_change>").expect("valid regex")
    });
    static NEW_DISP_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<new_disposition>(.*?)</new_disposition>").expect("valid regex")
    });
    static MAGNITUDE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<magnitude>(.*?)</magnitude>").expect("valid regex"));
    static DISP_REASON_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<disposition_recommendation>.*?<reason>(.*?)</reason>.*?</disposition_recommendation>")
            .expect("valid regex")
    });

    let should_deflect = DEFLECT_RE
        .captures(raw)
        .map(|c| c[1].trim().eq_ignore_ascii_case("YES"))
        .unwrap_or(false);

    let deflection_behavior = DEFLECT_BEHAVIOR_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.eq_ignore_ascii_case("none"));

    let emotional_stance = STANCE_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_else(|| "neutral".to_string());

    let is_deceptive = DECEPTIVE_RE
        .captures(raw)
        .map(|c| c[1].trim().eq_ignore_ascii_case("YES"))
        .unwrap_or(false);

    let recommended_mood = MOOD_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.eq_ignore_ascii_case("no_change") && !s.eq_ignore_ascii_case("none"));

    let disposition_change = DISP_CHANGE_RE
        .captures(raw)
        .filter(|c| c[1].trim().eq_ignore_ascii_case("YES"))
        .and_then(|_| {
            let new_disp = NEW_DISP_RE
                .captures(raw)?
                .get(1)?
                .as_str()
                .trim()
                .to_string();
            let magnitude = MAGNITUDE_RE
                .captures(raw)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_else(|| "slight".to_string());
            let reason = DISP_REASON_RE
                .captures(raw)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default();

            Some(DispositionChange {
                new_disposition: new_disp,
                reason,
                magnitude,
            })
        });

    if !raw.contains("<behavior_analysis>") {
        return Err(PhaseParseError::missing_element(
            PHASE,
            "<behavior_analysis> block",
        ));
    }

    if should_deflect && deflection_behavior.as_deref().unwrap_or("").is_empty() {
        return Err(PhaseParseError::invalid_content(
            PHASE,
            "<should_deflect>",
            "YES but <deflection_behavior> is missing or empty",
        ));
    }

    Ok(Phase4BehaviorAnalysis {
        should_deflect,
        deflection_behavior,
        recommended_mood,
        disposition_change,
        emotional_stance,
        is_deceptive,
    })
}

// =============================================================================
// Phase 5: Final Response Parser
// =============================================================================

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

/// Strip all known XML tags from a response.
fn strip_all_tags(raw: &str) -> String {
    use regex_lite::Regex;

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
        assert_eq!(result.challenges[0].should_trigger, true);
        assert_eq!(
            result.challenges[0].trigger_quote.as_deref(),
            Some("test quote")
        );
    }

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
        assert_eq!(result.events[0].should_trigger, true);
        assert_eq!(result.selected_event, Some("Test Event".to_string()));
    }

    #[test]
    fn test_parse_phase4_behavior() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>YES</should_deflect>
    <deflection_behavior>Change subject</deflection_behavior>
  </deflection>
  <emotional_stance>
    <stance>Nervous</stance>
    <is_deceptive>YES</is_deceptive>
  </emotional_stance>
  <mood_recommendation>
    <recommended_mood>anxious</recommended_mood>
  </mood_recommendation>
  <disposition_recommendation>
    <should_change>YES</should_change>
    <new_disposition>friendly</new_disposition>
    <magnitude>slight</magnitude>
    <reason>Player showed kindness.</reason>
  </disposition_recommendation>
</behavior_analysis>
"#;

        let result = parse_phase4_behavior(raw).expect("parse phase4");
        assert_eq!(result.should_deflect, true);
        assert_eq!(
            result.deflection_behavior.as_deref(),
            Some("Change subject")
        );
        assert_eq!(result.is_deceptive, true);
        assert_eq!(result.recommended_mood.as_deref(), Some("anxious"));
        let change = result
            .disposition_change
            .expect("missing disposition change");
        assert_eq!(change.new_disposition, "friendly");
        assert_eq!(change.magnitude, "slight");
        assert_eq!(change.reason, "Player showed kindness.");
    }

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
            crate::game_tools::ToolArgs::GiveItem {
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
        assert_eq!(parsed.has_tools, false);
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

    // =============================================================================
    // High Priority: Expected Order Reordering Tests
    // =============================================================================

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

    // =============================================================================
    // High Priority: Empty Expected Order Validation
    // =============================================================================

    #[test]
    fn test_parse_phase1_challenges_rejects_when_no_active_challenges() {
        let raw = r#"<challenge id="CHAL_1" name="Test"><trigger>YES</trigger></challenge>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected"),
            "Error should indicate unexpected challenge: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_when_no_active_events() {
        let raw = r#"<event id="EVT_1" name="Test"><trigger>YES</trigger></event>"#;
        let order: Vec<String> = vec![];
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected"),
            "Error should indicate unexpected event: {}",
            err_str
        );
    }

    // =============================================================================
    // High Priority: Should_Act=True Without Tools
    // =============================================================================

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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("YES but no valid tools"),
            "Error should indicate missing tools: {}",
            err_str
        );
    }

    // =============================================================================
    // Medium Priority: Missing ID Attribute Handling
    // =============================================================================

    #[test]
    fn test_parse_phase1_challenges_requires_id_when_expected_order_present() {
        let raw = r#"<challenge name="Test Challenge"><trigger>YES</trigger></challenge>"#;
        let order = vec!["CHAL_1".to_string()];
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("challenge id attribute"),
            "Error should indicate missing id: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_requires_id_when_expected_order_present() {
        let raw = r#"<event name="Test Event"><trigger>YES</trigger></event>"#;
        let order = vec!["EVT_1".to_string()];
        let result = parse_phase2_events(raw, &order);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("event id attribute"),
            "Error should indicate missing id: {}",
            err_str
        );
    }

    // =============================================================================
    // Medium Priority: Count Mismatch Validation
    // =============================================================================

    #[test]
    fn test_parse_phase1_challenges_rejects_count_mismatch() {
        // When expected IDs are missing, we now get a specific missing ID error
        // This is more informative than a generic count mismatch
        let raw = r#"<challenge id="CHAL_1" name="Test"><trigger>YES</trigger></challenge>"#;
        let order = vec!["CHAL_1".to_string(), "CHAL_2".to_string()]; // Expect 2, get 1
        let result = parse_phase1_challenges(raw, &order);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        // With exact coverage validation, we detect the missing ID specifically
        assert!(
            err_str.contains("Missing") && err_str.contains("CHAL_2"),
            "Error should indicate missing expected challenge CHAL_2: {}",
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        // With exact coverage validation, we detect the missing ID specifically
        assert!(
            err_str.contains("Missing") && err_str.contains("EVT_2"),
            "Error should indicate missing expected event EVT_2: {}",
            err_str
        );
    }

    // =============================================================================
    // High Priority: Unknown ID Rejection
    // =============================================================================

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
        let err = result.unwrap_err();
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("FAKE_1"),
            "Error should indicate first unknown id: {}",
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
        let err = result.unwrap_err();
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Unexpected") && err_str.contains("FAKE_1"),
            "Error should indicate first unknown id: {}",
            err_str
        );
    }

    // =============================================================================
    // Critical: Duplicate ID Detection
    // =============================================================================

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
        let err = result.unwrap_err();
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("duplicate"),
            "Error should indicate duplicate: {}",
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
        let err = result.unwrap_err();
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("duplicate"),
            "Error should indicate duplicate: {}",
            err_str
        );
    }

    // =============================================================================
    // Critical: Missing Expected ID Detection
    // =============================================================================

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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") && err_str.contains("CHAL_2"),
            "Error should indicate missing challenge CHAL_2: {}",
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") && err_str.contains("EVT_2"),
            "Error should indicate missing event EVT_2: {}",
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("challenge") || err_str.contains("Missing"),
            "Error should indicate missing challenges: {}",
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("event") || err_str.contains("Missing"),
            "Error should indicate missing events: {}",
            err_str
        );
    }

    // =============================================================================
    // Medium Priority: Phase 3 Missing Should_Act
    // =============================================================================

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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") || err_str.contains("<tool_analysis>"),
            "Error should indicate missing should_act: {}",
            err_str
        );
    }

    // =============================================================================
    // Medium Priority: Phase 4 Deflection Validation
    // =============================================================================

    #[test]
    fn test_parse_phase4_behavior_requires_deflection_behavior_when_yes() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>YES</should_deflect>
    <!-- Missing deflection_behavior -->
  </deflection>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("deflection_behavior"),
            "Error should indicate missing deflection_behavior: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_requires_behavior_analysis_block() {
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let result = parse_phase4_behavior(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("behavior_analysis"),
            "Error should indicate missing behavior_analysis: {}",
            err_str
        );
    }

    // =============================================================================
    // Medium Priority: Phase 5 Dialogue Validation
    // =============================================================================

    #[test]
    fn test_parse_phase5_response_requires_dialogue() {
        let raw = r#"<reasoning>Some thoughts</reasoning>"#; // No dialogue
        let result = parse_phase5_response(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("dialogue"),
            "Error should indicate missing dialogue: {}",
            err_str
        );
    }

    // =============================================================================
    // Medium Priority: Phase 0 Digest Parsing
    // =============================================================================

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
        let digest = result.unwrap();
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
        let digest = result.unwrap();
        assert_eq!(digest.scene_summary, "Tavern");
        assert_eq!(digest.player_action_digest, "");
        assert_eq!(digest.npc_summary, "");
    }

    // =============================================================================
    // Medium Priority: Phase 2 Selected Event
    // =============================================================================

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

    // =============================================================================
    // Medium Priority: Phase 4 Optional Field Handling
    // =============================================================================

    #[test]
    fn test_parse_phase4_behavior_no_disposition_change() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>NO</should_deflect>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>NO</is_deceptive>
  </emotional_stance>
  <mood_recommendation>
    <recommended_mood>no_change</recommended_mood>
  </mood_recommendation>
  <disposition_recommendation>
    <should_change>NO</should_change>
  </disposition_recommendation>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw).expect("parse phase4");

        assert!(!result.should_deflect);
        assert!(result.deflection_behavior.is_none());
        assert!(result.recommended_mood.is_none()); // "no_change" is filtered
        assert!(result.disposition_change.is_none());
    }

    // =============================================================================
    // Medium Priority: Phase 5 Optional Field Handling
    // =============================================================================

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

    // =============================================================================
    // Low Priority: No Tags When Expected
    // =============================================================================

    #[test]
    fn test_parse_phase1_challenges_rejects_no_tags_when_expected() {
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let order = vec!["CHAL_1".to_string()];
        let result = parse_phase1_challenges(raw, &order);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("challenge"),
            "Error should indicate missing challenge: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase2_events_rejects_no_tags_when_expected() {
        let raw = r#"<some_other_tag>content</some_other_tag>"#;
        let order = vec!["EVT_1".to_string()];
        let result = parse_phase2_events(raw, &order);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("event"),
            "Error should indicate missing event: {}",
            err_str
        );
    }

    // =============================================================================
    // Phase 3 Strict Should_Act Validation
    // =============================================================================

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
        let err = result.unwrap_err();
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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content"),
            "Error should indicate invalid should_act value: {}",
            err_str
        );
    }

    // =============================================================================
    // Phase 3 Invalid JSON Arguments
    // =============================================================================

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
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("JSON parse error") && err_str.contains("give_item"),
            "Error should indicate JSON parse error for tool: {}",
            err_str
        );
    }

    // =============================================================================
    // Typed Error Tests
    // =============================================================================

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
            serde_json::from_str::<serde_json::Value>("{ invalid }").unwrap_err();
        let err = PhaseParseError::JsonParse {
            phase: "Phase 3",
            message: json_err.to_string(),
        };

        // Verify the error implements std::error::Error
        let err_str = format!("{}", err);
        assert!(err_str.contains("JSON parse error in Phase 3"));
    }

    // =============================================================================
    // Tool ID Derivation Hardening Tests
    // =============================================================================

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
}
