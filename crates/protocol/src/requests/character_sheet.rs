//! Character Sheet Request Types
//!
//! Requests for character creation, sheet retrieval, and field updates.

use serde::{Deserialize, Serialize};

/// Character sheet operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CharacterSheetRequest {
    // =========================================================================
    // Schema Operations
    // =========================================================================
    /// Get the character sheet schema for a game system.
    ///
    /// Returns the schema describing all fields, sections, and creation steps.
    GetSchema {
        /// Game system ID (e.g., "dnd5e", "pf2e", "blades")
        system_id: String,
    },

    /// List all available game systems.
    ///
    /// Returns a list of system IDs and display names.
    ListSystems,

    // =========================================================================
    // Character Creation
    // =========================================================================
    /// Start a new character creation session.
    ///
    /// Returns a character ID and the creation schema.
    StartCreation {
        /// World the character will belong to
        world_id: String,
        /// Game system ID
        system_id: String,
        /// Optional character name (can be set later)
        #[serde(default)]
        name: Option<String>,
    },

    /// Update a field during character creation.
    ///
    /// Validates the value and returns any calculated field updates.
    UpdateCreationField {
        /// Character being created
        character_id: String,
        /// Field to update
        field_id: String,
        /// New value
        value: serde_json::Value,
    },

    /// Complete character creation.
    ///
    /// Validates all required fields are set and finalizes the character.
    CompleteCreation {
        /// Character being created
        character_id: String,
    },

    /// Cancel character creation.
    ///
    /// Removes the draft character.
    CancelCreation {
        /// Character being created
        character_id: String,
    },

    // =========================================================================
    // Character Sheet Operations
    // =========================================================================
    /// Get the full character sheet with schema and values.
    ///
    /// Returns the schema, current values, and calculated values.
    GetSheet {
        /// Character ID
        character_id: String,
    },

    /// Update a field on an existing character.
    ///
    /// Validates and persists the change, returns calculated field updates.
    UpdateField {
        /// Character ID
        character_id: String,
        /// Field to update
        field_id: String,
        /// New value
        value: serde_json::Value,
    },

    /// Update multiple fields atomically.
    ///
    /// All fields are validated and updated together.
    UpdateFields {
        /// Character ID
        character_id: String,
        /// Fields to update
        updates: Vec<FieldUpdateData>,
    },

    /// Get only calculated values for a character.
    ///
    /// Useful after a batch update to refresh all derived values.
    GetCalculatedValues {
        /// Character ID
        character_id: String,
    },

    /// Recalculate all derived fields for a character.
    ///
    /// Forces recalculation of all derived values.
    RecalculateAll {
        /// Character ID
        character_id: String,
    },

    /// Unknown request type for forward compatibility.
    ///
    /// If a new request type is added and an older client/server doesn't recognize it,
    /// this variant will be used instead of failing to deserialize.
    #[serde(other)]
    Unknown,
}

/// Data for a single field update.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldUpdateData {
    /// Field ID
    pub field_id: String,
    /// New value
    pub value: serde_json::Value,
}

/// Response for system list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSystemInfo {
    /// System ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Short description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether this system supports spellcasting
    #[serde(default)]
    pub has_spellcasting: bool,
}
