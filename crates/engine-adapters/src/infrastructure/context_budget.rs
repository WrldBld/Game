//! Context budget enforcement for LLM prompts
//!
//! DEPRECATED: Core types have been moved to `wrldbldr_domain::value_objects::context_budget_enforcement`.
//! This module re-exports those types and adds adapter-specific logging functionality.
//!
//! For new code, import directly from domain:
//! ```rust,ignore
//! use wrldbldr_domain::value_objects::{
//!     ContextBudgetEnforcer, ContextBuilder, EnforcementResult, EnforcementStats,
//! };
//! ```
//!
//! This module can be removed once all adapter-specific logging is handled elsewhere.

use tracing::{debug, warn};

// Re-export all types from domain
pub use wrldbldr_domain::value_objects::{
    ContextBudgetEnforcer, ContextBuilder, EnforcementResult, EnforcementStats,
    ContextBudgetConfig, ContextCategory, TokenCounter,
};

/// Extension trait for adapter-specific logging functionality
pub trait EnforcementStatsLogging {
    /// Log a summary of enforcement actions using tracing
    fn log_summary(&self);
}

impl EnforcementStatsLogging for EnforcementStats {
    fn log_summary(&self) {
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

/// Extension trait for adapter-specific logging on the enforcer
pub trait ContextBudgetEnforcerLogging {
    /// Log enforcement summary and return whether any truncation occurred
    fn finalize_with_logging(&self) -> bool;
}

impl ContextBudgetEnforcerLogging for ContextBudgetEnforcer {
    fn finalize_with_logging(&self) -> bool {
        self.stats().log_summary();
        self.finalize()
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

    #[test]
    fn test_logging_extension() {
        let config = ContextBudgetConfig::default();
        let enforcer = ContextBudgetEnforcer::new(config);
        
        // Verify the extension trait works
        let stats = enforcer.stats();
        stats.log_summary(); // Should not panic
    }
}
