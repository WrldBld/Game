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
    world_id: Option<WorldId>,

    // ============================================================================
    // Session & Conversation
    // ============================================================================
    /// Maximum conversation turns to store in session memory
    max_conversation_turns: usize,

    /// Number of conversation turns to include in LLM prompt context
    #[serde(default = "default_conversation_history_turns")]
    conversation_history_turns: usize,

    // ============================================================================
    // Circuit Breaker & Health
    // ============================================================================
    circuit_breaker_failure_threshold: u32,
    circuit_breaker_open_duration_secs: u64,
    health_check_cache_ttl_secs: u64,

    // ============================================================================
    // Validation Limits
    // ============================================================================
    /// Maximum length for entity names (characters, locations, etc.)
    max_name_length: usize,
    /// Maximum length for descriptions
    max_description_length: usize,

    // ============================================================================
    // Animation (synced to Player)
    // ============================================================================
    typewriter_sentence_delay_ms: u64,
    typewriter_pause_delay_ms: u64,
    typewriter_char_delay_ms: u64,

    // ============================================================================
    // Game Defaults
    // ============================================================================
    default_max_stat_value: i32,

    // ============================================================================
    // Staging System
    // ============================================================================
    /// Default NPC presence cache TTL in game hours for new locations
    #[serde(default = "default_presence_cache_ttl_hours")]
    default_presence_cache_ttl_hours: i32,

    /// Whether to use LLM for staging decisions by default
    #[serde(default = "default_use_llm_presence")]
    default_use_llm_presence: bool,

    // ============================================================================
    // Challenge System
    // ============================================================================
    /// Number of outcome branches to generate for each challenge result tier
    #[serde(default = "default_outcome_branch_count")]
    outcome_branch_count: usize,
    /// Minimum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_min")]
    outcome_branch_min: usize,
    /// Maximum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_max")]
    outcome_branch_max: usize,

    // ============================================================================
    // LLM Settings
    // ============================================================================
    /// Max tokens per outcome branch when generating suggestions
    #[serde(default = "default_suggestion_tokens_per_branch")]
    suggestion_tokens_per_branch: u32,

    /// Token budget configuration for LLM context building
    #[serde(default)]
    context_budget: ContextBudgetConfig,

    // ============================================================================
    // Asset Generation
    // ============================================================================
    /// Default style reference asset ID for image generation
    /// When set, new asset generations will use this asset's style by default
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style_reference_asset_id: Option<String>,

    /// Policy for how to handle failures while queueing prompts for a batch.
    #[serde(default = "default_batch_queue_failure_policy")]
    batch_queue_failure_policy: BatchQueueFailurePolicy,
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

    // ============================================================================
    // Accessors
    // ============================================================================

    /// World ID for per-world settings. None = global defaults.
    pub fn world_id(&self) -> Option<WorldId> {
        self.world_id
    }

    /// Maximum conversation turns to store in session memory
    pub fn max_conversation_turns(&self) -> usize {
        self.max_conversation_turns
    }

    /// Number of conversation turns to include in LLM prompt context
    pub fn conversation_history_turns(&self) -> usize {
        self.conversation_history_turns
    }

    /// Circuit breaker failure threshold
    pub fn circuit_breaker_failure_threshold(&self) -> u32 {
        self.circuit_breaker_failure_threshold
    }

    /// Circuit breaker open duration in seconds
    pub fn circuit_breaker_open_duration_secs(&self) -> u64 {
        self.circuit_breaker_open_duration_secs
    }

    /// Health check cache TTL in seconds
    pub fn health_check_cache_ttl_secs(&self) -> u64 {
        self.health_check_cache_ttl_secs
    }

    /// Maximum length for entity names (characters, locations, etc.)
    pub fn max_name_length(&self) -> usize {
        self.max_name_length
    }

    /// Maximum length for descriptions
    pub fn max_description_length(&self) -> usize {
        self.max_description_length
    }

    /// Typewriter sentence delay in milliseconds
    pub fn typewriter_sentence_delay_ms(&self) -> u64 {
        self.typewriter_sentence_delay_ms
    }

    /// Typewriter pause delay in milliseconds
    pub fn typewriter_pause_delay_ms(&self) -> u64 {
        self.typewriter_pause_delay_ms
    }

    /// Typewriter character delay in milliseconds
    pub fn typewriter_char_delay_ms(&self) -> u64 {
        self.typewriter_char_delay_ms
    }

    /// Default maximum stat value
    pub fn default_max_stat_value(&self) -> i32 {
        self.default_max_stat_value
    }

    /// Default NPC presence cache TTL in game hours for new locations
    pub fn default_presence_cache_ttl_hours(&self) -> i32 {
        self.default_presence_cache_ttl_hours
    }

    /// Whether to use LLM for staging decisions by default
    pub fn default_use_llm_presence(&self) -> bool {
        self.default_use_llm_presence
    }

    /// Number of outcome branches to generate for each challenge result tier
    pub fn outcome_branch_count(&self) -> usize {
        self.outcome_branch_count
    }

    /// Minimum allowed branch count (for UI validation)
    pub fn outcome_branch_min(&self) -> usize {
        self.outcome_branch_min
    }

    /// Maximum allowed branch count (for UI validation)
    pub fn outcome_branch_max(&self) -> usize {
        self.outcome_branch_max
    }

    /// Max tokens per outcome branch when generating suggestions
    pub fn suggestion_tokens_per_branch(&self) -> u32 {
        self.suggestion_tokens_per_branch
    }

    /// Token budget configuration for LLM context building
    pub fn context_budget(&self) -> &ContextBudgetConfig {
        &self.context_budget
    }

    /// Default style reference asset ID for image generation
    pub fn style_reference_asset_id(&self) -> Option<&str> {
        self.style_reference_asset_id.as_deref()
    }

    /// Policy for how to handle failures while queueing prompts for a batch
    pub fn batch_queue_failure_policy(&self) -> BatchQueueFailurePolicy {
        self.batch_queue_failure_policy
    }

    // ============================================================================
    // Builder-style setters (consume self)
    // ============================================================================

    /// Set the world ID
    pub fn with_world_id(self, world_id: Option<WorldId>) -> Self {
        Self { world_id, ..self }
    }

    /// Set maximum conversation turns
    pub fn with_max_conversation_turns(self, max_conversation_turns: usize) -> Self {
        Self {
            max_conversation_turns,
            ..self
        }
    }

    /// Set conversation history turns
    pub fn with_conversation_history_turns(self, conversation_history_turns: usize) -> Self {
        Self {
            conversation_history_turns,
            ..self
        }
    }

    /// Set circuit breaker failure threshold
    pub fn with_circuit_breaker_failure_threshold(
        self,
        circuit_breaker_failure_threshold: u32,
    ) -> Self {
        Self {
            circuit_breaker_failure_threshold,
            ..self
        }
    }

    /// Set circuit breaker open duration in seconds
    pub fn with_circuit_breaker_open_duration_secs(
        self,
        circuit_breaker_open_duration_secs: u64,
    ) -> Self {
        Self {
            circuit_breaker_open_duration_secs,
            ..self
        }
    }

    /// Set health check cache TTL in seconds
    pub fn with_health_check_cache_ttl_secs(self, health_check_cache_ttl_secs: u64) -> Self {
        Self {
            health_check_cache_ttl_secs,
            ..self
        }
    }

    /// Set maximum name length
    pub fn with_max_name_length(self, max_name_length: usize) -> Self {
        Self {
            max_name_length,
            ..self
        }
    }

    /// Set maximum description length
    pub fn with_max_description_length(self, max_description_length: usize) -> Self {
        Self {
            max_description_length,
            ..self
        }
    }

    /// Set typewriter sentence delay in milliseconds
    pub fn with_typewriter_sentence_delay_ms(self, typewriter_sentence_delay_ms: u64) -> Self {
        Self {
            typewriter_sentence_delay_ms,
            ..self
        }
    }

    /// Set typewriter pause delay in milliseconds
    pub fn with_typewriter_pause_delay_ms(self, typewriter_pause_delay_ms: u64) -> Self {
        Self {
            typewriter_pause_delay_ms,
            ..self
        }
    }

    /// Set typewriter character delay in milliseconds
    pub fn with_typewriter_char_delay_ms(self, typewriter_char_delay_ms: u64) -> Self {
        Self {
            typewriter_char_delay_ms,
            ..self
        }
    }

    /// Set default maximum stat value
    pub fn with_default_max_stat_value(self, default_max_stat_value: i32) -> Self {
        Self {
            default_max_stat_value,
            ..self
        }
    }

    /// Set default presence cache TTL in hours
    pub fn with_default_presence_cache_ttl_hours(
        self,
        default_presence_cache_ttl_hours: i32,
    ) -> Self {
        Self {
            default_presence_cache_ttl_hours,
            ..self
        }
    }

    /// Set whether to use LLM for staging decisions by default
    pub fn with_default_use_llm_presence(self, default_use_llm_presence: bool) -> Self {
        Self {
            default_use_llm_presence,
            ..self
        }
    }

    /// Set outcome branch count
    pub fn with_outcome_branch_count(self, outcome_branch_count: usize) -> Self {
        Self {
            outcome_branch_count,
            ..self
        }
    }

    /// Set outcome branch minimum
    pub fn with_outcome_branch_min(self, outcome_branch_min: usize) -> Self {
        Self {
            outcome_branch_min,
            ..self
        }
    }

    /// Set outcome branch maximum
    pub fn with_outcome_branch_max(self, outcome_branch_max: usize) -> Self {
        Self {
            outcome_branch_max,
            ..self
        }
    }

    /// Set suggestion tokens per branch
    pub fn with_suggestion_tokens_per_branch(self, suggestion_tokens_per_branch: u32) -> Self {
        Self {
            suggestion_tokens_per_branch,
            ..self
        }
    }

    /// Set context budget configuration
    pub fn with_context_budget(self, context_budget: ContextBudgetConfig) -> Self {
        Self {
            context_budget,
            ..self
        }
    }

    /// Set style reference asset ID
    pub fn with_style_reference_asset_id(self, style_reference_asset_id: Option<String>) -> Self {
        Self {
            style_reference_asset_id,
            ..self
        }
    }

    /// Set batch queue failure policy
    pub fn with_batch_queue_failure_policy(
        self,
        batch_queue_failure_policy: BatchQueueFailurePolicy,
    ) -> Self {
        Self {
            batch_queue_failure_policy,
            ..self
        }
    }
}
