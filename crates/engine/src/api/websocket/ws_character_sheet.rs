//! WebSocket handlers for character sheet schema operations.
//!
//! Handles requests for character sheet schemas and character creation flow.

use super::*;

use crate::api::connections::ConnectionInfo;
use crate::infrastructure::ports::RepoError;
use crate::use_cases::character_sheet::{CharacterSheetError, FieldUpdate};
use serde_json::json;
use wrldbldr_domain::types::EntityType;
use wrldbldr_shared::game_systems::GameSystemRegistry;
use wrldbldr_shared::{CharacterSheetRequest, ErrorCode, EntityChangedData, ResponseResult, ServerMessage};

// Import helper functions from use case module for pure operations
use crate::use_cases::character_sheet::{get_schema_for_system, has_schema_for_system};

pub(super) async fn handle_character_sheet_request(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: CharacterSheetRequest,
) -> Result<ResponseResult, ServerMessage> {
    let registry = GameSystemRegistry::new();

    match request {
        // Pure functions - no repo access, keep in handler
        CharacterSheetRequest::GetSchema { system_id } => {
            if registry.get(&system_id).is_none() {
                return Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    format!("Unknown game system: {}", system_id),
                ));
            }

            match get_schema_for_system(&system_id) {
                Some(schema) => {
                    tracing::debug!(
                        system_id = %system_id,
                        sections = %schema.sections.len(),
                        "Retrieved character sheet schema"
                    );

                    Ok(ResponseResult::success(
                        serde_json::to_value(&schema).unwrap_or_else(
                            |e| json!({"error": format!("Failed to serialize schema: {}", e)}),
                        ),
                    ))
                }
                None => Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    format!(
                        "Character sheet schema not available for system: {}",
                        system_id
                    ),
                )),
            }
        }

        // Pure function - no repo access, keep in handler
        CharacterSheetRequest::ListSystems => {
            let systems: Vec<serde_json::Value> = registry
                .list_systems_with_names()
                .iter()
                .map(|(id, name)| {
                    json!({
                        "id": id,
                        "name": name,
                        "has_spellcasting": false,
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
            let world_id_typed = match Uuid::parse_str(&world_id) {
                Ok(id) => wrldbldr_domain::WorldId::from(id),
                Err(_) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Invalid world ID format",
                    ));
                }
            };

            match state
                .app
                .use_cases
                .character_sheet
                .start_creation(world_id_typed, &system_id, name)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(json!({
                    "character_id": result.character_id.to_string(),
                    "schema": result.schema,
                    "defaults": result.defaults,
                }))),
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::UpdateCreationField {
            character_id,
            field_id,
            value,
        } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .update_field(character_id_typed, field_id.clone(), value.clone())
                .await
            {
                Ok(result) => Ok(ResponseResult::success(json!({
                    "field_id": result.field_id,
                    "value": result.value,
                    "calculated": result.calculated,
                }))),
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::UpdateField {
            character_id,
            field_id,
            value,
        } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .update_field(character_id_typed, field_id.clone(), value.clone())
                .await
            {
                Ok(result) => {
                    // Resolve world_id from character_id
                    let world_id = match get_character_world_id(&state, character_id_typed).await {
                        Ok(wid) => wid,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to resolve world_id for character");
                            return Ok(map_character_sheet_error(CharacterSheetError::CharacterNotFound(
                                character_id_typed,
                            )));
                        }
                    };

                    // Emit EntityChanged for field update
                    let entity_data = json!({
                        "character_id": character_id,
                        "field_id": result.field_id,
                        "value": result.value,
                        "calculated": result.calculated,
                    });

                    let entity_changed_msg = ServerMessage::EntityChanged(
                        EntityChangedData::updated(
                            EntityType::Character,
                            character_id.clone(),
                            world_id.to_string(),
                            &entity_data,
                        )
                    );
                    state.connections.broadcast_to_world(world_id, entity_changed_msg).await;

                    Ok(ResponseResult::success(json!({
                        "field_id": result.field_id,
                        "value": result.value,
                        "calculated": result.calculated,
                    })))
                },
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::UpdateFields {
            character_id,
            updates,
        } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            // Convert from wire format to use case format
            let field_updates: Vec<FieldUpdate> = updates
                .into_iter()
                .map(|u| FieldUpdate {
                    field_id: u.field_id,
                    value: u.value,
                })
                .collect();

            match state
                .app
                .use_cases
                .character_sheet
                .update_fields(character_id_typed, field_updates)
                .await
            {
                Ok(result) => {
                    // Resolve world_id from character_id
                    let world_id = match get_character_world_id(&state, character_id_typed).await {
                        Ok(wid) => wid,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to resolve world_id for character");
                            return Ok(map_character_sheet_error(CharacterSheetError::CharacterNotFound(
                                character_id_typed,
                            )));
                        }
                    };

                    // Emit EntityChanged for fields update
                    let entity_data = json!({
                        "character_id": character_id,
                        "updated_count": result.updated_count,
                        "calculated": result.calculated,
                    });

                    let entity_changed_msg = ServerMessage::EntityChanged(
                        EntityChangedData::updated(
                            EntityType::Character,
                            character_id.clone(),
                            world_id.to_string(),
                            &entity_data,
                        )
                    );
                    state.connections.broadcast_to_world(world_id, entity_changed_msg).await;

                    Ok(ResponseResult::success(json!({
                        "updated": result.updated_count,
                        "calculated": result.calculated,
                    })))
                },
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::CompleteCreation { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .complete_creation(character_id_typed)
                .await
            {
                Ok(result) => {
                    // Emit EntityChanged for character sheet creation
                    // Note: We can't easily get world_id here without loading character,
                    // so we'll rely on the use case to handle broadcasts or
                    // create a separate endpoint for this
                    Ok(ResponseResult::success(json!({
                        "character_id": result.character_id.to_string(),
                        "name": result.name,
                        "status": "created",
                    })))
                },
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::GetSheet { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .get_sheet(character_id_typed)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(json!({
                    "character_id": character_id,
                    "name": result.character.name().to_string(),
                    "schema": result.schema,
                    "values": result.values,
                    "calculated": result.calculated,
                }))),
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::RecalculateAll { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .recalculate_all(character_id_typed)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(json!({
                    "calculated": result.calculated,
                }))),
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::GetCalculatedValues { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .get_calculated_values(character_id_typed)
                .await
            {
                Ok(calculated) => Ok(ResponseResult::success(json!({
                    "calculated": calculated,
                }))),
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }

        CharacterSheetRequest::CancelCreation { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state
                .app
                .use_cases
                .character_sheet
                .cancel_creation(character_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(json!({
                    "character_id": character_id,
                    "status": "cancelled",
                }))),
                Err(e) => Ok(map_character_sheet_error(e)),
            }
        }
    }
}

/// Map CharacterSheetError to ResponseResult with appropriate ErrorCode.
fn map_character_sheet_error(err: CharacterSheetError) -> ResponseResult {
    let (code, message) = match &err {
        CharacterSheetError::CharacterNotFound(_) => (ErrorCode::NotFound, err.to_string()),
        CharacterSheetError::WorldNotFound(_) => (ErrorCode::NotFound, err.to_string()),
        CharacterSheetError::GameSystemNotFound(_) => (ErrorCode::NotFound, err.to_string()),
        CharacterSheetError::SchemaNotAvailable(_) => (ErrorCode::BadRequest, err.to_string()),
        CharacterSheetError::FieldValidation { .. } => {
            (ErrorCode::ValidationError, err.to_string())
        }
        CharacterSheetError::MissingRequiredFields(_) => {
            (ErrorCode::ValidationError, err.to_string())
        }
        CharacterSheetError::InvalidCharacterId => (ErrorCode::BadRequest, err.to_string()),
        CharacterSheetError::InvalidWorldId => (ErrorCode::BadRequest, err.to_string()),
        CharacterSheetError::Domain(_) => (ErrorCode::ValidationError, err.to_string()),
        CharacterSheetError::Repo(_) => {
            // Don't expose internal repo errors to clients
            (
                ErrorCode::InternalError,
                "An internal error occurred".to_string(),
            )
        }
    };

    ResponseResult::error(code, message)
}

/// Helper function to resolve world_id from character_id.
/// Fetches the character from the CharacterRepo and returns its world_id.
async fn get_character_world_id(
    state: &WsState,
    character_id: wrldbldr_domain::CharacterId,
) -> Result<wrldbldr_domain::WorldId, CharacterSheetError> {
    state
        .app
        .repositories
        .character
        .get(character_id)
        .await
        .map_err(|e| CharacterSheetError::Repo(RepoError::database("get_character", e)))?
        .map(|character| character.world_id())
        .ok_or(CharacterSheetError::CharacterNotFound(character_id))
}
