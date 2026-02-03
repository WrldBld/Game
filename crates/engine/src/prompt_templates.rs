//! Configurable LLM prompt templates used by the engine.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub mod defaults_store;

pub use defaults_store::{DefaultsError, PromptDefaultsStore};

/// Resolved prompt template with optional metadata.
///
/// This struct carries both the template text and any metadata
/// parsed from YAML front matter in the template file.
#[derive(Debug, Clone)]
pub struct ResolvedPromptTemplate {
    /// The template text (prompt content without front matter)
    pub text: String,
    /// Optional temperature from front matter (0.0 - 2.0)
    pub temperature: Option<f32>,
}

impl ResolvedPromptTemplate {
    /// Create a new resolved template with just text (no temperature)
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            temperature: None,
        }
    }

    /// Create a new resolved template with text and temperature
    pub fn with_temperature(text: impl Into<String>, temperature: f32) -> Self {
        Self {
            text: text.into(),
            temperature: Some(temperature.clamp(0.0, 2.0)),
        }
    }
}

/// Categories for organizing prompt templates in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PromptTemplateCategory {
    /// NPC dialogue and roleplay.
    Dialogue,
    /// NPC presence/staging decisions.
    Staging,
    /// Challenge outcome suggestions.
    Outcomes,
    /// Worldbuilding content suggestions.
    Suggestions,
    /// Context summarization.
    Summarization,
    /// System/internal prompts.
    System,
}

impl PromptTemplateCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dialogue => "dialogue",
            Self::Staging => "staging",
            Self::Outcomes => "outcomes",
            Self::Suggestions => "suggestions",
            Self::Summarization => "summarization",
            Self::System => "system",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dialogue => "NPC Dialogue",
            Self::Staging => "NPC Staging",
            Self::Outcomes => "Challenge Outcomes",
            Self::Suggestions => "Worldbuilding Suggestions",
            Self::Summarization => "Context Summarization",
            Self::System => "System Prompts",
        }
    }
}

/// Metadata about a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateMetadata {
    /// Unique key for this template.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Description of what this template is used for.
    pub description: String,
    /// Category for UI grouping.
    pub category: PromptTemplateCategory,
    /// The default value (loaded from file).
    pub default_value: String,
    /// Environment variable name for override.
    pub env_var: String,
    /// Path to the default file (for debugging/monitoring).
    pub default_path: String,
}

/// Static definition for a prompt template (without the default value).
///
/// This is used to define templates at compile time. The actual default values
/// are loaded from files at runtime via `PromptDefaultsStore`.
#[derive(Debug, Clone)]
pub struct PromptTemplateDefinition {
    /// Unique key for this template.
    pub key: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Description of what this template is used for.
    pub description: &'static str,
    /// Category for UI grouping.
    pub category: PromptTemplateCategory,
    /// Environment variable name for override.
    pub env_var: &'static str,
}

/// All prompt template keys as constants.
pub mod keys {
    // === Dialogue System ===
    /// The response format instructions shown to the LLM for NPC dialogue.
    pub const DIALOGUE_RESPONSE_FORMAT: &str = "dialogue.response_format";
    /// Challenge suggestion format instructions.
    pub const DIALOGUE_CHALLENGE_SUGGESTION_FORMAT: &str = "dialogue.challenge_suggestion_format";
    /// Narrative event suggestion format instructions.
    pub const DIALOGUE_NARRATIVE_EVENT_FORMAT: &str = "dialogue.narrative_event_format";

