//! Session event handler
//!
//! This module handles SessionEvent from the application layer and updates
//! presentation state accordingly. This is where the application-to-presentation
//! boundary is properly maintained.

use crate::application::application::services::port_connection_state_to_status;
use crate::application::application::services::SessionEvent;
use crate::ports::outbound::{ConnectionState as PortConnectionState, PlatformPort};

use crate::presentation::handlers::handle_server_message;
use crate::presentation::state::{
    ConnectionStatus, DialogueState, GameState, GenerationState, LoreState, SessionState,
};
use dioxus::prelude::WritableExt;

/// Process a session event and update presentation state
///
/// This function receives events from the application layer's SessionService
/// and updates the presentation layer's state signals accordingly.
pub fn handle_session_event(
    event: SessionEvent,
    session_state: &mut SessionState,
    game_state: &mut GameState,
    dialogue_state: &mut DialogueState,
    generation_state: &mut GenerationState,
    lore_state: &mut LoreState,
    platform: &dyn PlatformPort,
) {
    match event {
        SessionEvent::StateChanged(state) => {
            // Convert application connection state to presentation status
            let status = port_connection_state_to_status(state);

            // Map application status to presentation status
            let presentation_status = match status {
                crate::application::application::dto::AppConnectionStatus::Disconnected => {
                    ConnectionStatus::Disconnected
                }
                crate::application::application::dto::AppConnectionStatus::Connecting => {
                    ConnectionStatus::Connecting
                }
                crate::application::application::dto::AppConnectionStatus::Connected => {
                    ConnectionStatus::Connected
                }
                crate::application::application::dto::AppConnectionStatus::Reconnecting => {
                    ConnectionStatus::Reconnecting
                }
                crate::application::application::dto::AppConnectionStatus::Failed => {
                    ConnectionStatus::Failed
                }
            };

            session_state.connection_status().set(presentation_status);

            // Clear all state on disconnect or failure to prevent stale data
            if matches!(
                state,
                PortConnectionState::Disconnected | PortConnectionState::Failed
            ) {
                session_state.engine_client().set(None);
                game_state.clear();
                dialogue_state.clear();
                generation_state.clear();
                lore_state.clear();
                session_state.clear();
                tracing::info!("Connection lost - cleared all session state");
            }
        }
        SessionEvent::MessageReceived(message) => {
            handle_server_message(
                message,
                session_state,
                game_state,
                dialogue_state,
                generation_state,
                lore_state,
                platform,
            );
        }
    }
}
