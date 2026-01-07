//! Time Control Panel for DM
//!
//! Displays current game time and provides controls for:
//! - Viewing current time and period
//! - Approving/modifying/skipping time suggestions
//! - Manually advancing time
//! - Changing time mode (Manual/Suggested)

use dioxus::prelude::*;

use crate::presentation::game_time_format::{display_date, display_time, time_of_day};
use crate::presentation::state::{use_game_state, use_session_state, TimeMode, TimeSuggestionData};
use crate::application::dto::GameTime;

/// Time Control Panel component for DM view
#[component]
pub fn TimeControlPanel() -> Element {
    let game_state = use_game_state();
    let session_state = use_session_state();

    let game_time = game_state.game_time.read().clone();
    let time_mode = game_state.time_mode.read().clone();
    let time_paused = *game_state.time_paused.read();
    let pending_suggestions = game_state.pending_time_suggestions.read().clone();

    // State for modals
    let mut show_advance_modal = use_signal(|| false);
    let mut show_set_time_modal = use_signal(|| false);

    rsx! {
        div {
            class: "time-control-panel bg-dark-surface rounded-lg p-4",

            // Header with current time
            div {
                class: "flex items-center justify-between mb-4",

                h3 { class: "text-gray-400 text-sm uppercase m-0", "Game Time" }

                // Pause indicator
                if time_paused {
                    span {
                        class: "text-amber-500 text-xs px-2 py-1 bg-amber-500/20 rounded",
                        "PAUSED"
                    }
                }
            }

            // Current time display
            if let Some(ref gt) = game_time {
                TimeDisplay { game_time: gt.clone() }
            } else {
                div { class: "text-gray-500 italic text-center py-4", "Time not set" }
            }

            // Time mode indicator
            div {
                class: "mt-3 flex items-center gap-2 text-sm",
                span { class: "text-gray-500", "Mode:" }
                span {
                    class: match time_mode {
                        TimeMode::Manual => "text-gray-400",
                        TimeMode::Suggested => "text-blue-400",
                    },
                    "{time_mode.display_name()}"
                }
            }

            // Pending time suggestions
            if !pending_suggestions.is_empty() {
                div {
                    class: "mt-4 border-t border-gray-700 pt-4",

                    h4 {
                        class: "text-amber-400 text-sm mb-3 flex items-center gap-2",
                        span { class: "text-lg", "" }
                        "Pending Time Suggestions ({pending_suggestions.len()})"
                    }

                    div {
                        class: "flex flex-col gap-2",
                        for suggestion in pending_suggestions.iter() {
                            TimeSuggestionCard {
                                key: "{suggestion.suggestion_id}",
                                suggestion: suggestion.clone(),
                            }
                        }
                    }
                }
            }

            // Quick actions
            div {
                class: "mt-4 flex flex-wrap gap-2",

                button {
                    onclick: move |_| show_advance_modal.set(true),
                    class: "px-3 py-1.5 bg-blue-600 hover:bg-blue-500 text-white text-sm rounded transition-colors",
                    "+1 Hour"
                }

                button {
                    onclick: move |_| {
                        // Quick skip to next period
                        if let Some(client) = session_state.engine_client().read().as_ref() {
                            if let Some(ref gt) = game_time {
                                if let Some(world_id) = *session_state.world_id().read() {
                                    let next_period = time_of_day(gt.clone()).to_string();
                                    let _ = client.skip_to_period(&world_id.to_string(), &next_period);
                                }
                            }
                        }
                    },
                    class: "px-3 py-1.5 bg-purple-600 hover:bg-purple-500 text-white text-sm rounded transition-colors",
                    "Skip to Period"
                }

                button {
                    onclick: move |_| show_set_time_modal.set(true),
                    class: "px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded transition-colors",
                    "Set Time"
                }
            }

            // Advance time modal
            if *show_advance_modal.read() {
                AdvanceTimeModal {
                    on_close: move |_| show_advance_modal.set(false),
                }
            }

            // Set time modal
            if *show_set_time_modal.read() {
                SetTimeModal {
                    current_time: game_time.clone(),
                    on_close: move |_| show_set_time_modal.set(false),
                }
            }
        }
    }
}

/// Displays current game time with period icon
#[component]
fn TimeDisplay(game_time: GameTime) -> Element {
    let period = time_of_day(game_time.clone());
    let time_str = display_time(game_time.clone());
    let _date_str = display_date(game_time.clone());

    let period_icon = match period {
        crate::presentation::game_time_format::TimeOfDay::Morning => "",
        crate::presentation::game_time_format::TimeOfDay::Afternoon => "",
        crate::presentation::game_time_format::TimeOfDay::Evening => "",
        crate::presentation::game_time_format::TimeOfDay::Night => "",
    };

    let period_color = match period {
        crate::presentation::game_time_format::TimeOfDay::Morning => "text-yellow-400",
        crate::presentation::game_time_format::TimeOfDay::Afternoon => "text-orange-400",
        crate::presentation::game_time_format::TimeOfDay::Evening => "text-purple-400",
        crate::presentation::game_time_format::TimeOfDay::Night => "text-blue-400",
    };

    rsx! {
        div {
            class: "bg-dark-bg rounded-lg p-4 text-center",

            // Period icon and name
            div {
                class: "flex items-center justify-center gap-2 mb-2",
                span { class: "text-2xl {period_color}", "{period_icon}" }
                span { class: "text-lg {period_color}", "{period}" }
            }

            // Time display
            div {
                class: "text-2xl text-white font-mono",
                "{time_str}"
            }

            // Day display
            div {
                class: "text-gray-400 text-sm mt-1",
                "Day {game_time.day}"
            }
        }
    }
}

