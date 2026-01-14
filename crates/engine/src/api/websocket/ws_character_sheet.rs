//! WebSocket handlers for character sheet schema operations.
//!
//! Handles requests for character sheet schemas and character creation flow.

use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use serde_json::json;
use wrldbldr_domain::{CharacterSheetProvider, CharacterSheetSchema, GameSystemRegistry};
use wrldbldr_protocol::{CharacterSheetRequest, ErrorCode, ResponseResult};

// Import all game systems that implement CharacterSheetProvider
use wrldbldr_domain::game_systems::{
    BladesSystem, Coc7eSystem, Dnd5eSystem, FateCoreSystem, PbtaSystem, Pf2eSystem,
};

/// Check if a game system has a character sheet schema implementation.
fn has_schema_for_system(system_id: &str) -> bool {
    matches!(
        system_id,
        "dnd5e" | "pf2e" | "coc7e" | "fate_core" | "blades" | "pbta" | "pbta_aw" | "pbta_dw" | "pbta_motw"
    )
}

/// Get the character sheet schema for a game system.
fn get_schema_for_system(system_id: &str) -> Option<CharacterSheetSchema> {
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
fn get_provider_for_system(system_id: &str) -> Option<Box<dyn CharacterSheetProvider>> {
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
fn variant_to_system_id(variant: &wrldbldr_domain::RuleSystemVariant) -> String {
    use wrldbldr_domain::RuleSystemVariant;
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

pub(super) async fn handle_character_sheet_request(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: CharacterSheetRequest,
) -> Result<ResponseResult, ServerMessage> {
    let registry = GameSystemRegistry::new();

    match request {
        CharacterSheetRequest::GetSchema { system_id } => {
            let _system = match registry.get(&system_id) {
                Some(sys) => sys,
                None => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        format!("Unknown game system: {}", system_id),
                    ));
                }
            };

            // Get the schema from the CharacterSheetProvider trait
            let schema = get_schema_for_system(&system_id);

            match schema {
                Some(schema) => {
                    tracing::debug!(
                        system_id = %system_id,
                        sections = %schema.sections.len(),
                        "Retrieved character sheet schema"
                    );

                    Ok(ResponseResult::success(
                        serde_json::to_value(&schema).unwrap_or_else(|e| {
                            json!({"error": format!("Failed to serialize schema: {}", e)})
                        }),
                    ))
                }
                None => {
                    Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!(
                            "Character sheet schema not available for system: {}",
                            system_id
                        ),
                    ))
                }
            }
        }

        CharacterSheetRequest::ListSystems => {
            let systems: Vec<serde_json::Value> = registry
                .list_systems_with_names()
                .iter()
                .map(|(id, name)| {
                    let sys = registry.get(id);
                    json!({
                        "id": id,
                        "name": name,
                        "has_spellcasting": sys
                            .as_ref()
                            .map(|s| s.spellcasting_system().is_some())
                            .unwrap_or(false),
                        "has_sheet_schema": has_schema_for_system(id),
                    })
                })
                .collect();

            tracing::debug!(
                systems_count = %systems.len(),
                "Listed available game systems"
            );

            Ok(ResponseResult::success(json!({
                "systems": systems
            })))
        }

        CharacterSheetRequest::StartCreation {
            world_id,
            system_id,
            name,
        } => {
            // Verify the system exists
            if registry.get(&system_id).is_none() {
                return Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    format!("Unknown game system: {}", system_id),
                ));
            }

            // Parse world ID
            let world_id_typed = match Uuid::parse_str(&world_id) {
                Ok(id) => wrldbldr_domain::WorldId::from(id),
                Err(_) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Invalid world ID format",
                    ));
                }
            };

            // Verify the world exists
            match state.app.repositories.world.get(world_id_typed).await {
                Ok(Some(_)) => {}
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            }

            // Create a draft character
            let character_name_str = name.unwrap_or_else(|| "New Character".to_string());
            let character_name = match wrldbldr_domain::CharacterName::new(character_name_str) {
                Ok(n) => n,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::ValidationError,
                        format!("Invalid character name: {}", e),
                    ));
                }
            };
            let character = wrldbldr_domain::Character::new(
                world_id_typed,
                character_name,
                wrldbldr_domain::CampbellArchetype::Hero,
            );
            let character_id = character.id();

            // Save the draft character
            if let Err(e) = state.app.repositories.character.save(&character).await {
                return Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create character"),
                ));
            }

            // Get the schema if available
            let schema = get_schema_for_system(&system_id);

            // Get default values from the provider
            let defaults = get_provider_for_system(&system_id)
                .map(|p| p.default_values())
                .unwrap_or_default();

            tracing::info!(
                character_id = %character_id,
                world_id = %world_id,
                system_id = %system_id,
                "Started character creation"
            );

            Ok(ResponseResult::success(json!({
                "character_id": character_id.to_string(),
                "schema": schema,
                "defaults": defaults,
            })))
        }

        CharacterSheetRequest::UpdateCreationField {
            character_id,
            field_id,
            value,
        } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            // Get the system ID from the world's rule system
            let system_id = variant_to_system_id(&world.rule_system.variant);
            let provider = get_provider_for_system(&system_id);

            // Validate the field if we have a provider
            let all_values = get_character_values(&character);
            if let Some(ref p) = provider {
                if let Some(error_msg) = p.validate_field(&field_id, &value, &all_values) {
                    return Ok(ResponseResult::error(ErrorCode::ValidationError, error_msg));
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
            if let Err(e) = state.app.repositories.character.save(&character).await {
                return Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "save character"),
                ));
            }

            tracing::debug!(
                character_id = %character_id,
                field_id = %field_id,
                system_id = %system_id,
                "Updated creation field"
            );

            Ok(ResponseResult::success(json!({
                "field_id": field_id,
                "value": value,
                "calculated": calculated,
            })))
        }

        CharacterSheetRequest::CompleteCreation { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            // Get the system ID from the world's rule system
            let system_id = variant_to_system_id(&world.rule_system.variant);
            let schema = get_schema_for_system(&system_id);

            // Validate required fields
            let values = get_character_values(&character);

            let mut missing_required = Vec::new();
            if let Some(ref schema) = schema {
                for section in &schema.sections {
                    for field in &section.fields {
                        if field.required {
                            if !values.contains_key(&field.id)
                                || values.get(&field.id) == Some(&serde_json::Value::Null)
                            {
                                missing_required.push(field.id.clone());
                            }
                        }
                    }
                }
            }

            if !missing_required.is_empty() {
                return Ok(ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Missing required fields: {}", missing_required.join(", ")),
                ));
            }

            tracing::info!(
                character_id = %character_id,
                name = %character.name(),
                "Completed character creation"
            );

            Ok(ResponseResult::success(json!({
                "character_id": character_id,
                "name": character.name().to_string(),
                "status": "created",
            })))
        }

        CharacterSheetRequest::CancelCreation { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            // Delete the draft character
            if let Err(e) = state
                .app
                .repositories
                .character
                .delete(character_id_typed)
                .await
            {
                return Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete character"),
                ));
            }

            tracing::info!(
                character_id = %character_id,
                "Cancelled character creation"
            );

            Ok(ResponseResult::success(json!({
                "character_id": character_id,
                "status": "cancelled",
            })))
        }

        CharacterSheetRequest::GetSheet { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            let system_id = variant_to_system_id(&world.rule_system.variant);

            // Get schema and calculate derived values
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

            Ok(ResponseResult::success(json!({
                "character_id": character_id,
                "name": character.name().to_string(),
                "schema": schema,
                "values": values,
                "calculated": calculated,
            })))
        }

        CharacterSheetRequest::UpdateField {
            character_id,
            field_id,
            value,
        } => {
            // Same as UpdateCreationField for now
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            // Get the system ID from the world's rule system
            let system_id = variant_to_system_id(&world.rule_system.variant);
            let provider = get_provider_for_system(&system_id);

            // Validate and update
            let all_values = get_character_values(&character);
            if let Some(ref p) = provider {
                if let Some(error_msg) = p.validate_field(&field_id, &value, &all_values) {
                    return Ok(ResponseResult::error(ErrorCode::ValidationError, error_msg));
                }
            }

            update_character_field(&mut character, &field_id, &value);

            let updated_values = get_character_values(&character);
            let calculated = provider
                .as_ref()
                .map(|p| p.calculate_derived_values(&updated_values))
                .unwrap_or_default();

            for (field, val) in &calculated {
                update_character_field(&mut character, field, val);
            }

            if let Err(e) = state.app.repositories.character.save(&character).await {
                return Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "save character"),
                ));
            }

            tracing::debug!(
                character_id = %character_id,
                field_id = %field_id,
                "Updated character field"
            );

            Ok(ResponseResult::success(json!({
                "field_id": field_id,
                "value": value,
                "calculated": calculated,
            })))
        }

        CharacterSheetRequest::UpdateFields {
            character_id,
            updates,
        } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            // Get the system ID from the world's rule system
            let system_id = variant_to_system_id(&world.rule_system.variant);
            let provider = get_provider_for_system(&system_id);

            // Validate all fields first
            let all_values = get_character_values(&character);
            if let Some(ref p) = provider {
                for update in &updates {
                    if let Some(error_msg) =
                        p.validate_field(&update.field_id, &update.value, &all_values)
                    {
                        return Ok(ResponseResult::error(
                            ErrorCode::ValidationError,
                            format!("{}: {}", update.field_id, error_msg),
                        ));
                    }
                }
            }

            // Apply all updates
            for update in &updates {
                update_character_field(&mut character, &update.field_id, &update.value);
            }

            // Recalculate
            let updated_values = get_character_values(&character);
            let calculated = provider
                .as_ref()
                .map(|p| p.calculate_derived_values(&updated_values))
                .unwrap_or_default();

            for (field, val) in &calculated {
                update_character_field(&mut character, field, val);
            }

            if let Err(e) = state.app.repositories.character.save(&character).await {
                return Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "save character"),
                ));
            }

            tracing::debug!(
                character_id = %character_id,
                fields_updated = %updates.len(),
                "Updated multiple character fields"
            );

            Ok(ResponseResult::success(json!({
                "updated": updates.len(),
                "calculated": calculated,
            })))
        }

        CharacterSheetRequest::GetCalculatedValues { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            // Get the system ID from the world's rule system
            let system_id = variant_to_system_id(&world.rule_system.variant);
            let values = get_character_values(&character);
            let calculated = get_provider_for_system(&system_id)
                .map(|p| p.calculate_derived_values(&values))
                .unwrap_or_default();

            Ok(ResponseResult::success(json!({
                "calculated": calculated,
            })))
        }

        CharacterSheetRequest::RecalculateAll { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match state
                .app
                .repositories
                .character
                .get(character_id_typed)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "Character not found",
                    ));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get character"),
                    ));
                }
            };

            // Get the world to determine the system
            let world = match state.app.repositories.world.get(character.world_id()).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get world"),
                    ));
                }
            };

            // Get the system ID from the world's rule system
            let system_id = variant_to_system_id(&world.rule_system.variant);
            let values = get_character_values(&character);
            let calculated = get_provider_for_system(&system_id)
                .map(|p| p.calculate_derived_values(&values))
                .unwrap_or_default();

            // Apply calculated values
            for (field, val) in &calculated {
                update_character_field(&mut character, field, val);
            }

            if let Err(e) = state.app.repositories.character.save(&character).await {
                return Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "save character"),
                ));
            }

            tracing::debug!(
                character_id = %character_id,
                system_id = %system_id,
                "Recalculated all derived values"
            );

            Ok(ResponseResult::success(json!({
                "calculated": calculated,
            })))
        }
    }
}

