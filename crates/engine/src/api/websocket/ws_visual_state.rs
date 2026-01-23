use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::visual_state::VisualStateDetails;
use wrldbldr_shared::{
    requests::visual_state::{
        LocationStateData, RegionStateData, VisualStateCatalogData,
    },
    ErrorCode, ResponseResult, ServerMessage, VisualStateRequest,
};

/// Maximum string lengths for visual state fields.
const MAX_STATE_NAME: usize = 200;
const MAX_STATE_DESCRIPTION: usize = 5000;
const MAX_PROMPT_LENGTH: usize = 5000;
const MAX_WORKFLOW_LENGTH: usize = 100;
const MAX_NEGATIVE_PROMPT_LENGTH: usize = 2000;
const MAX_TAGS_COUNT: usize = 20;
const MAX_TAG_LENGTH: usize = 50;
const MAX_ASSET_PATH_LENGTH: usize = 500;

pub(super) async fn handle_visual_state_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: VisualStateRequest,
) -> Result<ResponseResult, ServerMessage> {
    let msg = match request {
        VisualStateRequest::GetCatalog { request } => {
            handle_get_catalog(state, request_id, conn_info, request).await
        }
        VisualStateRequest::GetDetails { request } => {
            handle_get_details(state, request_id, conn_info, request).await
        }
        VisualStateRequest::Create { request } => {
            handle_create_visual_state(state, request_id, conn_info, request).await
        }
        VisualStateRequest::Update { request } => {
            handle_update_visual_state(state, request_id, conn_info, request).await
        }
        VisualStateRequest::Delete { request } => {
            handle_delete_visual_state(state, request_id, conn_info, request).await
        }
        VisualStateRequest::SetActive { request } => {
            handle_set_active_visual_state(state, request_id, conn_info, request).await
        }
        VisualStateRequest::Generate { request } => {
            handle_generate_visual_state(state, request_id, conn_info, request).await
        }
    };

    // Extract the result from ServerMessage::Response
    match msg {
        ServerMessage::Response { result, .. } => Ok(result),
        ServerMessage::Error { code, message, .. } => {
            Err(ServerMessage::Error { code, message })
        }
        _ => {
            // Serialize ErrorCode to snake_case string
            let code_str = serde_json::to_string(&ErrorCode::InternalError)
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to serialize error code: {}", e);
                    "\"internal_error\"".to_string()
                })
                .trim_matches('"')
                .to_string();
            Err(ServerMessage::Error {
                code: code_str,
                message: "Unexpected response type".to_string(),
            })
        }
    }
}

/// Handle GetCatalog request
async fn handle_get_catalog(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: wrldbldr_shared::GetVisualStateCatalogRequest,
) -> ServerMessage {
    let location_id = request
        .location_id
        .map(wrldbldr_domain::LocationId::from_uuid);
    let region_id = request
        .region_id
        .map(wrldbldr_domain::RegionId::from_uuid);

    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .get_catalog(location_id, region_id)
        .await
    {
        Ok(catalog) => {
            // Convert domain types to protocol types
            let location_states: Vec<LocationStateData> = catalog
                .location_states
                .into_iter()
                .map(|ls| domain_to_location_state_data(ls))
                .collect();
            let region_states: Vec<RegionStateData> = catalog
                .region_states
                .into_iter()
                .map(|rs| domain_to_region_state_data(rs))
                .collect();

            let data = VisualStateCatalogData {
                location_states,
                region_states,
            };
            ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(data),
            }
        }
        Err(e) => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                map_catalog_error_to_code(&e),
                sanitize_repo_error(&e, "get visual state catalog"),
            ),
        },
    }
}

/// Handle GetDetails request
async fn handle_get_details(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: wrldbldr_shared::GetVisualStateDetailsRequest,
) -> ServerMessage {
    let location_state_id = request
        .location_state_id
        .map(wrldbldr_domain::LocationStateId::from_uuid);
    let region_state_id = request
        .region_state_id
        .map(wrldbldr_domain::RegionStateId::from_uuid);

    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .get_details(location_state_id, region_state_id)
        .await
    {
        Ok(VisualStateDetails::LocationState(ls)) => {
            ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(domain_to_location_state_data(ls)),
            }
        }
        Ok(VisualStateDetails::RegionState(rs)) => {
            ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(domain_to_region_state_data(rs)),
            }
        }
        Err(e) => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                map_catalog_error_to_code(&e),
                sanitize_repo_error(&e, "get visual state details"),
            ),
        },
    }
}

