//! Expression configuration for characters
//!
//! Part of the three-tier emotional model (Tier 3: Expression).
//! This defines what expressions and actions a character can display.

use serde::{Deserialize, Serialize};

/// Configuration for a character's available expressions and actions
///
/// Each character can have a unique set of expressions based on their
/// sprite sheet. This is typically configured when setting up character assets.
///
/// # Example
/// ```
/// use wrldbldr_domain::ExpressionConfig;
///
/// let config = ExpressionConfig::new()
///     .with_expressions(vec!["neutral", "happy", "sad", "angry", "suspicious"])
///     .with_actions(vec!["sighs", "laughs", "nods", "shakes head"])
///     .with_default_expression("neutral");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionConfig {
    /// Available expression names for this character's sprite sheet
    /// Examples: "neutral", "happy", "sad", "angry", "surprised", "afraid", "thoughtful"
    pub expressions: Vec<String>,

    /// Custom action names this character can perform
    /// Examples: "sighs", "laughs", "nods", "shakes head", "crosses arms"
    /// Actions are rendered as stage directions in dialogue
    pub actions: Vec<String>,

    /// Default expression to show when no marker is specified
    /// This is overridden by MoodState.default_expression() when NPC has a mood set
    pub default_expression: String,
}

impl Default for ExpressionConfig {
    fn default() -> Self {
        Self {
            expressions: Self::standard_expressions(),
            actions: Self::standard_actions(),
            default_expression: "neutral".to_string(),
        }
    }
}

impl ExpressionConfig {
    /// Create a new empty config (use with builder methods)
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
            actions: Vec::new(),
            default_expression: "neutral".to_string(),
        }
    }

    /// Create a config with standard VN expressions
    pub fn standard() -> Self {
        Self::default()
    }

    /// Standard expression set for most characters
    pub fn standard_expressions() -> Vec<String> {
        vec![
            "neutral".to_string(),
            "happy".to_string(),
            "sad".to_string(),
            "angry".to_string(),
            "surprised".to_string(),
            "afraid".to_string(),
            "thoughtful".to_string(),
            "suspicious".to_string(),
        ]
    }

    /// Standard action set for most characters
    pub fn standard_actions() -> Vec<String> {
        vec![
            "sighs".to_string(),
            "laughs".to_string(),
            "nods".to_string(),
            "shakes head".to_string(),
            "smiles".to_string(),
            "frowns".to_string(),
        ]
    }

    /// Builder: set available expressions
    pub fn with_expressions(
        mut self,
        expressions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expressions = expressions.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Builder: set available actions
    pub fn with_actions(mut self, actions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.actions = actions.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Builder: set default expression
    pub fn with_default_expression(mut self, expression: impl Into<String>) -> Self {
        self.default_expression = expression.into();
        self
    }

    /// Add an expression to the list
    pub fn add_expression(&mut self, expression: impl Into<String>) {
        let expr = expression.into();
        if !self.expressions.contains(&expr) {
            self.expressions.push(expr);
        }
    }

    /// Add an action to the list
    pub fn add_action(&mut self, action: impl Into<String>) {
        let act = action.into();
        if !self.actions.contains(&act) {
            self.actions.push(act);
        }
    }

    /// Check if an expression is available for this character
    pub fn has_expression(&self, expression: &str) -> bool {
        self.expressions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(expression))
    }

    /// Check if an action is available for this character
    pub fn has_action(&self, action: &str) -> bool {
        self.actions.iter().any(|a| a.eq_ignore_ascii_case(action))
    }

    /// Get a formatted list for LLM context
    pub fn format_for_llm(&self) -> String {
        let mut result = String::new();

        if !self.expressions.is_empty() {
            result.push_str("Available expressions: ");
            result.push_str(&self.expressions.join(", "));
        }

        if !self.actions.is_empty() {
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str("Available actions: ");
            result.push_str(&self.actions.join(", "));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ExpressionConfig::default();
        assert!(config.has_expression("neutral"));
        assert!(config.has_expression("happy"));
        assert!(config.has_action("sighs"));
        assert_eq!(config.default_expression, "neutral");
    }

    #[test]
    fn test_builder_pattern() {
        let config = ExpressionConfig::new()
            .with_expressions(vec!["calm", "excited"])
            .with_actions(vec!["waves"])
            .with_default_expression("calm");

        assert!(config.has_expression("calm"));
        assert!(config.has_expression("excited"));
        assert!(!config.has_expression("neutral"));
        assert!(config.has_action("waves"));
        assert_eq!(config.default_expression, "calm");
    }

    #[test]
    fn test_case_insensitive_lookup() {
        let config = ExpressionConfig::default();
        assert!(config.has_expression("HAPPY"));
        assert!(config.has_expression("Happy"));
        assert!(config.has_action("SIGHS"));
    }

    #[test]
    fn test_format_for_llm() {
        let config = ExpressionConfig::new()
            .with_expressions(vec!["neutral", "happy"])
            .with_actions(vec!["nods"]);

        let formatted = config.format_for_llm();
        assert!(formatted.contains("neutral, happy"));
        assert!(formatted.contains("nods"));
    }

    #[test]
    fn test_add_expression_deduplication() {
        let mut config = ExpressionConfig::new().with_expressions(vec!["neutral"]);

        config.add_expression("neutral");
        config.add_expression("happy");

        assert_eq!(config.expressions.len(), 2);
    }
}
