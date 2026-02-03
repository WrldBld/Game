//! Settings metadata for UI rendering.

use serde::{Deserialize, Serialize};

// =============================================================================
// LLM Model Preference (Shared Contract)
// =============================================================================

/// DM preference for LLM model selection.
///
/// This is a stable contract type used by both Engine and Player UI.
/// Stored in settings and used for model resolution fallback logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum LlmModelPreference {
    /// Automatically select the best available model using fallback logic
    Auto,
    /// Use a specific model ID
    Specific { model_id: String },
}

impl LlmModelPreference {
    /// Create a new Specific preference with the given model ID.
    pub fn specific(model_id: impl Into<String>) -> Self {
        Self::Specific {
            model_id: model_id.into(),
        }
    }

    /// Returns true if this is the Auto variant.
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Returns the model ID if this is a Specific preference.
    pub fn model_id(&self) -> Option<&str> {
        match self {
            Self::Specific { model_id } => Some(model_id),
            Self::Auto => None,
        }
    }
}

impl Default for LlmModelPreference {
    fn default() -> Self {
        Self::Auto
    }
}

// =============================================================================
// Settings Field Metadata
// =============================================================================

/// Settings field metadata for dynamic UI rendering.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsFieldMetadata {
    /// Field key (matches JSON field name).
    pub key: String,
    /// Display name for UI.
    pub display_name: String,
    /// Description/help text.
    pub description: String,
    /// Field type: "integer", "float", "boolean", "string".
    pub field_type: String,
    /// Default value.
    pub default_value: serde_json::Value,
    /// Minimum value (for numeric fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<serde_json::Value>,
    /// Maximum value (for numeric fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<serde_json::Value>,
    /// Category for grouping in UI.
    pub category: String,
    /// Whether changing this setting requires a restart.
    pub requires_restart: bool,
}