/// Handle Create request
async fn handle_create_visual_state(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: wrldbldr_shared::CreateVisualStateRequest,
) -> ServerMessage {
    if let Err(err) = require_dm_for_request(conn_info, request_id) {
        return err;
    }

    // Validate name length
    if request.name.len() > MAX_STATE_NAME {
        return ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                format!("Name too long (max {} chars)", MAX_STATE_NAME),
            ),
        };
    }

    // Validate description length
    if let Some(desc) = &request.description {
        if desc.len() > MAX_STATE_DESCRIPTION {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Description too long (max {} chars)", MAX_STATE_DESCRIPTION),
                ),
            };
        }
    }

    // Validate asset paths
    if let Some(asset) = &request.backdrop_asset {
        if asset.len() > MAX_ASSET_PATH_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Backdrop asset path too long (max {} chars)", MAX_ASSET_PATH_LENGTH),
                ),
            };
        }
    }

    if let Some(asset) = &request.ambient_sound {
        if asset.len() > MAX_ASSET_PATH_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Ambient sound path too long (max {} chars)", MAX_ASSET_PATH_LENGTH),
                ),
            };
        }
    }

    if let Some(asset) = &request.map_overlay {
        if asset.len() > MAX_ASSET_PATH_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Map overlay path too long (max {} chars)", MAX_ASSET_PATH_LENGTH),
                ),
            };
        }
    }

    // Determine state type
    let location_id = request
        .location_id
        .map(wrldbldr_domain::LocationId::from_uuid);
    let region_id = request
        .region_id
        .map(wrldbldr_domain::RegionId::from_uuid);

    match request.state_type {
        wrldbldr_shared::VisualStateType::Location => {
            let loc_id = match location_id {
                Some(id) => id,
                None => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            "location_id required for Location state type".to_string(),
                        ),
                    }
                }
            };

            match state
                .app
                .use_cases
                .visual_state
                .catalog
                .create_location_state(
                    loc_id,
                    request.name,
                    request.description,
                    request.backdrop_asset,
                    request.atmosphere,
                    request.ambient_sound,
                    request.map_overlay,
                    request.activation_rules,
                    request.activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(state) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::success(domain_to_location_state_data(state)),
                },
                Err(e) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        map_catalog_error_to_code(&e),
                        sanitize_repo_error(&e, "create location state"),
                    ),
                },
            }
        }
        wrldbldr_shared::VisualStateType::Region => {
            let reg_id = match region_id {
                Some(id) => id,
                None => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            "region_id required for Region state type".to_string(),
                        ),
                    }
                }
            };

            match state
                .app
                .use_cases
                .visual_state
                .catalog
                .create_region_state(
                    reg_id,
                    request.name,
                    request.description,
                    request.backdrop_asset,
                    request.atmosphere,
                    request.ambient_sound,
                    request.activation_rules,
                    request.activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(state) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::success(domain_to_region_state_data(state)),
                },
                Err(e) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        map_catalog_error_to_code(&e),
                        sanitize_repo_error(&e, "create region state"),
                    ),
                },
            }
        }
        wrldbldr_shared::VisualStateType::Unknown => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                "Unknown visual state type".to_string(),
            ),
        },
    }
}

