//! Miscellaneous WebSocket message handlers.
//!
//! Handlers for utility and DM-specific operations:
//! - ComfyUI health checks
//! - NPC location sharing, approach events, location events (DM only)

use uuid::Uuid;

use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};
use wrldbldr_engine_ports::inbound::AppStatePort;
use wrldbldr_engine_ports::outbound::{
    ShareNpcLocationInput, TriggerApproachInput, TriggerLocationEventInput,
};
use wrldbldr_protocol::ServerMessage;

use super::common::extract_dm_context_opt;

/// Handles a ComfyUI health check request.
///
/// Spawns an async task to perform the health check and broadcast
/// the result to all connected clients as a `ComfyUIStateChanged` message.
pub async fn handle_check_comfyui_health(state: &dyn AppStatePort) -> Option<ServerMessage> {
    let comfyui_client = state.comfyui().clone();
    let connection_query = state.connection_query().clone();
    let connection_broadcast = state.connection_broadcast().clone();

    tokio::spawn(async move {
        let (state_str, message) = match comfyui_client.health_check().await {
            Ok(true) => ("connected".to_string(), None),
            Ok(false) => (
                "disconnected".to_string(),
                Some("ComfyUI is not responding".to_string()),
            ),
            Err(e) => (
                "disconnected".to_string(),
                Some(format!("Health check failed: {}", e)),
            ),
        };

        let msg = ServerMessage::ComfyUIStateChanged {
            state: state_str,
            message,
            retry_in_seconds: None,
        };
        // Serialize to JSON for the port interface
        if let Ok(json_msg) = serde_json::to_value(&msg) {
            for world_id in connection_query.get_all_world_ids().await {
                connection_broadcast
                    .broadcast_to_world(world_id, json_msg.clone())
                    .await;
            }
        }
    });

    None
}

/// Handles sharing an NPC's location with a player character.
pub async fn handle_share_npc_location(
    state: &dyn AppStatePort,
    client_id: Uuid,
    pc_id: String,
    npc_id: String,
    location_id: String,
    region_id: String,
    notes: Option<String>,
) -> Option<ServerMessage> {
    let ctx = extract_dm_context_opt(state, client_id).await?;

    let input = ShareNpcLocationInput {
        pc_id: parse_pc_id(&pc_id)?,
        npc_id: parse_npc_id(&npc_id)?,
        location_id: LocationId::from_uuid(Uuid::parse_str(&location_id).ok()?),
        region_id: RegionId::from_uuid(Uuid::parse_str(&region_id).ok()?),
        notes,
    };

    match state
        .observation_use_case()
        .share_npc_location(ctx, input)
        .await
    {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles triggering an NPC approach event.
pub async fn handle_trigger_approach_event(
    state: &dyn AppStatePort,
    client_id: Uuid,
    npc_id: String,
    target_pc_id: String,
    description: String,
    reveal: bool,
) -> Option<ServerMessage> {
    let ctx = extract_dm_context_opt(state, client_id).await?;

    let input = TriggerApproachInput {
        npc_id: parse_npc_id(&npc_id)?,
        target_pc_id: parse_pc_id(&target_pc_id)?,
        description,
        reveal,
    };

    match state
        .observation_use_case()
        .trigger_approach_event(ctx, input)
        .await
    {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles triggering a location-wide event.
pub async fn handle_trigger_location_event(
    state: &dyn AppStatePort,
    client_id: Uuid,
    region_id: String,
    description: String,
) -> Option<ServerMessage> {
    let ctx = extract_dm_context_opt(state, client_id).await?;

    let input = TriggerLocationEventInput {
        region_id: RegionId::from_uuid(Uuid::parse_str(&region_id).ok()?),
        description,
    };

    match state
        .observation_use_case()
        .trigger_location_event(ctx, input)
        .await
    {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_pc_id(id: &str) -> Option<PlayerCharacterId> {
    Some(PlayerCharacterId::from_uuid(Uuid::parse_str(id).ok()?))
}

fn parse_npc_id(id: &str) -> Option<CharacterId> {
    Some(CharacterId::from_uuid(Uuid::parse_str(id).ok()?))
}
