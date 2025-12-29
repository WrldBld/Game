//! Session event handler
//!
//! This module handles SessionEvent from the application layer and updates
//! presentation state accordingly. This is where the application-to-presentation
//! boundary is properly maintained.

use wrldbldr_player_app::application::services::port_connection_state_to_status;
use wrldbldr_player_app::application::services::SessionEvent;
use wrldbldr_player_adapters::Platform;
use wrldbldr_player_ports::outbound::ConnectionState as PortConnectionState;

use crate::presentation::handlers::handle_server_message;
use crate::presentation::state::{
    ConnectionStatus, DialogueState, GameState, GenerationState, SessionState,
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
    platform: &Platform,
) {
    match event {
        SessionEvent::StateChanged(state) => {
            // Convert application connection state to presentation status
            let status = port_connection_state_to_status(state);

            // Map application status to presentation status
            let presentation_status = match status {
                wrldbldr_player_app::application::dto::AppConnectionStatus::Disconnected => {
                    ConnectionStatus::Disconnected
                }
                wrldbldr_player_app::application::dto::AppConnectionStatus::Connecting => {
                    ConnectionStatus::Connecting
                }
                wrldbldr_player_app::application::dto::AppConnectionStatus::Connected => {
                    ConnectionStatus::Connected
                }
                wrldbldr_player_app::application::dto::AppConnectionStatus::Reconnecting => {
                    ConnectionStatus::Reconnecting
                }
                wrldbldr_player_app::application::dto::AppConnectionStatus::Failed => {
                    ConnectionStatus::Failed
                }
            };

            session_state.connection_status().set(presentation_status);

            if matches!(
                state,
                PortConnectionState::Disconnected | PortConnectionState::Failed
            ) {
                session_state.engine_client().set(None);
            }
        }
        SessionEvent::MessageReceived(message) => {
            handle_server_message(
                message,
                session_state,
                game_state,
                dialogue_state,
                generation_state,
                platform,
            );
        }
    }
}
