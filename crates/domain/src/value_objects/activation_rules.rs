//! Activation rules - Shared rules for LocationState and RegionState activation
//!
//! Rules determine when visual states become active. Hard rules are evaluated
//! by the engine directly. Soft rules require LLM evaluation at staging time.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::game_time::TimeOfDay;
use crate::ids::{CharacterId, NarrativeEventId};

/// A rule that determines when a state should be activated
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
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
///
/// This value object enforces invariants through its constructors:
/// - Rules cannot appear in both matched and unmatched lists
/// - Rule descriptions cannot be empty
/// - Soft rules cannot have duplicates
/// - `is_active` is only set for resolved evaluations (not when pending LLM)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivationEvaluation {
    is_active: bool,
    matched_rules: Vec<String>,
    unmatched_rules: Vec<String>,
    pending_soft_rules: Vec<String>,
    llm_reasoning: Option<String>,
}

impl ActivationEvaluation {
    /// Create a new evaluation result for a fully resolved state.
    ///
    /// # Invariants
    /// - No rule can appear in both `matched` and `unmatched`
    /// - Rule descriptions cannot be empty
    pub fn resolved(
        is_active: bool,
        matched: Vec<String>,
        unmatched: Vec<String>,
    ) -> Result<Self, DomainError> {
        let eval = Self {
            is_active,
            matched_rules: matched,
            unmatched_rules: unmatched,
            pending_soft_rules: Vec::new(),
            llm_reasoning: None,
        };
        eval.validate()?;
        Ok(eval)
    }

    /// Create an evaluation that needs LLM evaluation.
    ///
    /// # Invariants
    /// - No rule can appear in both `matched_hard` and `unmatched_hard`
    /// - Rule descriptions cannot be empty
    /// - Soft rules cannot have duplicates
    /// - `is_active` is always `false` (not yet determined)
    pub fn needs_llm(
        matched_hard: Vec<String>,
        unmatched_hard: Vec<String>,
        pending_soft: Vec<String>,
    ) -> Result<Self, DomainError> {
        let eval = Self {
            is_active: false,
            matched_rules: matched_hard,
            unmatched_rules: unmatched_hard,
            pending_soft_rules: pending_soft,
            llm_reasoning: None,
        };
        eval.validate()?;
        Ok(eval)
    }

