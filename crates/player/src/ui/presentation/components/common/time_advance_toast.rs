//! Time advance toast notification for player view
//!
//! Shows a temporary overlay when game time advances, displaying
//! reason and delta of time change. Respects
//! `show_time_to_players` server flag.

use crate::infrastructure::spawn_task;
use crate::presentation::game_time_format::display_date;
use crate::presentation::state::{use_game_state, use_session_state};
use dioxus::prelude::*;
use std::time::Duration;

/// Format seconds into human-readable time delta
fn format_delta(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours > 0 && minutes > 0 {
        format!("+{}h {}m", hours, minutes)
    } else if hours > 0 {
        format!("+{} hour{}", hours, if hours > 1 { "s" } else { "" })
    } else if minutes > 0 {
        format!("+{} minute{}", minutes, if minutes > 1 { "s" } else { "" })
    } else {
        format!("+{} sec", seconds)
    }
}

/// Time advance toast component
///
/// Auto-dismisses after 5 seconds or can be manually dismissed.
/// Only shows if `show_time_to_players` flag is enabled.
#[component]
pub fn TimeAdvanceToast() -> Element {
    let game_state = use_game_state();
    let session_state = use_session_state();

    let notification = game_state.time_advance_notification.read().clone();

    // Hook for auto-dismiss timer - clone notification for effect to avoid move
    let game_state_for_dismiss = game_state.clone();
    let game_state_for_button = game_state.clone();  // Clone for second closure
    let notification_for_effect = notification.clone();

    use_effect(move || {
        if notification_for_effect.is_some() {
            // Auto-dismiss after 5 seconds
            let mut game_state = game_state_for_dismiss.clone();
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
                        class: "bg-slate-900/95 text-white rounded-lg shadow-2xl p-4 max-w-sm border border-slate-700 animate-slide-up",
                        onclick: move |_| {
                            let mut gs = game_state.clone();
                            gs.clear_time_advance_notification();
                        },

                        // Header
                        div {
                            class: "flex items-center gap-2 mb-3 pb-2 border-b border-slate-700",
                            span { class: "text-xl", "⏰" }
                            h3 { class: "text-lg font-semibold", "Time Advanced" }
                        }

                        // Reason
                        if !data.reason.is_empty() {
                            p {
                                class: "text-sm mb-3 text-slate-200",
                                "{data.reason}"
                            }
                        }

                        // Time delta
                        div {
                            class: "text-2xl font-bold text-blue-400 mb-3",
                            "{format_delta(data.seconds_advanced)}"
                        }

                        // Current time display
                        div {
                            class: "text-sm text-slate-400 mb-4",
                            "Current time: ",
                            span { class: "text-white font-medium", "{display_date(&data.new_time)}" }
                        }

                        // Period change highlight
                        if data.period_changed {
                            if let Some(ref period) = data.new_period {
                                div {
                                    class: "flex items-center gap-2 text-sm text-yellow-300 mb-4",
                                    span { "🌅" }
                                    span { "It is now {period}" }
                                }
                            }
                        }

                        // Dismiss button
                        button {
                            class: "w-full py-2 bg-slate-700 hover:bg-slate-600 text-white rounded border border-slate-600 transition-colors",
                            onclick: move |e| {
                                e.stop_propagation();
                                let mut gs = game_state_for_button.clone();
                                gs.clear_time_advance_notification();
                            },
                            "Dismiss"
                        }
                    }
                }
            }
        }
    }
}
