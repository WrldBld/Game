//! Shared connection logic for all world-scoped routes
//!
//! This module provides centralized WebSocket connection handling used by
//! WorldSessionLayout. It ensures consistent connection behavior across
//! all roles (DM, Player, Spectator).

use dioxus::prelude::*;

use crate::presentation::state::{
    ConnectionStatus, DialogueState, GameState, GenerationState, SessionState,
};
use wrldbldr_player_adapters::Platform;
use wrldbldr_player_app::application::services::{SessionService, DEFAULT_ENGINE_URL};
use wrldbldr_player_ports::outbound::storage_keys;
use wrldbldr_player_ports::session_types::ParticipantRole;

/// Ensure a WebSocket connection is established for the given world and role.
///
/// This function checks the current connection status and only initiates
/// a new connection if we're disconnected or failed. This prevents duplicate
/// connections when navigating between views.
pub fn ensure_connection(
    world_id: &str,
    role: ParticipantRole,
    session_state: SessionState,
    game_state: GameState,
    dialogue_state: DialogueState,
    generation_state: GenerationState,
    platform: Platform,
) {
    let status = *session_state.connection_status().read();

    // Only attempt a new connection if we're not already connecting/connected
    if matches!(
        status,
        ConnectionStatus::Connecting | ConnectionStatus::Connected | ConnectionStatus::Reconnecting
    ) {
        return;
    }

    // Load server URL from storage or use default
    let server_url = platform
        .storage_load(storage_keys::SERVER_URL)
        .unwrap_or_else(|| DEFAULT_ENGINE_URL.to_string());
    platform.storage_save(storage_keys::SERVER_URL, &server_url);

    // Configure Engine HTTP base URL from the WebSocket URL
    platform.configure_engine_url(&server_url);

    // Use the stable anonymous user ID from storage
    let user_id = platform.get_user_id();

    initiate_connection(
        server_url,
        user_id,
        role,
        world_id.to_string(),
        session_state,
        game_state,
        dialogue_state,
        generation_state,
        platform,
    );
}

/// Initiate WebSocket connection (platform-agnostic)
///
/// This spawns an async task that:
/// 1. Creates a GameConnectionPort via the platform
/// 2. Sets up SessionService with event callbacks
/// 3. Processes events in a loop until the connection closes
fn initiate_connection(
    server_url: String,
    user_id: String,
    role: ParticipantRole,
    world_id: String,
    mut session_state: SessionState,
    mut game_state: GameState,
    mut dialogue_state: DialogueState,
    mut generation_state: GenerationState,
    platform: Platform,
) {
    // Update session state to connecting
    session_state.start_connecting(&server_url);
    session_state.set_user(user_id.clone(), role);

    // Spawn async task to handle connection
    spawn(async move {
        use futures_util::StreamExt;

        // Use the platform's connection factory to create a game connection
        let connection = platform.create_game_connection(&server_url);
        session_state.set_connection_handle(connection.clone());
        let session_service = SessionService::new(connection.clone());

        match session_service.connect(user_id, role, world_id).await {
            Ok(mut rx) => {
                // Process events from the stream
                while let Some(event) = rx.next().await {
                    crate::presentation::handlers::handle_session_event(
                        event,
                        &mut session_state,
                        &mut game_state,
                        &mut dialogue_state,
                        &mut generation_state,
                        &platform,
                    );
                }

                tracing::info!("Event channel closed");
            }
            Err(e) => {
                tracing::error!("Connection failed: {}", e);
                session_state.set_failed(e.to_string());
            }
        }
    });
}

/// Handle disconnection and cleanup
///
/// Disconnects the WebSocket client and clears all session-related state.
pub fn handle_disconnect(
    mut session_state: SessionState,
    mut game_state: GameState,
    mut dialogue_state: DialogueState,
) {
    // Disconnect client if present
    if let Some(client) = session_state.engine_client().read().as_ref() {
        client.disconnect();
    }

    // Clear all state
    session_state.clear();
    game_state.clear();
    dialogue_state.clear();
}
