//! Settings DTOs - Application settings types
//!
//! This module contains DTOs for Engine application settings.
//! These types mirror the Engine's AppSettings structure.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchQueueFailurePolicy {
    /// Any failure while queueing prompts fails the entire batch.
    AllOrNothing,
    /// Continue queueing remaining prompts; fail only if none queued successfully.
    BestEffort,
}

fn default_batch_queue_failure_policy() -> BatchQueueFailurePolicy {
    BatchQueueFailurePolicy::AllOrNothing
}

/// Token budget configuration for LLM context building
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContextBudgetConfig {
    /// Total token budget for the entire system prompt
    pub total_budget_tokens: usize,
    /// Budget for scene context (location, time, atmosphere)
    pub scene_tokens: usize,
    /// Budget for character details (personality, wants, relationships)
    pub character_tokens: usize,
    /// Budget for conversation history
    pub conversation_history_tokens: usize,
    /// Budget for active challenges
    pub challenges_tokens: usize,
    /// Budget for narrative events
    pub narrative_events_tokens: usize,
    /// Budget for directorial notes
    pub directorial_notes_tokens: usize,
    /// Budget for location-specific context
    pub location_context_tokens: usize,
    /// Budget for player character context
    pub player_context_tokens: usize,
    /// Whether to enable automatic summarization when over budget
    pub enable_summarization: bool,
    /// Model to use for summarization (uses main model if None)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summarization_model: Option<String>,
}

impl Default for ContextBudgetConfig {
    fn default() -> Self {
        Self {
            total_budget_tokens: 4000,
            scene_tokens: 500,
            character_tokens: 800,
            conversation_history_tokens: 1000,
            challenges_tokens: 400,
            narrative_events_tokens: 400,
            directorial_notes_tokens: 300,
            location_context_tokens: 300,
            player_context_tokens: 300,
            enable_summarization: true,
            summarization_model: None,
        }
    }
}

/// Application settings from the Engine
///
/// These settings control various aspects of the Engine's behavior,
/// including conversation limits, circuit breaker configuration,
/// animation timing, and validation constraints.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// World ID for per-world settings. None = global defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_id: Option<String>,

    // ============================================================================
    // Session & Conversation
    // ============================================================================
    /// Maximum number of conversation turns to store in session memory
    pub max_conversation_turns: usize,

    /// Number of conversation turns to include in LLM prompt context
    #[serde(default = "default_conversation_history_turns")]
    pub conversation_history_turns: usize,

    // ============================================================================
    // Circuit Breaker & Health
    // ============================================================================
    /// Number of consecutive failures before circuit breaker opens
    pub circuit_breaker_failure_threshold: u32,

    /// Duration (in seconds) that circuit breaker remains open
    pub circuit_breaker_open_duration_secs: u64,

    /// TTL (in seconds) for health check cache entries
    pub health_check_cache_ttl_secs: u64,

    // ============================================================================
    // Validation Limits
    // ============================================================================
    /// Maximum allowed length for name fields
    pub max_name_length: usize,

    /// Maximum allowed length for description fields
    pub max_description_length: usize,

    // ============================================================================
    // Animation
    // ============================================================================
    /// Delay (in milliseconds) between sentences in typewriter effect
    pub typewriter_sentence_delay_ms: u64,

    /// Delay (in milliseconds) for pause punctuation in typewriter effect
    pub typewriter_pause_delay_ms: u64,

    /// Delay (in milliseconds) between characters in typewriter effect
    pub typewriter_char_delay_ms: u64,

    // ============================================================================
    // Game Defaults
    // ============================================================================
    /// Default maximum value for character stats
    pub default_max_stat_value: i32,

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

/// Settings field metadata for dynamic UI rendering
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsFieldMetadata {
    /// Field key (matches JSON field name)
    pub key: String,
    /// Display name for UI
    pub display_name: String,
    /// Description/help text
    pub description: String,
    /// Field type: "integer", "float", "boolean", "string"
    pub field_type: String,
    /// Default value
    pub default_value: serde_json::Value,
    /// Minimum value (for numeric fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<serde_json::Value>,
    /// Maximum value (for numeric fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<serde_json::Value>,
    /// Category for grouping in UI
    pub category: String,
    /// Whether changing this setting requires a restart
    pub requires_restart: bool,
}
