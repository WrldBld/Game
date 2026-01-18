//! Character sheet use cases.
//!
//! Handles character sheet operations including creation, field updates,
//! validation, and derived value calculation.

mod error;

pub use error::CharacterSheetError;

use std::collections::HashMap;
use std::sync::Arc;

use wrldbldr_domain::{
    CampbellArchetype, Character, CharacterId, CharacterName, Description, RuleSystemVariant,
    WorldId,
};
use wrldbldr_shared::character_sheet::{CharacterSheetSchema, SheetValue};
use wrldbldr_shared::game_systems::{
    BladesSystem, CharacterSheetProvider, Coc7eSystem, Dnd5eSystem, FateCoreSystem,
    GameSystemRegistry, PbtaSystem, Pf2eSystem,
};

use crate::infrastructure::ports::{CharacterRepo, WorldRepo};

// =============================================================================
// Result Types
// =============================================================================

/// Result of starting character creation.
#[derive(Debug, Clone)]
pub struct StartCreationResult {
    /// The created draft character ID.
    pub character_id: CharacterId,
    /// The character sheet schema for this system.
    pub schema: Option<CharacterSheetSchema>,
    /// Default values for the character sheet.
    pub defaults: HashMap<String, SheetValue>,
}

/// Result of updating a field.
#[derive(Debug, Clone)]
pub struct UpdateFieldResult {
    /// The field that was updated.
    pub field_id: String,
    /// The new value.
    pub value: SheetValue,
    /// Calculated/derived values that were recalculated.
    pub calculated: HashMap<String, SheetValue>,
}

/// Result of updating multiple fields.
#[derive(Debug, Clone)]
pub struct UpdateFieldsResult {
    /// Number of fields updated.
    pub updated_count: usize,
    /// Calculated/derived values that were recalculated.
    pub calculated: HashMap<String, SheetValue>,
}

/// Result of completing character creation.
#[derive(Debug, Clone)]
pub struct CompleteCreationResult {
    /// The character ID.
    pub character_id: CharacterId,
    /// The character name.
    pub name: String,
}

/// Result of getting a character sheet.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GetSheetResult {
    /// The character.
    pub character: Character,
    /// The system ID.
    pub system_id: String,
    /// The character sheet schema.
    pub schema: Option<CharacterSheetSchema>,
    /// Current values.
    pub values: HashMap<String, SheetValue>,
    /// Calculated/derived values.
    pub calculated: HashMap<String, SheetValue>,
}

/// Result of recalculating all derived values.
#[derive(Debug, Clone)]
pub struct RecalculateResult {
    /// All calculated/derived values.
    pub calculated: HashMap<String, SheetValue>,
}

/// A field update request.
#[derive(Debug, Clone)]
pub struct FieldUpdate {
    pub field_id: String,
    pub value: SheetValue,
}

// =============================================================================
// Use Cases
// =============================================================================

/// Container for character sheet use cases.
pub struct CharacterSheetUseCases {
    character_repo: Arc<dyn CharacterRepo>,
    world_repo: Arc<dyn WorldRepo>,
}

impl CharacterSheetUseCases {
    pub fn new(character_repo: Arc<dyn CharacterRepo>, world_repo: Arc<dyn WorldRepo>) -> Self {
        Self {
            character_repo,
            world_repo,
        }
    }

    /// Start character creation for a world.
    ///
    /// Creates a draft character and returns the schema and default values.
    pub async fn start_creation(
        &self,
        world_id: WorldId,
        system_id: &str,
        name: Option<String>,
    ) -> Result<StartCreationResult, CharacterSheetError> {
        // Verify the system exists
        let registry = GameSystemRegistry::new();
        if registry.get(system_id).is_none() {
            return Err(CharacterSheetError::GameSystemNotFound(
                system_id.to_string(),
            ));
        }

        // Verify the world exists
        self.world_repo
            .get(world_id)
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(world_id))?;

