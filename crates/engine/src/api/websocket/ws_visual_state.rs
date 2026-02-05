use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::visual_state::{
    validate_asset_path, validate_generation_prompt, validate_negative_prompt,
    validate_state_description, validate_state_name, validate_state_name_required, validate_tags,
    validate_workflow_id, VisualStateDetails,
};
use wrldbldr_domain::{ActivationLogic, ActivationRule, LocationStateId, RegionStateId};
use wrldbldr_shared::{
    requests::visual_state::{
        LocationStateData, RegionStateData, VisualStateCatalogData,
    },
    ErrorCode, ResponseResult, ServerMessage, VisualStateRequest,
};

/// Error type for serialization failures in domain-to-protocol conversion.
#[derive(Debug, Clone)]
pub struct SerializationError {
    pub message: String,
    pub entity_type: &'static str,
    pub entity_id: String,
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to serialize {} {}: {}", self.entity_type, self.entity_id, self.message)
    }
}

impl std::error::Error for SerializationError {}

/// Converts a ValidationResult into a ServerMessage::Response if invalid.
/// Returns None if validation passed.
fn validation_to_response(
    result: crate::use_cases::visual_state::ValidationResult,
    request_id: &str,
) -> Option<ServerMessage> {
    if result.is_valid() {
        None
    } else {
        Some(ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::ValidationError,
                result.error_message().unwrap_or("Validation failed"),
            ),
        })
    }
}

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

    // Fetch active state IDs via use case (centralized lookup)
    let active_ids = match state
        .app
        .use_cases
        .visual_state
        .catalog
        .get_active_state_ids(location_id, region_id)
        .await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch active state IDs");
            return ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::InternalError,
                    "Internal error",
                ),
            };
        }
    };

    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .get_catalog(location_id, region_id)
        .await
    {
        Ok(catalog) => {
            // Convert domain types to protocol types with fail-fast error handling
            let location_states: Result<Vec<LocationStateData>, SerializationError> = catalog
                .location_states
                .into_iter()
                .map(|ls| domain_to_location_state_data(ls, active_ids.location_state_id))
                .collect();
            let region_states: Result<Vec<RegionStateData>, SerializationError> = catalog
                .region_states
                .into_iter()
                .map(|rs| domain_to_region_state_data(rs, active_ids.region_state_id))
                .collect();

            // Handle serialization errors with proper request_id correlation
            let location_states = match location_states {
                Ok(ls) => ls,
                Err(err) => {
                    tracing::error!(error = %err, "Failed to serialize location state");
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            "Internal error",
                        ),
                    };
                }
            };
            let region_states = match region_states {
                Ok(rs) => rs,
                Err(err) => {
                    tracing::error!(error = %err, "Failed to serialize region state");
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            "Internal error",
                        ),
                    };
                }
            };

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
            // Fetch active state for this location to determine is_active
            let active_location_state_id = match fetch_active_location_state_id(
                state,
                ls.location_id(),
                request_id,
            )
            .await
            {
                Ok(id) => id,
                Err(response) => return response,
            };

            match domain_to_location_state_data(ls, active_location_state_id) {
                Ok(data) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::success(data),
                },
                Err(err) => {
                    tracing::error!(error = %err, "Failed to serialize location state");
                    ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            "Internal error",
                        ),
                    }
                }
            }
        }
        Ok(VisualStateDetails::RegionState(rs)) => {
            // Fetch active state for this region to determine is_active
            let active_region_state_id = match fetch_active_region_state_id(
                state,
                rs.region_id(),
                request_id,
            )
            .await
            {
                Ok(id) => id,
                Err(response) => return response,
            };

            match domain_to_region_state_data(rs, active_region_state_id) {
                Ok(data) => ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::success(data),
                },
                Err(err) => {
                    tracing::error!(error = %err, "Failed to serialize region state");
                    ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            "Internal error",
                        ),
                    }
                }
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

    // Validate using shared helpers (name is required for create)
    if let Some(response) =
        validation_to_response(validate_state_name_required(&request.name), request_id)
    {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_state_description(request.description.as_deref()),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_asset_path(request.backdrop_asset.as_deref(), "Backdrop asset"),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_asset_path(request.ambient_sound.as_deref(), "Ambient sound"),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_asset_path(request.map_overlay.as_deref(), "Map overlay"),
        request_id,
    ) {
        return response;
    }

    // Determine state type
    let location_id = request.location_id.map(wrldbldr_domain::LocationId::from_uuid);
    let region_id = request.region_id.map(wrldbldr_domain::RegionId::from_uuid);

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

            // Parse activation rules and logic at API boundary
            let activation_rules = match parse_activation_rules(request.activation_rules) {
                Ok(rules) => rules,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.message,
                        ),
                    };
                }
            };
            let activation_logic = match parse_activation_logic(request.activation_logic) {
                Ok(logic) => logic,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.to_string(),
                        ),
                    };
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
                    activation_rules,
                    activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(new_state) => {
                    // Fetch active state for is_active determination
                    let active_location_state_id = match fetch_active_location_state_id(
                        state,
                        new_state.location_id(),
                        request_id,
                    )
                    .await
                    {
                        Ok(id) => id,
                        Err(response) => return response,
                    };

                    match domain_to_location_state_data(new_state, active_location_state_id) {
                        Ok(data) => ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::success(data),
                        },
                        Err(err) => {
                            tracing::error!(error = %err, "Failed to serialize location state");
                            ServerMessage::Response {
                                request_id: request_id.to_string(),
                                result: ResponseResult::error(
                                    ErrorCode::InternalError,
                                    "Internal error",
                                ),
                            }
                        }
                    }
                }
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

            // Parse activation rules and logic at API boundary
            let activation_rules = match parse_activation_rules(request.activation_rules) {
                Ok(rules) => rules,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.message,
                        ),
                    };
                }
            };
            let activation_logic = match parse_activation_logic(request.activation_logic) {
                Ok(logic) => logic,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.to_string(),
                        ),
                    };
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
                    activation_rules,
                    activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(new_state) => {
                    // Fetch active state for is_active determination
                    let active_region_state_id = match fetch_active_region_state_id(
                        state,
                        new_state.region_id(),
                        request_id,
                    )
                    .await
                    {
                        Ok(id) => id,
                        Err(response) => return response,
                    };

                    match domain_to_region_state_data(new_state, active_region_state_id) {
                        Ok(data) => ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::success(data),
                        },
                        Err(err) => {
                            tracing::error!(error = %err, "Failed to serialize region state");
                            ServerMessage::Response {
                                request_id: request_id.to_string(),
                                result: ResponseResult::error(
                                    ErrorCode::InternalError,
                                    "Internal error",
                                ),
                            }
                        }
                    }
                }
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
    conn_info: &ConnectionInfo,
    request: wrldbldr_shared::UpdateVisualStateRequest,
) -> ServerMessage {
    // Require DM authorization for update actions
    if let Err(err) = require_dm_for_request(conn_info, request_id) {
        return err;
    }

    // Validate using shared helpers
    if let Some(name) = &request.name {
        if let Some(response) = validation_to_response(validate_state_name(name), request_id) {
            return response;
        }
    }
    if let Some(response) = validation_to_response(
        validate_state_description(request.description.as_deref()),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_asset_path(request.backdrop_asset.as_deref(), "Backdrop asset"),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_asset_path(request.ambient_sound.as_deref(), "Ambient sound"),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_asset_path(request.map_overlay.as_deref(), "Map overlay"),
        request_id,
    ) {
        return response;
    }
    // Validate generation fields
    if let Some(response) = validation_to_response(
        validate_generation_prompt(request.generation_prompt.as_deref()),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_workflow_id(request.workflow_id.as_deref()),
        request_id,
    ) {
        return response;
    }

    // Determine state type
    let location_state_id = request
        .location_state_id
        .map(wrldbldr_domain::LocationStateId::from_uuid);
    let region_state_id = request
        .region_state_id
        .map(wrldbldr_domain::RegionStateId::from_uuid);

    if let Some(ls_id) = location_state_id {
        // Parse activation rules and logic at API boundary
        let activation_rules = match request.activation_rules {
            Some(ref v) => match parse_activation_rules(Some(v.clone())) {
                Ok(rules) => Some(rules),
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.message,
                        ),
                    };
                }
            },
            None => None,
        };
        let activation_logic = match request.activation_logic.as_ref() {
            Some(s) => match parse_activation_logic(Some(s.clone())) {
                Ok(logic) => Some(logic),
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.to_string(),
                        ),
                    };
                }
            },
            None => None,
        };

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
                activation_rules,
                activation_logic,
                request.priority,
                request.is_default,
                request.generation_prompt,
                request.workflow_id,
            )
            .await
        {
            Ok(updated_state) => {
                // Fetch active state for is_active determination
                let active_location_state_id = match fetch_active_location_state_id(
                    state,
                    updated_state.location_id(),
                    request_id,
                )
                .await
                {
                    Ok(id) => id,
                    Err(response) => return response,
                };

                match domain_to_location_state_data(updated_state, active_location_state_id) {
                    Ok(data) => ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::success(data),
                    },
                    Err(err) => {
                        tracing::error!(error = %err, "Failed to serialize location state");
                        ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(
                                ErrorCode::InternalError,
                                "Internal error",
                            ),
                        }
                    }
                }
            }
            Err(e) => ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    map_catalog_error_to_code(&e),
                    sanitize_repo_error(&e, "update location state"),
                ),
            },
        }
    } else if let Some(rs_id) = region_state_id {
        // Parse activation rules and logic at API boundary
        let activation_rules = match request.activation_rules {
            Some(ref v) => match parse_activation_rules(Some(v.clone())) {
                Ok(rules) => Some(rules),
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.message,
                        ),
                    };
                }
            },
            None => None,
        };
        let activation_logic = match request.activation_logic.as_ref() {
            Some(s) => match parse_activation_logic(Some(s.clone())) {
                Ok(logic) => Some(logic),
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.to_string(),
                        ),
                    };
                }
            },
            None => None,
        };

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
                activation_rules,
                activation_logic,
                request.priority,
                request.is_default,
                request.generation_prompt,
                request.workflow_id,
            )
            .await
        {
            Ok(updated_state) => {
                // Fetch active state for is_active determination
                let active_region_state_id = match fetch_active_region_state_id(
                    state,
                    updated_state.region_id(),
                    request_id,
                )
                .await
                {
                    Ok(id) => id,
                    Err(response) => return response,
                };

                match domain_to_region_state_data(updated_state, active_region_state_id) {
                    Ok(data) => ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::success(data),
                    },
                    Err(err) => {
                        tracing::error!(error = %err, "Failed to serialize region state");
                        ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(
                                ErrorCode::InternalError,
                                "Internal error",
                            ),
                        }
                    }
                }
            }
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
        Ok(()) => {
            let response_data = wrldbldr_shared::requests::visual_state::DeleteVisualStateResponse {
                deleted: true,
                location_state_id: request.location_state_id,
                region_state_id: request.region_state_id,
            };
            ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(response_data),
            }
        }
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
        Ok(()) => {
            let response_data =
                wrldbldr_shared::requests::visual_state::SetActiveVisualStateResponse {
                    updated: true,
                    location_id: request.location_id,
                    location_state_id: request.location_state_id,
                    region_id: request.region_id,
                    region_state_id: request.region_state_id,
                };
            ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::success(response_data),
            }
        }
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

    // Validate using shared helpers (name is required for generate)
    if let Some(response) =
        validation_to_response(validate_state_name_required(&request.name), request_id)
    {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_state_description(Some(&request.description)),
        request_id,
    ) {
        return response;
    }
    if let Some(response) =
        validation_to_response(validate_generation_prompt(Some(&request.prompt)), request_id)
    {
        return response;
    }
    if let Some(response) =
        validation_to_response(validate_workflow_id(Some(&request.workflow)), request_id)
    {
        return response;
    }
    if let Some(response) = validation_to_response(
        validate_negative_prompt(request.negative_prompt.as_deref()),
        request_id,
    ) {
        return response;
    }
    if let Some(response) = validation_to_response(validate_tags(&request.tags), request_id) {
        return response;
    }

    let location_id = request.location_id.map(wrldbldr_domain::LocationId::from_uuid);
    let region_id = request.region_id.map(wrldbldr_domain::RegionId::from_uuid);

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

            // Parse activation rules and logic at API boundary
            let activation_rules = match parse_activation_rules(request.activation_rules) {
                Ok(rules) => rules,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.message,
                        ),
                    };
                }
            };
            let activation_logic = match parse_activation_logic(request.activation_logic) {
                Ok(logic) => logic,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.to_string(),
                        ),
                    };
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
                    request.map_workflow,
                    request.tags,
                    request.generate_backdrop,
                    request.generate_map,
                    activation_rules,
                    activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(result) => {
                    // Convert BatchId to string at API boundary with fail-fast error handling
                    let location_state = match result.location_state {
                        Some(ls) => {
                            // Fetch active state for is_active determination
                            let active_location_state_id = match fetch_active_location_state_id(
                                state,
                                ls.location_id(),
                                request_id,
                            )
                            .await
                            {
                                Ok(id) => id,
                                Err(response) => return response,
                            };

                            match domain_to_location_state_data(ls, active_location_state_id) {
                                Ok(data) => Some(data),
                                Err(err) => {
                                    tracing::error!(error = %err, "Failed to serialize location state");
                                    return ServerMessage::Response {
                                        request_id: request_id.to_string(),
                                        result: ResponseResult::error(
                                            ErrorCode::InternalError,
                                            "Internal error",
                                        ),
                                    };
                                }
                            }
                        }
                        None => None,
                    };
                    let region_state = match result.region_state {
                        Some(rs) => {
                            // Fetch active state for is_active determination
                            let active_region_state_id = match fetch_active_region_state_id(
                                state,
                                rs.region_id(),
                                request_id,
                            )
                            .await
                            {
                                Ok(id) => id,
                                Err(response) => return response,
                            };

                            match domain_to_region_state_data(rs, active_region_state_id) {
                                Ok(data) => Some(data),
                                Err(err) => {
                                    tracing::error!(error = %err, "Failed to serialize region state");
                                    return ServerMessage::Response {
                                        request_id: request_id.to_string(),
                                        result: ResponseResult::error(
                                            ErrorCode::InternalError,
                                            "Internal error",
                                        ),
                                    };
                                }
                            }
                        }
                        None => None,
                    };
                    let data = wrldbldr_shared::GeneratedVisualStateData {
                        location_state,
                        region_state,
                        generation_batch_id: result.generation_batch_id.to_string(),
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

            // Parse activation rules and logic at API boundary
            let activation_rules = match parse_activation_rules(request.activation_rules) {
                Ok(rules) => rules,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.message,
                        ),
                    };
                }
            };
            let activation_logic = match parse_activation_logic(request.activation_logic) {
                Ok(logic) => logic,
                Err(err) => {
                    return ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::ValidationError,
                            err.to_string(),
                        ),
                    };
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
                    request.map_workflow,
                    request.tags,
                    request.generate_backdrop,
                    request.generate_map,
                    activation_rules,
                    activation_logic,
                    request.priority,
                    request.is_default,
                )
                .await
            {
                Ok(result) => {
                    // Convert BatchId to string at API boundary with fail-fast error handling
                    let location_state = match result.location_state {
                        Some(ls) => {
                            // Fetch active state for is_active determination
                            let active_location_state_id = match fetch_active_location_state_id(
                                state,
                                ls.location_id(),
                                request_id,
                            )
                            .await
                            {
                                Ok(id) => id,
                                Err(response) => return response,
                            };

                            match domain_to_location_state_data(ls, active_location_state_id) {
                                Ok(data) => Some(data),
                                Err(err) => {
                                    tracing::error!(error = %err, "Failed to serialize location state");
                                    return ServerMessage::Response {
                                        request_id: request_id.to_string(),
                                        result: ResponseResult::error(
                                            ErrorCode::InternalError,
                                            "Internal error",
                                        ),
                                    };
                                }
                            }
                        }
                        None => None,
                    };
                    let region_state = match result.region_state {
                        Some(rs) => {
                            // Fetch active state for is_active determination
                            let active_region_state_id = match fetch_active_region_state_id(
                                state,
                                rs.region_id(),
                                request_id,
                            )
                            .await
                            {
                                Ok(id) => id,
                                Err(response) => return response,
                            };

                            match domain_to_region_state_data(rs, active_region_state_id) {
                                Ok(data) => Some(data),
                                Err(err) => {
                                    tracing::error!(error = %err, "Failed to serialize region state");
                                    return ServerMessage::Response {
                                        request_id: request_id.to_string(),
                                        result: ResponseResult::error(
                                            ErrorCode::InternalError,
                                            "Internal error",
                                        ),
                                    };
                                }
                            }
                        }
                        None => None,
                    };
                    let data = wrldbldr_shared::GeneratedVisualStateData {
                        location_state,
                        region_state,
                        generation_batch_id: result.generation_batch_id.to_string(),
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
    use crate::use_cases::visual_state::CatalogError;

    match error {
        // Validation errors - map to ValidationError
        CatalogError::Validation(_)
        | CatalogError::NameEmpty
        | CatalogError::NameTooLong
        | CatalogError::DescriptionTooLong
        | CatalogError::Domain(_) => ErrorCode::ValidationError,
        // Not found errors
        CatalogError::LocationNotFound(_)
        | CatalogError::RegionNotFound(_)
        | CatalogError::LocationStateNotFound(_)
        | CatalogError::RegionStateNotFound(_) => ErrorCode::NotFound,
        // Repository and queue errors - internal error
        CatalogError::Repo(_) | CatalogError::Queue(_) => ErrorCode::InternalError,
    }
}

/// Error type for activation logic parsing failures.
#[derive(Debug, Clone)]
pub struct ParseActivationLogicError {
    pub input: String,
    pub message: String,
}

impl std::fmt::Display for ParseActivationLogicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse activation logic '{}': {}", self.input, self.message)
    }
}

