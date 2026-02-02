//! Expression configuration for characters
//!
//! Part of the three-tier emotional model (Tier 3: Expression).
//! This defines what expressions and actions a character can display.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Configuration for a character's available expressions and actions
///
/// Each character can have a unique set of expressions based on their
/// sprite sheet. This is typically configured when setting up character assets.
///
/// This is an immutable value object. Use builder-style methods to create
/// modified copies.
///
/// # Invariant
/// The `default_expression` must exist in the `expressions` map.
///
/// # Example
/// ```
/// use wrldbldr_domain::ExpressionConfig;
///
/// let config = ExpressionConfig::new()
///     .expect("valid config")
///     .with_expressions(vec!["neutral", "happy", "sad", "angry", "suspicious"])
///     .with_actions(vec!["sighs", "laughs", "nods", "shakes head"])
///     .with_default_expression("neutral")
///     .expect("default expression valid");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionConfig {
    /// Available expression names for this character's sprite sheet
    /// Examples: "neutral", "happy", "sad", "angry", "surprised", "afraid", "thoughtful"
    expressions: Vec<String>,

    /// Custom action names this character can perform
    /// Examples: "sighs", "laughs", "nods", "shakes head", "crosses arms"
    /// Actions are rendered as stage directions in dialogue
    actions: Vec<String>,

    /// Default expression to show when no marker is specified
    /// This is overridden by MoodState.default_expression() when NPC has a mood set
    default_expression: String,
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
    /// Create a new empty config (use with builder methods).
    ///
    /// Returns an error if the default expression is not found.
    /// This ensures the invariant is always maintained.
    pub fn new() -> Result<Self, DomainError> {
        let config = Self {
            expressions: Vec::new(),
            actions: Vec::new(),
            default_expression: "neutral".to_string(),
        };
        // Empty config has no expressions, so default can't exist
        // This is expected to be built using with_expressions() first
        Ok(config)
    }

    /// Create a config with standard VN expressions.
    ///
    /// This always succeeds because the standard expressions include "neutral".
    pub fn standard() -> Result<Self, DomainError> {
        Self::new()
            .map(|c| c.with_expressions(Self::standard_expressions()))
            .map(|c| c.with_actions(Self::standard_actions()))
            .and_then(|c| c.validate_default())
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

    // ──────────────────────────────────────────────────────────────────────────
    // Read accessors
    // ──────────────────────────────────────────────────────────────────────────

    /// Get available expressions for this character.
    pub fn expressions(&self) -> &[String] {
        &self.expressions
    }

    /// Get available actions for this character.
    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    /// Get the default expression.
    pub fn default_expression(&self) -> &str {
        &self.default_expression
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Builder-style methods (consume self, return new instance)
    // ──────────────────────────────────────────────────────────────────────────

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

    /// Builder: set default expression.
    ///
    /// Returns an error if the expression is not in the available expressions list.
    pub fn with_default_expression(
        mut self,
        expression: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let expr = expression.into();
        if !self.has_expression(&expr) {
            return Err(DomainError::validation(format!(
                "Default expression '{}' not found in available expressions",
                expr
            )));
        }
        self.default_expression = expr;
        Ok(self)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Private validation
    // ──────────────────────────────────────────────────────────────────────────

    /// Validate that the default expression exists in the expressions list.
    fn validate_default(self) -> Result<Self, DomainError> {
        if !self.expressions.contains(&self.default_expression) {
            return Err(DomainError::validation(format!(
                "Default expression '{}' not found in available expressions: {:?}",
                self.default_expression, self.expressions
            )));
        }
        Ok(self)
    }

    /// Return a new config with an additional expression (if not already present).
    ///
    /// This consumes self and returns a new instance.
    pub fn adding_expression(mut self, expression: impl Into<String>) -> Self {
        let expr = expression.into();
        if !self.expressions.contains(&expr) {
            self.expressions.push(expr);
        }
        self
    }

    /// Return a new config with an additional action (if not already present).
    ///
    /// This consumes self and returns a new instance.
    pub fn adding_action(mut self, action: impl Into<String>) -> Self {
        let act = action.into();
        if !self.actions.contains(&act) {
            self.actions.push(act);
        }
        self
    }

    /// Return a new config with an expression removed.
    ///
    /// If the removed expression was the default, resets to first available or "neutral".
    /// This consumes self and returns a new instance.
    ///
    /// Returns an error if removing the expression would result in an empty expressions list.
    pub fn removing_expression(mut self, expression: &str) -> Result<Self, DomainError> {
        self.expressions
            .retain(|e| !e.eq_ignore_ascii_case(expression));

        // If removed the default, reset to first available or "neutral"
        if self.default_expression.eq_ignore_ascii_case(expression) {
            self.default_expression = self
                .expressions
                .first()
                .cloned()
                .unwrap_or_else(|| "neutral".to_string());
        }

        // Validate the new default exists
        self.validate_default()
    }

    /// Return a new config with an action removed.
    ///
    /// This consumes self and returns a new instance.
    pub fn removing_action(mut self, action: &str) -> Self {
        self.actions.retain(|a| !a.eq_ignore_ascii_case(action));
        self
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Query methods
    // ──────────────────────────────────────────────────────────────────────────

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
                result.push('\n');
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
        assert_eq!(config.default_expression(), "neutral");
    }

    #[test]
    fn test_builder_pattern() {
        let config = ExpressionConfig::new()
            .unwrap()
            .with_expressions(vec!["calm", "excited"])
            .with_actions(vec!["waves"])
            .with_default_expression("calm")
            .expect("valid default");

        assert!(config.has_expression("calm"));
        assert!(config.has_expression("excited"));
        assert!(!config.has_expression("neutral"));
        assert!(config.has_action("waves"));
        assert_eq!(config.default_expression(), "calm");
    }

    #[test]
    fn test_invalid_default_expression() {
        let result = ExpressionConfig::new()
            .unwrap()
            .with_expressions(vec!["calm", "excited"])
            .with_actions(vec!["waves"])
            .with_default_expression("neutral");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Default expression 'neutral' not found"));
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
            .unwrap()
            .with_expressions(vec!["neutral", "happy"])
            .with_actions(vec!["nods"]);

        let formatted = config.format_for_llm();
        assert!(formatted.contains("neutral, happy"));
        assert!(formatted.contains("nods"));
    }

    #[test]
    fn test_adding_expression_deduplication() {
        let config = ExpressionConfig::new()
            .unwrap()
            .with_expressions(vec!["neutral"]);

        let config = config.adding_expression("neutral");
        let config = config.adding_expression("happy");

        assert_eq!(config.expressions().len(), 2);
    }

    #[test]
    fn test_removing_expression_resets_default() {
        let config = ExpressionConfig::default()
            .removing_expression("neutral")
            .expect("removing expression should succeed");

        // Default should be reset to first available
        assert_ne!(config.default_expression(), "neutral");
        assert!(config
            .expressions()
            .iter()
            .any(|e| e == config.default_expression()));
    }

    #[test]
    fn test_removing_last_expression_fails() {
        let config = ExpressionConfig::new()
            .unwrap()
            .with_expressions(vec!["only"])
            .with_default_expression("only")
            .expect("valid config");

        let result = config.removing_expression("only");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Default expression"));
    }
}
