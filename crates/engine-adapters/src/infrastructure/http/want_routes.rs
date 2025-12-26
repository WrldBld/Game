//! Want API routes
//!
//! Endpoints for managing NPC wants (motivations) and actantial views.
//! Part of P1.5: Actantial Model System.
//!
//! # WebSocket Broadcasts (P3.6)
//!
//! State-modifying endpoints broadcast changes to connected WebSocket clients
//! for multiplayer consistency. Broadcasts are fire-and-forget to avoid
//! slowing down REST responses.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_domain::entities::{ActantialRole, WantVisibility};
use wrldbldr_domain::{CharacterId, WantId};
use wrldbldr_engine_app::application::services::{
    ActantialContextService, ActorTargetType, CharacterService, CreateWantRequest, UpdateWantRequest,
};
use wrldbldr_protocol::{ServerMessage, WantData, WantVisibilityData};

use crate::infrastructure::state::AppState;
use crate::infrastructure::state_broadcast::broadcast_to_world_sessions;

// =============================================================================
// DTOs
// =============================================================================

/// Request to create a new want
#[derive(Debug, Deserialize)]
pub struct CreateWantHttpRequest {
    pub description: String,
    #[serde(default = "default_intensity")]
    pub intensity: f32,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default)]
    pub visibility: WantVisibilityDto,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub deflection_behavior: Option<String>,
    #[serde(default)]
    pub tells: Vec<String>,
}

fn default_intensity() -> f32 {
    0.5
}

fn default_priority() -> u32 {
    1
}

/// Request to update an existing want
#[derive(Debug, Deserialize)]
pub struct UpdateWantHttpRequest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub intensity: Option<f32>,
    #[serde(default)]
    pub priority: Option<u32>,
    #[serde(default)]
    pub visibility: Option<WantVisibilityDto>,
    #[serde(default)]
    pub deflection_behavior: Option<String>,
    #[serde(default)]
    pub tells: Option<Vec<String>>,
}

/// Request to set a want's target
#[derive(Debug, Deserialize)]
pub struct SetWantTargetRequest {
    pub target_id: String,
    /// "Character", "Item", or "Goal"
    pub target_type: String,
}

/// Request to add an actantial view
#[derive(Debug, Deserialize)]
pub struct AddActantialViewRequest {
    pub want_id: String,
    pub target_id: String,
    /// "npc" or "pc"
    pub target_type: String,
    /// "helper", "opponent", "sender", "receiver"
    pub role: String,
    pub reason: String,
}

/// Request to remove an actantial view
#[derive(Debug, Deserialize)]
pub struct RemoveActantialViewRequest {
    pub want_id: String,
    pub target_id: String,
    /// "npc" or "pc"
    pub target_type: String,
    /// "helper", "opponent", "sender", "receiver"
    pub role: String,
}

/// Visibility DTO
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WantVisibilityDto {
    Known,
    Suspected,
    #[default]
    Hidden,
}

impl From<WantVisibilityDto> for WantVisibility {
    fn from(dto: WantVisibilityDto) -> Self {
        match dto {
            WantVisibilityDto::Known => WantVisibility::Known,
            WantVisibilityDto::Suspected => WantVisibility::Suspected,
            WantVisibilityDto::Hidden => WantVisibility::Hidden,
        }
    }
}

impl From<WantVisibility> for WantVisibilityDto {
    fn from(v: WantVisibility) -> Self {
        match v {
            WantVisibility::Known => WantVisibilityDto::Known,
            WantVisibility::Suspected => WantVisibilityDto::Suspected,
            WantVisibility::Hidden => WantVisibilityDto::Hidden,
        }
    }
}

/// Response for a created want
#[derive(Debug, Serialize)]
pub struct WantCreatedResponse {
    pub id: String,
}

/// Full want response (matches protocol WantData)
#[derive(Debug, Serialize)]
pub struct WantResponse {
    pub id: String,
    pub description: String,
    pub intensity: f32,
    pub priority: u32,
    pub visibility: WantVisibilityDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<WantTargetResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deflection_behavior: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tells: Vec<String>,
}

/// Want target response
#[derive(Debug, Serialize)]
pub struct WantTargetResponse {
    pub id: String,
    pub name: String,
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Full actantial context response
#[derive(Debug, Serialize)]
pub struct ActantialContextResponse {
    pub npc_id: String,
    pub npc_name: String,
    pub wants: Vec<WantResponse>,
    pub social_views: SocialViewsResponse,
}

/// Social views response
#[derive(Debug, Default, Serialize)]
pub struct SocialViewsResponse {
    pub allies: Vec<SocialRelationResponse>,
    pub enemies: Vec<SocialRelationResponse>,
}

/// Social relation response
#[derive(Debug, Serialize)]
pub struct SocialRelationResponse {
    pub id: String,
    pub name: String,
    pub actor_type: String,
    pub reasons: Vec<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_role(role_str: &str) -> Result<ActantialRole, (StatusCode, String)> {
    match role_str.to_lowercase().as_str() {
        "helper" => Ok(ActantialRole::Helper),
        "opponent" => Ok(ActantialRole::Opponent),
        "sender" => Ok(ActantialRole::Sender),
        "receiver" => Ok(ActantialRole::Receiver),
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid role: {}. Must be 'helper', 'opponent', 'sender', or 'receiver'", role_str),
        )),
    }
}

