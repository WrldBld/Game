//! Common handler utilities
//!
//! Shared helper functions for WebSocket message handlers.
//! These extract connection context and create error messages.

use uuid::Uuid;
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_protocol::ServerMessage;

use wrldbldr_engine_ports::inbound::AppStatePort;

// =============================================================================
// Error Helpers
// =============================================================================

/// Create a ServerMessage::Error with the given code and message
pub fn error_msg(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

// =============================================================================
// Context Extraction - Result-based (with error messages)
// =============================================================================

/// Extract player context (world_id, pc_id) for player-facing operations.
///
/// Returns Err(ServerMessage) with appropriate error code if:
/// - Connection not found
/// - Not connected to a world
/// - No player character selected
pub async fn extract_player_context(
    state: &dyn AppStatePort,
    client_id: Uuid,
) -> Result<(WorldId, PlayerCharacterId), ServerMessage> {
    let client_id_str = client_id.to_string();
    let connection = state
        .connection_context()
        .get_connection_by_client_id(&client_id_str)
        .await
        .ok_or_else(|| error_msg("NOT_CONNECTED", "Connection not found"))?;

    let world_id = connection
        .world_id
        .map(WorldId::from_uuid)
        .ok_or_else(|| error_msg("NO_WORLD", "Not connected to a world"))?;

    let pc_id = connection
        .pc_id
        .map(PlayerCharacterId::from_uuid)
        .ok_or_else(|| error_msg("NO_PC", "No player character selected"))?;

    Ok((world_id, pc_id))
}

/// Extract DM context for DM-only operations (Result-based).
///
/// Returns Err(ServerMessage) with appropriate error code if:
/// - Connection not found
/// - Not connected to a world
/// - Not a DM
pub async fn extract_dm_context(
    state: &dyn AppStatePort,
    client_id: Uuid,
) -> Result<UseCaseContext, ServerMessage> {
    let client_id_str = client_id.to_string();
    let connection = state
        .connection_context()
        .get_connection_by_client_id(&client_id_str)
        .await
        .ok_or_else(|| error_msg("NOT_CONNECTED", "Connection not found"))?;

    let world_id = connection
        .world_id
        .map(WorldId::from_uuid)
        .ok_or_else(|| error_msg("NO_WORLD", "Not connected to a world"))?;

    if !connection.is_dm() {
        return Err(error_msg(
            "NOT_AUTHORIZED",
            "Only the DM can perform this action",
        ));
    }

    Ok(UseCaseContext {
        world_id,
        user_id: connection.user_id.clone(),
        is_dm: true,
        pc_id: connection.pc_id.map(PlayerCharacterId::from_uuid),
    })
}

// =============================================================================
// Context Extraction - Option-based (for simpler handlers)
// =============================================================================

/// Extract UseCaseContext from connection state (Option-based).
///
/// Returns None if connection not found or not connected to a world.
/// Does NOT check DM status - use `extract_dm_context_opt` for that.
pub async fn extract_context_opt(state: &dyn AppStatePort, client_id: Uuid) -> Option<UseCaseContext> {
    let conn = state
        .connection_context()
        .get_connection_by_client_id(&client_id.to_string())
        .await?;

    Some(UseCaseContext {
        world_id: WorldId::from_uuid(conn.world_id?),
        user_id: conn.user_id.clone(),
        is_dm: conn.is_dm(),
        pc_id: conn.pc_id.map(PlayerCharacterId::from_uuid),
    })
}

/// Extract DM context (Option-based).
///
/// Returns None if connection not found, not in world, or not a DM.
pub async fn extract_dm_context_opt(
    state: &dyn AppStatePort,
    client_id: Uuid,
) -> Option<UseCaseContext> {
    extract_context_opt(state, client_id)
        .await
        .filter(|c| c.is_dm)
}

// =============================================================================
// ID Parsing Helpers
// =============================================================================

/// Parse a string as a PlayerCharacterId
pub fn parse_pc_id(id: &str) -> Option<PlayerCharacterId> {
    Uuid::parse_str(id).ok().map(PlayerCharacterId::from_uuid)
}