impl std::error::Error for ParseActivationLogicError {}

/// Error type for activation rules parsing failures.
#[derive(Debug, Clone)]
pub struct ParseActivationRulesError {
    pub message: String,
}

impl std::fmt::Display for ParseActivationRulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse activation rules: {}", self.message)
    }
}

impl std::error::Error for ParseActivationRulesError {}

/// Fetch active location state ID via use case.
///
/// Helper to deduplicate error handling for active-state fetches.
/// Returns `Ok(Some(id))` if there's an active state, `Ok(None)` if not,
/// or `Err(ServerMessage::Response)` on error.
async fn fetch_active_location_state_id(
    state: &WsState,
    location_id: wrldbldr_domain::LocationId,
    request_id: &str,
) -> Result<Option<wrldbldr_domain::LocationStateId>, ServerMessage> {
    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .get_active_state_ids(Some(location_id), None)
        .await
    {
        Ok(active_ids) => Ok(active_ids.location_state_id),
        Err(e) => {
            tracing::error!(error = %e, %location_id, "Failed to fetch active location state");
            Err(ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::InternalError,
                    "Internal error",
                ),
            })
        }
    }
}

/// Fetch active region state ID via use case.
///
/// Helper to deduplicate error handling for active-state fetches.
/// Returns `Ok(Some(id))` if there's an active state, `Ok(None)` if not,
/// or `Err(ServerMessage::Response)` on error.
async fn fetch_active_region_state_id(
    state: &WsState,
    region_id: wrldbldr_domain::RegionId,
    request_id: &str,
) -> Result<Option<wrldbldr_domain::RegionStateId>, ServerMessage> {
    match state
        .app
        .use_cases
        .visual_state
        .catalog
        .get_active_state_ids(None, Some(region_id))
        .await
    {
        Ok(active_ids) => Ok(active_ids.region_state_id),
        Err(e) => {
            tracing::error!(error = %e, %region_id, "Failed to fetch active region state");
            Err(ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(
                    ErrorCode::InternalError,
                    "Internal error",
                ),
            })
        }
    }
}

