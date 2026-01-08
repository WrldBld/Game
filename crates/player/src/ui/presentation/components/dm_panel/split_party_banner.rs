//! Split Party Banner - Warning displayed when party members are at different locations

use crate::application::dto::SplitPartyLocation;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SplitPartyBannerProps {
    /// Locations where party members are distributed
    pub locations: Vec<SplitPartyLocation>,
}

/// Banner component showing split party warning
///
/// Displayed at the top of the Director view when PCs are at different locations.
/// Collapsible to show/hide the detailed location breakdown.
#[component]
pub fn SplitPartyBanner(props: SplitPartyBannerProps) -> Element {
    let mut is_expanded = use_signal(|| true);

    // Don't render if party is together (0 or 1 location)
    if props.locations.len() <= 1 {
        return rsx! {};
    }

    let location_count = props.locations.len();

    rsx! {
        div {
            class: "split-party-banner bg-amber-900/30 border border-amber-500/50 rounded-lg mb-4 overflow-hidden",

            // Header (always visible)
            button {
                class: "w-full flex items-center justify-between p-3 hover:bg-amber-900/20 transition-colors",
                onclick: move |_| {
                    let current = *is_expanded.read();
                    is_expanded.set(!current);
                },

                div {
                    class: "flex items-center gap-2",
                    span { class: "text-amber-400 text-lg", "" }
                    span {
                        class: "text-amber-200 font-medium",
                        "Party Split Across {location_count} Locations"
                    }
                }

                span {
                    class: "text-amber-400 text-sm transition-transform duration-200",
                    style: if *is_expanded.read() { "transform: rotate(180deg)" } else { "" },
                    ""
                }
            }

            // Expandable details
            if *is_expanded.read() {
                div {
                    class: "border-t border-amber-500/30 p-3 space-y-2",

                    for location in props.locations.iter() {
                        LocationRow {
                            key: "{location.location_id}",
                            location: location.clone(),
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct LocationRowProps {
    location: SplitPartyLocation,
}

#[component]
fn LocationRow(props: LocationRowProps) -> Element {
    let pc_list = props.location.pc_names.join(", ");
    let pc_count = props.location.pc_count;
    let pc_suffix = if pc_count != 1 { "s" } else { "" };

    rsx! {
        div {
            class: "flex items-start gap-2 text-sm",

            span { class: "text-amber-400 mt-0.5", "" }

            div {
                class: "flex-1",

                span {
                    class: "text-white font-medium",
                    "{props.location.location_name}"
                }

                span {
                    class: "text-gray-400 ml-2",
                    "({pc_count} PC{pc_suffix})"
                }

                div {
                    class: "text-gray-300 text-xs mt-0.5",
                    "{pc_list}"
                }
            }
        }
    }
}
