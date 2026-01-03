//! WebSocket handling.

use std::sync::Arc;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};

use crate::app::App;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<Arc<App>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn handle_socket(socket: WebSocket, _app: Arc<App>) {
    let (mut sender, mut receiver) = socket.split();

    // TODO: Generate client ID, register with connection manager

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // TODO: Parse message, dispatch to appropriate use case
                tracing::debug!("Received: {}", text);
                
                // Echo for now
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // TODO: Unregister from connection manager
}
