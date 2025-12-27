//! Context budget enforcement for LLM prompts
//!
//! Applies token counting and truncation to context sections
//! based on `ContextBudgetConfig` settings from world configuration.
//!
//! # Usage
//!
//! ```rust,ignore
//! let enforcer = ContextBudgetEnforcer::new(config);
//! let result = enforcer.enforce(
//!     ContextCategory::Scene,
//!     &scene_context_string,
//! );
//! ```

use wrldbldr_domain::value_objects::{
    ContextBudgetConfig, ContextCategory, TokenCounter,
};
use tracing::{debug, warn};

/// Result of enforcing budget on a context section
#[derive(Debug, Clone)]
pub struct EnforcementResult {
    /// The text after enforcement (may be truncated)
    pub text: String,
    /// Original token count before enforcement
    pub original_tokens: usize,
    /// Final token count after enforcement
    pub final_tokens: usize,
    /// Whether truncation occurred
    pub was_truncated: bool,
    /// The budget that was applied
    pub budget: usize,
}

impl EnforcementResult {
    /// Returns true if the content exceeded its budget
    pub fn exceeded_budget(&self) -> bool {
        self.was_truncated
    }

    /// Returns the number of tokens saved by truncation
    pub fn tokens_saved(&self) -> usize {
        self.original_tokens.saturating_sub(self.final_tokens)
    }
}

/// Enforces token budgets on context sections for LLM prompts
///
/// Uses `TokenCounter` to estimate token usage and truncates sections
/// that exceed their configured budget.
#[derive(Debug, Clone)]
pub struct ContextBudgetEnforcer {
    config: ContextBudgetConfig,
    counter: TokenCounter,
    /// Track enforcement stats for logging
    enforcement_stats: EnforcementStats,
}

/// Statistics about budget enforcement for a single prompt
#[derive(Debug, Clone, Default)]
pub struct EnforcementStats {
    /// Total tokens before enforcement
    pub total_original_tokens: usize,
    /// Total tokens after enforcement
    pub total_final_tokens: usize,
    /// Number of categories that were truncated
    pub categories_truncated: usize,
    /// Details per category
    pub category_results: Vec<(ContextCategory, EnforcementResult)>,
}

impl EnforcementStats {
    /// Returns true if any category was truncated
    pub fn any_truncated(&self) -> bool {
        self.categories_truncated > 0
    }

    /// Returns total tokens saved across all categories
    pub fn total_tokens_saved(&self) -> usize {
        self.total_original_tokens.saturating_sub(self.total_final_tokens)
    }

    /// Log a summary of enforcement actions
    pub fn log_summary(&self) {
        if self.any_truncated() {
            warn!(
                original_tokens = self.total_original_tokens,
                final_tokens = self.total_final_tokens,
                tokens_saved = self.total_tokens_saved(),
                categories_truncated = self.categories_truncated,
                "Context budget enforcement: truncated {} categories, saved {} tokens",
                self.categories_truncated,
                self.total_tokens_saved()
            );

            for (category, result) in &self.category_results {
                if result.was_truncated {
                    debug!(
                        category = %category.display_name(),
                        original = result.original_tokens,
                        final_tokens = result.final_tokens,
                        budget = result.budget,
                        "Truncated {} from {} to {} tokens (budget: {})",
                        category.display_name(),
                        result.original_tokens,
                        result.final_tokens,
                        result.budget
                    );
                }
            }
        } else {
            debug!(
                total_tokens = self.total_final_tokens,
                "Context budget enforcement: all categories within budget"
            );
        }
    }
}

impl ContextBudgetEnforcer {
    /// Create a new enforcer with the given configuration
    pub fn new(config: ContextBudgetConfig) -> Self {
        Self {
            config,
            counter: TokenCounter::default(),
            enforcement_stats: EnforcementStats::default(),
        }
    }

    /// Create an enforcer with a custom token counter
    pub fn with_counter(config: ContextBudgetConfig, counter: TokenCounter) -> Self {
        Self {
            config,
            counter,
            enforcement_stats: EnforcementStats::default(),
        }
    }

    /// Get the current enforcement statistics
    pub fn stats(&self) -> &EnforcementStats {
        &self.enforcement_stats
    }

    /// Reset enforcement statistics (call before building a new prompt)
    pub fn reset_stats(&mut self) {
        self.enforcement_stats = EnforcementStats::default();
    }

    /// Enforce budget on a context section
    ///
    /// Returns the (possibly truncated) text and enforcement details.
    /// Also updates internal statistics.
    pub fn enforce(&mut self, category: ContextCategory, text: &str) -> EnforcementResult {
        let budget = self.config.budget_for(category);
        let original_tokens = self.counter.count(text);

        let (final_text, was_truncated) = if original_tokens > budget {
            self.counter.truncate_to_budget(text, budget)
        } else {
            (text.to_string(), false)
        };

        let final_tokens = self.counter.count(&final_text);

        let result = EnforcementResult {
            text: final_text,
            original_tokens,
            final_tokens,
            was_truncated,
            budget,
        };

        // Update stats
        self.enforcement_stats.total_original_tokens += original_tokens;
        self.enforcement_stats.total_final_tokens += final_tokens;
        if was_truncated {
            self.enforcement_stats.categories_truncated += 1;
        }
        self.enforcement_stats
            .category_results
            .push((category, result.clone()));

        result
    }