    // === Multi-Phase Dialogue Pipeline ===
    /// Phase 0: Context digest (deterministic game state summary).
    pub const DIALOGUE_PHASE0_CONTEXT_DIGEST: &str = "dialogue.phase0_context_digest";
    /// Phase 1: Challenge trigger analysis.
    pub const DIALOGUE_PHASE1_CHALLENGE_TRIGGERS: &str = "dialogue.phase1_challenge_triggers";
    /// Phase 2: Narrative event trigger analysis.
    pub const DIALOGUE_PHASE2_EVENT_TRIGGERS: &str = "dialogue.phase2_event_triggers";
    /// Phase 3: Tool intent detection.
    pub const DIALOGUE_PHASE3_TOOL_INTENT: &str = "dialogue.phase3_tool_intent";
    /// Phase 4: Behavior analysis (deflection, mood, disposition).
    pub const DIALOGUE_PHASE4_BEHAVIOR_ANALYSIS: &str = "dialogue.phase4_behavior_analysis";
    /// Phase 5: Final response generation.
    pub const DIALOGUE_PHASE5_FINAL_RESPONSE: &str = "dialogue.phase5_final_response";

    // === Staging System ===
    /// System prompt for NPC staging decisions.
    pub const STAGING_SYSTEM_PROMPT: &str = "staging.system_prompt";
    /// Instructions for staging response format.
    pub const STAGING_RESPONSE_FORMAT: &str = "staging.response_format";
    /// Role description for staging LLM.
    pub const STAGING_ROLE_INSTRUCTIONS: &str = "staging.role_instructions";

    // === Outcome Suggestions ===
    /// System prompt for outcome description generation.
    pub const OUTCOME_SYSTEM_PROMPT: &str = "outcome.system_prompt";
    /// System prompt for outcome branch generation.
    pub const OUTCOME_BRANCH_SYSTEM_PROMPT: &str = "outcome.branch_system_prompt";

    // === Worldbuilding Suggestions ===
    /// Character name generation prompt.
    pub const SUGGESTION_CHARACTER_NAME: &str = "suggestion.character_name";
    /// Character description generation prompt.
    pub const SUGGESTION_CHARACTER_DESCRIPTION: &str = "suggestion.character_description";
    /// Character wants/desires generation prompt.
    pub const SUGGESTION_CHARACTER_WANTS: &str = "suggestion.character_wants";
    /// Character fears generation prompt.
    pub const SUGGESTION_CHARACTER_FEARS: &str = "suggestion.character_fears";
    /// Character backstory generation prompt.
    pub const SUGGESTION_CHARACTER_BACKSTORY: &str = "suggestion.character_backstory";
    /// Location name generation prompt.
    pub const SUGGESTION_LOCATION_NAME: &str = "suggestion.location_name";
    /// Location description generation prompt.
    pub const SUGGESTION_LOCATION_DESCRIPTION: &str = "suggestion.location_description";
    /// Location atmosphere generation prompt.
    pub const SUGGESTION_LOCATION_ATMOSPHERE: &str = "suggestion.location_atmosphere";
    /// Location features generation prompt.
    pub const SUGGESTION_LOCATION_FEATURES: &str = "suggestion.location_features";
    /// Location secrets generation prompt.
    pub const SUGGESTION_LOCATION_SECRETS: &str = "suggestion.location_secrets";

    // === Actantial Model Suggestions ===
    /// Deflection behavior suggestion prompt.
    pub const SUGGESTION_DEFLECTION_BEHAVIOR: &str = "suggestion.deflection_behavior";
    /// Behavioral tells suggestion prompt.
    pub const SUGGESTION_BEHAVIORAL_TELLS: &str = "suggestion.behavioral_tells";
    /// Want description suggestion prompt (actantial-aware).
    pub const SUGGESTION_WANT_DESCRIPTION: &str = "suggestion.want_description";
    /// Actantial reason suggestion prompt.
    pub const SUGGESTION_ACTANTIAL_REASON: &str = "suggestion.actantial_reason";

    // === System Prompts (for hard-coded prompts) ===
    /// System prompt for custom condition evaluation.
    pub const SYSTEM_CUSTOM_CONDITION_EVALUATOR: &str = "system.custom_condition_evaluator";
    /// System prompt for staging suggestions (LLM-based NPC selection).
    pub const SYSTEM_STAGING_SUGGESTIONS: &str = "system.staging_suggestions_system";
    /// System prompt for outcome suggestions.
    pub const SYSTEM_OUTCOME_SUGGESTIONS: &str = "system.outcome_suggestions_system";
    /// System prompt for generic worldbuilding suggestions.
    pub const SYSTEM_GENERIC_SUGGESTION: &str = "system.generic_suggestion_system";
}