    /// Update with LLM evaluation results.
    ///
    /// # Invariants
    /// - Clears `pending_soft_rules` (now resolved)
    /// - Sets `is_active` based on LLM result
    /// - Stores reasoning
    pub fn with_llm_result(mut self, is_active: bool, reasoning: String) -> Self {
        self.is_active = is_active;
        self.llm_reasoning = Some(reasoning);
        self.pending_soft_rules.clear();
        self
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Accessors
    // ──────────────────────────────────────────────────────────────────────────

    /// Check if the state should be activated.
    ///
    /// Returns `false` when evaluation is pending LLM.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get rules that matched.
    pub fn matched_rules(&self) -> &[String] {
        &self.matched_rules
    }

    /// Get rules that didn't match.
    pub fn unmatched_rules(&self) -> &[String] {
        &self.unmatched_rules
    }

    /// Get soft rules that need LLM evaluation.
    pub fn pending_soft_rules(&self) -> &[String] {
        &self.pending_soft_rules
    }

    /// Get LLM reasoning (if soft rules were evaluated).
    pub fn llm_reasoning(&self) -> Option<&String> {
        self.llm_reasoning.as_ref()
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Validation (private)
    // ──────────────────────────────────────────────────────────────────────────

    /// Validate the activation evaluation.
    ///
    /// Checks for:
    /// - No overlapping rules (a rule can't be in both matched and unmatched)
    /// - No duplicate soft rules
    /// - Non-empty rule descriptions
    fn validate(&self) -> Result<(), DomainError> {
        // Check for rule overlaps
        let matched_set: std::collections::HashSet<_> = self.matched_rules.iter().collect();
        let unmatched_set: std::collections::HashSet<_> = self.unmatched_rules.iter().collect();
        let overlap: Vec<_> = matched_set.intersection(&unmatched_set).cloned().collect();
        if !overlap.is_empty() {
            return Err(DomainError::validation(format!(
                "Rules appear in both matched and unmatched: {:?}",
                overlap
            )));
        }

        // Check for empty rule descriptions
        for rule in &self.matched_rules {
            if rule.trim().is_empty() {
                return Err(DomainError::validation(
                    "Matched rule descriptions cannot be empty".to_string(),
                ));
            }
        }

        for rule in &self.unmatched_rules {
            if rule.trim().is_empty() {
                return Err(DomainError::validation(
                    "Unmatched rule descriptions cannot be empty".to_string(),
                ));
            }
        }

        // Check for duplicate soft rules
        let pending_set: std::collections::HashSet<_> = self.pending_soft_rules.iter().collect();
        if pending_set.len() != self.pending_soft_rules.len() {
            return Err(DomainError::validation(
                "Duplicate soft rules detected".to_string(),
            ));
        }

        Ok(())
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

    // -------------------------------------------------------------------------
    // ActivationEvaluation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_activation_evaluation_resolved_active() {
        let eval = ActivationEvaluation::resolved(
            true,
            vec!["Rule A".to_string(), "Rule B".to_string()],
            vec!["Rule C".to_string()],
        )
        .expect("valid evaluation");

        assert!(eval.is_active());
        assert_eq!(eval.matched_rules(), &["Rule A", "Rule B"]);
        assert_eq!(eval.unmatched_rules(), &["Rule C"]);
        assert!(eval.pending_soft_rules().is_empty());
        assert!(eval.llm_reasoning().is_none());
    }

    #[test]
    fn test_activation_evaluation_resolved_inactive() {
        let eval = ActivationEvaluation::resolved(false, vec![], vec!["Rule A".to_string()])
            .expect("valid evaluation");

        assert!(!eval.is_active());
        assert_eq!(eval.matched_rules().len(), 0);
        assert_eq!(eval.unmatched_rules(), &["Rule A"]);
    }

    #[test]
    fn test_activation_evaluation_needs_llm() {
        let eval = ActivationEvaluation::needs_llm(
            vec!["Rule A".to_string()],
            vec!["Rule B".to_string()],
            vec!["Custom Rule X".to_string(), "Custom Rule Y".to_string()],
        )
        .expect("valid evaluation");

        assert!(!eval.is_active());
        assert_eq!(eval.matched_rules(), &["Rule A"]);
        assert_eq!(eval.unmatched_rules(), &["Rule B"]);
        assert_eq!(
            eval.pending_soft_rules(),
            &["Custom Rule X", "Custom Rule Y"]
        );
        assert!(eval.llm_reasoning().is_none());
    }

    #[test]
    fn test_activation_evaluation_with_llm_result() {
        let eval = ActivationEvaluation::needs_llm(
            vec!["Rule A".to_string()],
            vec![],
            vec!["Custom Rule".to_string()],
        )
        .expect("valid evaluation");

        let resolved = eval.with_llm_result(true, "The custom rule matched".to_string());

        assert!(resolved.is_active());
        assert_eq!(resolved.matched_rules(), &["Rule A"]);
        assert!(resolved.pending_soft_rules().is_empty());
        assert_eq!(
            resolved.llm_reasoning(),
            Some(&"The custom rule matched".to_string())
        );
    }

    #[test]
    fn test_activation_evaluation_rejects_overlapping_rules() {
        let result = ActivationEvaluation::resolved(
            true,
            vec!["Rule A".to_string()],
            vec!["Rule A".to_string()], // Same rule in both!
        );

        assert!(result.is_err());
        let err = result.expect_err("overlap should fail");
        assert!(err
            .to_string()
            .contains("Rules appear in both matched and unmatched"));
    }

    #[test]
    fn test_activation_evaluation_rejects_empty_rule_descriptions() {
        let result = ActivationEvaluation::resolved(
            true,
            vec!["Valid Rule".to_string(), "".to_string()], // Empty description
            vec![],
        );

        assert!(result.is_err());
        let err = result.expect_err("empty rule should fail");
        assert!(err
            .to_string()
            .contains("Matched rule descriptions cannot be empty"));
    }

    #[test]
    fn test_activation_evaluation_rejects_duplicate_soft_rules() {
        let result = ActivationEvaluation::needs_llm(
            vec![],
            vec![],
            vec!["Custom Rule".to_string(), "Custom Rule".to_string()], // Duplicate
        );

        assert!(result.is_err());
        let err = result.expect_err("duplicate should fail");
        assert!(err.to_string().contains("Duplicate soft rules detected"));
    }
}