fn parse_actor_type(type_str: &str) -> Result<ActorTargetType, (StatusCode, String)> {
    match type_str.to_lowercase().as_str() {
        "npc" => Ok(ActorTargetType::Npc),
        "pc" => Ok(ActorTargetType::Pc),
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid actor type: {}. Must be 'npc' or 'pc'", type_str),
        )),
    }
}

// =============================================================================
// Route Handlers
// =============================================================================

/// List all wants for a character
///
/// GET /api/characters/{id}/wants
pub async fn list_wants(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<WantResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let char_id = CharacterId::from_uuid(uuid);

    // Get full context and extract wants
    let context = state
        .game
        .actantial_context_service
        .get_context(char_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let wants: Vec<WantResponse> = context
        .wants
        .iter()
        .map(|w| {
            let target = w.target.as_ref().map(|t| WantTargetResponse {
                id: t.id().to_string(),
                name: t.name().to_string(),
                target_type: t.target_type().to_string(),
                description: match t {
                    wrldbldr_domain::value_objects::WantTarget::Goal { description, .. } => {
                        description.clone()
                    }
                    _ => None,
                },
            });

            WantResponse {
                id: w.want_id.to_string(),
                description: w.description.clone(),
                intensity: w.intensity,
                priority: w.priority,
                visibility: w.visibility.into(),
                target,
                deflection_behavior: w.deflection_behavior.clone(),
                tells: w.tells.clone(),
            }
        })
        .collect();

    Ok(Json(wants))
}

/// Create a new want for a character
///
/// POST /api/characters/{id}/wants
pub async fn create_want(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
    Json(req): Json<CreateWantHttpRequest>,
) -> Result<(StatusCode, Json<WantCreatedResponse>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let char_id = CharacterId::from_uuid(uuid);

    // Clone description for broadcast before moving into create_req
    let description_for_broadcast = req.description.clone();
    let visibility_for_broadcast = match req.visibility {
        WantVisibilityDto::Known => WantVisibilityData::Known,
        WantVisibilityDto::Suspected => WantVisibilityData::Suspected,
        WantVisibilityDto::Hidden => WantVisibilityData::Hidden,
    };

    let create_req = CreateWantRequest {
        description: req.description,
        intensity: req.intensity,
        priority: req.priority,
        visibility: req.visibility.into(),
        target_id: req.target_id,
        target_type: req.target_type,
        deflection_behavior: req.deflection_behavior.clone(),
        tells: req.tells.clone(),
    };

    let want_id = state
        .game
        .actantial_context_service
        .create_want(char_id, create_req)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Broadcast to connected clients (fire-and-forget)
    // Get character's world_id to find the right session
    if let Ok(Some(character)) = state.core.character_service.get_character(char_id).await {
        let want_data = WantData {
            id: want_id.to_string(),
            description: description_for_broadcast,
            intensity: req.intensity,
            priority: req.priority,
            visibility: visibility_for_broadcast,
            target: None, // New wants start without target
            deflection_behavior: req.deflection_behavior,
            tells: req.tells,
            helpers: vec![],
            opponents: vec![],
            sender: None,
            receiver: None,
        };
        let message = ServerMessage::NpcWantCreated {
            npc_id: char_id.to_string(),
            want: want_data,
        };
        broadcast_to_world_sessions(&state.world_connection_manager, character.world_id, message).await;
    }

    Ok((
        StatusCode::CREATED,
        Json(WantCreatedResponse {
            id: want_id.to_string(),
        }),
    ))
}

/// Update a want
///
/// PUT /api/wants/{want_id}
pub async fn update_want(
    State(state): State<Arc<AppState>>,
    Path(want_id): Path<String>,
    Json(req): Json<UpdateWantHttpRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&want_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid want ID".to_string()))?;
    let want_id = WantId::from_uuid(uuid);

    let update_req = UpdateWantRequest {
        description: req.description,
        intensity: req.intensity,
        priority: req.priority,
        visibility: req.visibility.map(|v| v.into()),
        deflection_behavior: req.deflection_behavior,
        tells: req.tells,
    };

    state
        .game
        .actantial_context_service
        .update_want(want_id, update_req)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a want
///
/// DELETE /api/wants/{want_id}
pub async fn delete_want(
    State(state): State<Arc<AppState>>,
    Path(want_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&want_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid want ID".to_string()))?;
    let want_id = WantId::from_uuid(uuid);

    state
        .game
        .actantial_context_service
        .delete_want(want_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Set a want's target
///
/// PUT /api/wants/{want_id}/target
pub async fn set_want_target(
    State(state): State<Arc<AppState>>,
    Path(want_id): Path<String>,
    Json(req): Json<SetWantTargetRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&want_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid want ID".to_string()))?;
    let want_id = WantId::from_uuid(uuid);

    state
        .game
        .actantial_context_service
        .set_want_target(want_id, &req.target_id, &req.target_type)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Remove a want's target
///
/// DELETE /api/wants/{want_id}/target
pub async fn remove_want_target(
    State(state): State<Arc<AppState>>,
    Path(want_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&want_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid want ID".to_string()))?;
    let want_id = WantId::from_uuid(uuid);

    state
        .game
        .actantial_context_service
        .remove_want_target(want_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get full actantial context for a character
///
/// GET /api/characters/{id}/actantial-context
pub async fn get_actantial_context(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
) -> Result<Json<ActantialContextResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let char_id = CharacterId::from_uuid(uuid);

    let context = state
        .game
        .actantial_context_service
        .get_context(char_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert to response
    let wants: Vec<WantResponse> = context
        .wants
        .iter()
        .map(|w| {
            let target = w.target.as_ref().map(|t| WantTargetResponse {
                id: t.id().to_string(),
                name: t.name().to_string(),
                target_type: t.target_type().to_string(),
                description: match t {
                    wrldbldr_domain::value_objects::WantTarget::Goal { description, .. } => {
                        description.clone()
                    }
                    _ => None,
                },
            });

            WantResponse {
                id: w.want_id.to_string(),
                description: w.description.clone(),
                intensity: w.intensity,
                priority: w.priority,
                visibility: w.visibility.into(),
                target,
                deflection_behavior: w.deflection_behavior.clone(),
                tells: w.tells.clone(),
            }
        })
        .collect();

    let allies: Vec<SocialRelationResponse> = context
        .social_views
        .allies
        .iter()
        .map(|(target, name, reasons)| SocialRelationResponse {
            id: target.id_string(),
            name: name.clone(),
            actor_type: target.type_label().to_string(),
            reasons: reasons.clone(),
        })
        .collect();

    let enemies: Vec<SocialRelationResponse> = context
        .social_views
        .enemies
        .iter()
        .map(|(target, name, reasons)| SocialRelationResponse {
            id: target.id_string(),
            name: name.clone(),
            actor_type: target.type_label().to_string(),
            reasons: reasons.clone(),
        })
        .collect();

    Ok(Json(ActantialContextResponse {
        npc_id: context.character_id.to_string(),
        npc_name: context.character_name.clone(),
        wants,
        social_views: SocialViewsResponse { allies, enemies },
    }))
}

/// Add an actantial view
///
/// POST /api/characters/{id}/actantial-views
pub async fn add_actantial_view(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
    Json(req): Json<AddActantialViewRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let char_id = CharacterId::from_uuid(uuid);

    let want_uuid = Uuid::parse_str(&req.want_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid want ID".to_string()))?;
    let want_id = WantId::from_uuid(want_uuid);

    let role = parse_role(&req.role)?;
    let target_type = parse_actor_type(&req.target_type)?;

    state
        .game
        .actantial_context_service
        .add_actantial_view(char_id, want_id, &req.target_id, target_type, role, req.reason)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Remove an actantial view
///
/// DELETE /api/characters/{id}/actantial-views
pub async fn remove_actantial_view(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
    Json(req): Json<RemoveActantialViewRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    remove_actantial_view_impl(&state, &character_id, req).await
}

/// Remove an actantial view (POST variant for clients that don't support DELETE with body)
///
/// POST /api/characters/{id}/actantial-views/remove
pub async fn remove_actantial_view_post(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
    Json(req): Json<RemoveActantialViewRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    remove_actantial_view_impl(&state, &character_id, req).await
}

/// Shared implementation for removing actantial views
async fn remove_actantial_view_impl(
    state: &AppState,
    character_id: &str,
    req: RemoveActantialViewRequest,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(character_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid character ID".to_string()))?;
    let char_id = CharacterId::from_uuid(uuid);

    let want_uuid = Uuid::parse_str(&req.want_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid want ID".to_string()))?;
    let want_id = WantId::from_uuid(want_uuid);

    let role = parse_role(&req.role)?;
    let target_type = parse_actor_type(&req.target_type)?;

    state
        .game
        .actantial_context_service
        .remove_actantial_view(char_id, want_id, &req.target_id, target_type, role)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
