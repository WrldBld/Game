//! WorldSessionLayout - Shared wrapper for all world-scoped routes
//!
//! This component wraps DM, Player, and Spectator views with:
//! - Automatic connection handling on mount
//! - Always-visible connection status bar
//! - Error overlay for connection failures
//! - Back navigation with proper cleanup

use dioxus::prelude::*;

use crate::ports::outbound::storage_keys;
use crate::ports::session_types::ParticipantRole;
use crate::presentation::state::{
    ConnectionStatus, DialogueState, GameState, GenerationState, LoreState, SessionState,
};
use crate::use_platform;
use uuid::Uuid;

use super::connection::{ensure_connection, handle_disconnect};
use super::Route;

/// Props for WorldSessionLayout
#[derive(Props, Clone, PartialEq)]
pub struct WorldSessionLayoutProps {
    /// The world ID to connect to
    pub world_id: String,
    /// The role for this session (DungeonMaster, Player, Spectator)
    pub role: ParticipantRole,
    /// Page title shown in browser tab
    pub page_title: &'static str,
    /// Whether to show the built-in connection status bar (default: true)
    /// Set to false for views that have their own header with status indicator
    #[props(default = true)]
    pub show_status_bar: bool,
    /// Child content to render
    pub children: Element,
}

/// WorldSessionLayout - wraps world views with connection handling
///
/// This component handles:
/// - Connection establishment on mount
/// - Page title management
/// - Connection status display
/// - Error handling
/// - Back navigation with cleanup
#[component]
pub fn WorldSessionLayout(props: WorldSessionLayoutProps) -> Element {
    let navigator = use_navigator();
    let platform = use_platform();
    let session_state = use_context::<SessionState>();
    let game_state = use_context::<GameState>();
    let dialogue_state = use_context::<DialogueState>();
    let generation_state = use_context::<GenerationState>();
    let lore_state = use_context::<LoreState>();

    // Set page title
    let title = props.page_title;
    let platform_for_title = platform.clone();
    use_effect(move || {
        platform_for_title.set_page_title(title);
    });

    // Ensure connection on mount
    let world_id = props.world_id.clone();
    let role = props.role;
    let platform_for_connection = platform.clone();
    let session_state_for_effect = session_state.clone();
    let game_state_for_effect = game_state.clone();
    let dialogue_state_for_effect = dialogue_state.clone();
    let lore_state_for_effect = lore_state.clone();
    use_effect(move || {
        // If we're already connected to a different world (e.g., deep link without a full reload),
        // reset the session first so we can join to the requested world.
        let requested_world = Uuid::parse_str(&world_id).ok();
        let current_world = *session_state_for_effect.world_id().read();

        if *session_state_for_effect.connection_status().read() == ConnectionStatus::Connected
            && requested_world.is_some()
            && current_world.is_some()
            && current_world != requested_world
        {
            handle_disconnect(
                session_state_for_effect.clone(),
                game_state_for_effect.clone(),
                dialogue_state_for_effect.clone(),
                lore_state_for_effect.clone(),
            );
        }

        ensure_connection(
            &world_id,
            role,
            session_state_for_effect.clone(),
            game_state_for_effect.clone(),
            dialogue_state_for_effect.clone(),
            generation_state,
            lore_state_for_effect.clone(),
            platform_for_connection.clone(),
        );
    });

    let connection_status = *session_state.connection_status().read();
    let requested_world = Uuid::parse_str(&props.world_id).ok();
    let current_world = *session_state.world_id().read();
    let is_connected_to_requested_world = connection_status == ConnectionStatus::Connected
        && requested_world.is_some()
        && current_world == requested_world;

    rsx! {
        div {
            class: "world-session-layout h-full flex flex-col bg-dark-bg",

            // Connection status bar
            //
            // Even for DM views that normally render their own header, we show the
            // global status bar while connecting so users have retry/back controls,
            // and (critically) so child views don't mount and fire requests early.
            if props.show_status_bar || !is_connected_to_requested_world {
                ConnectionStatusBar {
                    status: connection_status,
                    on_retry: {
                        let world_id = props.world_id.clone();
                        let role = props.role;
                        let platform = platform.clone();
                        let mut session_state = session_state.clone();
                        let game_state = game_state.clone();
                        let dialogue_state = dialogue_state.clone();
                        let lore_state = lore_state.clone();
                        move |_| {
                            // Force reconnection attempt by setting disconnected first
                            session_state.set_disconnected();
                            ensure_connection(
                                &world_id,
                                role,
                                session_state.clone(),
                                game_state.clone(),
                                dialogue_state.clone(),
                                generation_state,
                                lore_state.clone(),
                                platform.clone(),
                            );
                        }
                    },
                    on_back: {
                        let platform = platform.clone();
                        let session_state = session_state.clone();
                        let game_state = game_state.clone();
                        let dialogue_state = dialogue_state.clone();
                        let lore_state = lore_state.clone();
                        move |_| {
                            handle_disconnect(
                                session_state.clone(),
                                game_state.clone(),
                                dialogue_state.clone(),
                                lore_state.clone(),
                            );
                            platform.storage_remove(storage_keys::LAST_WORLD);
                            navigator.push(Route::RoleSelectRoute {});
                        }
                    },
                }
            }

            // Main content area
            main {
                class: "flex-1 overflow-hidden relative",
                if is_connected_to_requested_world {
                    {props.children}
                } else {
                    div {
                        class: "flex items-center justify-center h-full text-gray-400",
                        "Connecting..."
                    }
                }
            }

            // Error overlay (modal)
            if let Some(error) = session_state.error_message().read().as_ref() {
                ErrorOverlay {
                    message: error.clone(),
                    on_dismiss: {
                        let session_state = session_state.clone();
                        move |_| {
                            session_state.error_message().set(None);
                        }
                    },
                }
            }
        }
    }
}

