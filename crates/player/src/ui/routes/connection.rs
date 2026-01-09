//! Shared connection logic for all world-scoped routes
//!
//! This module provides centralized WebSocket connection handling used by
//! WorldSessionLayout. It ensures consistent connection behavior across
//! all roles (DM, Player, Spectator).

use dioxus::prelude::*;

use crate::application::services::{SessionService, DEFAULT_ENGINE_URL};
use crate::ports::outbound::storage_keys;
use crate::ports::session_types::ParticipantRole;
use crate::presentation::state::{
    ConnectionStatus, DialogueState, GameState, GenerationState, LoreState, SessionState,
};
use crate::Platform;
use uuid::Uuid;

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
    lore_state: LoreState,
    platform: Platform,
) {
    let status = *session_state.connection_status().read();

    // If we're already connected (or in-flight) but for a different world/role,
    // we must restart the connection. Otherwise the Engine will keep using the
    // previous server-side role and DM-only actions will be rejected.
    let desired_world = Uuid::parse_str(world_id).ok();
    let current_world = *session_state.world_id().read();
    let current_role = *session_state.user_role().read();
    let world_mismatch =
        current_world.is_some() && desired_world.is_some() && current_world != desired_world;
    let role_mismatch = current_role.is_some() && current_role != Some(role);
    let needs_restart = world_mismatch || role_mismatch;

    if needs_restart {
        // Request disconnect from the session service if stored
        // The session service handles its own cleanup
        let mut session_state_reset = session_state.clone();
        session_state_reset.set_disconnected();
    }

    // Only attempt a new connection if we're not already connecting/connected
    if !needs_restart
        && matches!(
            status,
            ConnectionStatus::Connecting
                | ConnectionStatus::Connected
                | ConnectionStatus::Reconnecting
        )
    {
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
        lore_state,
        platform,
    );
}

/// Initiate WebSocket connection (platform-agnostic)
///
/// This spawns an async task that:
/// 1. Creates a SessionService with the server URL
/// 2. Subscribes to events with auto-join
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
    mut lore_state: LoreState,
    platform: Platform,
) {
    // Update session state to connecting
    session_state.start_connecting(&server_url);
    session_state.set_user(user_id.clone(), role);

    // Spawn async task to handle connection
    spawn(async move {
        use futures_util::StreamExt;

        // Create session service with URL - this establishes the connection
        let session_service = SessionService::new(&server_url);

        // Subscribe to events with auto-join behavior
        let mut rx = session_service
            .subscribe_with_auto_join(user_id, role.into(), world_id)
            .await;

        // Process events from the stream
        while let Some(event) = rx.next().await {
            crate::presentation::handlers::handle_session_event(
                event,
                &mut session_state,
                &mut game_state,
                &mut dialogue_state,
                &mut generation_state,
                &mut lore_state,
                platform.as_ref(),
            );
        }

        tracing::info!("Event channel closed");
    });
}

/// Handle disconnection and cleanup
///
/// Clears all session-related state. The session service handles
/// actual WebSocket disconnection when dropped.
pub fn handle_disconnect(
    mut session_state: SessionState,
    mut game_state: GameState,
    mut dialogue_state: DialogueState,
    mut lore_state: LoreState,
) {
    // Clear all state - the session service will disconnect when dropped
    session_state.clear();
    game_state.clear();
    dialogue_state.clear();
    lore_state.clear();
}