/// Card for a pending time suggestion
#[component]
fn TimeSuggestionCard(suggestion: TimeSuggestionData) -> Element {
    let session_state = use_session_state();
    let mut game_state = use_game_state();
    let mut custom_minutes = use_signal(|| suggestion.suggested_minutes);

    let current_display = display_time(suggestion.current_time.clone());
    let resulting_display = display_time(suggestion.resulting_time.clone());

    let suggestion_id_approve = suggestion.suggestion_id.clone();
    let suggestion_id_skip = suggestion.suggestion_id.clone();

    // Clone for the second closure
    let session_state_skip = session_state.clone();
    let mut game_state_skip = game_state.clone();

    rsx! {
        div {
            class: "bg-dark-bg rounded-lg p-3 border border-amber-500/30",

            // Header
            div {
                class: "flex items-start justify-between mb-2",

                div {
                    span { class: "text-white font-medium", "{suggestion.pc_name}" }
                    span { class: "text-gray-400 text-sm ml-2", "{suggestion.action_description}" }
                }
            }

            // Time change display
            div {
                class: "flex items-center gap-2 text-sm mb-3",
                span { class: "text-gray-400", "{current_display}" }
                span { class: "text-gray-500", "" }
                span { class: "text-blue-400", "{resulting_display}" }
                span { class: "text-gray-500 ml-2", "(+{suggestion.suggested_minutes} min)" }
            }

            // Period change warning
            if let Some((from, to)) = &suggestion.period_change {
                div {
                    class: "bg-purple-500/20 text-purple-300 text-xs px-2 py-1 rounded mb-3",
                    "Period change: {from} -> {to}"
                }
            }

            // Modify time input
            div {
                class: "flex items-center gap-2 mb-3",
                label { class: "text-gray-400 text-sm", "Minutes:" }
                input {
                    r#type: "number",
                    value: "{custom_minutes}",
                    min: "0",
                    max: "1440",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<u32>() {
                            custom_minutes.set(v);
                        }
                    },
                    class: "w-20 px-2 py-1 bg-dark-surface border border-gray-600 rounded text-white text-sm",
                }
            }

            // Action buttons
            div {
                class: "flex gap-2",

                button {
                    onclick: move |_| {
                        let minutes = *custom_minutes.read();
                        if let Some(client) = session_state.engine_client().read().as_ref() {
                            // Use the time suggestion response method
                            let _ = client.respond_to_time_suggestion(
                                &suggestion_id_approve,
                                "approve",
                                Some(minutes),
                            );
                        }
                        game_state.remove_time_suggestion(&suggestion_id_approve);
                    },
                    class: "flex-1 px-3 py-1.5 bg-green-600 hover:bg-green-500 text-white text-sm rounded transition-colors",
                    "Approve"
                }

                button {
                    onclick: move |_| {
                        if let Some(client) = session_state_skip.engine_client().read().as_ref() {
                            let _ = client.respond_to_time_suggestion(&suggestion_id_skip, "skip", None);
                        }
                        game_state_skip.remove_time_suggestion(&suggestion_id_skip);
                    },
                    class: "px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded transition-colors",
                    "Skip"
                }
            }
        }
    }
}

