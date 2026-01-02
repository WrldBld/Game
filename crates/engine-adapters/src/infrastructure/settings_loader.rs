//! Settings loader for reading AppSettings from environment variables
//!
//! This module provides infrastructure-layer functionality to load AppSettings
//! from environment variables, keeping environment I/O out of the domain layer.
//!
//! # Hexagonal Architecture
//!
//! The domain layer (AppSettings struct) should have zero external I/O.
//! This loader belongs in the adapters/infrastructure layer because:
//! - It performs I/O (reading environment variables)
//! - It's an external system concern, not domain logic
//! - The domain only defines WHAT settings exist, not HOW they're loaded

use wrldbldr_domain::value_objects::{AppSettings, ContextBudgetConfig};

/// Helper function to read environment variable with default fallback
fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Load AppSettings from environment variables with defaults for missing values.
///
/// This function reads environment variables prefixed with `WRLDBLDR_` and constructs
/// an AppSettings instance. Any missing or unparseable values fall back to defaults.
///
/// # Environment Variables
///
/// Session & Conversation:
/// - `WRLDBLDR_MAX_CONVERSATION_TURNS` - Max turns stored in session memory (default: 30)
/// - `WRLDBLDR_CONVERSATION_HISTORY_TURNS` - Turns included in LLM context (default: 20)
///
/// Circuit Breaker & Health:
/// - `WRLDBLDR_CIRCUIT_BREAKER_FAILURES` - Failures before circuit opens (default: 5)
/// - `WRLDBLDR_CIRCUIT_BREAKER_OPEN_SECS` - Duration circuit stays open (default: 60)
/// - `WRLDBLDR_HEALTH_CHECK_CACHE_TTL` - Health check cache duration (default: 30)
///
/// Validation:
/// - `WRLDBLDR_MAX_NAME_LENGTH` - Max entity name length (default: 255)
/// - `WRLDBLDR_MAX_DESCRIPTION_LENGTH` - Max description length (default: 10000)
///
/// Animation:
/// - `WRLDBLDR_TYPEWRITER_SENTENCE_DELAY` - Sentence delay in ms (default: 150)
/// - `WRLDBLDR_TYPEWRITER_PAUSE_DELAY` - Pause delay in ms (default: 80)
/// - `WRLDBLDR_TYPEWRITER_CHAR_DELAY` - Character delay in ms (default: 30)
///
/// Game Defaults:
/// - `WRLDBLDR_DEFAULT_MAX_STAT` - Default max stat value (default: 20)
///
/// Staging:
/// - `WRLDBLDR_DEFAULT_PRESENCE_CACHE_TTL_HOURS` - NPC presence cache TTL (default: 3)
/// - `WRLDBLDR_DEFAULT_USE_LLM_PRESENCE` - Use LLM for staging (default: true)
///
/// Challenge System:
/// - `WRLDBLDR_OUTCOME_BRANCH_COUNT` - Branches per challenge result (default: 2)
/// - `WRLDBLDR_OUTCOME_BRANCH_MIN` - Min branches (default: 1)
/// - `WRLDBLDR_OUTCOME_BRANCH_MAX` - Max branches (default: 4)
/// - `WRLDBLDR_SUGGESTION_TOKENS_PER_BRANCH` - Tokens per branch (default: 200)
///
/// LLM Context Budget:
/// - `WRLDBLDR_LLM_TOTAL_BUDGET_TOKENS` - Total token budget (default: 4000)
/// - `WRLDBLDR_LLM_SCENE_TOKENS` - Scene context tokens (default: 500)
/// - `WRLDBLDR_LLM_CHARACTER_TOKENS` - Character context tokens (default: 800)
/// - `WRLDBLDR_LLM_CONVERSATION_HISTORY_TOKENS` - History tokens (default: 1000)
/// - `WRLDBLDR_LLM_CHALLENGES_TOKENS` - Challenges tokens (default: 400)
/// - `WRLDBLDR_LLM_NARRATIVE_EVENTS_TOKENS` - Narrative tokens (default: 400)
/// - `WRLDBLDR_LLM_DIRECTORIAL_NOTES_TOKENS` - Directorial tokens (default: 300)
/// - `WRLDBLDR_LLM_LOCATION_CONTEXT_TOKENS` - Location tokens (default: 300)
/// - `WRLDBLDR_LLM_PLAYER_CONTEXT_TOKENS` - Player tokens (default: 300)
/// - `WRLDBLDR_LLM_ENABLE_SUMMARIZATION` - Enable summarization (default: true)
/// - `WRLDBLDR_LLM_SUMMARIZATION_MODEL` - Summarization model (optional)
///
/// # Example
///
/// ```rust,ignore
/// use wrldbldr_engine_adapters::infrastructure::settings_loader::load_settings_from_env;
///
/// let settings = load_settings_from_env();
/// println!("Max conversation turns: {}", settings.max_conversation_turns);
/// ```
pub fn load_settings_from_env() -> AppSettings {
    let defaults = AppSettings::default();
    let context_budget_defaults = ContextBudgetConfig::default();

    let batch_queue_failure_policy = std::env::var("WRLDBLDR_BATCH_QUEUE_FAILURE_POLICY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(defaults.batch_queue_failure_policy);

    AppSettings {
        world_id: None, // Global settings from env
        max_conversation_turns: env_or(
            "WRLDBLDR_MAX_CONVERSATION_TURNS",
            defaults.max_conversation_turns,
        ),
        conversation_history_turns: env_or(
            "WRLDBLDR_CONVERSATION_HISTORY_TURNS",
            defaults.conversation_history_turns,
        ),
        circuit_breaker_failure_threshold: env_or(
            "WRLDBLDR_CIRCUIT_BREAKER_FAILURES",
            defaults.circuit_breaker_failure_threshold,
        ),
        circuit_breaker_open_duration_secs: env_or(
            "WRLDBLDR_CIRCUIT_BREAKER_OPEN_SECS",
            defaults.circuit_breaker_open_duration_secs,
        ),
        health_check_cache_ttl_secs: env_or(
            "WRLDBLDR_HEALTH_CHECK_CACHE_TTL",
            defaults.health_check_cache_ttl_secs,
        ),
        max_name_length: env_or("WRLDBLDR_MAX_NAME_LENGTH", defaults.max_name_length),
        max_description_length: env_or(
            "WRLDBLDR_MAX_DESCRIPTION_LENGTH",
            defaults.max_description_length,
        ),
        typewriter_sentence_delay_ms: env_or(
            "WRLDBLDR_TYPEWRITER_SENTENCE_DELAY",
            defaults.typewriter_sentence_delay_ms,
        ),
        typewriter_pause_delay_ms: env_or(
            "WRLDBLDR_TYPEWRITER_PAUSE_DELAY",
            defaults.typewriter_pause_delay_ms,
        ),
        typewriter_char_delay_ms: env_or(
            "WRLDBLDR_TYPEWRITER_CHAR_DELAY",
            defaults.typewriter_char_delay_ms,
        ),
        default_max_stat_value: env_or(
            "WRLDBLDR_DEFAULT_MAX_STAT",
            defaults.default_max_stat_value,
        ),
        default_presence_cache_ttl_hours: env_or(
            "WRLDBLDR_DEFAULT_PRESENCE_CACHE_TTL_HOURS",
            defaults.default_presence_cache_ttl_hours,
        ),
        default_use_llm_presence: env_or(
            "WRLDBLDR_DEFAULT_USE_LLM_PRESENCE",
            defaults.default_use_llm_presence,
        ),
        outcome_branch_count: env_or(
            "WRLDBLDR_OUTCOME_BRANCH_COUNT",
            defaults.outcome_branch_count,
        ),
        outcome_branch_min: env_or("WRLDBLDR_OUTCOME_BRANCH_MIN", defaults.outcome_branch_min),
        outcome_branch_max: env_or("WRLDBLDR_OUTCOME_BRANCH_MAX", defaults.outcome_branch_max),
        suggestion_tokens_per_branch: env_or(
            "WRLDBLDR_SUGGESTION_TOKENS_PER_BRANCH",
            defaults.suggestion_tokens_per_branch,
        ),
        // Load context budget from environment variables
        context_budget: ContextBudgetConfig {
            total_budget_tokens: env_or(
                "WRLDBLDR_LLM_TOTAL_BUDGET_TOKENS",
                context_budget_defaults.total_budget_tokens,
            ),
            scene_tokens: env_or(
                "WRLDBLDR_LLM_SCENE_TOKENS",
                context_budget_defaults.scene_tokens,
            ),
            character_tokens: env_or(
                "WRLDBLDR_LLM_CHARACTER_TOKENS",
                context_budget_defaults.character_tokens,
            ),
            conversation_history_tokens: env_or(
                "WRLDBLDR_LLM_CONVERSATION_HISTORY_TOKENS",
                context_budget_defaults.conversation_history_tokens,
            ),
            challenges_tokens: env_or(
                "WRLDBLDR_LLM_CHALLENGES_TOKENS",
                context_budget_defaults.challenges_tokens,
            ),
            narrative_events_tokens: env_or(
                "WRLDBLDR_LLM_NARRATIVE_EVENTS_TOKENS",
                context_budget_defaults.narrative_events_tokens,
            ),
            directorial_notes_tokens: env_or(
                "WRLDBLDR_LLM_DIRECTORIAL_NOTES_TOKENS",
                context_budget_defaults.directorial_notes_tokens,
            ),
            location_context_tokens: env_or(
                "WRLDBLDR_LLM_LOCATION_CONTEXT_TOKENS",
                context_budget_defaults.location_context_tokens,
            ),
            player_context_tokens: env_or(
                "WRLDBLDR_LLM_PLAYER_CONTEXT_TOKENS",
                context_budget_defaults.player_context_tokens,
            ),
            enable_summarization: env_or(
                "WRLDBLDR_LLM_ENABLE_SUMMARIZATION",
                context_budget_defaults.enable_summarization,
            ),
            summarization_model: std::env::var("WRLDBLDR_LLM_SUMMARIZATION_MODEL").ok(),
        },
        // Style reference is not loaded from env - only stored per-world
        style_reference_asset_id: None,
        batch_queue_failure_policy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_settings_returns_defaults() {
        // Without any env vars set, should return defaults
        let settings = load_settings_from_env();
        let defaults = AppSettings::default();

        assert_eq!(
            settings.max_conversation_turns,
            defaults.max_conversation_turns
        );
        assert_eq!(
            settings.circuit_breaker_failure_threshold,
            defaults.circuit_breaker_failure_threshold
        );
        assert!(settings.world_id.is_none());
    }

    #[test]
    fn test_env_or_helper_with_default() {
        // Test with a definitely-not-set variable
        let result: usize = env_or("WRLDBLDR_TEST_DEFINITELY_NOT_SET_12345", 42);
        assert_eq!(result, 42);
    }
}