/// Connection status bar - always visible at top of world views
#[derive(Props, Clone, PartialEq)]
struct ConnectionStatusBarProps {
    status: ConnectionStatus,
    on_retry: EventHandler<()>,
    on_back: EventHandler<()>,
}

#[component]
fn ConnectionStatusBar(props: ConnectionStatusBarProps) -> Element {
    let (indicator_class, status_text) = match props.status {
        ConnectionStatus::Connected => ("bg-green-500", "Connected"),
        ConnectionStatus::Connecting => ("bg-yellow-500 animate-pulse", "Connecting..."),
        ConnectionStatus::Reconnecting => ("bg-yellow-500 animate-pulse", "Reconnecting..."),
        ConnectionStatus::Disconnected => ("bg-red-500", "Disconnected"),
        ConnectionStatus::Failed => ("bg-red-600", "Connection Failed"),
    };

    let can_retry = !matches!(
        props.status,
        ConnectionStatus::Connected | ConnectionStatus::Connecting
    );

    rsx! {
        div {
            class: "connection-status-bar flex items-center justify-between px-4 py-2 bg-dark-surface border-b border-gray-700",

            // Left: Back button
            button {
                onclick: move |_| props.on_back.call(()),
                class: "px-3 py-1.5 text-gray-400 hover:text-white border border-gray-700 rounded text-sm transition-colors",
                "← Back"
            }

            // Right: Status indicator (clickable to retry when disconnected)
            div {
                class: "flex items-center gap-2",
                class: if can_retry { "cursor-pointer" } else { "" },
                onclick: move |_| {
                    if can_retry {
                        props.on_retry.call(());
                    }
                },

                span {
                    class: "w-2.5 h-2.5 rounded-full {indicator_class}",
                }
                span {
                    class: "text-gray-400 text-sm",
                    "{status_text}"
                }
                if can_retry {
                    span {
                        class: "text-gray-500 text-xs ml-1",
                        "(click to retry)"
                    }
                }
            }
        }
    }
}

/// Error overlay modal for connection errors
#[derive(Props, Clone, PartialEq)]
struct ErrorOverlayProps {
    message: String,
    on_dismiss: EventHandler<()>,
}

#[component]
fn ErrorOverlay(props: ErrorOverlayProps) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 bg-black/75 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_dismiss.call(()),

            div {
                class: "bg-dark-surface border border-red-500 rounded-lg p-6 max-w-md m-4",
                onclick: move |e| e.stop_propagation(),

                h3 {
                    class: "text-red-500 m-0 mb-2 text-lg",
                    "Connection Error"
                }
                p {
                    class: "text-gray-300 m-0 mb-4 text-sm",
                    "{props.message}"
                }
                button {
                    onclick: move |_| props.on_dismiss.call(()),
                    class: "bg-gray-700 hover:bg-gray-600 text-white border-none py-2 px-4 rounded cursor-pointer transition-colors",
                    "Dismiss"
                }
            }
        }
    }
}