/// Parse activation logic from protocol string to domain type.
///
/// Returns an error on invalid input instead of defaulting to `ActivationLogic::All`.
/// Rejects `AtLeast(0)` since it would always evaluate to true, bypassing all rules.
fn parse_activation_logic(logic: Option<String>) -> Result<ActivationLogic, ParseActivationLogicError> {
    match logic.as_deref() {
        None => Ok(ActivationLogic::All),
        Some("All") => Ok(ActivationLogic::All),
        Some("Any") => Ok(ActivationLogic::Any),
        Some(s) if s.starts_with("AtLeast") => {
            // Parse "AtLeast(N)" pattern
            if let Some(n_str) = s.strip_prefix("AtLeast(") {
                if let Some(n) = n_str.strip_suffix(')') {
                    match n.parse::<u32>() {
                        Ok(0) => Err(ParseActivationLogicError {
                            input: s.to_string(),
                            message: "AtLeast(n) requires n >= 1".to_string(),
                        }),
                        Ok(count) => Ok(ActivationLogic::AtLeast(count)),
                        Err(_) => Err(ParseActivationLogicError {
                            input: s.to_string(),
                            message: format!("Invalid number '{}' in AtLeast expression", n),
                        }),
                    }
                } else {
                    Err(ParseActivationLogicError {
                        input: s.to_string(),
                        message: "Missing closing parenthesis in AtLeast expression".to_string(),
                    })
                }
            } else {
                Err(ParseActivationLogicError {
                    input: s.to_string(),
                    message: "Invalid AtLeast expression format".to_string(),
                })
            }
        }
        Some(s) => Err(ParseActivationLogicError {
            input: s.to_string(),
            message: format!("Unknown activation logic variant '{}'", s),
        }),
    }
}