/// Static template definitions (without default values).
///
/// These are used to define the structure of templates. Default values are
/// loaded from files at runtime.
pub fn template_definitions() -> Vec<PromptTemplateDefinition> {
    vec![
        // Dialogue
        PromptTemplateDefinition {
            key: keys::DIALOGUE_RESPONSE_FORMAT,
            label: "Response Format Instructions",
            description: "Instructions shown to the LLM for how to format NPC dialogue responses",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_RESPONSE_FORMAT",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT,
            label: "Challenge Suggestion Format",
            description: "Format instructions for suggesting skill challenges during dialogue",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_CHALLENGE_SUGGESTION_FORMAT",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_NARRATIVE_EVENT_FORMAT,
            label: "Narrative Event Suggestion Format",
            description: "Format instructions for suggesting narrative events during dialogue",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_NARRATIVE_EVENT_FORMAT",
        },
        // Multi-Phase Dialogue Pipeline
        PromptTemplateDefinition {
            key: keys::DIALOGUE_PHASE0_CONTEXT_DIGEST,
            label: "Phase 0: Context Digest",
            description: "Deterministic game state summary for downstream phases",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_PHASE0_CONTEXT_DIGEST",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_PHASE1_CHALLENGE_TRIGGERS,
            label: "Phase 1: Challenge Trigger Analysis",
            description: "Analyze player action against active challenges",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_PHASE1_CHALLENGE_TRIGGERS",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_PHASE2_EVENT_TRIGGERS,
            label: "Phase 2: Event Trigger Analysis",
            description: "Analyze player action against active narrative events",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_PHASE2_EVENT_TRIGGERS",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_PHASE3_TOOL_INTENT,
            label: "Phase 3: Tool Intent Detection",
            description: "Identify appropriate game tools based on player action",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_PHASE3_TOOL_INTENT",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_PHASE4_BEHAVIOR_ANALYSIS,
            label: "Phase 4: Behavior Analysis",
            description: "Determine deflection behavior and emotional stance",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_PHASE4_BEHAVIOR_ANALYSIS",
        },
        PromptTemplateDefinition {
            key: keys::DIALOGUE_PHASE5_FINAL_RESPONSE,
            label: "Phase 5: Final Response Generation",
            description: "Generate player-facing dialogue based on prior phase outputs",
            category: PromptTemplateCategory::Dialogue,
            env_var: "WRLDBLDR_PROMPT_DIALOGUE_PHASE5_FINAL_RESPONSE",
        },
        // Staging
        PromptTemplateDefinition {
            key: keys::STAGING_SYSTEM_PROMPT,
            label: "Staging System Prompt",
            description: "System prompt for NPC presence/staging decisions",
            category: PromptTemplateCategory::Staging,
            env_var: "WRLDBLDR_PROMPT_STAGING_SYSTEM_PROMPT",
        },
        PromptTemplateDefinition {
            key: keys::STAGING_ROLE_INSTRUCTIONS,
            label: "Staging Role Instructions",
            description: "Instructions explaining the LLM's role in staging decisions",
            category: PromptTemplateCategory::Staging,
            env_var: "WRLDBLDR_PROMPT_STAGING_ROLE_INSTRUCTIONS",
        },
        PromptTemplateDefinition {
            key: keys::STAGING_RESPONSE_FORMAT,
            label: "Staging Response Format",
            description: "Expected JSON response format for staging decisions",
            category: PromptTemplateCategory::Staging,
            env_var: "WRLDBLDR_PROMPT_STAGING_RESPONSE_FORMAT",
        },
        // Outcomes
        PromptTemplateDefinition {
            key: keys::OUTCOME_SYSTEM_PROMPT,
            label: "Outcome System Prompt",
            description: "System prompt for generating challenge outcome descriptions",
            category: PromptTemplateCategory::Outcomes,
            env_var: "WRLDBLDR_PROMPT_OUTCOME_SYSTEM_PROMPT",
        },
        PromptTemplateDefinition {
            key: keys::OUTCOME_BRANCH_SYSTEM_PROMPT,
            label: "Outcome Branch System Prompt",
            description: "System prompt for generating outcome branches (use {branch_count} placeholder)",
            category: PromptTemplateCategory::Outcomes,
            env_var: "WRLDBLDR_PROMPT_OUTCOME_BRANCH_SYSTEM_PROMPT",
        },
        // Suggestions
        PromptTemplateDefinition {
            key: keys::SUGGESTION_CHARACTER_NAME,
            label: "Character Name Suggestions",
            description: "Prompt for generating character name suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_CHARACTER_NAME",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_CHARACTER_DESCRIPTION,
            label: "Character Description Suggestions",
            description: "Prompt for generating character description suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_CHARACTER_DESCRIPTION",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_CHARACTER_WANTS,
            label: "Character Wants Suggestions",
            description: "Prompt for generating character motivation suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_CHARACTER_WANTS",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_CHARACTER_FEARS,
            label: "Character Fears Suggestions",
            description: "Prompt for generating character fear suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_CHARACTER_FEARS",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_CHARACTER_BACKSTORY,
            label: "Character Backstory Suggestions",
            description: "Prompt for generating character backstory suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_CHARACTER_BACKSTORY",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_LOCATION_NAME,
            label: "Location Name Suggestions",
            description: "Prompt for generating location name suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_LOCATION_NAME",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_LOCATION_DESCRIPTION,
            label: "Location Description Suggestions",
            description: "Prompt for generating location description suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_LOCATION_DESCRIPTION",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_LOCATION_ATMOSPHERE,
            label: "Location Atmosphere Suggestions",
            description: "Prompt for generating location atmosphere suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_LOCATION_ATMOSPHERE",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_LOCATION_FEATURES,
            label: "Location Features Suggestions",
            description: "Prompt for generating location feature suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_LOCATION_FEATURES",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_LOCATION_SECRETS,
            label: "Location Secrets Suggestions",
            description: "Prompt for generating location secret suggestions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_LOCATION_SECRETS",
        },
        // Actantial Model Suggestions
        PromptTemplateDefinition {
            key: keys::SUGGESTION_DEFLECTION_BEHAVIOR,
            label: "Deflection Behavior Suggestions",
            description: "Prompt for generating NPC deflection behaviors for hidden wants",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_DEFLECTION_BEHAVIOR",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_BEHAVIORAL_TELLS,
            label: "Behavioral Tells Suggestions",
            description: "Prompt for generating behavioral tells that reveal hidden NPC wants",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_BEHAVIORAL_TELLS",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_WANT_DESCRIPTION,
            label: "Want Description Suggestions",
            description: "Prompt for generating actantial-aware want descriptions",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_WANT_DESCRIPTION",
        },
        PromptTemplateDefinition {
            key: keys::SUGGESTION_ACTANTIAL_REASON,
            label: "Actantial Reason Suggestions",
            description: "Prompt for generating reasons for actantial relationships",
            category: PromptTemplateCategory::Suggestions,
            env_var: "WRLDBLDR_PROMPT_SUGGESTION_ACTANTIAL_REASON",
        },
        // System Prompts
        PromptTemplateDefinition {
            key: keys::SYSTEM_CUSTOM_CONDITION_EVALUATOR,
            label: "Custom Condition Evaluator",
            description: "System prompt for LLM-based custom condition evaluation (use {confidence_threshold} placeholder)",
            category: PromptTemplateCategory::System,
            env_var: "WRLDBLDR_PROMPT_SYSTEM_CUSTOM_CONDITION_EVALUATOR",
        },
        PromptTemplateDefinition {
            key: keys::SYSTEM_STAGING_SUGGESTIONS,
            label: "Staging Suggestions System",
            description: "System prompt for LLM-based NPC staging suggestions",
            category: PromptTemplateCategory::System,
            env_var: "WRLDBLDR_PROMPT_SYSTEM_STAGING_SUGGESTIONS",
        },
        PromptTemplateDefinition {
            key: keys::SYSTEM_OUTCOME_SUGGESTIONS,
            label: "Outcome Suggestions System",
            description: "System prompt for generating alternative outcome descriptions",
            category: PromptTemplateCategory::System,
            env_var: "WRLDBLDR_PROMPT_SYSTEM_OUTCOME_SUGGESTIONS",
        },
        PromptTemplateDefinition {
            key: keys::SYSTEM_GENERIC_SUGGESTION,
            label: "Generic Suggestion System",
            description: "System prompt for generic worldbuilding suggestions",
            category: PromptTemplateCategory::System,
            env_var: "WRLDBLDR_PROMPT_SYSTEM_GENERIC_SUGGESTION",
        },
    ]
}

