//! Shared connection logic for all world-scoped routes
//!
//! This module provides centralized WebSocket connection handling used by
//! WorldSessionLayout. It ensures consistent connection behavior across
//! all roles (DM, Player, Spectator).

use dioxus::prelude::*;

use crate::application::services::DEFAULT_ENGINE_URL;
use crate::infrastructure::spawn_task;
use crate::infrastructure::messaging::{CommandBus, ConnectionState, ConnectionStateObserver, EventBus};
use crate::infrastructure::session_type_converters::participant_role_to_world_role;
use crate::infrastructure::websocket::ClientMessageBuilder;
use crate::ports::outbound::player_events::PlayerEvent;
use crate::ports::outbound::storage_keys;
use crate::ports::session_types::ParticipantRole;
use crate::presentation::services::{use_command_bus, use_event_bus, use_state_observer};
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

    // Need to restart if switching to a different world or role
    let needs_restart = world_mismatch || role_mismatch;

    if needs_restart {
        tracing::info!(
            ?current_world,
            ?desired_world,
            ?current_role,
            ?role,
            "Connection restart needed - clearing state"
        );
        // Request disconnect from the session service if stored
        // The session service handles its own cleanup
        let mut session_state_reset = session_state.clone();
        session_state_reset.set_disconnected();
        // After setting disconnected, we'll proceed to initiate a new connection
    }

    // Skip if we're already connecting/connected (unless we need to restart).
    // The SessionService's monitoring loop will handle sending JoinWorld when
    // the connection becomes ready.
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

    // Get the shared connection components from Services context
    let command_bus = use_command_bus();
    let event_bus = use_event_bus();
    let state_observer = use_state_observer();

    initiate_connection(
        user_id,
        role,
        world_id.to_string(),
        command_bus,
        event_bus,
        state_observer,
        session_state,
        game_state,
        dialogue_state,
        generation_state,
        lore_state,
        platform,
    );
}

