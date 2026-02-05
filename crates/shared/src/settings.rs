//! Settings metadata for UI rendering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// LLM Model Preference (Shared Contract)
// =============================================================================

/// DM preference for LLM model selection.
///
/// This is a stable contract type used by both Engine and Player UI.
/// Stored in settings and used for model resolution fallback logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "value")]
pub enum LlmModelPreference {
    /// Automatically select the best available model using fallback logic
    #[default]
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

// =============================================================================
// ComfyUI Settings (Shared Contract)
// =============================================================================

/// Per-workflow ComfyUI configuration override.
///
/// This is a Tier-4 simple data struct per ADR-008.
/// Stored in AppSettings for global/per-world workflow overrides.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ComfyWorkflowConfig {
    /// Workflow ID/slot this config applies to
    pub workflow_id: String,
    /// Model ID override (e.g., "sdxl.safetensors")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Prompt prefix prepended to all prompts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_prefix: Option<String>,
    /// Prompt suffix appended to all prompts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_suffix: Option<String>,
    /// Negative prompt override (replaces global negative prompt)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt_override: Option<String>,
    /// Whether to prepend SCORE tokens (default: true)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_tokens_enabled: Option<bool>,
    /// Pose asset ID for ControlNet pose injection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlnet_pose_asset_id: Option<String>,
    /// ControlNet node mappings (node_id -> input_key)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlnet_mappings: Option<Vec<ControlNetMapping>>,
}

/// ControlNet node mapping for pose injection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlNetMapping {
    /// Node ID in the workflow
    pub node_id: String,
    /// Input key on that node
    pub input_key: String,
    /// Whether this mapping is enabled
    pub enabled: bool,
}

/// Global ComfyUI settings stored in AppSettings.
///
/// This is a Tier-4 simple data struct per ADR-008.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ComfyUiSettings {
    /// Base URL for ComfyUI server (overrides env var if set)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Default model ID for image generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model_id: Option<String>,
    /// Global prompt prefix prepended to all prompts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_prefix: Option<String>,
    /// Global prompt suffix appended to all prompts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_suffix: Option<String>,
    /// Global negative prompt
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Whether SCORE tokens are enabled by default
    #[serde(default = "default_score_tokens_enabled")]
    pub score_tokens_enabled: bool,
    /// Per-workflow overrides keyed by workflow_id
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub workflow_overrides: HashMap<String, ComfyWorkflowConfig>,
}

fn default_score_tokens_enabled() -> bool {
    true
}

impl ComfyUiSettings {
    /// Create default ComfyUI settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the effective workflow config by merging global defaults with per-workflow overrides.
    pub fn effective_workflow_config(&self, workflow_id: &str) -> ComfyWorkflowConfig {
        let mut config = ComfyWorkflowConfig {
            workflow_id: workflow_id.to_string(),
            model_id: self.default_model_id.clone(),
            prompt_prefix: self.prompt_prefix.clone(),
            prompt_suffix: self.prompt_suffix.clone(),
            negative_prompt_override: self.negative_prompt.clone(),
            score_tokens_enabled: Some(self.score_tokens_enabled),
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };

        // Apply per-workflow overrides
        if let Some(override_config) = self.workflow_overrides.get(workflow_id) {
            if override_config.model_id.is_some() {
                config.model_id = override_config.model_id.clone();
            }
            if override_config.prompt_prefix.is_some() {
                config.prompt_prefix = override_config.prompt_prefix.clone();
            }
            if override_config.prompt_suffix.is_some() {
                config.prompt_suffix = override_config.prompt_suffix.clone();
            }
            if override_config.negative_prompt_override.is_some() {
                config.negative_prompt_override = override_config.negative_prompt_override.clone();
            }
            if override_config.score_tokens_enabled.is_some() {
                config.score_tokens_enabled = override_config.score_tokens_enabled;
            }
            if override_config.controlnet_pose_asset_id.is_some() {
                config.controlnet_pose_asset_id = override_config.controlnet_pose_asset_id.clone();
            }
            if override_config.controlnet_mappings.is_some() {
                config.controlnet_mappings = override_config.controlnet_mappings.clone();
            }
        }

        config
    }

