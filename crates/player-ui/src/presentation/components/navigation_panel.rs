//! Navigation Panel - Player UI for moving between regions and locations
//!
//! Displays navigation options for the current region:
//! - Connected regions within the same location
//! - Exits to other locations
//! - Game time display

use dioxus::prelude::*;

use wrldbldr_player_app::application::dto::{GameTime, NavigationData, NavigationExit, NavigationTarget};

use crate::presentation::game_time_format;

/// Props for NavigationPanel component
#[derive(Props, Clone, PartialEq)]
pub struct NavigationPanelProps {
    /// Navigation data from the current region
    pub navigation: NavigationData,
    /// Current region name for display
    pub current_region_name: String,
    /// Current location name for display
    pub current_location_name: String,
    /// Handler for moving to a region
    pub on_move_to_region: EventHandler<String>,
    /// Handler for exiting to a location
    pub on_exit_to_location: EventHandler<(String, String)>, // (location_id, arrival_region_id)
    /// Handler for closing the panel
    pub on_close: EventHandler<()>,
    /// Whether navigation is disabled (e.g., during LLM processing)
    #[props(default = false)]
    pub disabled: bool,
}

/// Navigation Panel - Modal overlay for navigation options
#[component]
pub fn NavigationPanel(props: NavigationPanelProps) -> Element {
    let has_regions = !props.navigation.connected_regions.is_empty();
    let has_exits = !props.navigation.exits.is_empty();
    let has_any_navigation = has_regions || has_exits;

    rsx! {
        // Modal overlay
        div {
            class: "navigation-panel-overlay fixed inset-0 bg-black/80 z-[1000] flex items-center justify-center p-4",
            onclick: move |_| props.on_close.call(()),

            // Modal content
            div {
                class: "navigation-panel bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl w-full max-w-lg max-h-[80vh] overflow-hidden flex flex-col shadow-2xl border border-white/10",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-4 border-b border-white/10 flex items-center justify-between",

                    div {
                        h2 {
                            class: "text-xl font-bold text-white m-0",
                            "Navigation"
                        }
                        p {
                            class: "text-gray-400 text-sm m-0 mt-1",
                            "📍 {props.current_region_name}"
                        }
                    }

                    button {
                        class: "w-8 h-8 flex items-center justify-center bg-white/5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition-colors",
                        onclick: move |_| props.on_close.call(()),
                        "✕"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    if !has_any_navigation {
                        // No navigation options
                        div {
                            class: "text-center py-8",
                            p {
                                class: "text-gray-400",
                                "No navigation options available from this location."
                            }
                        }
                    } else {
                        // Connected regions section
                        if has_regions {
                            div {
                                class: "mb-6",

                                h3 {
                                    class: "text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3",
                                    "Move Within {props.current_location_name}"
                                }

                                div {
                                    class: "space-y-2",

                                    for target in props.navigation.connected_regions.iter() {
                                        RegionButton {
                                            key: "{target.region_id}",
                                            target: target.clone(),
                                            disabled: props.disabled,
                                            on_click: {
                                                let on_move = props.on_move_to_region.clone();
                                                let region_id = target.region_id.clone();
                                                move |_| on_move.call(region_id.clone())
                                            },
                                        }
                                    }
                                }
                            }
                        }

                        // Exits section
                        if has_exits {
                            div {
                                h3 {
                                    class: "text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3",
                                    "Exit to Other Locations"
                                }

                                div {
                                    class: "space-y-2",

                                    for exit in props.navigation.exits.iter() {
                                        ExitButton {
                                            key: "{exit.location_id}",
                                            exit: exit.clone(),
                                            disabled: props.disabled,
                                            on_click: {
                                                let on_exit = props.on_exit_to_location.clone();
                                                let location_id = exit.location_id.clone();
                                                let arrival_region_id = exit.arrival_region_id.clone();
                                                move |_| on_exit.call((location_id.clone(), arrival_region_id.clone()))
                                            },
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
}

/// Props for RegionButton
#[derive(Props, Clone, PartialEq)]
struct RegionButtonProps {
    target: NavigationTarget,
    disabled: bool,
    on_click: EventHandler<()>,
}

/// Button for navigating to a connected region
#[component]
fn RegionButton(props: RegionButtonProps) -> Element {
    let is_locked = props.target.is_locked;
    let is_disabled = props.disabled || is_locked;

    let button_class = if is_locked {
        "w-full p-4 bg-gray-800/50 rounded-xl border border-gray-700 cursor-not-allowed opacity-60"
    } else if props.disabled {
        "w-full p-4 bg-amber-500/10 rounded-xl border border-amber-500/20 cursor-not-allowed opacity-60"
    } else {
        "w-full p-4 bg-amber-500/10 hover:bg-amber-500/20 rounded-xl border border-amber-500/30 hover:border-amber-500/50 cursor-pointer transition-all"
    };

    rsx! {
        button {
            class: button_class,
            disabled: is_disabled,
            onclick: move |_| {
                if !is_disabled {
                    props.on_click.call(());
                }
            },

            div {
                class: "flex items-center gap-3",

                // Icon
                span {
                    class: if is_locked { "text-gray-500 text-xl" } else { "text-amber-400 text-xl" },
                    if is_locked { "🔒" } else { "→" }
                }

                // Content
                div {
                    class: "flex-1 text-left",

                    div {
                        class: if is_locked { "font-medium text-gray-500" } else { "font-medium text-white" },
                        "{props.target.name}"
                    }

                    if let Some(ref lock_desc) = props.target.lock_description {
                        p {
                            class: "text-sm text-gray-500 m-0 mt-1",
                            "{lock_desc}"
                        }
                    }
                }
            }
        }
    }
}

/// Props for ExitButton
#[derive(Props, Clone, PartialEq)]
struct ExitButtonProps {
    exit: NavigationExit,
    disabled: bool,
    on_click: EventHandler<()>,
}

/// Button for exiting to another location
#[component]
fn ExitButton(props: ExitButtonProps) -> Element {
    let button_class = if props.disabled {
        "w-full p-4 bg-blue-500/10 rounded-xl border border-blue-500/20 cursor-not-allowed opacity-60"
    } else {
        "w-full p-4 bg-blue-500/10 hover:bg-blue-500/20 rounded-xl border border-blue-500/30 hover:border-blue-500/50 cursor-pointer transition-all"
    };

    rsx! {
        button {
            class: button_class,
            disabled: props.disabled,
            onclick: move |_| {
                if !props.disabled {
                    props.on_click.call(());
                }
            },

            div {
                class: "flex items-center gap-3",

                // Icon
                span {
                    class: "text-blue-400 text-xl",
                    "⇐"
                }

                // Content
                div {
                    class: "flex-1 text-left",

                    div {
                        class: "font-medium text-white",
                        "Exit to {props.exit.location_name}"
                    }

                    if let Some(ref description) = props.exit.description {
                        p {
                            class: "text-sm text-gray-400 m-0 mt-1 italic",
                            "{description}"
                        }
                    }
                }
            }
        }
    }
}

/// Compact navigation buttons for inline display (alternative to modal)
#[derive(Props, Clone, PartialEq)]
pub struct NavigationButtonsProps {
    /// Navigation data from the current region
    pub navigation: NavigationData,
    /// Handler for moving to a region
    pub on_move_to_region: EventHandler<String>,
    /// Handler for exiting to a location
    pub on_exit_to_location: EventHandler<(String, String)>,
    /// Whether navigation is disabled
    #[props(default = false)]
    pub disabled: bool,
}

/// Compact inline navigation buttons (for action panel integration)
#[component]
pub fn NavigationButtons(props: NavigationButtonsProps) -> Element {
    let has_options = !props.navigation.connected_regions.is_empty() 
        || !props.navigation.exits.is_empty();

    if !has_options {
        return rsx! {};
    }

    rsx! {
        div {
            class: "navigation-buttons flex flex-wrap gap-2",

            // Region buttons
            for target in props.navigation.connected_regions.iter() {
                {
                    let is_locked = target.is_locked;
                    let is_disabled = props.disabled || is_locked;
                    let region_id = target.region_id.clone();
                    let on_move = props.on_move_to_region.clone();

                    rsx! {
                        button {
                            key: "{target.region_id}",
                            class: if is_locked {
                                "px-3 py-2 bg-gray-700/50 text-gray-500 rounded-lg text-sm cursor-not-allowed"
                            } else if props.disabled {
                                "px-3 py-2 bg-amber-500/20 text-amber-300/50 rounded-lg text-sm cursor-not-allowed"
                            } else {
                                "px-3 py-2 bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 rounded-lg text-sm cursor-pointer transition-colors"
                            },
                            disabled: is_disabled,
                            title: if is_locked { target.lock_description.clone().unwrap_or_default() } else { "".to_string() },
                            onclick: move |_| {
                                if !is_disabled {
                                    on_move.call(region_id.clone());
                                }
                            },
                            if is_locked { "🔒 " } else { "→ " }
                            "{target.name}"
                        }
                    }
                }
            }

            // Exit buttons
            for exit in props.navigation.exits.iter() {
                {
                    let location_id = exit.location_id.clone();
                    let arrival_region_id = exit.arrival_region_id.clone();
                    let on_exit = props.on_exit_to_location.clone();

                    rsx! {
                        button {
                            key: "{exit.location_id}",
                            class: if props.disabled {
                                "px-3 py-2 bg-blue-500/20 text-blue-300/50 rounded-lg text-sm cursor-not-allowed"
                            } else {
                                "px-3 py-2 bg-blue-500/20 hover:bg-blue-500/30 text-blue-300 rounded-lg text-sm cursor-pointer transition-colors"
                            },
                            disabled: props.disabled,
                            title: exit.description.clone().unwrap_or_default(),
                            onclick: move |_| {
                                if !props.disabled {
                                    on_exit.call((location_id.clone(), arrival_region_id.clone()));
                                }
                            },
                            "⇐ {exit.location_name}"
                        }
                    }
                }
            }
        }
    }
}

// =============================================================================
// Game Time Display Component
// =============================================================================

/// Props for GameTimeDisplay component
#[derive(Props, Clone, PartialEq)]
pub struct GameTimeDisplayProps {
    /// Game time data
    pub time: GameTime,
}

/// Compact game time display (for top-right corner)
#[component]
pub fn GameTimeDisplay(props: GameTimeDisplayProps) -> Element {
    let time_icon = match game_time_format::time_of_day(props.time) {
        game_time_format::TimeOfDay::Morning => "🌅",
        game_time_format::TimeOfDay::Afternoon => "☀️",
        game_time_format::TimeOfDay::Evening => "🌆",
        game_time_format::TimeOfDay::Night => "🌙",
    };

    let pause_indicator = if props.time.is_paused { " ⏸" } else { "" };
    let display = game_time_format::display_date(props.time);

    rsx! {
        div {
            class: "game-time-display px-3 py-2 bg-black/60 text-white rounded-lg text-sm backdrop-blur-sm",
            title: if props.time.is_paused { "Time is paused" } else { "Game time" },

            span {
                class: "mr-1",
                "{time_icon}"
            }
            span {
                class: if props.time.is_paused { "text-gray-400" } else { "text-white" },
                "{display}{pause_indicator}"
            }
        }
    }
}
