//! Application settings value object
//!
//! # Architectural Note (ADR-002: Settings Serialization)
//!
//! AppSettings intentionally includes serde derives because:
//! 1. Settings are stored in SQLite as key-value pairs
//! 2. Settings are transmitted via REST API for UI configuration
//! 3. The JSON schema IS the API contract for settings
//!
//! # Per-World Settings
//!
//! Settings can be global (applied to all worlds) or per-world (override global).
//! The `world_id` field determines scope:
//! - `None` = global settings (default)
//! - `Some(world_id)` = per-world override

use serde::{Deserialize, Serialize};

use super::context_budget::ContextBudgetConfig;
use uuid::Uuid;
use wrldbldr_domain::WorldId;

/// All configurable application settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// World ID for per-world settings. None = global defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_id: Option<Uuid>,

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

/// Settings field metadata for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<serde_json::Value>,
    /// Maximum value (for numeric fields)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<serde_json::Value>,
    /// Category for grouping in UI
    pub category: String,
    /// Whether changing this setting requires a restart
    pub requires_restart: bool,
}

/// Get metadata for all settings fields
pub fn settings_metadata() -> Vec<SettingsFieldMetadata> {
    vec![
        // Session & Conversation
        SettingsFieldMetadata {
            key: "max_conversation_turns".into(),
            display_name: "Max Stored Conversation Turns".into(),
            description: "Maximum number of conversation turns to store in session memory".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(5)),
            max_value: Some(serde_json::json!(100)),
            category: "Conversation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "conversation_history_turns".into(),
            display_name: "LLM Context History Turns".into(),
            description: "Number of conversation turns to include in LLM prompt context".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(20),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(50)),
            category: "Conversation".into(),
            requires_restart: false,
        },
        // Validation
        SettingsFieldMetadata {
            key: "max_name_length".into(),
            display_name: "Max Name Length".into(),
            description: "Maximum length for entity names (characters, locations, etc.)".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(255),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(1000)),
            category: "Validation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "max_description_length".into(),
            display_name: "Max Description Length".into(),
            description: "Maximum length for description fields".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(10000),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(50000)),
            category: "Validation".into(),
            requires_restart: false,
        },
        // Challenge System
        SettingsFieldMetadata {
            key: "outcome_branch_count".into(),
            display_name: "Outcome Branches".into(),
            description: "Number of outcome branches to generate for each challenge result".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(2),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(4)),
            category: "Challenges".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "suggestion_tokens_per_branch".into(),
            display_name: "Suggestion Tokens per Branch".into(),
            description: "Maximum tokens per outcome branch when generating LLM suggestions".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(200),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "Challenges".into(),
            requires_restart: false,
        },
        // Animation
        SettingsFieldMetadata {
            key: "typewriter_sentence_delay_ms".into(),
            display_name: "Sentence Delay (ms)".into(),
            description: "Delay after completing a sentence in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(150),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(1000)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_pause_delay_ms".into(),
            display_name: "Pause Delay (ms)".into(),
            description: "Delay at punctuation marks in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(80),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(500)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_char_delay_ms".into(),
            display_name: "Character Delay (ms)".into(),
            description: "Delay between characters in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(200)),
            category: "Animation".into(),
            requires_restart: false,
        },
        // LLM Context Budgets
        SettingsFieldMetadata {
            key: "context_budget.total_budget_tokens".into(),
            display_name: "Total Token Budget".into(),
            description: "Maximum total tokens for LLM system prompt".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(4000),
            min_value: Some(serde_json::json!(1000)),
            max_value: Some(serde_json::json!(32000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.scene_tokens".into(),
            display_name: "Scene Context Tokens".into(),
            description: "Token budget for scene description, location, atmosphere".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(500),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(2000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.character_tokens".into(),
            display_name: "Character Context Tokens".into(),
            description: "Token budget for NPC personality, motivations, relationships".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(800),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(3000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.conversation_history_tokens".into(),
            display_name: "Conversation History Tokens".into(),
            description: "Token budget for recent conversation history".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(1000),
            min_value: Some(serde_json::json!(200)),
            max_value: Some(serde_json::json!(4000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.challenges_tokens".into(),
            display_name: "Challenges Context Tokens".into(),
            description: "Token budget for active challenges".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(400),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(1500)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.narrative_events_tokens".into(),
            display_name: "Narrative Events Tokens".into(),
            description: "Token budget for active story events".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(400),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(1500)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.directorial_notes_tokens".into(),
            display_name: "Directorial Notes Tokens".into(),
            description: "Token budget for DM guidance and notes".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(300),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.location_context_tokens".into(),
            display_name: "Location Context Tokens".into(),
            description: "Token budget for location-specific details".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(300),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.player_context_tokens".into(),
            display_name: "Player Context Tokens".into(),
            description: "Token budget for player character information".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(300),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "LLM Context".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget.enable_summarization".into(),
            display_name: "Enable Auto-Summarization".into(),
            description: "Automatically summarize context when over budget".into(),
            field_type: "boolean".into(),
            default_value: serde_json::json!(true),
            min_value: None,
            max_value: None,
            category: "LLM Context".into(),
            requires_restart: false,
        },
        // System settings (require restart)
        SettingsFieldMetadata {
            key: "circuit_breaker_failure_threshold".into(),
            display_name: "Circuit Breaker Failures".into(),
            description: "Number of failures before circuit breaker opens".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(5),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(20)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "circuit_breaker_open_duration_secs".into(),
            display_name: "Circuit Breaker Open Duration (s)".into(),
            description: "How long circuit breaker stays open before retry".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(60),
            min_value: Some(serde_json::json!(10)),
            max_value: Some(serde_json::json!(600)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "health_check_cache_ttl_secs".into(),
            display_name: "Health Check Cache TTL (s)".into(),
            description: "How long to cache health check results".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(5)),
            max_value: Some(serde_json::json!(300)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "default_max_stat_value".into(),
            display_name: "Default Max Stat Value".into(),
            description: "Default maximum value for character stats".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(20),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(100)),
            category: "Game".into(),
            requires_restart: false,
        },
        // Staging System
        SettingsFieldMetadata {
            key: "default_presence_cache_ttl_hours".into(),
            display_name: "Default Staging TTL (hours)".into(),
            description: "Default duration for NPC presence approvals in game hours. Busy venues: 1-2h, Calm locations: 3-4h, Static locations: 8-24h.".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(3),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(24)),
            category: "Staging".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "default_use_llm_presence".into(),
            display_name: "Use LLM for Staging".into(),
            description: "Whether to use LLM reasoning for NPC presence suggestions by default. When disabled, only rule-based logic is used.".into(),
            field_type: "boolean".into(),
            default_value: serde_json::json!(true),
            min_value: None,
            max_value: None,
            category: "Staging".into(),
            requires_restart: false,
        },
        // Asset Generation
        SettingsFieldMetadata {
            key: "style_reference_asset_id".into(),
            display_name: "Default Style Reference".into(),
            description: "Asset ID to use as default style reference for image generation. Set via 'Use as Style Reference' in asset gallery.".into(),
            field_type: "string".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "Assets".into(),
            requires_restart: false,
        },
    ]
}
