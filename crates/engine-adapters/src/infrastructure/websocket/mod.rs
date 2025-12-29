//! WebSocket handler for Player connections
//!
//! This module handles WebSocket communication between the Engine and Player clients.
//! Message types are aligned between Engine and Player for seamless communication.
//!
//! # Module Structure
//!
//! - `mod.rs` - Module re-exports and WebSocket upgrade handler
//! - `dispatch.rs` - Main message dispatch routing ClientMessage to handlers
//! - `converters.rs` - Type conversion helpers
//! - `handlers/` - Handler modules organized by domain:
//!   - `connection.rs` - Connection lifecycle (Join/Leave world)
//!   - `player_action.rs` - Player action processing
//!   - `scene.rs` - Scene changes and directorial updates
//!   - `challenge.rs` - Challenge system handlers
//!   - `narrative.rs` - Narrative event handlers
//!   - `movement.rs` - PC movement between regions/locations
//!   - `staging.rs` - NPC presence staging system
//!   - `inventory.rs` - Item equip/drop/pickup
//!   - `misc.rs` - ComfyUI health, NPC events
//!   - `request.rs` - Generic Request/Response pattern
//!
//! # Architecture
//!
//! The WebSocket handler receives `ClientMessage` variants and produces `ServerMessage`
//! responses. Most CRUD operations use the `Request`/`Response` pattern which is
//! delegated to the `AppRequestHandler` in the engine-ports crate.

mod approval_converters;
mod broadcast_adapter;
pub mod context;
pub mod converters;
pub mod directorial_converters;
pub mod dispatch;
pub mod error_conversion;
pub mod handlers;
mod messages;

pub use error_conversion::IntoServerError;

pub use approval_converters::{
    // Domain to Protocol converters
    domain_tool_to_proto, domain_tools_to_proto,
    domain_challenge_to_proto, domain_challenge_suggestion_to_proto,
    domain_narrative_to_proto, domain_narrative_suggestion_to_proto,
    domain_outcomes_to_proto, domain_decision_to_proto,
    // Protocol to Domain converters
    proto_tool_to_domain, proto_tools_to_domain,
    proto_challenge_to_domain, proto_challenge_suggestion_to_domain,
    proto_narrative_to_domain, proto_narrative_suggestion_to_domain,
    proto_outcomes_to_domain, proto_decision_to_domain,
};

pub use broadcast_adapter::WebSocketBroadcastAdapter;
pub use context::{
    error_response, invalid_id_error, not_found_error, parse_player_character_id, parse_region_id,
    parse_uuid, parse_world_id, DmContext, HandlerContext, PlayerContext,
};

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_protocol::{ClientMessage, ServerMessage};

/// WebSocket upgrade handler - entry point for new connections
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Buffer size for per-connection message channel
/// This provides backpressure when a client is slow to consume messages
const CONNECTION_CHANNEL_BUFFER: usize = 256;

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a unique client ID for this connection
    let client_id = Uuid::new_v4();

    // Create a bounded channel for sending messages to this client
    // This provides backpressure when client is slow to consume messages
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(CONNECTION_CHANNEL_BUFFER);

    tracing::info!("New WebSocket connection established: {}", client_id);

    // Spawn a task to forward messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(msg) => {
                    if let Some(response) =
                        dispatch::handle_message(msg, &state, client_id, tx.clone()).await
                    {
                        match tx.try_send(response) {
                            Ok(_) => {}
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                tracing::warn!(
                                    "Message buffer full for client {}, dropping message",
                                    client_id
                                );
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {}", e);
                    let error = ServerMessage::Error {
                        code: "PARSE_ERROR".to_string(),
                        message: format!("Invalid message format: {}", e),
                    };
                    match tx.try_send(error) {
                        Ok(_) => {}
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            tracing::warn!(
                                "Message buffer full for client {}, dropping error message",
                                client_id
                            );
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            break;
                        }
                    }
                }
            },
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket connection closed by client: {}", client_id);
                break;
            }
            Ok(Message::Ping(_data)) => {
                // Ping/Pong is handled by the send task through the channel
                let _ = tx.try_send(ServerMessage::Pong);
            }
            Err(e) => {
                tracing::error!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Clean up: remove client from world connection
    let client_id_str = client_id.to_string();
    if let Some(connection) = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        if let Some(world_id) = connection.world_id {
            state
                .world_connection_manager
                .unregister_connection(connection.connection_id)
                .await;
            tracing::info!(
                "Client {} (user: {:?}) disconnected from world {}",
                client_id,
                connection.user_id,
                world_id
            );
        }
    }

    // Cancel the send task
    send_task.abort();

    tracing::info!("WebSocket connection terminated: {}", client_id);
}
