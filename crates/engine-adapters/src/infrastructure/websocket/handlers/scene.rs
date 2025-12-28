//! Scene-related WebSocket message handlers.
//!
//! Thin routing layer for scene management. Business logic is in SceneUseCase.

use uuid::Uuid;
use crate::infrastructure::state::AppState;
use wrldbldr_domain::{PlayerCharacterId, SceneId, WorldId};
use wrldbldr_engine_app::application::use_cases::{
    ErrorCode, NpcMotivation, RequestSceneChangeInput, SceneApprovalDecision,
    SceneApprovalDecisionInput, SceneChangeResult, UpdateDirectorialInput,
};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_protocol::{CharacterPosition, DirectorialContext, ServerMessage};

/// Handles a request to change the current scene.
pub async fn handle_request_scene_change(
    state: &AppState, client_id: Uuid, scene_id: String,
) -> Option<ServerMessage> {
    let scene_uuid = match Uuid::parse_str(&scene_id) {
        Ok(uuid) => SceneId::from_uuid(uuid),
        Err(_) => return Some(error_msg("INVALID_SCENE_ID", "Invalid scene ID format")),
    };
    let ctx = extract_context(state, client_id).await?;
    let input = RequestSceneChangeInput { scene_id: scene_uuid };
    match state.use_cases.scene.request_scene_change(ctx, input).await {
        Ok(result) => convert_scene_result(result),
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a directorial update from the DM.
pub async fn handle_directorial_update(
    state: &AppState, client_id: Uuid, context: DirectorialContext,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Some(c) => c,
        None => return Some(error_msg("NOT_AUTHORIZED", "Only the DM can perform this action")),
    };
    let input = UpdateDirectorialInput {
        npc_motivations: context.npc_motivations.into_iter().map(|m| NpcMotivation {
            character_id: m.character_id, motivation: m.immediate_goal,
            emotional_state: Some(m.emotional_guidance),
        }).collect(),
        scene_mood: Some(context.tone), pacing: None, dm_notes: Some(context.scene_notes),
    };
    match state.use_cases.scene.update_directorial_context(ctx, input).await {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles an approval decision from the DM.
pub async fn handle_approval_decision(
    state: &AppState, client_id: Uuid, request_id: String,
    decision: wrldbldr_protocol::ApprovalDecision,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Some(c) => c,
        None => return Some(error_msg("NOT_AUTHORIZED", "Only the DM can approve responses")),
    };
    let input = SceneApprovalDecisionInput { request_id, decision: convert_decision(decision) };
    match state.use_cases.scene.handle_approval_decision(ctx, input).await {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

// === Helpers ===

async fn extract_context(state: &AppState, client_id: Uuid) -> Option<UseCaseContext> {
    let conn = state.world_connection_manager.get_connection_by_client_id(&client_id.to_string()).await?;
    Some(UseCaseContext {
        world_id: WorldId::from_uuid(conn.world_id?),
        user_id: conn.user_id.clone(), is_dm: conn.is_dm(),
        pc_id: conn.pc_id.map(PlayerCharacterId::from_uuid),
    })
}

async fn extract_dm_context(state: &AppState, client_id: Uuid) -> Option<UseCaseContext> {
    extract_context(state, client_id).await.filter(|c| c.is_dm)
}

fn convert_scene_result(result: SceneChangeResult) -> Option<ServerMessage> {
    let s = result.scene?;
    Some(ServerMessage::SceneUpdate {
        scene: wrldbldr_protocol::SceneData {
            id: s.id, name: s.name, location_id: s.location_id, location_name: s.location_name,
            backdrop_asset: s.backdrop_asset, time_context: s.time_context,
            directorial_notes: s.directorial_notes.unwrap_or_default(),
        },
        characters: result.characters.into_iter().map(|c| wrldbldr_protocol::CharacterData {
            id: c.id, name: c.name, sprite_asset: c.sprite_asset, portrait_asset: c.portrait_asset,
            position: CharacterPosition::Center, is_speaking: c.is_speaking, emotion: c.emotion,
        }).collect(),
        interactions: result.interactions.into_iter().map(|i| wrldbldr_protocol::InteractionData {
            id: i.id, name: i.name, interaction_type: i.interaction_type,
            target_name: i.target_name, is_available: i.is_available,
        }).collect(),
    })
}

fn convert_decision(d: wrldbldr_protocol::ApprovalDecision) -> SceneApprovalDecision {
    use wrldbldr_protocol::ApprovalDecision::*;
    match d {
        Accept | AcceptWithRecipients { .. } => SceneApprovalDecision::Approve,
        AcceptWithModification { modified_dialogue, .. } =>
            SceneApprovalDecision::ApproveWithEdits { modified_text: modified_dialogue },
        Reject { feedback } => SceneApprovalDecision::Reject { reason: feedback },
        TakeOver { dm_response } => SceneApprovalDecision::ApproveWithEdits { modified_text: dm_response },
    }
}

fn error_msg(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error { code: code.to_string(), message: message.to_string() }
}