/// Handle Update request
async fn handle_update_visual_state(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: wrldbldr_shared::UpdateVisualStateRequest,
) -> ServerMessage {
    // Validate lengths
    if let Some(name) = &request.name {
        if name.len() > MAX_STATE_NAME {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Name too long (max {} chars)", MAX_STATE_NAME),
                ),
            };
        }
    }

    if let Some(desc) = &request.description {
        if desc.len() > MAX_STATE_DESCRIPTION {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Description too long (max {} chars)", MAX_STATE_DESCRIPTION),
                ),
            };
        }
    }

    // Validate asset paths
    if let Some(asset) = &request.backdrop_asset {
        if asset.len() > MAX_ASSET_PATH_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Backdrop asset path too long (max {} chars)", MAX_ASSET_PATH_LENGTH),
                ),
            };
        }
    }

    if let Some(asset) = &request.ambient_sound {
        if asset.len() > MAX_ASSET_PATH_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Ambient sound path too long (max {} chars)", MAX_ASSET_PATH_LENGTH),
                ),
            };
        }
    }

    if let Some(asset) = &request.map_overlay {
        if asset.len() > MAX_ASSET_PATH_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Map overlay path too long (max {} chars)", MAX_ASSET_PATH_LENGTH),
                ),
            };
        }
    }

    // Determine state type
    let location_state_id = request
        .location_state_id
        .map(wrldbldr_domain::LocationStateId::from_uuid);
    let region_state_id = request
        .region_state_id
        .map(wrldbldr_domain::RegionStateId::from_uuid);

    if let Some(ls_id) = location_state_id {
        match state
            .app
            .use_cases
            .visual_state
            .catalog
            .update_location_state(
                ls_id,
                request.name,
                request.description,
                request.backdrop_asset,
                request.atmosphere,
                request.ambient_sound,
                request.map_overlay,
                request.activation_rules,
                request.activation_logic,
                request.priority,
                request.is_default,
                request.generation_prompt,
                request.workflow_id,
            )
            .await
        {
            Ok(state) => ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(domain_to_location_state_data(state)),
            },
            Err(e) => ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    map_catalog_error_to_code(&e),
                    sanitize_repo_error(&e, "update location state"),
                ),
            },
        }
    } else if let Some(rs_id) = region_state_id {
        match state
            .app
            .use_cases
            .visual_state
            .catalog
            .update_region_state(
                rs_id,
                request.name,
                request.description,
                request.backdrop_asset,
                request.atmosphere,
                request.ambient_sound,
                request.activation_rules,
                request.activation_logic,
                request.priority,
                request.is_default,
                request.generation_prompt,
                request.workflow_id,
            )
            .await
        {
            Ok(state) => ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(domain_to_region_state_data(state)),
            },
            Err(e) => ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    map_catalog_error_to_code(&e),
                    sanitize_repo_error(&e, "update region state"),
                ),
            },
        }
    } else {
        ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                "Either location_state_id or region_state_id must be provided".to_string(),
            ),
        }
    }
}

/// Handle Delete request
async fn handle_delete_visual_state(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: wrldbldr_shared::DeleteVisualStateRequest,
) -> ServerMessage {
    if let Err(err) = require_dm_for_request(conn_info, request_id) {
        return err;
    }

    let location_state_id = request
        .location_state_id
        .map(wrldbldr_domain::LocationStateId::from_uuid);
    let region_state_id = request
        .region_state_id
        .map(wrldbldr_domain::RegionStateId::from_uuid);

    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .delete(location_state_id, region_state_id)
        .await
    {
        Ok(()) => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::success(serde_json::json!({ "deleted": true })),
        },
        Err(e) => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                map_catalog_error_to_code(&e),
                sanitize_repo_error(&e, "delete visual state"),
            ),
        },
    }
}

/// Handle SetActive request
async fn handle_set_active_visual_state(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: wrldbldr_shared::SetActiveVisualStateRequest,
) -> ServerMessage {
    if let Err(err) = require_dm_for_request(conn_info, request_id) {
        return err;
    }

    let location_id = request
        .location_id
        .map(wrldbldr_domain::LocationId::from_uuid);
    let location_state_id = request
        .location_state_id
        .map(wrldbldr_domain::LocationStateId::from_uuid);
    let region_id = request
        .region_id
        .map(wrldbldr_domain::RegionId::from_uuid);
    let region_state_id = request
        .region_state_id
        .map(wrldbldr_domain::RegionStateId::from_uuid);

    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .set_active(location_id, location_state_id, region_id, region_state_id)
        .await
    {
        Ok(()) => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::success(serde_json::json!({ "updated": true })),
        },
        Err(e) => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                map_catalog_error_to_code(&e),
                sanitize_repo_error(&e, "set active visual state"),
            ),
        },
    }
}

