//! Activation rules - Shared rules for LocationState and RegionState activation
//!
//! Rules determine when visual states become active. Hard rules are evaluated
//! by the engine directly. Soft rules require LLM evaluation at staging time.

use serde::{Deserialize, Serialize};

use crate::game_time::TimeOfDay;
use crate::ids::{CharacterId, NarrativeEventId};

/// A rule that determines when a state should be activated
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ActivationRule {
    // ==========================================================================
    // Hard Rules (engine-evaluated)
    // ==========================================================================
    /// Always active (used for default states)
    Always,

    /// Active on a specific date (month/day, ignoring year)
    DateExact {
        /// Month (1-12)
        month: u32,
        /// Day (1-31)
        day: u32,
    },

    /// Active within a date range (inclusive, ignoring year)
    DateRange {
        start_month: u32,
        start_day: u32,
        end_month: u32,
        end_day: u32,
    },

    /// Active during a specific time of day
    TimeOfDay {
        /// The time period when this rule is active
        period: TimeOfDay,
    },

    /// Active when a narrative event has been triggered
    EventTriggered {
        event_id: NarrativeEventId,
        /// Display name for UI
        event_name: String,
    },

    /// Active when a game flag is set
    FlagSet { flag_name: String },

    /// Active when a specific character is present in the staging
    CharacterPresent {
        character_id: CharacterId,
        /// Display name for UI
        character_name: String,
    },

    // ==========================================================================
    // Soft Rules (LLM-evaluated at staging time)
    // ==========================================================================
    /// Custom condition evaluated by LLM based on current context
    Custom {
        /// Human-readable description of the condition
        description: String,
        /// Optional additional context/prompt for the LLM
        llm_prompt: Option<String>,
    },
}

/// How multiple activation rules are combined
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationLogic {
    /// All rules must match (AND)
    #[default]
    All,
    /// Any single rule can activate (OR)
    Any,
    /// At least N rules must match
    AtLeast(u32),
}

impl ActivationRule {
    /// Returns true if this is a hard rule (engine-evaluated)
    pub fn is_hard_rule(&self) -> bool {
        !matches!(self, ActivationRule::Custom { .. })
    }

    /// Returns true if this is a soft rule (LLM-evaluated)
    pub fn is_soft_rule(&self) -> bool {
        matches!(self, ActivationRule::Custom { .. })
    }

    /// Get a display description of this rule
    pub fn description(&self) -> String {
        match self {
            ActivationRule::Always => "Always active".to_string(),
            ActivationRule::DateExact { month, day } => {
                format!("On {}/{}", month, day)
            }
            ActivationRule::DateRange {
                start_month,
                start_day,
                end_month,
                end_day,
            } => {
                format!(
                    "From {}/{} to {}/{}",
                    start_month, start_day, end_month, end_day
                )
            }
            ActivationRule::TimeOfDay { period } => {
                format!("During {}", period.display_name())
            }
            ActivationRule::EventTriggered { event_name, .. } => {
                format!("After event: {}", event_name)
            }
            ActivationRule::FlagSet { flag_name } => {
                format!("When flag '{}' is set", flag_name)
            }
            ActivationRule::CharacterPresent { character_name, .. } => {
                format!("When {} is present", character_name)
            }
            ActivationRule::Custom { description, .. } => description.clone(),
        }
    }
}

/// Result of evaluating activation rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationEvaluation {
    /// Whether the state should be activated
    is_active: bool,
    /// Rules that matched
    matched_rules: Vec<String>,
    /// Rules that didn't match
    unmatched_rules: Vec<String>,
    /// Soft rules that need LLM evaluation
    pending_soft_rules: Vec<String>,
    /// LLM reasoning (if soft rules were evaluated)
    llm_reasoning: Option<String>,
}

impl ActivationEvaluation {
    /// Create a new evaluation result for a fully resolved state
    pub fn resolved(is_active: bool, matched: Vec<String>, unmatched: Vec<String>) -> Self {
        Self {
            is_active,
            matched_rules: matched,
            unmatched_rules: unmatched,
            pending_soft_rules: Vec::new(),
            llm_reasoning: None,
        }
    }

    /// Create an evaluation that needs LLM evaluation
    pub fn needs_llm(
        matched_hard: Vec<String>,
        unmatched_hard: Vec<String>,
        pending_soft: Vec<String>,
    ) -> Self {
        Self {
            is_active: false, // Not yet determined
            matched_rules: matched_hard,
            unmatched_rules: unmatched_hard,
            pending_soft_rules: pending_soft,
            llm_reasoning: None,
        }
    }

    /// Update with LLM evaluation results
    pub fn with_llm_result(mut self, is_active: bool, reasoning: String) -> Self {
        self.is_active = is_active;
        self.llm_reasoning = Some(reasoning);
        self.pending_soft_rules.clear();
        self
    }

    // ============================================================================
    // Accessors
    // ============================================================================

    /// Whether the state should be activated
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Rules that matched
    pub fn matched_rules(&self) -> &[String] {
        &self.matched_rules
    }

    /// Rules that didn't match
    pub fn unmatched_rules(&self) -> &[String] {
        &self.unmatched_rules
    }

    /// Soft rules that need LLM evaluation
    pub fn pending_soft_rules(&self) -> &[String] {
        &self.pending_soft_rules
    }

    /// LLM reasoning (if soft rules were evaluated)
    pub fn llm_reasoning(&self) -> Option<&str> {
        self.llm_reasoning.as_deref()
    }
}

impl std::fmt::Display for ActivationLogic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivationLogic::All => write!(f, "all"),
            ActivationLogic::Any => write!(f, "any"),
            ActivationLogic::AtLeast(n) => write!(f, "at_least_{}", n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_rule_is_hard_rule() {
        assert!(ActivationRule::Always.is_hard_rule());
        assert!(ActivationRule::DateExact { month: 6, day: 20 }.is_hard_rule());
        assert!(ActivationRule::TimeOfDay {
            period: TimeOfDay::Morning
        }
        .is_hard_rule());
        assert!(ActivationRule::FlagSet {
            flag_name: "test".to_string()
        }
        .is_hard_rule());
    }

    #[test]
    fn test_activation_rule_is_soft_rule() {
        assert!(ActivationRule::Custom {
            description: "When tension is high".to_string(),
            llm_prompt: None,
        }
        .is_soft_rule());
    }

    #[test]
    fn test_activation_rule_description() {
        let rule = ActivationRule::DateRange {
            start_month: 6,
            start_day: 20,
            end_month: 6,
            end_day: 25,
        };
        assert_eq!(rule.description(), "From 6/20 to 6/25");

        let rule = ActivationRule::TimeOfDay {
            period: TimeOfDay::Evening,
        };
        assert_eq!(rule.description(), "During Evening");
    }
}