        // Create a draft character
        let character_name_str = name.unwrap_or_else(|| "New Character".to_string());
        let character_name = CharacterName::new(character_name_str)?;
        let character = Character::new(world_id, character_name, CampbellArchetype::Hero);
        let character_id = character.id();

        // Save the draft character
        self.character_repo.save(&character).await?;

        // Get the schema and defaults
        let schema = get_schema_for_system(system_id);
        let defaults = get_provider_for_system(system_id)
            .map(|p| p.default_values())
            .unwrap_or_default();

        tracing::info!(
            character_id = %character_id,
            world_id = %world_id,
            system_id = %system_id,
            "Started character creation"
        );

        Ok(StartCreationResult {
            character_id,
            schema,
            defaults,
        })
    }

    /// Update a single field on a character sheet.
    ///
    /// Validates the field, updates the character, and recalculates derived values.
    pub async fn update_field(
        &self,
        character_id: CharacterId,
        field_id: String,
        value: SheetValue,
    ) -> Result<UpdateFieldResult, CharacterSheetError> {
        let mut character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or(CharacterSheetError::CharacterNotFound(character_id))?;

        // Get the world to determine the system
        let world = self
            .world_repo
            .get(character.world_id())
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(character.world_id()))?;

        let system_id = variant_to_system_id(&world.rule_system().variant);
        let provider = get_provider_for_system(&system_id);

        // Validate the field if we have a provider
        let all_values = get_character_values(&character);
        if let Some(ref p) = provider {
            if let Some(error_msg) = p.validate_field(&field_id, &value, &all_values) {
                return Err(CharacterSheetError::FieldValidation {
                    field_id: field_id.clone(),
                    message: error_msg,
                });
            }
        }

        // Update the field
        update_character_field(&mut character, &field_id, &value);

        // Recalculate derived values
        let updated_values = get_character_values(&character);
        let calculated = provider
            .as_ref()
            .map(|p| p.calculate_derived_values(&updated_values))
            .unwrap_or_default();

        // Apply calculated values back to the character
        for (field, val) in &calculated {
            update_character_field(&mut character, field, val);
        }

        // Save the character
        self.character_repo.save(&character).await?;

        tracing::debug!(
            character_id = %character_id,
            field_id = %field_id,
            system_id = %system_id,
            "Updated character field"
        );

        Ok(UpdateFieldResult {
            field_id,
            value,
            calculated,
        })
    }

    /// Update multiple fields on a character sheet in a batch.
    ///
    /// Validates all fields first, then applies updates and recalculates.
    pub async fn update_fields(
        &self,
        character_id: CharacterId,
        updates: Vec<FieldUpdate>,
    ) -> Result<UpdateFieldsResult, CharacterSheetError> {
        let mut character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or(CharacterSheetError::CharacterNotFound(character_id))?;

        // Get the world to determine the system
        let world = self
            .world_repo
            .get(character.world_id())
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(character.world_id()))?;

        let system_id = variant_to_system_id(&world.rule_system().variant);
        let provider = get_provider_for_system(&system_id);

        // Validate all fields first
        let all_values = get_character_values(&character);
        if let Some(ref p) = provider {
            for update in &updates {
                if let Some(error_msg) =
                    p.validate_field(&update.field_id, &update.value, &all_values)
                {
                    return Err(CharacterSheetError::FieldValidation {
                        field_id: update.field_id.clone(),
                        message: error_msg,
                    });
                }
            }
        }

        // Apply all updates
        let updated_count = updates.len();
        for update in &updates {
            update_character_field(&mut character, &update.field_id, &update.value);
        }

        // Recalculate derived values
        let updated_values = get_character_values(&character);
        let calculated = provider
            .as_ref()
            .map(|p| p.calculate_derived_values(&updated_values))
            .unwrap_or_default();

        // Apply calculated values back to the character
        for (field, val) in &calculated {
            update_character_field(&mut character, field, val);
        }

        // Save the character
        self.character_repo.save(&character).await?;

        tracing::debug!(
            character_id = %character_id,
            fields_updated = %updated_count,
            system_id = %system_id,
            "Updated multiple character fields"
        );

        Ok(UpdateFieldsResult {
            updated_count,
            calculated,
        })
    }

    /// Complete character creation.
    ///
    /// Validates that all required fields are present.
    pub async fn complete_creation(
        &self,
        character_id: CharacterId,
    ) -> Result<CompleteCreationResult, CharacterSheetError> {
        let character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or(CharacterSheetError::CharacterNotFound(character_id))?;

        // Get the world to determine the system
        let world = self
            .world_repo
            .get(character.world_id())
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(character.world_id()))?;

        let system_id = variant_to_system_id(&world.rule_system().variant);
        let schema = get_schema_for_system(&system_id);

        // Validate required fields
        let values = get_character_values(&character);
        let mut missing_required = Vec::new();

        if let Some(ref schema) = schema {
            for section in &schema.sections {
                for field in &section.fields {
                    if field.required
                        && (!values.contains_key(&field.id)
                            || values.get(&field.id) == Some(&SheetValue::Null))
                    {
                        missing_required.push(field.id.clone());
                    }
                }
            }
        }

        if !missing_required.is_empty() {
            return Err(CharacterSheetError::MissingRequiredFields(
                missing_required.join(", "),
            ));
        }

        tracing::info!(
            character_id = %character_id,
            name = %character.name(),
            "Completed character creation"
        );

        Ok(CompleteCreationResult {
            character_id,
            name: character.name().to_string(),
        })
    }

    /// Get the full character sheet including schema, values, and calculated values.
    pub async fn get_sheet(
        &self,
        character_id: CharacterId,
    ) -> Result<GetSheetResult, CharacterSheetError> {
        let character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or(CharacterSheetError::CharacterNotFound(character_id))?;

        // Get the world to determine the system
        let world = self
            .world_repo
            .get(character.world_id())
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(character.world_id()))?;

        let system_id = variant_to_system_id(&world.rule_system().variant);
        let schema = get_schema_for_system(&system_id);
        let values = get_character_values(&character);
        let calculated = get_provider_for_system(&system_id)
            .map(|p| p.calculate_derived_values(&values))
            .unwrap_or_default();

        tracing::debug!(
            character_id = %character_id,
            system_id = %system_id,
            "Retrieved character sheet"
        );

        Ok(GetSheetResult {
            character,
            system_id,
            schema,
            values,
            calculated,
        })
    }

    /// Recalculate all derived values for a character and save.
    pub async fn recalculate_all(
        &self,
        character_id: CharacterId,
    ) -> Result<RecalculateResult, CharacterSheetError> {
        let mut character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or(CharacterSheetError::CharacterNotFound(character_id))?;

        // Get the world to determine the system
        let world = self
            .world_repo
            .get(character.world_id())
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(character.world_id()))?;

        let system_id = variant_to_system_id(&world.rule_system().variant);
        let values = get_character_values(&character);
        let calculated = get_provider_for_system(&system_id)
            .map(|p| p.calculate_derived_values(&values))
            .unwrap_or_default();

        // Apply calculated values to character
        for (field, val) in &calculated {
            update_character_field(&mut character, field, val);
        }

        // Save the character
        self.character_repo.save(&character).await?;

        tracing::debug!(
            character_id = %character_id,
            system_id = %system_id,
            "Recalculated all derived values"
        );

        Ok(RecalculateResult { calculated })
    }

    /// Get calculated values without saving (read-only).
    pub async fn get_calculated_values(
        &self,
        character_id: CharacterId,
    ) -> Result<HashMap<String, SheetValue>, CharacterSheetError> {
        let character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or(CharacterSheetError::CharacterNotFound(character_id))?;

        // Get the world to determine the system
        let world = self
            .world_repo
            .get(character.world_id())
            .await?
            .ok_or(CharacterSheetError::WorldNotFound(character.world_id()))?;

        let system_id = variant_to_system_id(&world.rule_system().variant);
        let values = get_character_values(&character);
        let calculated = get_provider_for_system(&system_id)
            .map(|p| p.calculate_derived_values(&values))
            .unwrap_or_default();

        Ok(calculated)
    }

    /// Cancel character creation by deleting the draft character.
    pub async fn cancel_creation(
        &self,
        character_id: CharacterId,
    ) -> Result<(), CharacterSheetError> {
        self.character_repo.delete(character_id).await?;

        tracing::info!(
            character_id = %character_id,
            "Cancelled character creation"
        );

        Ok(())
    }
}