/// Handle Generate request
async fn handle_generate_visual_state(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: wrldbldr_shared::GenerateVisualStateRequest,
) -> ServerMessage {
    if let Err(err) = require_dm_for_request(conn_info, request_id) {
        return err;
    }

    // Validate lengths
    if request.name.len() > MAX_STATE_NAME {
        return ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                format!("Name too long (max {} chars)", MAX_STATE_NAME),
            ),
        };
    }

    if request.description.len() > MAX_STATE_DESCRIPTION {
        return ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                format!("Description too long (max {} chars)", MAX_STATE_DESCRIPTION),
            ),
        };
    }

    if request.prompt.len() > MAX_PROMPT_LENGTH {
        return ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                format!("Prompt too long (max {} chars)", MAX_PROMPT_LENGTH),
            ),
        };
    }

    if request.workflow.len() > MAX_WORKFLOW_LENGTH {
        return ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                format!("Workflow name too long (max {} chars)", MAX_WORKFLOW_LENGTH),
            ),
        };
    }

    if let Some(neg_prompt) = &request.negative_prompt {
        if neg_prompt.len() > MAX_NEGATIVE_PROMPT_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!(
                        "Negative prompt too long (max {} chars)",
                        MAX_NEGATIVE_PROMPT_LENGTH
                    ),
                ),
            };
        }
    }

    if request.tags.len() > MAX_TAGS_COUNT {
        return ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                format!("Too many tags (max {})", MAX_TAGS_COUNT),
            ),
        };
    }

    for tag in &request.tags {
        if tag.len() > MAX_TAG_LENGTH {
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::ValidationError,
                    format!("Tag too long (max {} chars)", MAX_TAG_LENGTH),
                ),
            };
        }
    }

    let location_id = request
        .location_id
        .map(wrldbldr_domain::LocationId::from_uuid);
    let region_id = request
        .region_id
        .map(wrldbldr_domain::RegionId::from_uuid);

    match request.state_type {
        wrldbldr_shared::VisualStateType::Location => {
            let loc_id = match location_id {
                Some(id) => id,
                None => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            "location_id required for Location state type".to_string(),
                        ),
                    }
                }
            };

            match state
                .app
                .use_cases
                .visual_state
                .catalog
                .generate_visual_state(
                    Some(loc_id),
                    None,
                    request.name,
                    request.description,
                    request.prompt,
                    request.workflow,
                    request.tags,
                    request.generate_backdrop,
                    request.generate_map,
                    request.activation_rules,
                    request.activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(result) => {
                    let data = wrldbldr_shared::GeneratedVisualStateData {
                        location_state: result
                            .location_state
                            .map(domain_to_location_state_data),
                        region_state: result.region_state.map(domain_to_region_state_data),
                        generation_batch_id: result.generation_batch_id,
                        is_complete: result.is_complete,
                    };
                    ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::success(data),
                    }
                }
                Err(e) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        map_catalog_error_to_code(&e),
                        sanitize_repo_error(&e, "generate location visual state"),
                    ),
                },
            }
        }
        wrldbldr_shared::VisualStateType::Region => {
            let reg_id = match region_id {
                Some(id) => id,
                None => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            "region_id required for Region state type".to_string(),
                        ),
                    }
                }
            };

            match state
                .app
                .use_cases
                .visual_state
                .catalog
                .generate_visual_state(
                    None,
                    Some(reg_id),
                    request.name,
                    request.description,
                    request.prompt,
                    request.workflow,
                    request.tags,
                    request.generate_backdrop,
                    request.generate_map,
                    request.activation_rules,
                    request.activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(result) => {
                    let data = wrldbldr_shared::GeneratedVisualStateData {
                        location_state: result
                            .location_state
                            .map(domain_to_location_state_data),
                        region_state: result.region_state.map(domain_to_region_state_data),
                        generation_batch_id: result.generation_batch_id,
                        is_complete: result.is_complete,
                    };
                    ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::success(data),
                    }
                }
                Err(e) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        map_catalog_error_to_code(&e),
                        sanitize_repo_error(&e, "generate region visual state"),
                    ),
                },
            }
        }
        wrldbldr_shared::VisualStateType::Unknown => ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                "Unknown visual state type".to_string(),
            ),
        },
    }
}

