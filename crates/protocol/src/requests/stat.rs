//! Stat-related request types for character stat management

use serde::{Deserialize, Serialize};

/// Requests for character stat operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatRequest {
    /// Get all stats for a character (includes base values, modifiers, and effective values)
    GetCharacterStats { character_id: String },

    /// Set a base stat value for a character
    SetBaseStat {
        character_id: String,
        stat_name: String,
        value: i32,
    },

    /// Add a modifier to a stat
    AddModifier {
        character_id: String,
        data: AddModifierData,
    },

    /// Remove a modifier from a stat
    RemoveModifier {
        character_id: String,
        stat_name: String,
        modifier_id: String,
    },

    /// Toggle a modifier's active state
    ToggleModifier {
        character_id: String,
        stat_name: String,
        modifier_id: String,
    },

    /// Clear all modifiers for a stat
    ClearStatModifiers {
        character_id: String,
        stat_name: String,
    },

    /// Clear all modifiers for all stats on a character
    ClearAllModifiers { character_id: String },

    /// Get stat templates (stat definitions from rule system presets)
    GetStatTemplates {
        /// Optional: specific rule system variant (e.g., "dnd5e", "call_of_cthulhu_7e")
        /// If not provided, returns all available templates
        #[serde(default)]
        variant: Option<String>,
    },

    /// Initialize stats for a character from a template
    InitializeFromTemplate {
        character_id: String,
        /// Rule system variant to use (e.g., "dnd5e", "fate_core")
        variant: String,
    },
}

/// Data for adding a modifier to a stat
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddModifierData {
    /// Name of the stat to modify
    pub stat_name: String,
    /// Source of the modifier (e.g., "Sword of Strength", "Bless spell")
    pub source: String,
    /// Value to add (positive) or subtract (negative)
    pub value: i32,
    /// Whether the modifier is active (defaults to true)
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}