/// Initiate world join using the shared WebSocket connection.
///
/// This spawns an async task that:
/// 1. Subscribes to events on the shared connection
/// 2. Sends JoinWorld when connected
/// 3. Processes events in a loop
fn initiate_connection(
    user_id: String,
    role: ParticipantRole,
    world_id: String,
    command_bus: CommandBus,
    event_bus: EventBus,
    state_observer: ConnectionStateObserver,
    mut session_state: SessionState,
    mut game_state: GameState,
    mut dialogue_state: DialogueState,
    mut generation_state: GenerationState,
    mut lore_state: LoreState,
    platform: Platform,
) {
    // Update session state to connecting
    session_state.start_connecting("shared");
    session_state.set_user(user_id.clone(), role);

    // Set up event subscription for this world session
    #[cfg(not(target_arch = "wasm32"))]
    {
        use futures_channel::mpsc;

        let (tx, mut rx) = mpsc::unbounded::<crate::application::services::SessionEvent>();

        // Subscribe to events
        let tx_for_events = tx.clone();
        let event_bus_clone = event_bus.clone();
        spawn_task(async move {
            event_bus_clone
                .subscribe(move |event: PlayerEvent| {
                    let _ = tx_for_events.unbounded_send(
                        crate::application::services::SessionEvent::MessageReceived(event),
                    );
                })
                .await;
        });

        // Set up state monitoring and auto-join
        let state_observer_clone = state_observer.clone();
        let command_bus_clone = command_bus.clone();
        let tx_for_state = tx.clone();
        let world_id_clone = world_id.clone();
        let user_id_clone = user_id.clone();

        spawn_task(async move {
            let mut last_state = state_observer_clone.state();
            let mut join_sent = false;

            // If already connected, send JoinWorld immediately
            if last_state == ConnectionState::Connected {
                if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id_clone) {
                    let proto_role: wrldbldr_protocol::ParticipantRole = role.into();
                    let world_role = participant_role_to_world_role(proto_role);
                    tracing::info!(
                        ?role,
                        ?world_role,
                        world_id = %world_uuid,
                        user_id = %user_id_clone,
                        "Sending JoinWorld message (native) - already connected"
                    );
                    let _ = command_bus_clone.send(ClientMessageBuilder::join_world(
                        world_uuid,
                        world_role,
                        user_id_clone.clone(),
                        None,
                        None,
                    ));
                    join_sent = true;
                }
            }

            loop {
                let current_state = state_observer_clone.state();
                if current_state != last_state {
                    let _ = tx_for_state.unbounded_send(
                        crate::application::services::SessionEvent::StateChanged(current_state),
                    );

                    // Auto-join when connected (if not already sent)
                    if current_state == ConnectionState::Connected && !join_sent {
                        if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id_clone) {
                            let proto_role: wrldbldr_protocol::ParticipantRole = role.into();
                            let world_role = participant_role_to_world_role(proto_role);
                            tracing::info!(
                                ?role,
                                ?world_role,
                                world_id = %world_uuid,
                                user_id = %user_id_clone,
                                "Sending JoinWorld message (native)"
                            );
                            let _ = command_bus_clone.send(ClientMessageBuilder::join_world(
                                world_uuid,
                                world_role,
                                user_id_clone.clone(),
                                None,
                                None,
                            ));
                            join_sent = true;
                        }
                    }

                    last_state = current_state;
                }

                tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                if matches!(
                    current_state,
                    ConnectionState::Disconnected | ConnectionState::Failed
                ) {
                    break;
                }
            }
        });

        // Process events
        spawn_task(async move {
            use futures_util::StreamExt;
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

    #[cfg(target_arch = "wasm32")]
    {
        use futures_channel::mpsc;

        let (tx, mut rx) = mpsc::unbounded::<crate::application::services::SessionEvent>();

        // Subscribe to events (WASM subscribe is sync)
        let tx_for_events = tx.clone();
        event_bus.subscribe(move |event: PlayerEvent| {
            let _ = tx_for_events.unbounded_send(
                crate::application::services::SessionEvent::MessageReceived(event),
            );
        });

        // Set up state monitoring and auto-join
        let state_observer_clone = state_observer.clone();
        let command_bus_clone = command_bus.clone();
        let tx_for_state = tx.clone();
        let world_id_clone = world_id.clone();
        let user_id_clone = user_id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let mut last_state = state_observer_clone.state();
            let mut join_sent = false;

            // If already connected, send JoinWorld immediately
            if last_state == ConnectionState::Connected {
                if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id_clone) {
                    let proto_role: wrldbldr_protocol::ParticipantRole = role.into();
                    let world_role = participant_role_to_world_role(proto_role);
                    tracing::info!(
                        ?role,
                        ?world_role,
                        world_id = %world_uuid,
                        user_id = %user_id_clone,
                        "Sending JoinWorld message (WASM) - already connected"
                    );
                    let _ = command_bus_clone.send(ClientMessageBuilder::join_world(
                        world_uuid,
                        world_role,
                        user_id_clone.clone(),
                        None,
                        None,
                    ));
                    join_sent = true;
                }
            }

            loop {
                let current_state = state_observer_clone.state();
                if current_state != last_state {
                    let _ = tx_for_state.unbounded_send(
                        crate::application::services::SessionEvent::StateChanged(current_state),
                    );

                    // Auto-join when connected (if not already sent)
                    if current_state == ConnectionState::Connected && !join_sent {
                        if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id_clone) {
                            let proto_role: wrldbldr_protocol::ParticipantRole = role.into();
                            let world_role = participant_role_to_world_role(proto_role);
                            tracing::info!(
                                ?role,
                                ?world_role,
                                world_id = %world_uuid,
                                user_id = %user_id_clone,
                                "Sending JoinWorld message (WASM)"
                            );
                            let _ = command_bus_clone.send(ClientMessageBuilder::join_world(
                                world_uuid,
                                world_role,
                                user_id_clone.clone(),
                                None,
                                None,
                            ));
                            join_sent = true;
                        }
                    }

                    last_state = current_state;
                }

                gloo_timers::future::TimeoutFuture::new(50).await;

                if matches!(
                    current_state,
                    ConnectionState::Disconnected | ConnectionState::Failed
                ) {
                    break;
                }
            }
        });

        // Process events
        spawn_task(async move {
            use futures_util::StreamExt;
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