// =============================================================================
// Helper Functions (extracted from ws_character_sheet.rs)
// =============================================================================

/// Check if a game system has a character sheet schema implementation.
pub fn has_schema_for_system(system_id: &str) -> bool {
    matches!(
        system_id,
        "dnd5e"
            | "pf2e"
            | "coc7e"
            | "fate_core"
            | "blades"
            | "pbta"
            | "pbta_aw"
            | "pbta_dw"
            | "pbta_motw"
    )
}

/// Get the character sheet schema for a game system.
pub fn get_schema_for_system(system_id: &str) -> Option<CharacterSheetSchema> {
    match system_id {
        "dnd5e" => Some(Dnd5eSystem::new().character_sheet_schema()),
        "pf2e" => Some(Pf2eSystem::new().character_sheet_schema()),
        "coc7e" => Some(Coc7eSystem::new().character_sheet_schema()),
        "fate_core" => Some(FateCoreSystem::new().character_sheet_schema()),
        "blades" => Some(BladesSystem::new().character_sheet_schema()),
        "pbta" => Some(PbtaSystem::generic().character_sheet_schema()),
        "pbta_aw" => Some(PbtaSystem::apocalypse_world().character_sheet_schema()),
        "pbta_dw" => Some(PbtaSystem::dungeon_world().character_sheet_schema()),
        "pbta_motw" => Some(PbtaSystem::monster_of_the_week().character_sheet_schema()),
        _ => None,
    }
}

