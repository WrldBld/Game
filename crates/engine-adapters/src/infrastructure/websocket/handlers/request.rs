//! Request/Response pattern handler
//!
//! Handles the generic Request pattern that delegates to AppRequestHandler
//! for all CRUD operations. This is the WebSocket-first protocol's primary
//! mechanism for entity operations.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{RequestPayload, ServerMessage};

/// Handle Request message
///
/// Delegates all request handling to the AppRequestHandler which routes
/// to appropriate services based on the RequestPayload variant.
///
/// # Flow
///
/// 1. Parse connection ID from client_id
/// 2. Fetch connection context from world_connection_manager
/// 3. Build RequestContext (authenticated or anonymous)
/// 4. Delegate to AppRequestHandler.handle()
/// 5. Wrap result in ServerMessage::Response
///
/// # Arguments
///
/// * `state` - Shared application state containing services and managers
/// * `client_id` - WebSocket connection identifier
/// * `request_id` - Client-generated request ID for response correlation
/// * `payload` - The request payload (CRUD operation, action, etc.)
///
/// # Returns
///
/// Always returns `Some(ServerMessage::Response { ... })` with the request_id
/// and result from the AppRequestHandler.
pub async fn handle_request(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    payload: RequestPayload,
) -> Option<ServerMessage> {
    // client_id is already a valid Uuid, use it directly as connection_id
    let connection_id = client_id;
    let client_id_str = client_id.to_string();

    tracing::debug!(
        request_id = %request_id,
        connection_id = %connection_id,
        payload_type = ?std::mem::discriminant(&payload),
        "Request received"
    );

    // Get connection context
    let conn_info = state
        .world_connection_manager
        .get_connection(connection_id)
        .await;

    // Build request context
    let ctx = if let Some(info) = &conn_info {
        RequestContext {
            connection_id,
            user_id: info.user_id.clone(),
            world_id: info.world_id,
            role: info.role,
            pc_id: info.pc_id,
            is_dm: info.is_dm(),
            is_spectating: info.is_spectator(),
        }
    } else {
        // Anonymous context for users not in a world
        RequestContext::anonymous(connection_id, client_id_str)
    };

    // Delegate to the AppRequestHandler for all operations
    let result = state.request_handler.handle(payload, ctx).await;

    Some(ServerMessage::Response { request_id, result })
}
