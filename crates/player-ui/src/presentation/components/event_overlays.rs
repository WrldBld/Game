//! Event Overlay Components - Visual notifications for game events
//!
//! US-NPC-008: ApproachEventOverlay - NPC approaching player
//! US-NPC-009: LocationEventBanner - Location-wide events

use dioxus::prelude::*;

use crate::presentation::state::{ApproachEventData, LocationEventData};

// =============================================================================
// US-NPC-008: Approach Event Overlay
// =============================================================================

/// Props for ApproachEventOverlay
#[derive(Props, Clone, PartialEq)]
pub struct ApproachEventOverlayProps {
    /// The approach event data
    pub event: ApproachEventData,
    /// Handler for dismissing the overlay
    pub on_dismiss: EventHandler<()>,
}

/// Overlay shown when an NPC approaches the player
///
/// Displays the NPC sprite (if available) sliding in from the side,
/// with the approach description and a Continue button.
#[component]
pub fn ApproachEventOverlay(props: ApproachEventOverlayProps) -> Element {
    rsx! {
        // Semi-transparent overlay
        div {
            class: "approach-event-overlay fixed inset-0 bg-black/70 z-[900] flex items-center justify-center p-4",
            onclick: move |_| props.on_dismiss.call(()),

            // Event card
            div {
                class: "approach-event-card bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl max-w-lg w-full overflow-hidden shadow-2xl border border-amber-500/30 animate-slide-in",
                onclick: move |e| e.stop_propagation(),

                // NPC sprite area (if available)
                if let Some(ref sprite_url) = props.event.npc_sprite {
                    div {
                        class: "relative h-48 bg-gradient-to-b from-amber-900/20 to-transparent overflow-hidden",

                        // Sprite image
                        img {
                            src: "{sprite_url}",
                            alt: "{props.event.npc_name}",
                            class: "absolute bottom-0 left-1/2 -translate-x-1/2 h-full object-contain animate-fade-in",
                        }

                        // Gradient fade at bottom
                        div {
                            class: "absolute bottom-0 left-0 right-0 h-16 bg-gradient-to-t from-dark-surface to-transparent",
                        }
                    }
                }

                // Content area
                div {
                    class: "p-6",

                    // NPC name header
                    div {
                        class: "flex items-center gap-3 mb-4",

                        // Approach icon
                        span {
                            class: "text-2xl",
                            "~"
                        }

                        h2 {
                            class: "text-xl font-bold text-amber-400 m-0",
                            "{props.event.npc_name}"
                        }

                        span {
                            class: "text-gray-500 text-sm",
                            "approaches"
                        }
                    }

                    // Description
                    div {
                        class: "bg-black/30 rounded-lg p-4 mb-6",

                        p {
                            class: "text-gray-200 leading-relaxed m-0 italic",
                            "{props.event.description}"
                        }
                    }

                    // Continue button
                    button {
                        class: "w-full p-3 bg-gradient-to-r from-amber-500 to-amber-600 hover:from-amber-400 hover:to-amber-500 text-white border-none rounded-lg cursor-pointer font-semibold transition-all",
                        onclick: move |_| props.on_dismiss.call(()),
                        "Continue"
                    }
                }
            }
        }
    }
}

// =============================================================================
// US-NPC-009: Location Event Banner
// =============================================================================

/// Props for LocationEventBanner
#[derive(Props, Clone, PartialEq)]
pub struct LocationEventBannerProps {
    /// The location event data
    pub event: LocationEventData,
    /// Handler for dismissing the banner
    pub on_dismiss: EventHandler<()>,
}

/// Banner shown for location-wide events
///
/// A narrative overlay that appears at the top of the screen,
/// auto-dismisses after a timeout or on click.
#[component]
pub fn LocationEventBanner(props: LocationEventBannerProps) -> Element {
    rsx! {
        // Full-screen click target (semi-transparent)
        div {
            class: "location-event-overlay fixed inset-0 bg-black/50 z-[850] flex flex-col items-center justify-start pt-16 p-4",
            onclick: move |_| props.on_dismiss.call(()),

            // Banner card
            div {
                class: "location-event-banner bg-gradient-to-r from-blue-900/90 via-dark-surface/95 to-blue-900/90 rounded-xl max-w-2xl w-full p-6 shadow-2xl border border-blue-500/30 backdrop-blur-sm animate-slide-down",
                onclick: move |e| e.stop_propagation(),

                // Event header
                div {
                    class: "flex items-center gap-3 mb-3",

                    // Event icon
                    span {
                        class: "text-2xl",
                        "*"
                    }

                    span {
                        class: "text-blue-400 text-sm font-semibold uppercase tracking-wider",
                        "Something happens..."
                    }
                }

                // Description
                p {
                    class: "text-gray-100 text-lg leading-relaxed m-0 italic",
                    "{props.event.description}"
                }

                // Dismiss hint
                p {
                    class: "text-gray-500 text-xs text-center mt-4 m-0",
                    "Click anywhere to continue"
                }
            }
        }
    }
}