/// Convert a template key to its environment variable name.
pub fn key_to_env_var(key: &str) -> String {
    format!("WRLDBLDR_PROMPT_{}", key.to_uppercase().replace('.', "_"))
}

/// Check if a key is a known template key.
pub fn is_known_key(key: &str) -> bool {
    all_keys().contains(&key)
}

/// Get all known template keys.
pub fn all_keys() -> Vec<&'static str> {
    template_definitions()
        .into_iter()
        .map(|def| def.key)
        .collect()
}

/// Get metadata for all prompt templates.
///
/// This function loads default values from the `PromptDefaultsStore`.
/// Call this at startup and cache the result for efficiency.
pub async fn prompt_template_metadata(
    defaults_store: &PromptDefaultsStore,
) -> Result<Vec<PromptTemplateMetadata>, DefaultsError> {
    let mut metadata = Vec::new();

    for def in template_definitions() {
        let default_value = match defaults_store.get(def.key).await? {
            Some(value) => value,
            None => {
                // If file doesn't exist, return empty string but log a warning
                tracing::warn!(
                    key = def.key,
                    path = %defaults_store.path_for_key(def.key).display(),
                    "Default prompt file not found"
                );
                String::new()
            }
        };

        metadata.push(PromptTemplateMetadata {
            key: def.key.to_string(),
            label: def.label.to_string(),
            description: def.description.to_string(),
            category: def.category,
            default_value,
            env_var: def.env_var.to_string(),
            default_path: defaults_store.path_for_key(def.key).display().to_string(),
        });
    }

    Ok(metadata)
}

