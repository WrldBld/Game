//! Story Arc mode content - Timeline, Narrative Events, Event Chains

use dioxus::prelude::*;

use crate::presentation::components::story_arc::timeline_view::TimelineView;
use crate::presentation::components::story_arc::visual_timeline::VisualTimeline;
use crate::presentation::components::story_arc::narrative_event_library::NarrativeEventLibrary;
use super::{StoryArcSubTab, StoryArcTabLink, EventChainsView};

/// Story Arc mode content - Timeline, Narrative Events, Event Chains
#[derive(Props, Clone, PartialEq)]
pub struct StoryArcContentProps {
    pub world_id: String,
    #[props(default)]
    pub selected_tab: Option<String>,
}

#[component]
pub fn StoryArcContent(props: StoryArcContentProps) -> Element {
    // Parse selected tab from URL, default to Timeline
    let active_tab = props.selected_tab
        .as_ref()
        .map(|s| StoryArcSubTab::from_str(s))
        .unwrap_or(StoryArcSubTab::Timeline);

    rsx! {
        div {
            class: "h-full flex flex-col",

            // Sub-tab navigation using router Links
            div {
                class: "flex gap-0 bg-dark-bg border-b border-gray-700",

                StoryArcTabLink {
                    label: "Timeline",
                    icon: "📜",
                    subtab: "timeline",
                    world_id: props.world_id.clone(),
                    is_active: active_tab == StoryArcSubTab::Timeline,
                }
                StoryArcTabLink {
                    label: "Visual",
                    icon: "📊",
                    subtab: "visual",
                    world_id: props.world_id.clone(),
                    is_active: active_tab == StoryArcSubTab::Visual,
                }
                StoryArcTabLink {
                    label: "Narrative Events",
                    icon: "⭐",
                    subtab: "events",
                    world_id: props.world_id.clone(),
                    is_active: active_tab == StoryArcSubTab::NarrativeEvents,
                }
                StoryArcTabLink {
                    label: "Event Chains",
                    icon: "🔗",
                    subtab: "chains",
                    world_id: props.world_id.clone(),
                    is_active: active_tab == StoryArcSubTab::EventChains,
                }
            }

            // Content area
            div {
                class: "flex-1 overflow-hidden",

                match active_tab {
                    StoryArcSubTab::Timeline => rsx! {
                        TimelineView { world_id: props.world_id.clone() }
                    },
                    StoryArcSubTab::Visual => rsx! {
                        VisualTimeline { world_id: props.world_id.clone() }
                    },
                    StoryArcSubTab::NarrativeEvents => rsx! {
                        NarrativeEventLibrary { world_id: props.world_id.clone() }
                    },
                    StoryArcSubTab::EventChains => rsx! {
                        EventChainsView {
                            world_id: props.world_id.clone(),
                        }
                    },
                }
            }
        }
    }
}