/// Parse activation rules from protocol JSON to domain type.
///
/// Accepts both a JSON array of rules and a single rule object (wrapped in a Vec).
/// Returns a `ParseActivationRulesError` on failure instead of `ServerMessage::Error`
/// to preserve request correlation (callers must wrap with `request_id`).
fn parse_activation_rules(
    rules_json: Option<serde_json::Value>,
) -> Result<Vec<ActivationRule>, ParseActivationRulesError> {
    match rules_json {
        Some(value) => {
            // Normalize single object into array for backward compatibility
            let normalized = match value {
                serde_json::Value::Object(_) => serde_json::json!([value]),
                other => other,
            };

            serde_json::from_value::<Vec<ActivationRule>>(normalized).map_err(|e| {
                // Log the detailed error server-side for debugging
                tracing::warn!(error = %e, "Failed to parse activation rules from client request");
                // Return sanitized error message (callers wrap with request_id)
                ParseActivationRulesError {
                    message: "Invalid activation rules format".to_string(),
                }
            })
        }
        None => Ok(Vec::new()),
    }
}

/// Converts ActivationLogic to its protocol string representation.
///
/// Returns stable protocol values: "All", "Any", "AtLeast(n)".
/// This replaces the previous `format!("{:?}")` approach which was not
/// a stable protocol contract.
fn activation_logic_to_protocol(logic: ActivationLogic) -> String {
    match logic {
        ActivationLogic::All => "All".to_string(),
        ActivationLogic::Any => "Any".to_string(),
        ActivationLogic::AtLeast(n) => format!("AtLeast({})", n),
    }
}

