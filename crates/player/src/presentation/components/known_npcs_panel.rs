//! Known NPCs Panel - Player UI for viewing observed NPCs
//!
//! US-OBS-004/005: Player view of observed NPCs with last seen info.

use dioxus::prelude::*;

/// Observation data for a known NPC
#[derive(Clone, Debug, PartialEq)]
pub struct NpcObservationData {
    pub npc_id: String,
    pub npc_name: String,
    pub npc_portrait: Option<String>,
    pub location_name: String,
    pub region_name: String,
    pub game_time: String,
    pub observation_type: String,
    pub observation_type_icon: String,
    pub notes: Option<String>,
}

/// Props for the KnownNpcsPanel component
#[derive(Props, Clone, PartialEq)]
pub struct KnownNpcsPanelProps {
    /// List of NPC observations
    pub observations: Vec<NpcObservationData>,
    /// Whether data is still loading
    #[props(default = false)]
    pub is_loading: bool,
    /// Handler for closing the panel
    pub on_close: EventHandler<()>,
    /// Handler for clicking an NPC (to view details or interact)
    #[props(default)]
    pub on_npc_click: Option<EventHandler<String>>,
}

/// Known NPCs Panel - modal showing NPCs the player has observed
#[component]
pub fn KnownNpcsPanel(props: KnownNpcsPanelProps) -> Element {
    // Group observations by observation type for filtering
    let direct_obs: Vec<_> = props.observations.iter()
        .filter(|o| o.observation_type == "direct")
        .collect();
    let heard_obs: Vec<_> = props.observations.iter()
        .filter(|o| o.observation_type == "heard_about")
        .collect();
    let deduced_obs: Vec<_> = props.observations.iter()
        .filter(|o| o.observation_type == "deduced")
        .collect();

    rsx! {
        // Overlay background
        div {
            class: "known-npcs-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-4",
            onclick: move |_| props.on_close.call(()),

            // Panel container
            div {
                class: "known-npcs-panel bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl w-full max-w-2xl max-h-[85vh] overflow-hidden flex flex-col shadow-2xl border border-purple-500/20",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-4 border-b border-white/10 flex justify-between items-center",

                    div {
                        h2 {
                            class: "text-xl font-bold text-white m-0",
                            "Known NPCs"
                        }
                        p {
                            class: "text-gray-400 text-sm m-0 mt-1",
                            "People you've encountered or heard about"
                        }
                    }

                    button {
                        class: "w-8 h-8 flex items-center justify-center bg-white/5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition-colors",
                        onclick: move |_| props.on_close.call(()),
                        "x"
                    }
                }

                // Legend
                div {
                    class: "px-4 py-2 bg-black/20 border-b border-white/5 flex gap-4 text-xs text-gray-400",

                    span {
                        class: "flex items-center gap-1",
                        span { class: "text-blue-400", "@" }
                        "Seen directly"
                    }
                    span {
                        class: "flex items-center gap-1",
                        span { class: "text-yellow-400", "?" }
                        "Heard about"
                    }
                    span {
                        class: "flex items-center gap-1",
                        span { class: "text-purple-400", "*" }
                        "Deduced"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    if props.is_loading {
                        div {
                            class: "flex items-center justify-center py-12",
                            span {
                                class: "text-gray-400",
                                "Loading observations..."
                            }
                        }
                    } else if props.observations.is_empty() {
                        div {
                            class: "flex flex-col items-center justify-center py-12 text-center",
                            span {
                                class: "text-4xl mb-4",
                                "?"
                            }
                            p {
                                class: "text-gray-400 m-0",
                                "You haven't encountered any NPCs yet."
                            }
                            p {
                                class: "text-gray-500 text-sm m-0 mt-2",
                                "Explore the world to meet new people!"
                            }
                        }
                    } else {
                        div {
                            class: "space-y-6",

                            // Directly seen NPCs
                            if !direct_obs.is_empty() {
                                ObservationSection {
                                    title: "Seen Directly",
                                    icon: "@",
                                    icon_color: "text-blue-400",
                                    observations: direct_obs.into_iter().cloned().collect(),
                                    on_npc_click: props.on_npc_click.clone(),
                                }
                            }

                            // Heard about
                            if !heard_obs.is_empty() {
                                ObservationSection {
                                    title: "Heard About",
                                    icon: "?",
                                    icon_color: "text-yellow-400",
                                    observations: heard_obs.into_iter().cloned().collect(),
                                    on_npc_click: props.on_npc_click.clone(),
                                }
                            }

                            // Deduced
                            if !deduced_obs.is_empty() {
                                ObservationSection {
                                    title: "Deduced",
                                    icon: "*",
                                    icon_color: "text-purple-400",
                                    observations: deduced_obs.into_iter().cloned().collect(),
                                    on_npc_click: props.on_npc_click.clone(),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for ObservationSection
#[derive(Props, Clone, PartialEq)]
struct ObservationSectionProps {
    title: &'static str,
    icon: &'static str,
    icon_color: &'static str,
    observations: Vec<NpcObservationData>,
    on_npc_click: Option<EventHandler<String>>,
}

/// A section of observations grouped by type
#[component]
fn ObservationSection(props: ObservationSectionProps) -> Element {
    rsx! {
        div {
            class: "observation-section",

            // Section header
            h3 {
                class: "text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2",
                span { class: props.icon_color, "{props.icon}" }
                "{props.title}"
                span {
                    class: "text-xs text-gray-500",
                    "({props.observations.len()})"
                }
            }

            // NPC cards
            div {
                class: "grid gap-2",

                for obs in props.observations.iter() {
                    NpcObservationCard {
                        key: "{obs.npc_id}",
                        observation: obs.clone(),
                        on_click: props.on_npc_click.clone(),
                    }
                }
            }
        }
    }
}

/// Props for NpcObservationCard
#[derive(Props, Clone, PartialEq)]
struct NpcObservationCardProps {
    observation: NpcObservationData,
    on_click: Option<EventHandler<String>>,
}

/// Card displaying a single NPC observation
#[component]
fn NpcObservationCard(props: NpcObservationCardProps) -> Element {
    let icon_color = match props.observation.observation_type.as_str() {
        "direct" => "text-blue-400",
        "heard_about" => "text-yellow-400",
        "deduced" => "text-purple-400",
        _ => "text-gray-400",
    };

    let icon = match props.observation.observation_type_icon.as_str() {
        "eye" => "@",
        "ear" => "?",
        "brain" => "*",
        _ => ".",
    };

    let npc_id = props.observation.npc_id.clone();

    rsx! {
        div {
            class: "npc-observation-card bg-black/30 rounded-lg border border-white/10 p-3 hover:bg-white/5 transition-colors",
            onclick: {
                let on_click = props.on_click.clone();
                move |_| {
                    if let Some(handler) = &on_click {
                        handler.call(npc_id.clone());
                    }
                }
            },

            div {
                class: "flex items-start gap-3",

                // Portrait or icon
                div {
                    class: "w-12 h-12 rounded-lg bg-gradient-to-br from-gray-700 to-gray-800 flex items-center justify-center overflow-hidden shrink-0",

                    if let Some(ref portrait) = props.observation.npc_portrait {
                        img {
                            src: "{portrait}",
                            alt: "{props.observation.npc_name}",
                            class: "w-full h-full object-cover",
                        }
                    } else {
                        span {
                            class: "text-2xl text-gray-500",
                            "~"
                        }
                    }
                }

                // Info
                div {
                    class: "flex-1 min-w-0",

                    // Name and type icon
                    div {
                        class: "flex items-center gap-2 mb-1",
                        span { class: icon_color, "{icon}" }
                        span {
                            class: "text-white font-medium truncate",
                            "{props.observation.npc_name}"
                        }
                    }

                    // Location info
                    div {
                        class: "text-sm text-gray-400",
                        "Last seen: {props.observation.region_name}"
                    }
                    div {
                        class: "text-xs text-gray-500",
                        "{props.observation.location_name}"
                    }

                    // Notes (for heard_about/deduced)
                    if let Some(ref notes) = props.observation.notes {
                        if !notes.is_empty() {
                            p {
                                class: "text-xs text-gray-500 italic mt-1 m-0",
                                "\"{notes}\""
                            }
                        }
                    }
                }

                // Time indicator
                div {
                    class: "text-xs text-gray-500 shrink-0",
                    // TODO: Calculate relative time from game_time
                    "..."
                }
            }
        }
    }
}