/// Get metadata for all prompt templates synchronously.
///
/// This is a convenience function for cases where you don't have access to
/// the defaults store. It returns metadata with empty default values.
/// Use `prompt_template_metadata` when possible for full metadata.
pub fn prompt_template_metadata_sync() -> Vec<PromptTemplateMetadata> {
    template_definitions()
        .into_iter()
        .map(|def| PromptTemplateMetadata {
            key: def.key.to_string(),
            label: def.label.to_string(),
            description: def.description.to_string(),
            category: def.category,
            default_value: String::new(),
            env_var: def.env_var.to_string(),
            default_path: format!("prompts/{}.md", def.key.replace('.', "/")),
        })
        .collect()
}

/// Get the definition for a specific template key.
pub fn get_definition(key: &str) -> Option<PromptTemplateDefinition> {
    template_definitions().into_iter().find(|def| def.key == key)
}

/// Legacy function for backward compatibility.
///
/// This function always returns `None` since defaults are now loaded from files
/// at runtime via `PromptDefaultsStore`. Use `PromptDefaultsStore::get()` instead.
///
/// # Deprecation
/// This function is deprecated. Use `PromptDefaultsStore::get()` for async access
/// to file-backed defaults.
pub fn get_default(_key: &str) -> Option<&'static str> {
    None
}