/// Extract character values into a HashMap for schema operations.
fn get_character_values(
    character: &wrldbldr_domain::Character,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut values = std::collections::HashMap::new();

    // Add character name
    values.insert("NAME".to_string(), json!(character.name().to_string()));

    // Add stats
    for (name, stat) in character.stats().get_all_stats() {
        values.insert(name.to_string(), json!(stat.effective));
    }

    values
}

/// Update a character field based on field ID.
fn update_character_field(
    character: &mut wrldbldr_domain::Character,
    field_id: &str,
    value: &serde_json::Value,
) {
    match field_id {
        "NAME" => {
            if let Some(name) = value.as_str() {
                if let Ok(char_name) = wrldbldr_domain::CharacterName::new(name) {
                    character.set_name(char_name);
                }
            }
        }
        // Stats
        "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA" | "LEVEL" | "CURRENT_HP" | "MAX_HP"
        | "TEMP_HP" | "AC" | "SPEED" => {
            if let Some(val) = value.as_i64() {
                character.stats_mut().set_stat(field_id, val as i32);
            }
        }
        // Derived/calculated stats
        "PROF_BONUS" | "INITIATIVE" | "PASSIVE_PERCEPTION" => {
            if let Some(val) = value.as_i64() {
                character.stats_mut().set_stat(field_id, val as i32);
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
                character.stats_mut().set_stat(field_id, prof_value);
            }
        }
        // Saving throw proficiencies
        field if field.ends_with("_SAVE_PROF") => {
            if let Some(val) = value.as_bool() {
                character.stats_mut().set_stat(field_id, if val { 1 } else { 0 });
            }
        }
        // Saving throw modifiers (calculated)
        field if field.ends_with("_SAVE") => {
            if let Some(val) = value.as_i64() {
                character.stats_mut().set_stat(field_id, val as i32);
            }
        }
        // Skill modifiers (calculated)
        field if field.ends_with("_MOD") => {
            if let Some(val) = value.as_i64() {
                character.stats_mut().set_stat(field_id, val as i32);
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
                if let Ok(new_desc) = wrldbldr_domain::Description::new(&desc) {
                    character.set_description(new_desc);
                }
            }
        }
        _ => {
            tracing::debug!(field_id = %field_id, "Unknown field, storing as stat if numeric");
            if let Some(val) = value.as_i64() {
                character.stats_mut().set_stat(field_id, val as i32);
            }
        }
    }
}