/// Get a CharacterSheetProvider for calculating derived values and validation.
pub fn get_provider_for_system(system_id: &str) -> Option<Box<dyn CharacterSheetProvider>> {
    match system_id {
        "dnd5e" => Some(Box::new(Dnd5eSystem::new())),
        "pf2e" => Some(Box::new(Pf2eSystem::new())),
        "coc7e" => Some(Box::new(Coc7eSystem::new())),
        "fate_core" => Some(Box::new(FateCoreSystem::new())),
        "blades" => Some(Box::new(BladesSystem::new())),
        "pbta" => Some(Box::new(PbtaSystem::generic())),
        "pbta_aw" => Some(Box::new(PbtaSystem::apocalypse_world())),
        "pbta_dw" => Some(Box::new(PbtaSystem::dungeon_world())),
        "pbta_motw" => Some(Box::new(PbtaSystem::monster_of_the_week())),
        _ => None,
    }
}

/// Convert a RuleSystemVariant to the corresponding system ID string.
pub fn variant_to_system_id(variant: &RuleSystemVariant) -> String {
    match variant {
        RuleSystemVariant::Dnd5e => "dnd5e".to_string(),
        RuleSystemVariant::Pathfinder2e => "pf2e".to_string(),
        RuleSystemVariant::CallOfCthulhu7e => "coc7e".to_string(),
        RuleSystemVariant::FateCore => "fate_core".to_string(),
        RuleSystemVariant::BladesInTheDark => "blades".to_string(),
        RuleSystemVariant::PoweredByApocalypse => "pbta".to_string(),
        RuleSystemVariant::KidsOnBikes => "pbta".to_string(), // Use generic PbtA
        RuleSystemVariant::RuneQuest => "coc7e".to_string(),  // Similar to CoC (percentile)
        RuleSystemVariant::GenericD20 => "dnd5e".to_string(), // Closest to D&D
        RuleSystemVariant::GenericD100 => "coc7e".to_string(), // Percentile system
        RuleSystemVariant::Custom(_) => "dnd5e".to_string(),  // Default to D&D for custom systems
        RuleSystemVariant::Unknown => "dnd5e".to_string(),    // Default to D&D for unknown
    }
}