/// Modal for manually advancing time
#[component]
fn AdvanceTimeModal(on_close: EventHandler<()>) -> Element {
    let session_state = use_session_state();
    let mut hours = use_signal(|| 1u32);
    let mut reason = use_signal(|| "DM advanced time".to_string());

    rsx! {
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| on_close.call(()),

            div {
                class: "bg-dark-surface rounded-lg p-6 w-[400px] max-w-[90vw]",
                onclick: move |e| e.stop_propagation(),

                h2 { class: "text-white text-xl mb-4", "Advance Time" }

                div {
                    class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Hours to advance:" }
                    input {
                        r#type: "number",
                        value: "{hours}",
                        min: "1",
                        max: "24",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<u32>() {
                                hours.set(v.clamp(1, 24));
                            }
                        },
                        class: "w-full px-3 py-2 bg-dark-bg border border-gray-600 rounded text-white",
                    }
                }

                div {
                    class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Reason (optional):" }
                    input {
                        r#type: "text",
                        value: "{reason}",
                        oninput: move |e| reason.set(e.value()),
                        placeholder: "e.g., Party rested, Time skip...",
                        class: "w-full px-3 py-2 bg-dark-bg border border-gray-600 rounded text-white",
                    }
                }

                // Quick presets
                div {
                    class: "flex gap-2 mb-4",
                    button {
                        onclick: move |_| hours.set(1),
                        class: if *hours.read() == 1 { "px-3 py-1 bg-blue-600 text-white text-sm rounded" } else { "px-3 py-1 bg-gray-700 text-gray-300 text-sm rounded" },
                        "1h"
                    }
                    button {
                        onclick: move |_| hours.set(4),
                        class: if *hours.read() == 4 { "px-3 py-1 bg-blue-600 text-white text-sm rounded" } else { "px-3 py-1 bg-gray-700 text-gray-300 text-sm rounded" },
                        "4h"
                    }
                    button {
                        onclick: move |_| hours.set(8),
                        class: if *hours.read() == 8 { "px-3 py-1 bg-blue-600 text-white text-sm rounded" } else { "px-3 py-1 bg-gray-700 text-gray-300 text-sm rounded" },
                        "8h (Rest)"
                    }
                    button {
                        onclick: move |_| hours.set(12),
                        class: if *hours.read() == 12 { "px-3 py-1 bg-blue-600 text-white text-sm rounded" } else { "px-3 py-1 bg-gray-700 text-gray-300 text-sm rounded" },
                        "12h"
                    }
                }

                div {
                    class: "flex gap-2",

                    button {
                        onclick: move |_| {
                            let h = *hours.read();
                            let r = reason.read().clone();
                            if let Some(client) = session_state.engine_client().read().as_ref() {
                                if let Some(world_id) = *session_state.world_id().read() {
                                    let _ = client.advance_time(&world_id.to_string(), h * 60, &r);
                                }
                            }
                            on_close.call(());
                        },
                        class: "flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded transition-colors",
                        "Advance {hours} hour(s)"
                    }

                    button {
                        onclick: move |_| on_close.call(()),
                        class: "px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors",
                        "Cancel"
                    }
                }
            }
        }
    }
}

/// Modal for setting exact time
#[component]
fn SetTimeModal(current_time: Option<GameTime>, on_close: EventHandler<()>) -> Element {
    let session_state = use_session_state();

    let initial_day = current_time.as_ref().map(|t| t.day).unwrap_or(1);
    let initial_hour = current_time.as_ref().map(|t| t.hour).unwrap_or(8);

    let mut day = use_signal(move || initial_day);
    let mut hour = use_signal(move || initial_hour);

    rsx! {
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| on_close.call(()),

            div {
                class: "bg-dark-surface rounded-lg p-6 w-[400px] max-w-[90vw]",
                onclick: move |e| e.stop_propagation(),

                h2 { class: "text-white text-xl mb-4", "Set Game Time" }

                div {
                    class: "grid grid-cols-2 gap-4 mb-4",

                    div {
                        label { class: "block text-gray-400 text-sm mb-1", "Day:" }
                        input {
                            r#type: "number",
                            value: "{day}",
                            min: "1",
                            max: "365",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    day.set(v.clamp(1, 365));
                                }
                            },
                            class: "w-full px-3 py-2 bg-dark-bg border border-gray-600 rounded text-white",
                        }
                    }

                    div {
                        label { class: "block text-gray-400 text-sm mb-1", "Hour (0-23):" }
                        input {
                            r#type: "number",
                            value: "{hour}",
                            min: "0",
                            max: "23",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u8>() {
                                    hour.set(v.min(23));
                                }
                            },
                            class: "w-full px-3 py-2 bg-dark-bg border border-gray-600 rounded text-white",
                        }
                    }
                }

                // Period quick selects
                div {
                    class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-2", "Quick select period:" }
                    div {
                        class: "flex gap-2",
                        button {
                            onclick: move |_| hour.set(6),
                            class: "px-3 py-1 bg-yellow-600/20 text-yellow-400 text-sm rounded hover:bg-yellow-600/30",
                            " Morning (6am)"
                        }
                        button {
                            onclick: move |_| hour.set(12),
                            class: "px-3 py-1 bg-orange-600/20 text-orange-400 text-sm rounded hover:bg-orange-600/30",
                            " Noon (12pm)"
                        }
                        button {
                            onclick: move |_| hour.set(18),
                            class: "px-3 py-1 bg-purple-600/20 text-purple-400 text-sm rounded hover:bg-purple-600/30",
                            " Evening (6pm)"
                        }
                        button {
                            onclick: move |_| hour.set(22),
                            class: "px-3 py-1 bg-blue-600/20 text-blue-400 text-sm rounded hover:bg-blue-600/30",
                            " Night (10pm)"
                        }
                    }
                }

                div {
                    class: "flex gap-2",

                    button {
                        onclick: move |_| {
                            let d = *day.read();
                            let h = *hour.read();
                            if let Some(client) = session_state.engine_client().read().as_ref() {
                                if let Some(world_id) = *session_state.world_id().read() {
                                    let _ = client.set_game_time(&world_id.to_string(), d, h);
                                }
                            }
                            on_close.call(());
                        },
                        class: "flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded transition-colors",
                        "Set Time"
                    }

                    button {
                        onclick: move |_| on_close.call(()),
                        class: "px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors",
                        "Cancel"
                    }
                }
            }
        }
    }
}
