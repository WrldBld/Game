//! Application settings value object
//!
//! # Architectural Note (ADR-002: Settings Serialization)
//!
//! AppSettings intentionally includes serde derives because settings are stored and
//! transmitted across infrastructure boundaries. UI metadata belongs in protocol.
//!
//! # Per-World Settings
//!
//! Settings can be global (applied to all worlds) or per-world (override global).
//! The `world_id` field determines scope:
//! - `None` = global settings (default)
//! - `Some(world_id)` = per-world override

use serde::{Deserialize, Serialize};

use super::context_budget::ContextBudgetConfig;
use wrldbldr_domain::WorldId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchQueueFailurePolicy {
    /// Any failure while queueing prompts fails the entire batch.
    AllOrNothing,
    /// Continue queueing remaining prompts; fail only if none queued successfully.
    BestEffort,

    /// Forward-compatibility fallback for newer variants.
    #[serde(other)]
    Unknown,
}

fn default_batch_queue_failure_policy() -> BatchQueueFailurePolicy {
    BatchQueueFailurePolicy::AllOrNothing
}

impl std::fmt::Display for BatchQueueFailurePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchQueueFailurePolicy::AllOrNothing => write!(f, "all_or_nothing"),
            BatchQueueFailurePolicy::BestEffort => write!(f, "best_effort"),
            BatchQueueFailurePolicy::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for BatchQueueFailurePolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all_or_nothing" | "allornothing" | "all" => Ok(BatchQueueFailurePolicy::AllOrNothing),
            "best_effort" | "besteffort" | "best" => Ok(BatchQueueFailurePolicy::BestEffort),
            _ => Err(()),
        }
    }
}

/// All configurable application settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// World ID for per-world settings. None = global defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_id: Option<WorldId>,

    // ============================================================================
    // Session & Conversation
    // ============================================================================
    /// Maximum conversation turns to store in session memory
    pub max_conversation_turns: usize,

    /// Number of conversation turns to include in LLM prompt context
    #[serde(default = "default_conversation_history_turns")]
    pub conversation_history_turns: usize,

    // ============================================================================
    // Circuit Breaker & Health
    // ============================================================================
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_open_duration_secs: u64,
    pub health_check_cache_ttl_secs: u64,

    // ============================================================================
    // Validation Limits
    // ============================================================================
    /// Maximum length for entity names (characters, locations, etc.)
    pub max_name_length: usize,
    /// Maximum length for descriptions
    pub max_description_length: usize,

    // ============================================================================
    // Animation (synced to Player)
    // ============================================================================
    pub typewriter_sentence_delay_ms: u64,
    pub typewriter_pause_delay_ms: u64,
    pub typewriter_char_delay_ms: u64,

    // ============================================================================
    // Game Defaults
    // ============================================================================
    pub default_max_stat_value: i32,

    // ============================================================================
    // Staging System
    // ============================================================================
    /// Default NPC presence cache TTL in game hours for new locations
    #[serde(default = "default_presence_cache_ttl_hours")]
    pub default_presence_cache_ttl_hours: i32,

    /// Whether to use LLM for staging decisions by default
    #[serde(default = "default_use_llm_presence")]
    pub default_use_llm_presence: bool,

    // ============================================================================
    // Challenge System
    // ============================================================================
    /// Number of outcome branches to generate for each challenge result tier
    #[serde(default = "default_outcome_branch_count")]
    pub outcome_branch_count: usize,
    /// Minimum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_min")]
    pub outcome_branch_min: usize,
    /// Maximum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_max")]
    pub outcome_branch_max: usize,

    // ============================================================================
    // LLM Settings
    // ============================================================================
    /// Max tokens per outcome branch when generating suggestions
    #[serde(default = "default_suggestion_tokens_per_branch")]
    pub suggestion_tokens_per_branch: u32,

    /// Token budget configuration for LLM context building
    #[serde(default)]
    pub context_budget: ContextBudgetConfig,

    // ============================================================================
    // Asset Generation
    // ============================================================================
    /// Default style reference asset ID for image generation
    /// When set, new asset generations will use this asset's style by default
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_reference_asset_id: Option<String>,

    /// Policy for how to handle failures while queueing prompts for a batch.
    #[serde(default = "default_batch_queue_failure_policy")]
    pub batch_queue_failure_policy: BatchQueueFailurePolicy,
}

fn default_outcome_branch_count() -> usize {
    2
}
fn default_outcome_branch_min() -> usize {
    1
}
fn default_outcome_branch_max() -> usize {
    4
}
fn default_conversation_history_turns() -> usize {
    20
}
fn default_suggestion_tokens_per_branch() -> u32 {
    200
}
fn default_presence_cache_ttl_hours() -> i32 {
    3
}
fn default_use_llm_presence() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            world_id: None,
            max_conversation_turns: 30,
            conversation_history_turns: 20,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_open_duration_secs: 60,
            health_check_cache_ttl_secs: 30,
            max_name_length: 255,
            max_description_length: 10000,
            typewriter_sentence_delay_ms: 150,
            typewriter_pause_delay_ms: 80,
            typewriter_char_delay_ms: 30,
            default_max_stat_value: 20,
            default_presence_cache_ttl_hours: 3,
            default_use_llm_presence: true,
            outcome_branch_count: 2,
            outcome_branch_min: 1,
            outcome_branch_max: 4,
            suggestion_tokens_per_branch: 200,
            context_budget: ContextBudgetConfig::default(),
            style_reference_asset_id: None,
            batch_queue_failure_policy: default_batch_queue_failure_policy(),
        }
    }
}

impl AppSettings {
    /// Create settings for a specific world from base settings
    ///
    /// # Note
    /// To load from environment variables, use `load_settings_from_env()`
    /// from `wrldbldr_engine_adapters::infrastructure::settings_loader`.
    pub fn for_world(base: AppSettings, world_id: WorldId) -> Self {
        let mut settings = base;
        settings.world_id = Some(world_id.into());
        settings
    }

    /// Merge per-world settings with global settings.
    /// Per-world values override global where present.
    pub fn merge_with_global(&self, _global: &AppSettings) -> AppSettings {
        // If this is global settings, just return a clone
        if self.world_id.is_none() {
            return self.clone();
        }

        // Start with global, then override with per-world values
        // For now, per-world settings completely override global
        // In the future, we could make this more granular with Option<T> fields
        self.clone()
    }
}

    /// Merge per-world settings with global settings.
    /// Per-world values override global where present.
    pub fn merge_with_global(&self, _global: &AppSettings) -> AppSettings {
        // If this is global settings, just return a clone
        if self.world_id.is_none() {
            return self.clone();
        }

        // Start with global, then override with per-world values
        // For now, per-world settings completely override global
        // In the future, we could make this more granular with Option<T> fields
        self.clone()
    }
}