    /// Set a workflow override.
    pub fn set_workflow_override(&mut self, config: ComfyWorkflowConfig) {
        self.workflow_overrides
            .insert(config.workflow_id.clone(), config);
    }

    /// Remove a workflow override.
    pub fn remove_workflow_override(&mut self, workflow_id: &str) {
        self.workflow_overrides.remove(workflow_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // ComfyUiSettings::effective_workflow_config Tests
    // =============================================================================

    #[test]
    fn effective_workflow_config_returns_global_defaults_when_no_override() {
        let settings = ComfyUiSettings {
            default_model_id: Some("global_model.safetensors".to_string()),
            prompt_prefix: Some("global_prefix".to_string()),
            prompt_suffix: Some("global_suffix".to_string()),
            negative_prompt: Some("global_negative".to_string()),
            score_tokens_enabled: false,
            ..Default::default()
        };

        let config = settings.effective_workflow_config("test_workflow");

        assert_eq!(config.workflow_id, "test_workflow");
        assert_eq!(
            config.model_id,
            Some("global_model.safetensors".to_string())
        );
        assert_eq!(config.prompt_prefix, Some("global_prefix".to_string()));
        assert_eq!(config.prompt_suffix, Some("global_suffix".to_string()));
        assert_eq!(
            config.negative_prompt_override,
            Some("global_negative".to_string())
        );
        assert_eq!(config.score_tokens_enabled, Some(false));
        assert_eq!(config.controlnet_pose_asset_id, None);
        assert_eq!(config.controlnet_mappings, None);
    }

    #[test]
    fn effective_workflow_config_overrides_all_fields_when_set() {
        let mut settings = ComfyUiSettings::default();
        let override_config = ComfyWorkflowConfig {
            workflow_id: "test_workflow".to_string(),
            model_id: Some("override_model.safetensors".to_string()),
            prompt_prefix: Some("override_prefix".to_string()),
            prompt_suffix: Some("override_suffix".to_string()),
            negative_prompt_override: Some("override_negative".to_string()),
            score_tokens_enabled: Some(true),
            controlnet_pose_asset_id: Some("pose_123".to_string()),
            controlnet_mappings: Some(vec![ControlNetMapping {
                node_id: "node_1".to_string(),
                input_key: "image".to_string(),
                enabled: true,
            }]),
        };
        settings.set_workflow_override(override_config);

        let config = settings.effective_workflow_config("test_workflow");

        assert_eq!(
            config.model_id,
            Some("override_model.safetensors".to_string())
        );
        assert_eq!(config.prompt_prefix, Some("override_prefix".to_string()));
        assert_eq!(config.prompt_suffix, Some("override_suffix".to_string()));
        assert_eq!(
            config.negative_prompt_override,
            Some("override_negative".to_string())
        );
        assert_eq!(config.score_tokens_enabled, Some(true));
        assert_eq!(
            config.controlnet_pose_asset_id,
            Some("pose_123".to_string())
        );
        assert!(config.controlnet_mappings.is_some());
    }

    #[test]
    fn effective_workflow_config_partial_override_preserves_globals() {
        let mut settings = ComfyUiSettings {
            default_model_id: Some("global_model.safetensors".to_string()),
            prompt_prefix: Some("global_prefix".to_string()),
            prompt_suffix: Some("global_suffix".to_string()),
            negative_prompt: Some("global_negative".to_string()),
            score_tokens_enabled: true,
            ..Default::default()
        };
        let override_config = ComfyWorkflowConfig {
            workflow_id: "test_workflow".to_string(),
            model_id: Some("override_model.safetensors".to_string()),
            prompt_prefix: None,
            prompt_suffix: Some("override_suffix".to_string()),
            negative_prompt_override: None,
            score_tokens_enabled: Some(false),
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        settings.set_workflow_override(override_config);

        let config = settings.effective_workflow_config("test_workflow");

        // Overridden fields
        assert_eq!(
            config.model_id,
            Some("override_model.safetensors".to_string())
        );
        assert_eq!(config.prompt_suffix, Some("override_suffix".to_string()));
        assert_eq!(config.score_tokens_enabled, Some(false));

        // Preserved global fields
        assert_eq!(config.prompt_prefix, Some("global_prefix".to_string()));
        assert_eq!(
            config.negative_prompt_override,
            Some("global_negative".to_string())
        );
    }

    #[test]
    fn effective_workflow_config_different_workflow_uses_globals() {
        let mut settings = ComfyUiSettings {
            default_model_id: Some("global_model.safetensors".to_string()),
            prompt_prefix: Some("global_prefix".to_string()),
            ..Default::default()
        };
        let override_config = ComfyWorkflowConfig {
            workflow_id: "workflow_a".to_string(),
            model_id: Some("model_a.safetensors".to_string()),
            prompt_prefix: Some("prefix_a".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: None,
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        settings.set_workflow_override(override_config);

        // workflow_b should use global defaults
        let config = settings.effective_workflow_config("workflow_b");

        assert_eq!(
            config.model_id,
            Some("global_model.safetensors".to_string())
        );
        assert_eq!(config.prompt_prefix, Some("global_prefix".to_string()));
    }

    #[test]
    fn effective_workflow_config_empty_globals_uses_none() {
        let settings = ComfyUiSettings::default();

        let config = settings.effective_workflow_config("test_workflow");

        assert_eq!(config.workflow_id, "test_workflow");
        assert_eq!(config.model_id, None);
        assert_eq!(config.prompt_prefix, None);
        assert_eq!(config.prompt_suffix, None);
        assert_eq!(config.negative_prompt_override, None);
        assert_eq!(config.score_tokens_enabled, Some(true)); // default is true
        assert_eq!(config.controlnet_pose_asset_id, None);
        assert_eq!(config.controlnet_mappings, None);
    }

    #[test]
    fn set_workflow_override_replaces_existing() {
        let mut settings = ComfyUiSettings::default();

        let config1 = ComfyWorkflowConfig {
            workflow_id: "test_workflow".to_string(),
            model_id: Some("model_1.safetensors".to_string()),
            prompt_prefix: Some("prefix_1".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: None,
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        settings.set_workflow_override(config1);

        let config2 = ComfyWorkflowConfig {
            workflow_id: "test_workflow".to_string(),
            model_id: Some("model_2.safetensors".to_string()),
            prompt_prefix: Some("prefix_2".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: None,
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        settings.set_workflow_override(config2);

        let effective = settings.effective_workflow_config("test_workflow");
        assert_eq!(effective.model_id, Some("model_2.safetensors".to_string()));
        assert_eq!(effective.prompt_prefix, Some("prefix_2".to_string()));
    }

    #[test]
    fn remove_workflow_override_restores_globals() {
        let mut settings = ComfyUiSettings {
            default_model_id: Some("global_model.safetensors".to_string()),
            prompt_prefix: Some("global_prefix".to_string()),
            ..Default::default()
        };
        let override_config = ComfyWorkflowConfig {
            workflow_id: "test_workflow".to_string(),
            model_id: Some("override_model.safetensors".to_string()),
            prompt_prefix: Some("override_prefix".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: None,
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        settings.set_workflow_override(override_config);

        // Verify override is active
        let config_before = settings.effective_workflow_config("test_workflow");
        assert_eq!(
            config_before.model_id,
            Some("override_model.safetensors".to_string())
        );

        // Remove override
        settings.remove_workflow_override("test_workflow");

        // Verify globals are restored
        let config_after = settings.effective_workflow_config("test_workflow");
        assert_eq!(
            config_after.model_id,
            Some("global_model.safetensors".to_string())
        );
        assert_eq!(
            config_after.prompt_prefix,
            Some("global_prefix".to_string())
        );
    }

    #[test]
    fn effective_workflow_config_multiple_workflows_isolated() {
        let mut settings = ComfyUiSettings {
            default_model_id: Some("global_model.safetensors".to_string()),
            prompt_prefix: Some("global_prefix".to_string()),
            ..Default::default()
        };

        // Set overrides for two different workflows
        let config_a = ComfyWorkflowConfig {
            workflow_id: "workflow_a".to_string(),
            model_id: Some("model_a.safetensors".to_string()),
            prompt_prefix: Some("prefix_a".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: None,
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        let config_b = ComfyWorkflowConfig {
            workflow_id: "workflow_b".to_string(),
            model_id: Some("model_b.safetensors".to_string()),
            prompt_prefix: Some("prefix_b".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: None,
            controlnet_pose_asset_id: None,
            controlnet_mappings: None,
        };
        settings.set_workflow_override(config_a);
        settings.set_workflow_override(config_b);

        // Each workflow should have its own config
        let effective_a = settings.effective_workflow_config("workflow_a");
        let effective_b = settings.effective_workflow_config("workflow_b");

        assert_eq!(
            effective_a.model_id,
            Some("model_a.safetensors".to_string())
        );
        assert_eq!(effective_a.prompt_prefix, Some("prefix_a".to_string()));
        assert_eq!(
            effective_b.model_id,
            Some("model_b.safetensors".to_string())
        );
        assert_eq!(effective_b.prompt_prefix, Some("prefix_b".to_string()));
    }

    // =============================================================================
    // LlmModelPreference Tests
    // =============================================================================

    #[test]
    fn llm_model_preference_auto_is_auto() {
        let pref = LlmModelPreference::Auto;
        assert!(pref.is_auto());
        assert_eq!(pref.model_id(), None);
    }

    #[test]
    fn llm_model_preference_specific_is_not_auto() {
        let pref = LlmModelPreference::specific("llama3.2");
        assert!(!pref.is_auto());
        assert_eq!(pref.model_id(), Some("llama3.2"));
    }

    #[test]
    fn llm_model_preference_specific_creation() {
        let pref = LlmModelPreference::specific("gpt-4");
        match pref {
            LlmModelPreference::Specific { model_id } => assert_eq!(model_id, "gpt-4"),
            _ => panic!("Expected Specific variant"),
        }
    }

    // =============================================================================
    // ControlNetMapping Tests
    // =============================================================================

    #[test]
    fn controlnet_mapping_equality() {
        let mapping1 = ControlNetMapping {
            node_id: "node_1".to_string(),
            input_key: "image".to_string(),
            enabled: true,
        };
        let mapping2 = ControlNetMapping {
            node_id: "node_1".to_string(),
            input_key: "image".to_string(),
            enabled: true,
        };
        let mapping3 = ControlNetMapping {
            node_id: "node_2".to_string(),
            input_key: "image".to_string(),
            enabled: true,
        };

        assert_eq!(mapping1, mapping2);
        assert_ne!(mapping1, mapping3);
    }

    // =============================================================================
    // ComfyWorkflowConfig Default Tests
    // =============================================================================

    #[test]
    fn comfy_workflow_config_default_is_empty() {
        let config = ComfyWorkflowConfig::default();
        assert!(config.workflow_id.is_empty());
        assert_eq!(config.model_id, None);
        assert_eq!(config.prompt_prefix, None);
        assert_eq!(config.prompt_suffix, None);
        assert_eq!(config.negative_prompt_override, None);
        assert_eq!(config.score_tokens_enabled, None);
        assert_eq!(config.controlnet_pose_asset_id, None);
        assert_eq!(config.controlnet_mappings, None);
    }

    // =============================================================================
    // Serialization Tests
    // =============================================================================

    #[test]
    fn comfy_ui_settings_serialization_roundtrip() {
        let mut settings = ComfyUiSettings {
            base_url: Some("http://localhost:8188".to_string()),
            default_model_id: Some("model.safetensors".to_string()),
            prompt_prefix: Some("prefix".to_string()),
            prompt_suffix: Some("suffix".to_string()),
            negative_prompt: Some("negative".to_string()),
            score_tokens_enabled: false,
            workflow_overrides: HashMap::new(),
        };
        let override_config = ComfyWorkflowConfig {
            workflow_id: "test".to_string(),
            model_id: Some("override.safetensors".to_string()),
            prompt_prefix: Some("override_prefix".to_string()),
            prompt_suffix: None,
            negative_prompt_override: None,
            score_tokens_enabled: Some(true),
            controlnet_pose_asset_id: Some("pose".to_string()),
            controlnet_mappings: Some(vec![ControlNetMapping {
                node_id: "node_1".to_string(),
                input_key: "image".to_string(),
                enabled: true,
            }]),
        };
        settings.set_workflow_override(override_config);

        let json = serde_json::to_string(&settings).expect("should serialize");
        let deserialized: ComfyUiSettings =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.base_url, settings.base_url);
        assert_eq!(deserialized.default_model_id, settings.default_model_id);
        assert_eq!(deserialized.prompt_prefix, settings.prompt_prefix);
        assert_eq!(deserialized.prompt_suffix, settings.prompt_suffix);
        assert_eq!(deserialized.negative_prompt, settings.negative_prompt);
        assert_eq!(
            deserialized.score_tokens_enabled,
            settings.score_tokens_enabled
        );
        assert_eq!(deserialized.workflow_overrides.len(), 1);

        let override_roundtrip = deserialized
            .workflow_overrides
            .get("test")
            .expect("should have test workflow");
        assert_eq!(
            override_roundtrip.model_id,
            Some("override.safetensors".to_string())
        );
        assert_eq!(
            override_roundtrip.controlnet_pose_asset_id,
            Some("pose".to_string())
        );
    }

    #[test]
    fn comfy_ui_settings_deserialization_missing_fields_uses_defaults() {
        let json = r#"{}"#;
        let settings: ComfyUiSettings = serde_json::from_str(json).expect("should deserialize");

        assert_eq!(settings.base_url, None);
        assert_eq!(settings.default_model_id, None);
        assert_eq!(settings.prompt_prefix, None);
        assert_eq!(settings.prompt_suffix, None);
        assert_eq!(settings.negative_prompt, None);
        assert_eq!(settings.score_tokens_enabled, true); // default
        assert!(settings.workflow_overrides.is_empty());
    }

    #[test]
    fn comfy_ui_settings_deserialization_partial_fields() {
        let json = r#"{
            "base_url": "http://localhost:8188",
            "default_model_id": "model.safetensors"
        }"#;
        let settings: ComfyUiSettings = serde_json::from_str(json).expect("should deserialize");

        assert_eq!(settings.base_url, Some("http://localhost:8188".to_string()));
        assert_eq!(
            settings.default_model_id,
            Some("model.safetensors".to_string())
        );
        assert_eq!(settings.prompt_prefix, None);
        assert_eq!(settings.score_tokens_enabled, true); // default
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
        // ComfyUI Settings
        SettingsFieldMetadata {
            key: "comfyui_base_url".into(),
            display_name: "ComfyUI Base URL".into(),
            description: "Base URL for ComfyUI server (overrides COMFYUI_BASE_URL env var if set)".into(),
            field_type: "string".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: true,
        },
        SettingsFieldMetadata {
            key: "comfyui_default_model_id".into(),
            display_name: "Default Model ID".into(),
            description: "Default checkpoint/model for image generation (e.g., sdxl.safetensors)".into(),
            field_type: "string".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "comfyui_prompt_prefix".into(),
            display_name: "Prompt Prefix".into(),
            description: "Global prompt prefix prepended to all generation prompts".into(),
            field_type: "string".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "comfyui_prompt_suffix".into(),
            display_name: "Prompt Suffix".into(),
            description: "Global prompt suffix appended to all generation prompts".into(),
            field_type: "string".into(),
            default_value: serde_json::json!(null),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "comfyui_negative_prompt".into(),
            display_name: "Negative Prompt".into(),
            description: "Global negative prompt for all generations".into(),
            field_type: "string".into(),
            default_value: serde_json::json!("text, watermark, people, crowd,"),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "comfyui_score_tokens_enabled".into(),
            display_name: "SCORE Tokens Enabled".into(),
            description: "Whether to prepend SCORE_9, SCORE_8, etc. tokens for Pony/SDXL models".into(),
            field_type: "boolean".into(),
            default_value: serde_json::json!(true),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: false,
        },
        SettingsFieldMetadata {
            key: "comfyui_workflow_overrides".into(),
            display_name: "Workflow Overrides".into(),
            description: "Per-workflow configuration overrides (advanced)".into(),
            field_type: "object".into(),
            default_value: serde_json::json!({}),
            min_value: None,
            max_value: None,
            category: "ComfyUI".into(),
            requires_restart: false,
        },
    ]
}