/// Extract character values into a HashMap for schema operations.
pub fn get_character_values(character: &Character) -> HashMap<String, SheetValue> {
    let mut values = HashMap::new();

    // Add character name
    values.insert(
        "NAME".to_string(),
        SheetValue::String(character.name().to_string()),
    );

    // Add stats
    for (name, stat) in character.stats().get_all_stats() {
        values.insert(name.to_string(), SheetValue::Integer(stat.effective()));
    }

    values
}

/// Update a character field based on field ID.
pub fn update_character_field(character: &mut Character, field_id: &str, value: &SheetValue) {
    match field_id {
        "NAME" => {
            if let Some(name) = value.as_str() {
                if let Ok(char_name) = CharacterName::new(name) {
                    character.set_name(char_name);
                }
            }
        }
        // Stats
        "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA" | "LEVEL" | "CURRENT_HP" | "MAX_HP"
        | "TEMP_HP" | "AC" | "SPEED" => {
            if let Some(val) = value.as_i64() {
                character.set_stats(character.stats().clone().with_stat(field_id, val as i32));
            }
        }
        // Derived/calculated stats
        "PROF_BONUS" | "INITIATIVE" | "PASSIVE_PERCEPTION" => {
            if let Some(val) = value.as_i64() {
                character.set_stats(character.stats().clone().with_stat(field_id, val as i32));
            }
        }
        // Skill proficiencies
        field if field.ends_with("_PROF") => {
            if let Some(val) = value.as_str() {
                // Store as a stat for simplicity (could use a separate map)
                let prof_value = match val {
                    "expert" => 2,
                    "proficient" => 1,
                    "half" => -1, // Use negative as flag for half
                    _ => 0,
                };
                character.set_stats(character.stats().clone().with_stat(field_id, prof_value));
            }
        }
        // Saving throw proficiencies
        field if field.ends_with("_SAVE_PROF") => {
            if let Some(val) = value.as_bool() {
                character.set_stats(
                    character
                        .stats()
                        .clone()
                        .with_stat(field_id, if val { 1 } else { 0 }),
                );
            }
        }
        // Saving throw modifiers (calculated)
        field if field.ends_with("_SAVE") => {
            if let Some(val) = value.as_i64() {
                character.set_stats(character.stats().clone().with_stat(field_id, val as i32));
            }
        }
        // Skill modifiers (calculated)
        field if field.ends_with("_MOD") => {
            if let Some(val) = value.as_i64() {
                character.set_stats(character.stats().clone().with_stat(field_id, val as i32));
            }
        }
        // Identity fields (CLASS, RACE, BACKGROUND)
        "CLASS" | "RACE" | "BACKGROUND" => {
            // These would go in CharacterIdentity when we implement it fully
            // For now, store as a stat for simplicity
            if let Some(val) = value.as_str() {
                // Can't store strings directly in stats, so we'll need to extend the model
                // For now, log it
                tracing::debug!(field_id = %field_id, value = %val, "Identity field set (not yet persisted)");
            }
        }
        // Text fields (store in description for now)
        "FEATURES" => {
            if let Some(text) = value.as_str() {
                // Append to description for now until we have a proper features field
                let mut desc = character.description().to_string();
                if !desc.is_empty() {
                    desc.push_str("\n\nFeatures:\n");
                }
                desc.push_str(text);
                if let Ok(new_desc) = Description::new(&desc) {
                    character.set_description(new_desc);
                }
            }
        }
        _ => {
            tracing::debug!(field_id = %field_id, "Unknown field, storing as stat if numeric");
            if let Some(val) = value.as_i64() {
                character.set_stats(character.stats().clone().with_stat(field_id, val as i32));
            }
        }
    }
}