    /// Enforce budget and return just the text (convenience method)
    pub fn enforce_text(&mut self, category: ContextCategory, text: &str) -> String {
        self.enforce(category, text).text
    }

    /// Check if text exceeds budget without modifying it
    pub fn would_exceed(&self, category: ContextCategory, text: &str) -> bool {
        let budget = self.config.budget_for(category);
        self.counter.exceeds_budget(text, budget)
    }

    /// Get the budget for a category
    pub fn budget_for(&self, category: ContextCategory) -> usize {
        self.config.budget_for(category)
    }

    /// Get remaining budget after accounting for used tokens
    pub fn remaining_budget(&self, category: ContextCategory, used_tokens: usize) -> usize {
        self.config.budget_for(category).saturating_sub(used_tokens)
    }

    /// Count tokens in text
    pub fn count_tokens(&self, text: &str) -> usize {
        self.counter.count(text)
    }

    /// Check if total context exceeds total budget
    pub fn total_exceeds_budget(&self) -> bool {
        self.enforcement_stats.total_final_tokens > self.config.total_budget_tokens
    }

    /// Get the total budget
    pub fn total_budget(&self) -> usize {
        self.config.total_budget_tokens
    }

    /// Log enforcement summary and return whether any truncation occurred
    pub fn finalize(&self) -> bool {
        self.enforcement_stats.log_summary();
        self.enforcement_stats.any_truncated()
    }
}

/// Builder pattern for enforcing budgets on structured context
pub struct ContextBuilder {
    enforcer: ContextBudgetEnforcer,
    sections: Vec<(ContextCategory, String)>,
}

impl ContextBuilder {
    /// Create a new context builder with the given config
    pub fn new(config: ContextBudgetConfig) -> Self {
        Self {
            enforcer: ContextBudgetEnforcer::new(config),
            sections: Vec::new(),
        }
    }

    /// Add a section with budget enforcement
    pub fn add_section(mut self, category: ContextCategory, content: String) -> Self {
        let enforced = self.enforcer.enforce_text(category, &content);
        self.sections.push((category, enforced));
        self
    }

    /// Add a section only if content is non-empty
    pub fn add_section_if_present(
        self,
        category: ContextCategory,
        content: Option<String>,
    ) -> Self {
        match content {
            Some(c) if !c.trim().is_empty() => self.add_section(category, c),
            _ => self,
        }
    }

    /// Build the final context, joining all sections
    pub fn build(self) -> (String, EnforcementStats) {
        let stats = self.enforcer.enforcement_stats.clone();
        let result = self
            .sections
            .into_iter()
            .map(|(_, content)| content)
            .collect::<Vec<_>>()
            .join("\n\n");
        (result, stats)
    }

    /// Build with a custom separator
    pub fn build_with_separator(self, separator: &str) -> (String, EnforcementStats) {
        let stats = self.enforcer.enforcement_stats.clone();
        let result = self
            .sections
            .into_iter()
            .map(|(_, content)| content)
            .collect::<Vec<_>>()
            .join(separator);
        (result, stats)
    }

    /// Get current stats without consuming the builder
    pub fn current_stats(&self) -> &EnforcementStats {
        self.enforcer.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforce_within_budget() {
        let config = ContextBudgetConfig::default();
        let mut enforcer = ContextBudgetEnforcer::new(config);

        let text = "Short text";
        let result = enforcer.enforce(ContextCategory::Scene, text);

        assert!(!result.was_truncated);
        assert_eq!(result.text, text);
        assert_eq!(result.original_tokens, result.final_tokens);
    }

    #[test]
    fn test_enforce_exceeds_budget() {
        let mut config = ContextBudgetConfig::default();
        config.scene_tokens = 5; // Very small budget

        let mut enforcer = ContextBudgetEnforcer::new(config);

        let text = "This is a much longer text that should definitely exceed our tiny token budget and require truncation to fit.";
        let result = enforcer.enforce(ContextCategory::Scene, text);

        assert!(result.was_truncated);
        assert!(result.final_tokens <= 10); // Allow some margin
        assert!(result.text.ends_with("..."));
    }

    #[test]
    fn test_stats_tracking() {
        let mut config = ContextBudgetConfig::default();
        config.scene_tokens = 5;
        config.character_tokens = 1000;

        let mut enforcer = ContextBudgetEnforcer::new(config);

        // This will be truncated
        enforcer.enforce(ContextCategory::Scene, "This is a long scene description that exceeds budget");
        // This won't be truncated
        enforcer.enforce(ContextCategory::Character, "Short");

        let stats = enforcer.stats();
        assert_eq!(stats.categories_truncated, 1);
        assert_eq!(stats.category_results.len(), 2);
    }

    #[test]
    fn test_context_builder() {
        let config = ContextBudgetConfig::default();
        let builder = ContextBuilder::new(config)
            .add_section(ContextCategory::Scene, "Scene content".to_string())
            .add_section(ContextCategory::Character, "Character content".to_string());

        let (result, stats) = builder.build();

        assert!(result.contains("Scene content"));
        assert!(result.contains("Character content"));
        assert_eq!(stats.category_results.len(), 2);
    }

    #[test]
    fn test_would_exceed() {
        let mut config = ContextBudgetConfig::default();
        config.scene_tokens = 5;

        let enforcer = ContextBudgetEnforcer::new(config);

        assert!(enforcer.would_exceed(ContextCategory::Scene, "This is a longer text"));
        assert!(!enforcer.would_exceed(ContextCategory::Scene, "Hi"));
    }
}
