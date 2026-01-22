//! Time advance toast notification for player view
//!
//! Shows a temporary overlay when game time advances, displaying
//! reason and delta of the time change. Respects the
//! `show_time_to_players` server flag.

use crate::infrastructure::spawn_task;
use crate::presentation::game_time_format::display_time;
use crate::presentation::state::{use_game_state, use_session_state};
use dioxus::prelude::*;
use std::time::Duration;

/// Time advance toast component
///
/// Auto-dismisses after 5 seconds or can be manually dismissed.
/// Only shows if `show_time_to_players` flag is enabled.
#[component]
pub fn TimeAdvanceToast() -> Element {
    let game_state = use_game_state();
    let session_state = use_session_state();

    let notification = game_state.time_advance_notification.read().clone();

    // Hook for auto-dismiss timer - clone notification for the effect to avoid move
    let game_state_dismiss = game_state.clone();
    let notification_for_effect = notification.clone();

    use_effect(move || {
        if notification_for_effect.is_some() {
            // Auto-dismiss after 5 seconds
            let mut game_state = game_state_dismiss.clone();
            spawn_task(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                game_state.clear_time_advance_notification();
            });
        }
    });

    // Only show if there's a notification and show_time_to_players is enabled
    let should_show = notification.is_some() && *session_state.should_show_time_to_players().read();

    rsx! {
        if should_show {
            if let Some(ref data) = notification {
                div {
                    class: "fixed bottom-4 right-4 z-50 pointer-events-auto transition-all duration-300",
                    div {
                        class: "bg-gradient-to-r from-blue-900 to-purple-900 text-white rounded-lg shadow-lg p-4 max-w-md animate-slide-up",
                        onclick: move |_| {
                            let mut gs = game_state.clone();
                            gs.clear_time_advance_notification();
                        },

                        // Header with close icon
                        div {
                            class: "flex items-start justify-between mb-2",
                            div {
                                class: "flex items-center gap-2",
                                span { class: "text-xl", "🕐" } // Clock icon
                                h3 { class: "text-lg font-semibold", "Time Advanced" }
                            }
                            button {
                                class: "text-white/70 hover:text-white transition-colors ml-2",
                                "×"
                            }
                        }

                        // Reason
                        if !data.reason.is_empty() {
                            p {
                                class: "text-sm mb-2 text-white/90",
                                "{data.reason}"
                            }
                        }

                        // Time change display
                        div {
                            class: "flex items-center gap-3 text-sm",
                            // Previous time
                            div {
                                class: "text-white/70",
                                "From: ",
                                span { class: "text-white font-mono", "{display_time(&data.previous_time)}" }
                            }

                            // Arrow
                            span { class: "text-white/50 text-lg", "→" }

                            // New time
                            div {
                                class: "text-green-300",
                                "To: ",
                                span { class: "text-white font-mono", "{display_time(&data.new_time)}" }
                            }
                        }

                        // Period change highlight
                        if data.period_changed {
                            div {
                                class: "mt-2 pt-2 border-t border-white/20",
                                if let Some(ref period) = data.new_period {
                                    div {
                                        class: "flex items-center gap-2 text-sm",
                                        span { class: "text-yellow-300", "🌅" } // Sun icon
                                        span { class: "text-yellow-200", "It is now {period}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
