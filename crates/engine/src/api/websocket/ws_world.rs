use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;

use wrldbldr_shared::WorldRequest;

pub(super) async fn handle_world_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: WorldRequest,
) -> Result<ResponseResult, ServerMessage> {
    let _ = conn_info;
    match request {
        WorldRequest::ListWorlds => match state.app.use_cases.management.world.list().await {
            Ok(worlds) => {
                let data: Vec<serde_json::Value> = worlds
                    .into_iter()
                    .map(|w| {
                        serde_json::json!({
                            "id": w.id(),
                            "name": w.name().as_str(),
                            "description": w.description().as_str(),
                        })
                    })
                    .collect();
                Ok(ResponseResult::success(data))
            }
            Err(e) => Ok(ResponseResult::error(
                ErrorCode::InternalError,
                sanitize_repo_error(&e, "list worlds"),
            )),
        },

        WorldRequest::GetWorld { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .world
                .get(world_id_typed)
                .await
            {
                Ok(Some(world)) => Ok(ResponseResult::success(serde_json::json!({
                    "id": world.id(),
                    "name": world.name().as_str(),
                    "description": world.description().as_str(),
                }))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "World not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "retrieve world"),
                )),
            }
        }

        WorldRequest::CreateWorld { data } => {
            // Note: CreateWorld does NOT require DM auth - anyone can create a world.
            // The creator becomes the DM when they join the world.
            match state
                .app
                .use_cases
                .management
                .world
                .create(data.name, data.description, data.setting)
                .await
            {
                Ok(world) => Ok(ResponseResult::success(serde_json::json!({
                    "id": world.id().to_string(),
                    "name": world.name().as_str(),
                    "description": world.description().as_str(),
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create world"),
                )),
            }
        }

        WorldRequest::UpdateWorld { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .world
                .update(world_id_typed, data.name, data.description, data.setting)
                .await
            {
                Ok(world) => Ok(ResponseResult::success(serde_json::json!({
                    "id": world.id().to_string(),
                    "name": world.name().as_str(),
                    "description": world.description().as_str(),
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update world"),
                )),
            }
        }

        WorldRequest::DeleteWorld { world_id } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .world
                .delete(world_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete world"),
                )),
            }
        }

        WorldRequest::ExportWorld { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            require_dm_for_request(conn_info, request_id)?;

            match state
                .app
                .use_cases
                .world
                .export
                .execute(world_id_typed)
                .await
            {
                Ok(export) => Ok(ResponseResult::success(serde_json::json!(export))),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to export world");
                    Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        "Failed to export world",
                    ))
                },
            }
        }

        WorldRequest::GetSheetTemplate { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            // Get the world to determine its rule system
            let world = match state
                .app
                .use_cases
                .management
                .world
                .get(world_id_typed)
                .await
            {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::NotFound,
                        "World not found",
                    ));
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get world");
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        "Failed to get world",
                    ));
                }
            };

            // Get the schema based on the world's rule system
            use wrldbldr_shared::game_systems::{
                BladesSystem, CharacterSheetProvider, Coc7eSystem, Dnd5eSystem, FateCoreSystem,
                PbtaSystem, Pf2eSystem,
            };
            use wrldbldr_shared::RuleSystemVariant;

            let schema = match &world.rule_system().variant {
                RuleSystemVariant::Dnd5e => Some(Dnd5eSystem::new().character_sheet_schema()),
                RuleSystemVariant::Pathfinder2e => Some(Pf2eSystem::new().character_sheet_schema()),
                RuleSystemVariant::CallOfCthulhu7e => {
                    Some(Coc7eSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::FateCore => Some(FateCoreSystem::new().character_sheet_schema()),
                RuleSystemVariant::BladesInTheDark => {
                    Some(BladesSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::PoweredByApocalypse => {
                    Some(PbtaSystem::generic().character_sheet_schema())
                }
                RuleSystemVariant::KidsOnBikes => {
                    Some(PbtaSystem::generic().character_sheet_schema())
                }
                RuleSystemVariant::RuneQuest => Some(Coc7eSystem::new().character_sheet_schema()),
                RuleSystemVariant::GenericD20 | RuleSystemVariant::Custom(_) => {
                    Some(Dnd5eSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::GenericD100 => Some(Coc7eSystem::new().character_sheet_schema()),
                RuleSystemVariant::Unknown => Some(Dnd5eSystem::new().character_sheet_schema()),
            };

            match schema {
                Some(schema) => Ok(ResponseResult::success(
                    serde_json::to_value(&schema).unwrap_or_else(|e| {
                        serde_json::json!({"error": format!("Failed to serialize schema: {}", e)})
                    }),
                )),
                None => Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "No character sheet schema available for this game system",
                )),
            }
        }
    }
}
