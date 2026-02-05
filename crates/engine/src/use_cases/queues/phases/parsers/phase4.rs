//! Phase 4: Behavior Analysis Parser
//!
//! Parses behavior analysis from LLM response, extracting deflection behavior,
//! emotional stance, mood recommendations, and disposition changes.

use std::sync::LazyLock;

use regex_lite::Regex;

use super::common::PhaseParseError;
use crate::queue_types::{DispositionChange, Phase4BehaviorAnalysis};

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
        Regex::new(r"(?s)<disposition_recommendation>.*?.*?<reason>(.*?)</reason>.*?</disposition_recommendation>")
            .expect("valid regex")
    });

    // Parse should_deflect with strict YES/NO validation
    let should_deflect_value = DEFLECT_RE.captures(raw).map(|c| c[1].trim().to_string());
    let should_deflect = match should_deflect_value {
        Some(ref value) => {
            let upper = value.to_ascii_uppercase();
            if upper == "YES" {
                true
            } else if upper == "NO" {
                false
            } else {
                return Err(PhaseParseError::invalid_content(
                    PHASE,
                    "<should_deflect>",
                    format!("expected YES or NO, got '{}'", value),
                ));
            }
        }
        None => false,
    };

    let deflection_behavior = DEFLECT_BEHAVIOR_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.eq_ignore_ascii_case("none"));

    let emotional_stance = STANCE_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_else(|| "neutral".to_string());

    // Parse is_deceptive with strict YES/NO validation
    let is_deceptive_value = DECEPTIVE_RE.captures(raw).map(|c| c[1].trim().to_string());
    let is_deceptive = match is_deceptive_value {
        Some(ref value) => {
            let upper = value.to_ascii_uppercase();
            if upper == "YES" {
                true
            } else if upper == "NO" {
                false
            } else {
                return Err(PhaseParseError::invalid_content(
                    PHASE,
                    "<is_deceptive>",
                    format!("expected YES or NO, got '{}'", value),
                ));
            }
        }
        None => false,
    };

    let recommended_mood = MOOD_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.eq_ignore_ascii_case("no_change") && !s.eq_ignore_ascii_case("none"));

    // Parse should_change with strict YES/NO validation
    let should_change_value = DISP_CHANGE_RE
        .captures(raw)
        .map(|c| c[1].trim().to_string());
    let should_change = match should_change_value {
        Some(ref value) => {
            let upper = value.to_ascii_uppercase();
            if upper == "YES" {
                true
            } else if upper == "NO" {
                false
            } else {
                return Err(PhaseParseError::invalid_content(
                    PHASE,
                    "<should_change>",
                    format!("expected YES or NO, got '{}'", value),
                ));
            }
        }
        None => false,
    };

    let disposition_change = if should_change {
        // When should_change is true, require a non-empty new_disposition
        match NEW_DISP_RE.captures(raw).and_then(|c| c.get(1)) {
            Some(m) => {
                let new_disp = m.as_str().trim().to_string();
                if new_disp.is_empty() {
                    return Err(PhaseParseError::invalid_content(
                        PHASE,
                        "<new_disposition>",
                        "empty value when should_change=YES",
                    ));
                }
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
            }
            None => {
                return Err(PhaseParseError::missing_element(
                    PHASE,
                    "<new_disposition> (required when should_change=YES)",
                ));
            }
        }
    } else {
        None
    };

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(result.should_deflect);
        assert_eq!(
            result.deflection_behavior.as_deref(),
            Some("Change subject")
        );
        assert!(result.is_deceptive);
        assert_eq!(result.recommended_mood.as_deref(), Some("anxious"));
        let change = result
            .disposition_change
            .expect("missing disposition change");
        assert_eq!(change.new_disposition, "friendly");
        assert_eq!(change.magnitude, "slight");
        assert_eq!(change.reason, "Player showed kindness.");
    }

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
        let err = result.expect_err("should fail");
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
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("behavior_analysis"),
            "Error should indicate missing behavior_analysis: {}",
            err_str
        );
    }

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

    #[test]
    fn test_parse_phase4_behavior_rejects_invalid_should_deflect_value() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>MAYBE</should_deflect>
    <deflection_behavior>Change subject</deflection_behavior>
  </deflection>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(
            result.is_err(),
            "Should reject invalid should_deflect value"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content") && err_str.contains("MAYBE"),
            "Error should indicate invalid should_deflect value: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_rejects_invalid_is_deceptive_value() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>NO</should_deflect>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>UNKNOWN</is_deceptive>
  </emotional_stance>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(result.is_err(), "Should reject invalid is_deceptive value");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content") && err_str.contains("UNKNOWN"),
            "Error should indicate invalid is_deceptive value: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_rejects_invalid_should_change_value() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>NO</should_deflect>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>NO</is_deceptive>
  </emotional_stance>
  <disposition_recommendation>
    <should_change>PERHAPS</should_change>
    <new_disposition>friendly</new_disposition>
    <magnitude>slight</magnitude>
    <reason>Player showed kindness.</reason>
  </disposition_recommendation>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(result.is_err(), "Should reject invalid should_change value");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content") && err_str.contains("PERHAPS"),
            "Error should indicate invalid should_change value: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_rejects_empty_should_deflect() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>   </should_deflect>
    <deflection_behavior>Change subject</deflection_behavior>
  </deflection>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(result.is_err(), "Should reject empty should_deflect value");
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content"),
            "Error should indicate invalid should_deflect value: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_accepts_yes_no_lowercase() {
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>yes</should_deflect>
    <deflection_behavior>Change subject</deflection_behavior>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>no</is_deceptive>
  </emotional_stance>
  <disposition_recommendation>
    <should_change>yes</should_change>
    <new_disposition>friendly</new_disposition>
    <magnitude>slight</magnitude>
    <reason>Player showed kindness.</reason>
  </disposition_recommendation>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw).expect("parse phase4");
        assert!(result.should_deflect, "Should accept lowercase 'yes'");
        assert!(!result.is_deceptive, "Should accept lowercase 'no'");
        assert!(
            result.disposition_change.is_some(),
            "Should accept lowercase 'yes' for should_change"
        );
    }

    #[test]
    fn test_parse_phase4_behavior_requires_new_disposition_when_should_change_yes() {
        // When should_change=YES, missing <new_disposition> should fail
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>NO</should_deflect>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>NO</is_deceptive>
  </emotional_stance>
  <disposition_recommendation>
    <should_change>YES</should_change>
    <!-- Missing <new_disposition> tag -->
    <magnitude>slight</magnitude>
    <reason>Player showed kindness.</reason>
  </disposition_recommendation>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(
            result.is_err(),
            "Should fail when should_change=YES but <new_disposition> is missing"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Missing") && err_str.contains("new_disposition"),
            "Error should indicate missing new_disposition: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_rejects_empty_new_disposition_when_should_change_yes() {
        // When should_change=YES, empty <new_disposition> should fail
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>NO</should_deflect>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>NO</is_deceptive>
  </emotional_stance>
  <disposition_recommendation>
    <should_change>YES</should_change>
    <new_disposition>   </new_disposition>
    <magnitude>slight</magnitude>
    <reason>Player showed kindness.</reason>
  </disposition_recommendation>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw);
        assert!(
            result.is_err(),
            "Should fail when should_change=YES but <new_disposition> is empty"
        );
        let err = result.expect_err("should fail");
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("Invalid content") && err_str.contains("new_disposition"),
            "Error should indicate invalid/empty new_disposition: {}",
            err_str
        );
    }

    #[test]
    fn test_parse_phase4_behavior_allows_missing_new_disposition_when_should_change_no() {
        // When should_change=NO, missing <new_disposition> is fine
        let raw = r#"
<behavior_analysis>
  <deflection>
    <should_deflect>NO</should_deflect>
  </deflection>
  <emotional_stance>
    <stance>Neutral</stance>
    <is_deceptive>NO</is_deceptive>
  </emotional_stance>
  <disposition_recommendation>
    <should_change>NO</should_change>
    <!-- No <new_disposition> tag - this is fine when should_change=NO -->
  </disposition_recommendation>
</behavior_analysis>
"#;
        let result = parse_phase4_behavior(raw).expect("parse should succeed");
        // When should_change=NO, disposition_change should be None
        assert!(result.disposition_change.is_none());
    }
}