/// Map CatalogError to ErrorCode
fn map_catalog_error_to_code(error: &crate::use_cases::visual_state::CatalogError) -> ErrorCode {
    match error {
        crate::use_cases::visual_state::CatalogError::Validation(_) => ErrorCode::ValidationError,
        crate::use_cases::visual_state::CatalogError::LocationNotFound(_)
        | crate::use_cases::visual_state::CatalogError::RegionNotFound(_)
        | crate::use_cases::visual_state::CatalogError::LocationStateNotFound(_)
        | crate::use_cases::visual_state::CatalogError::RegionStateNotFound(_) => {
            ErrorCode::NotFound
        }
        _ => ErrorCode::InternalError,
    }
}

/// Convert domain LocationState to protocol LocationStateData
fn domain_to_location_state_data(
    state: wrldbldr_domain::LocationState,
) -> LocationStateData {
    LocationStateData {
        id: state.id().to_uuid(),
        location_id: state.location_id().to_uuid(),
        name: state.name().to_string(),
        description: Some(state.description().to_string()),
        backdrop_override: state.backdrop_override().map(|p| p.to_string()),
        atmosphere_override: state.atmosphere_override().map(|a| a.as_str().to_string()),
        ambient_sound: state.ambient_sound().map(|p| p.to_string()),
        map_overlay: state.map_overlay().map(|p| p.to_string()),
        priority: state.priority(),
        is_default: state.is_default(),
        is_active: false, // Would need to check active state from repo
        activation_rules: Some(serde_json::to_value(state.activation_rules()).unwrap_or_default()),
        activation_logic: Some(format!("{:?}", state.activation_logic())),
        created_at: state.created_at().to_rfc3339(),
        updated_at: state.updated_at().to_rfc3339(),
        generation_prompt: state.generation_prompt().map(|s| s.to_string()),
        workflow_id: state.workflow_id().map(|s| s.to_string()),
    }
}

/// Convert domain RegionState to protocol RegionStateData
fn domain_to_region_state_data(
    state: wrldbldr_domain::RegionState,
) -> RegionStateData {
    RegionStateData {
        id: state.id().to_uuid(),
        region_id: state.region_id().to_uuid(),
        location_id: state.location_id().to_uuid(),
        name: state.name().to_string(),
        description: Some(state.description().to_string()),
        backdrop_override: state.backdrop_override().map(|p| p.to_string()),
        atmosphere_override: state.atmosphere_override().map(|a| a.as_str().to_string()),
        ambient_sound: state.ambient_sound().map(|p| p.to_string()),
        priority: state.priority(),
        is_default: state.is_default(),
        is_active: false, // Would need to check active state from repo
        activation_rules: Some(serde_json::to_value(state.activation_rules()).unwrap_or_default()),
        activation_logic: Some(format!("{:?}", state.activation_logic())),
        created_at: state.created_at().to_rfc3339(),
        updated_at: state.updated_at().to_rfc3339(),
        generation_prompt: state.generation_prompt().map(|s| s.to_string()),
        workflow_id: state.workflow_id().map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_to_location_state_data_conversion() {
        use chrono::Utc;
        use wrldbldr_domain::{
            ActivationLogic, ActivationRule, AssetPath, Atmosphere, LocationId, WorldId,
        };

        let state = wrldbldr_domain::LocationState::new(
            LocationId::new(),
            WorldId::new(),
            "Test State",
            Utc::now(),
        )
        .with_backdrop(AssetPath::new("/assets/test.png").unwrap())
        .with_priority(100);

        let data = domain_to_location_state_data(state);
        assert_eq!(data.name, "Test State");
        assert_eq!(data.priority, 100);
        assert_eq!(data.backdrop_override, Some("/assets/test.png".to_string()));
    }

    #[test]
    fn test_map_catalog_error_to_code() {
        use crate::use_cases::visual_state::CatalogError;

        let err = CatalogError::Validation("test error".to_string());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::ValidationError);

        let err = CatalogError::LocationNotFound(LocationId::new());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::NotFound);
    }
}
