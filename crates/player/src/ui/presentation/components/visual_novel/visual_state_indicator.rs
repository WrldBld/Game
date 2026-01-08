//! Visual State Indicator - Shows current location/region visual state
//!
//! Displays a small indicator in the scene view showing the active visual state
//! (time of day, weather, special conditions, etc.)

use dioxus::prelude::*;

/// Visual state info for display
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VisualStateInfo {
    /// Name of the active location state (if any)
    pub location_state_name: Option<String>,
    /// Name of the active region state (if any)
    pub region_state_name: Option<String>,
    /// Atmosphere description override
    pub atmosphere: Option<String>,
    /// Ambient sound name
    pub ambient_sound: Option<String>,
}

impl VisualStateInfo {
    /// Returns true if there is any visual state to display
    pub fn has_state(&self) -> bool {
        self.location_state_name.is_some()
            || self.region_state_name.is_some()
            || self.atmosphere.is_some()
    }

    /// Get a combined display name for the visual state
    pub fn display_name(&self) -> Option<String> {
        match (&self.region_state_name, &self.location_state_name) {
            (Some(region), Some(location)) => Some(format!("{} - {}", region, location)),
            (Some(region), None) => Some(region.clone()),
            (None, Some(location)) => Some(location.clone()),
            (None, None) => None,
        }
    }
}

/// Props for the VisualStateIndicator component
#[derive(Props, Clone, PartialEq)]
pub struct VisualStateIndicatorProps {
    /// Current visual state info
    pub state: VisualStateInfo,
    /// Position: "top-left", "top-right", "bottom-left", "bottom-right"
    #[props(default = "top-right".to_string())]
    pub position: String,
    /// Whether to show in compact mode (icon only)
    #[props(default = false)]
    pub compact: bool,
    /// Whether the indicator is visible
    #[props(default = true)]
    pub visible: bool,
}

/// Visual State Indicator - small overlay showing current visual state
#[component]
pub fn VisualStateIndicator(props: VisualStateIndicatorProps) -> Element {
    if !props.visible || !props.state.has_state() {
        return rsx! {};
    }

    // Position classes
    let position_class = match props.position.as_str() {
        "top-left" => "top-2 left-2",
        "top-right" => "top-2 right-2",
        "bottom-left" => "bottom-2 left-2",
        "bottom-right" => "bottom-2 right-2",
        _ => "top-2 right-2",
    };

    let display_name = props.state.display_name();

    rsx! {
        div {
            class: "visual-state-indicator absolute {position_class} z-10",

            if props.compact {
                // Compact mode - just an icon with tooltip
                div {
                    class: "w-8 h-8 flex items-center justify-center bg-black/60 backdrop-blur-sm rounded-lg border border-white/10 text-amber-400 cursor-help",
                    title: display_name.clone().unwrap_or_default(),
                    "*"
                }
            } else {
                // Full mode - show state info
                div {
                    class: "bg-black/60 backdrop-blur-sm rounded-lg border border-white/10 p-2 min-w-[120px]",

                    // State name
                    if let Some(ref name) = display_name {
                        div {
                            class: "flex items-center gap-1.5 text-xs",
                            span {
                                class: "text-amber-400",
                                "*"
                            }
                            span {
                                class: "text-white/80 font-medium",
                                "{name}"
                            }
                        }
                    }

                    // Atmosphere
                    if let Some(ref atmosphere) = props.state.atmosphere {
                        div {
                            class: "text-xs text-gray-400 mt-1 italic",
                            "{atmosphere}"
                        }
                    }

                    // Ambient sound indicator
                    if let Some(ref sound) = props.state.ambient_sound {
                        div {
                            class: "flex items-center gap-1 text-xs text-gray-500 mt-1",
                            span { "~" }
                            span { "{sound}" }
                        }
                    }
                }
            }
        }
    }
}

/// Props for TimeOfDayIndicator
#[derive(Props, Clone, PartialEq)]
pub struct TimeOfDayIndicatorProps {
    /// Current time period: "morning", "afternoon", "evening", "night"
    pub period: String,
    /// Current hour (0-23)
    #[props(default = 12)]
    pub hour: u8,
    /// Position on screen
    #[props(default = "top-left".to_string())]
    pub position: String,
}

/// Time of Day Indicator - shows current game time period with visual icon
#[component]
pub fn TimeOfDayIndicator(props: TimeOfDayIndicatorProps) -> Element {
    // Icon and color based on time period
    let (icon, color_class, bg_class) = match props.period.to_lowercase().as_str() {
        "morning" => ("o", "text-yellow-400", "bg-yellow-500/20"),
        "afternoon" => ("O", "text-amber-400", "bg-amber-500/20"),
        "evening" => (")", "text-orange-400", "bg-orange-500/20"),
        "night" => ("*", "text-blue-300", "bg-blue-500/20"),
        _ => (".", "text-gray-400", "bg-gray-500/20"),
    };

    // Position classes
    let position_class = match props.position.as_str() {
        "top-left" => "top-2 left-2",
        "top-right" => "top-2 right-2",
        "bottom-left" => "bottom-2 left-2",
        "bottom-right" => "bottom-2 right-2",
        _ => "top-2 left-2",
    };

    rsx! {
        div {
            class: "time-indicator absolute {position_class} z-10 flex items-center gap-1.5 px-2 py-1 bg-black/60 backdrop-blur-sm rounded-lg border border-white/10",

            // Time icon
            span {
                class: "w-5 h-5 flex items-center justify-center rounded {bg_class} {color_class} text-sm",
                "{icon}"
            }

            // Time text
            span {
                class: "text-xs text-white/80",
                "{props.period}"
            }
        }
    }
}