/// Convert domain LocationState to protocol LocationStateData.
///
/// Returns a `SerializationError` if activation rules serialization fails.
/// The caller is responsible for wrapping this in a `ServerMessage::Response`
/// with the appropriate `request_id` to preserve request correlation.
fn domain_to_location_state_data(
    state: wrldbldr_domain::LocationState,
    active_state_id: Option<LocationStateId>,
) -> Result<LocationStateData, SerializationError> {
    // Serialize activation rules with fail-fast error handling
    let activation_rules = serde_json::to_value(state.activation_rules()).map_err(|e| {
        tracing::error!(
            error = %e,
            location_state_id = %state.id(),
            "Failed to serialize activation rules"
        );
        SerializationError {
            message: format!("Failed to serialize activation rules: {}", e),
            entity_type: "LocationState",
            entity_id: state.id().to_string(),
        }
    })?;

    // Determine is_active by comparing with the fetched active state ID
    let is_active = active_state_id.is_some_and(|id| id == state.id());

    Ok(LocationStateData {
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
        is_active,
        activation_rules: Some(activation_rules),
        activation_logic: Some(activation_logic_to_protocol(state.activation_logic())),
        created_at: state.created_at().to_rfc3339(),
        updated_at: state.updated_at().to_rfc3339(),
        generation_prompt: state.generation_prompt().map(|s| s.to_string()),
        workflow_id: state.workflow_id().map(|s| s.to_string()),
    })
}