/// Metadata describing settings fields for UI clients.
pub fn settings_metadata() -> Vec<SettingsFieldMetadata> {
    vec![
        SettingsFieldMetadata {
            key: "max_conversation_turns".into(),
            display_name: "Max Conversation Turns".into(),
            description: "Maximum number of conversation turns to store in session memory".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(200)),
            category: "Conversation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "conversation_history_turns".into(),
            display_name: "Conversation History Turns".into(),
            description: "Number of conversation turns to include in LLM prompt context".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(20),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(100)),
            category: "Conversation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "circuit_breaker_failure_threshold".into(),
            display_name: "Circuit Breaker Failure Threshold".into(),
            description: "Number of consecutive failures before circuit breaker opens".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(5),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(100)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "circuit_breaker_open_duration_secs".into(),
            display_name: "Circuit Breaker Open Duration (s)".into(),
            description: "How long the circuit breaker stays open after failures".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(60),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(3600)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "health_check_cache_ttl_secs".into(),
            display_name: "Health Check Cache TTL (s)".into(),
            description: "Time-to-live for health check cache entries".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(300)),
            category: "System".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "max_name_length".into(),
            display_name: "Max Name Length".into(),
            description: "Maximum allowed length for entity names".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(255),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(500)),
            category: "Validation".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "max_description_length".into(),
            display_name: "Max Description Length".into(),
            description: "Maximum allowed length for description fields".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(10000),
            min_value: Some(serde_json::json!(100)),
            max_value: Some(serde_json::json!(50000)),
            category: "Validation".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "list_default_page_size".into(),
            display_name: "List Page Size (Default)".into(),
            description: "Number of items returned per page when no limit is specified. Can be overridden by WRLDBLDR_LIST_DEFAULT_PAGE_SIZE environment variable.".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(50),
            min_value: Some(serde_json::json!(10)),
            max_value: Some(serde_json::json!(200)),
            category: "Performance".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "list_max_page_size".into(),
            display_name: "List Page Size (Maximum)".into(),
            description: "Maximum items allowed per page (DoS prevention). Can be overridden by WRLDBLDR_LIST_MAX_PAGE_SIZE environment variable.".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(200),
            min_value: Some(serde_json::json!(50)),
            max_value: Some(serde_json::json!(1000)),
            category: "Performance".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_sentence_delay_ms".into(),
            display_name: "Typewriter Sentence Delay (ms)".into(),
            description: "Delay between sentences in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(150),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(1000)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_pause_delay_ms".into(),
            display_name: "Typewriter Pause Delay (ms)".into(),
            description: "Delay for pause punctuation in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(80),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(1000)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "typewriter_char_delay_ms".into(),
            display_name: "Typewriter Char Delay (ms)".into(),
            description: "Delay between characters in typewriter effect".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(200)),
            category: "Animation".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "default_max_stat_value".into(),
            display_name: "Default Max Stat Value".into(),
            description: "Default maximum stat value for characters".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(20),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(100)),
            category: "Game".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "default_presence_cache_ttl_hours".into(),
            display_name: "Presence Cache TTL (hours)".into(),
            description: "Default NPC presence cache TTL for new locations".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(3),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(24)),
            category: "Staging".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "default_use_llm_presence".into(),
            display_name: "Use LLM Presence".into(),
            description: "Whether to use LLM for staging decisions by default".into(),
            field_type: "boolean".into(),
            default_value: serde_json::json!(true),
            min_value: None,
            max_value: None,
            category: "Staging".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "staging_timeout_seconds".into(),
            display_name: "Auto-Approve Timeout".into(),
            description: "Seconds to wait before auto-approving staging requests (0 = disabled)".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(30),
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(300)),
            category: "Staging".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "auto_approve_on_timeout".into(),
            display_name: "Auto-Approve on Timeout".into(),
            description: "Whether to auto-approve staging requests with rule-based NPCs when DM doesn't respond within timeout".into(),
            field_type: "boolean".into(),
            default_value: serde_json::json!(true),
            min_value: None,
            max_value: None,
            category: "Staging".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "outcome_branch_count".into(),
            display_name: "Outcome Branch Count".into(),
            description: "Number of outcome branches to generate per challenge tier".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(2),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(6)),
            category: "Challenge".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "outcome_branch_min".into(),
            display_name: "Outcome Branch Min".into(),
            description: "Minimum allowed outcome branch count".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(1),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(6)),
            category: "Challenge".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "outcome_branch_max".into(),
            display_name: "Outcome Branch Max".into(),
            description: "Maximum allowed outcome branch count".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(4),
            min_value: Some(serde_json::json!(1)),
            max_value: Some(serde_json::json!(10)),
            category: "Challenge".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "suggestion_tokens_per_branch".into(),
            display_name: "Suggestion Tokens Per Branch".into(),
            description: "Max tokens per outcome branch when generating suggestions".into(),
            field_type: "integer".into(),
            default_value: serde_json::json!(200),
            min_value: Some(serde_json::json!(20)),
            max_value: Some(serde_json::json!(2000)),
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "context_budget".into(),
            display_name: "Context Budget".into(),
            description: "Token budget configuration for LLM context building".into(),
            field_type: "object".into(),
            default_value: serde_json::json!({
                "total_budget_tokens": 4000,
                "scene_tokens": 500,
                "character_tokens": 800,
                "conversation_history_tokens": 1000,
                "challenges_tokens": 400,
                "narrative_events_tokens": 400,
                "directorial_notes_tokens": 300,
                "location_context_tokens": 300,
                "player_context_tokens": 300,
                "enable_summarization": true,
                "summarization_model": null,
            }),
            min_value: None,
            max_value: None,
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_base_url".into(),
            display_name: "LLM Base URL".into(),
            description: "Base URL for the LLM provider (e.g., http://localhost:11434 for Ollama)".into(),
            field_type: "string".into(),
            default_value: serde_json::json!("http://localhost:11434"),
            min_value: None,
            max_value: None,
            category: "LLM".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "llm_model_preference".into(),
            display_name: "LLM Model Preference".into(),
            description: "Preferred LLM model. Use 'Auto' to automatically select from available models, or specify a model ID.".into(),
            field_type: "object".into(),
            default_value: serde_json::json!({
                "type": "Auto"
            }),
            min_value: None,
            max_value: None,
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_temperature_npc_response".into(),
            display_name: "LLM Temperature (NPC Response)".into(),
            description: "Temperature for NPC response generation. Lower values produce more consistent responses, higher values more creative.".into(),
            field_type: "float".into(),
            default_value: serde_json::json!(0.7),
            min_value: Some(serde_json::json!(0.0)),
            max_value: Some(serde_json::json!(2.0)),
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_temperature_suggestion".into(),
            display_name: "LLM Temperature (Suggestions)".into(),
            description: "Temperature for generic content suggestions.".into(),
            field_type: "float".into(),
            default_value: serde_json::json!(0.8),
            min_value: Some(serde_json::json!(0.0)),
            max_value: Some(serde_json::json!(2.0)),
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_temperature_outcome_suggestion".into(),
            display_name: "LLM Temperature (Outcome Suggestions)".into(),
            description: "Temperature for challenge outcome suggestions.".into(),
            field_type: "float".into(),
            default_value: serde_json::json!(0.8),
            min_value: Some(serde_json::json!(0.0)),
            max_value: Some(serde_json::json!(2.0)),
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_temperature_staging".into(),
            display_name: "LLM Temperature (Staging)".into(),
            description: "Temperature for NPC staging suggestions.".into(),
            field_type: "float".into(),
            default_value: serde_json::json!(0.7),
            min_value: Some(serde_json::json!(0.0)),
            max_value: Some(serde_json::json!(2.0)),
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_temperature_custom_condition".into(),
            display_name: "LLM Temperature (Custom Conditions)".into(),
            description: "Temperature for custom condition evaluation. Lower values produce more deterministic evaluations.".into(),
            field_type: "float".into(),
            default_value: serde_json::json!(0.1),
            min_value: Some(serde_json::json!(0.0)),
            max_value: Some(serde_json::json!(2.0)),
            category: "LLM".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "llm_api_key".into(),
            display_name: "LLM API Key".into(),
            description: "API key for OpenAI-compatible LLM providers (e.g., OpenAI, Anthropic). Sent as Bearer token. Can also be set via OLLAMA_API_KEY env var.".into(),
            field_type: "secret".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "LLM".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "style_reference_asset_id".into(),
            display_name: "Style Reference Asset ID".into(),
            description: "Default style reference asset ID for image generation".into(),
            field_type: "string".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "Assets".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "batch_queue_failure_policy".into(),
            display_name: "Batch Queue Failure Policy".into(),
            description: "Policy for how to handle failures while queueing prompts for a batch"
                .into(),
            field_type: "string".into(),
            default_value: serde_json::json!("all_or_nothing"),
            min_value: None,
            max_value: None,
            category: "Assets".into(),
            requires_restart: false,
        },
    ]
}