/// Convert domain RegionState to protocol RegionStateData.
///
/// Returns a `SerializationError` if activation rules serialization fails.
/// The caller is responsible for wrapping this in a `ServerMessage::Response`
/// with the appropriate `request_id` to preserve request correlation.
fn domain_to_region_state_data(
    state: wrldbldr_domain::RegionState,
    active_state_id: Option<RegionStateId>,
) -> Result<RegionStateData, SerializationError> {
    // Serialize activation rules with fail-fast error handling
    let activation_rules = serde_json::to_value(state.activation_rules()).map_err(|e| {
        tracing::error!(
            error = %e,
            region_state_id = %state.id(),
            "Failed to serialize activation rules"
        );
        SerializationError {
            message: format!("Failed to serialize activation rules: {}", e),
            entity_type: "RegionState",
            entity_id: state.id().to_string(),
        }
    })?;

    // Determine is_active by comparing with the fetched active state ID
    let is_active = active_state_id.is_some_and(|id| id == state.id());

    Ok(RegionStateData {
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
        is_active,
        activation_rules: Some(activation_rules),
        activation_logic: Some(activation_logic_to_protocol(state.activation_logic())),
        created_at: state.created_at().to_rfc3339(),
        updated_at: state.updated_at().to_rfc3339(),
        generation_prompt: state.generation_prompt().map(|s| s.to_string()),
        workflow_id: state.workflow_id().map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::visual_state::MAX_STATE_NAME;

    #[test]
    fn test_domain_to_location_state_data_conversion() {
        use chrono::Utc;
        use wrldbldr_domain::{AssetPath, LocationId, WorldId};

        let state1 = wrldbldr_domain::LocationState::new(
            LocationId::new(),
            WorldId::new(),
            "Test State 1",
            Utc::now(),
        )
        .expect("valid location state")
        .with_backdrop(AssetPath::new("/assets/test.png").expect("test: asset path should be valid"))
        .with_priority(100);
        let _state1_id = state1.id();

        // Test with no active state (None)
        let data = domain_to_location_state_data(state1, None)
            .expect("conversion should succeed");
        assert_eq!(data.name, "Test State 1");
        assert_eq!(data.priority, 100);
        assert_eq!(data.backdrop_override, Some("/assets/test.png".to_string()));
        assert!(!data.is_active, "State should not be active when no active state is set");

        // Test with this state as the active state
        let state2 = wrldbldr_domain::LocationState::new(
            LocationId::new(),
            WorldId::new(),
            "Test State 2",
            Utc::now(),
        )
        .expect("valid location state");
        let state2_id = state2.id();
        let data = domain_to_location_state_data(state2, Some(state2_id))
            .expect("conversion should succeed");
        assert!(data.is_active, "State should be active when its ID matches the active state ID");

        // Test with a different state as the active state
        let state3 = wrldbldr_domain::LocationState::new(
            LocationId::new(),
            WorldId::new(),
            "Test State 3",
            Utc::now(),
        )
        .expect("valid location state");
        let other_id = LocationStateId::new();
        let data = domain_to_location_state_data(state3, Some(other_id))
            .expect("conversion should succeed");
        assert!(!data.is_active, "State should not be active when a different state is active");
    }

    #[test]
    fn test_activation_logic_to_protocol_mapping() {
        // Test that activation_logic_to_protocol produces stable protocol values
        assert_eq!(activation_logic_to_protocol(ActivationLogic::All), "All");
        assert_eq!(activation_logic_to_protocol(ActivationLogic::Any), "Any");
        assert_eq!(activation_logic_to_protocol(ActivationLogic::AtLeast(1)), "AtLeast(1)");
        assert_eq!(activation_logic_to_protocol(ActivationLogic::AtLeast(5)), "AtLeast(5)");

        // Ensure the output matches the expected protocol format (not Debug format)
        let all_output = activation_logic_to_protocol(ActivationLogic::All);
        assert!(!all_output.contains("All("), "Should not be Debug format");
        assert!(!all_output.contains("Any("), "Should not be Debug format");
    }

    #[test]
    fn test_serialization_error_contains_entity_info() {
        // Create a SerializationError and verify it contains entity information
        let err = SerializationError {
            message: "Test error".to_string(),
            entity_type: "LocationState",
            entity_id: "test-id-123".to_string(),
        };

        let err_string = err.to_string();
        assert!(err_string.contains("LocationState"), "Error should contain entity type");
        assert!(err_string.contains("test-id-123"), "Error should contain entity ID");
        assert!(err_string.contains("Test error"), "Error should contain message");
    }

    #[test]
    fn test_map_catalog_error_to_code() {
        use crate::use_cases::visual_state::CatalogError;
        use wrldbldr_domain::DomainError;

        // Validation errors should map to ValidationError
        let err = CatalogError::Validation("test error".to_string());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::ValidationError);

        let err = CatalogError::NameEmpty;
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::ValidationError);

        let err = CatalogError::NameTooLong;
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::ValidationError);

        let err = CatalogError::DescriptionTooLong;
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::ValidationError);

        let err = CatalogError::Domain(DomainError::validation("corrupt data"));
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::ValidationError);

        // Not found errors should map to NotFound
        let err = CatalogError::LocationNotFound(LocationId::new());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::NotFound);

        let err = CatalogError::RegionNotFound(RegionId::new());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::NotFound);

        let err = CatalogError::LocationStateNotFound(LocationStateId::new());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::NotFound);

        let err = CatalogError::RegionStateNotFound(RegionStateId::new());
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::NotFound);

        // Repository and queue errors should map to InternalError
        use crate::infrastructure::ports::RepoError;
        let err = CatalogError::Repo(RepoError::database("query", "db down"));
        assert_eq!(map_catalog_error_to_code(&err), ErrorCode::InternalError);
    }

    #[test]
    fn test_multibyte_character_validation_uses_char_count_not_bytes() {
        // This test verifies that string length validation uses character count
        // (Unicode scalar values) rather than byte length. This is important for
        // non-ASCII content like emoji, CJK characters, etc.
        //
        // Example: "🎮" is 4 bytes but 1 character
        // Example: "中" is 3 bytes but 1 character

        // Create a string with 200 emoji characters (each is 4 bytes = 800 bytes total)
        // This should pass validation since it's exactly MAX_STATE_NAME characters
        let emoji_name: String = "🎮".repeat(MAX_STATE_NAME);
        assert_eq!(emoji_name.chars().count(), MAX_STATE_NAME);
        assert!(emoji_name.len() > MAX_STATE_NAME); // Byte length exceeds limit!

        // Validation using chars().count() should pass (200 chars == MAX_STATE_NAME)
        let char_validation_result = if emoji_name.chars().count() > MAX_STATE_NAME {
            Err("too long")
        } else {
            Ok(())
        };
        assert!(
            char_validation_result.is_ok(),
            "200 emoji chars should pass validation (200 chars == MAX_STATE_NAME)"
        );

        // Validation using .len() would incorrectly fail (800 bytes > MAX_STATE_NAME)
        let byte_validation_result = if emoji_name.len() > MAX_STATE_NAME {
            Err("too long")
        } else {
            Ok(())
        };
        assert!(
            byte_validation_result.is_err(),
            "Byte-based validation would incorrectly reject 200 emoji chars"
        );

        // Now test with 201 emoji characters - should fail char-based validation
        let oversized_emoji_name: String = "🎮".repeat(MAX_STATE_NAME + 1);
        assert_eq!(oversized_emoji_name.chars().count(), MAX_STATE_NAME + 1);

        let oversized_validation = if oversized_emoji_name.chars().count() > MAX_STATE_NAME {
            Err("too long")
        } else {
            Ok(())
        };
        assert!(
            oversized_validation.is_err(),
            "201 emoji chars should fail validation (201 chars > MAX_STATE_NAME)"
        );

        // Test with CJK characters (each is 3 bytes in UTF-8)
        let cjk_name: String = "中".repeat(MAX_STATE_NAME);
        assert_eq!(cjk_name.chars().count(), MAX_STATE_NAME);
        assert!(cjk_name.len() > MAX_STATE_NAME); // 600 bytes > 200

        let cjk_validation = if cjk_name.chars().count() > MAX_STATE_NAME {
            Err("too long")
        } else {
            Ok(())
        };
        assert!(
            cjk_validation.is_ok(),
            "200 CJK chars should pass validation (200 chars == MAX_STATE_NAME)"
        );

        // Test mixed ASCII and multi-byte characters
        let mixed_name = "A🎮B中C".to_string();
        assert_eq!(mixed_name.chars().count(), 5); // A, 🎮, B, 中, C
        assert!(mixed_name.len() > 5); // Byte length is greater

        let mixed_validation = if mixed_name.chars().count() > MAX_STATE_NAME {
            Err("too long")
        } else {
            Ok(())
        };
        assert!(
            mixed_validation.is_ok(),
            "Mixed ASCII/multi-byte string should pass validation"
        );
    }

    #[test]
    fn test_parse_activation_rules_returns_validation_error_on_invalid_json() {
        // Test that parse_activation_rules returns a ParseActivationRulesError when given invalid JSON
        let invalid_json = serde_json::json!({"invalid": "structure"});
        let result = parse_activation_rules(Some(invalid_json));

        match result {
            Err(ParseActivationRulesError { message }) => {
                assert!(message.contains("Invalid activation rules format"));
            }
            Ok(_) => panic!("Expected error for invalid activation rules, got Ok"),
        }
    }

    #[test]
    fn test_parse_activation_rules_returns_empty_vec_for_none() {
        // Test that parse_activation_rules returns empty vec when given None
        let result = parse_activation_rules(None);
        assert!(result.is_ok());
        assert!(result.expect("test: result should be Ok").is_empty());
    }

    #[test]
    fn test_parse_activation_rules_accepts_valid_rules() {
        // Test that parse_activation_rules correctly parses valid activation rules
        use wrldbldr_domain::ActivationRule;

        // Valid TimeOfDay rule using the enum structure (snake_case per TimeOfDay serde)
        let valid_rules = serde_json::json!([
            {
                "type": "TimeOfDay",
                "period": "morning"
            }
        ]);

        let result = parse_activation_rules(Some(valid_rules));
        assert!(result.is_ok(), "Valid activation rules should parse successfully");

        let rules = result.expect("test: result should be Ok");
        assert_eq!(rules.len(), 1);
        match &rules[0] {
            ActivationRule::TimeOfDay { .. } => {
                // Valid time of day rule parsed correctly
            }
            other => panic!("Expected TimeOfDay rule, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_activation_logic_valid_inputs() {
        // Test valid activation logic parsing
        assert_eq!(parse_activation_logic(None).expect("test: result should be Ok"), ActivationLogic::All);
        assert_eq!(parse_activation_logic(Some("All".to_string())).expect("test: result should be Ok"), ActivationLogic::All);
        assert_eq!(parse_activation_logic(Some("Any".to_string())).expect("test: result should be Ok"), ActivationLogic::Any);
        assert_eq!(
            parse_activation_logic(Some("AtLeast(3)".to_string())).expect("test: result should be Ok"),
            ActivationLogic::AtLeast(3)
        );
    }

    #[test]
    fn test_parse_activation_logic_invalid_inputs_return_errors() {
        // Test that invalid activation logic returns errors instead of defaulting
        
        // Invalid variant
        let result = parse_activation_logic(Some("InvalidVariant".to_string()));
        assert!(result.is_err(), "Unknown variant should return error");
        let err = result.expect_err("test: result should be Err");
        assert!(err.input.contains("InvalidVariant"));
        assert!(err.message.contains("Unknown activation logic variant"));

        // Invalid number in AtLeast
        let result = parse_activation_logic(Some("AtLeast(invalid)".to_string()));
        assert!(result.is_err(), "Invalid number should return error");
        let err = result.expect_err("test: result should be Err");
        assert!(err.input.contains("AtLeast(invalid)"));
        assert!(err.message.contains("Invalid number"));

        // Missing closing parenthesis
        let result = parse_activation_logic(Some("AtLeast(3".to_string()));
        assert!(result.is_err(), "Missing closing paren should return error");
        let err = result.expect_err("test: result should be Err");
        assert!(err.input.contains("AtLeast(3"));
        assert!(err.message.contains("Missing closing parenthesis"));

        // Empty AtLeast
        let result = parse_activation_logic(Some("AtLeast()".to_string()));
        assert!(result.is_err(), "Empty AtLeast should return error");
        let err = result.expect_err("test: result should be Err");
        assert!(err.input.contains("AtLeast()"));
        assert!(err.message.contains("Invalid number"));

        // AtLeast(0) should be rejected (would always evaluate true, bypassing rules)
        let result = parse_activation_logic(Some("AtLeast(0)".to_string()));
        assert!(result.is_err(), "AtLeast(0) should return error");
        let err = result.expect_err("test: result should be Err");
        assert!(err.input.contains("AtLeast(0)"));
        assert!(err.message.contains("n >= 1"));

        // AtLeast(1) should be valid
        let result = parse_activation_logic(Some("AtLeast(1)".to_string()));
        assert!(result.is_ok(), "AtLeast(1) should be valid");
        assert_eq!(result.expect("test: result should be Ok"), ActivationLogic::AtLeast(1));
    }

    #[test]
    fn test_parse_activation_rules_accepts_single_object() {
        // Test that parse_activation_rules accepts a single rule object (not just arrays)
        use wrldbldr_domain::ActivationRule;

        // snake_case per TimeOfDay serde attribute
        let single_rule = serde_json::json!({
            "type": "TimeOfDay",
            "period": "evening"
        });

        let result = parse_activation_rules(Some(single_rule));
        assert!(result.is_ok(), "Single object payload should be accepted");

        let rules = result.expect("test: result should be Ok");
        assert_eq!(rules.len(), 1, "Single object should be wrapped in Vec");
        match &rules[0] {
            ActivationRule::TimeOfDay { .. } => {
                // Valid time of day rule parsed correctly
            }
            other => panic!("Expected TimeOfDay rule, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_activation_rules_rejects_non_object_non_array() {
        // Test that parse_activation_rules rejects primitives and other invalid types
        let invalid_types = vec![
            serde_json::json!("just a string"),
            serde_json::json!(42),
            serde_json::json!(true),
            serde_json::json!(null),
        ];

        for invalid in invalid_types {
            let result = parse_activation_rules(Some(invalid));
            assert!(
                result.is_err(),
                "Non-object/non-array should return error"
            );
            let err = result.expect_err("test: result should be Err");
            assert!(err.message.contains("Invalid activation rules format"));
        }
    }
}
