//! DM (Dungeon Master) view route handlers

use super::connection::handle_disconnect;
use super::world_session_layout::WorldSessionLayout;
use super::Route;
use crate::presentation::state::{
    ConnectionStatus, DialogueState, GameState, LoreState, SessionState,
};
use crate::presentation::views::dm_view::DMMode;
use crate::use_platform;
use dioxus::prelude::*;
use wrldbldr_player_ports::outbound::storage_keys;
use wrldbldr_player_ports::session_types::ParticipantRole;

/// DMViewRoute - renders Director tab directly (no redirect needed)
#[component]
pub fn DMViewRoute(world_id: String) -> Element {
    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::DungeonMaster,
            page_title: "Director",
            show_status_bar: false,

            DMViewContent {
                world_id: world_id,
                dm_mode: DMMode::Director,
                creator_subtab: None,
                settings_subtab: None,
                story_arc_subtab: None,
            }
        }
    }
}

/// DMViewTabRoute - DM view with specific tab
/// For tabs with subtabs (creator, settings, story-arc), render with default subtab
/// This avoids use_effect redirect race conditions
#[component]
pub fn DMViewTabRoute(world_id: String, tab: String) -> Element {
    // Determine mode and default subtab based on tab parameter
    let (dm_mode, creator_subtab, settings_subtab, story_arc_subtab, title) = match tab.as_str() {
        "director" => (DMMode::Director, None, None, None, "Director"),
        "creator" => (
            DMMode::Creator,
            Some("characters".to_string()),
            None,
            None,
            "Creator - Characters",
        ),
        "settings" => (
            DMMode::Settings,
            None,
            Some("workflows".to_string()),
            None,
            "Settings - Workflows",
        ),
        "story-arc" => (
            DMMode::StoryArc,
            None,
            None,
            Some("timeline".to_string()),
            "Story Arc - Timeline",
        ),
        _ => (DMMode::Director, None, None, None, "Director"),
    };

    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::DungeonMaster,
            page_title: title,
            show_status_bar: false,

            DMViewContent {
                world_id: world_id,
                dm_mode: dm_mode,
                creator_subtab: creator_subtab,
                settings_subtab: settings_subtab,
                story_arc_subtab: story_arc_subtab,
            }
        }
    }
}

/// DMCreatorSubTabRoute - Creator mode with specific sub-tab
#[component]
pub fn DMCreatorSubTabRoute(world_id: String, subtab: String) -> Element {
    // Set page title based on subtab
    let title = match subtab.as_str() {
        "characters" => "Creator - Characters",
        "locations" => "Creator - Locations",
        "items" => "Creator - Items",
        "maps" => "Creator - Maps",
        _ => "Creator",
    };

    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::DungeonMaster,
            page_title: title,
            show_status_bar: false,

            DMViewContent {
                world_id: world_id,
                dm_mode: DMMode::Creator,
                creator_subtab: Some(subtab),
                settings_subtab: None,
                story_arc_subtab: None,
            }
        }
    }
}

/// DMSettingsSubTabRoute - Settings with specific sub-tab
#[component]
pub fn DMSettingsSubTabRoute(world_id: String, subtab: String) -> Element {
    // Set page title based on subtab
    let title = match subtab.as_str() {
        "workflows" => "Settings - Workflows",
        "skills" => "Settings - Skills",
        _ => "Settings",
    };

    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::DungeonMaster,
            page_title: title,
            show_status_bar: false,

            DMViewContent {
                world_id: world_id,
                dm_mode: DMMode::Settings,
                creator_subtab: None,
                settings_subtab: Some(subtab),
                story_arc_subtab: None,
            }
        }
    }
}

/// DMStoryArcSubTabRoute - Story Arc with specific sub-tab
#[component]
pub fn DMStoryArcSubTabRoute(world_id: String, subtab: String) -> Element {
    // Set page title based on subtab
    let title = match subtab.as_str() {
        "timeline" => "Story Arc - Timeline",
        "events" => "Story Arc - Narrative Events",
        "chains" => "Story Arc - Event Chains",
        _ => "Story Arc",
    };

    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::DungeonMaster,
            page_title: title,
            show_status_bar: false,

            DMViewContent {
                world_id: world_id,
                dm_mode: DMMode::StoryArc,
                creator_subtab: None,
                settings_subtab: None,
                story_arc_subtab: Some(subtab),
            }
        }
    }
}

/// DMViewContent - inner content component for DM views
/// Connection handling is done by WorldSessionLayout wrapper
#[derive(Props, Clone, PartialEq)]
struct DMViewContentProps {
    world_id: String,
    dm_mode: DMMode,
    creator_subtab: Option<String>,
    settings_subtab: Option<String>,
    story_arc_subtab: Option<String>,
}

#[component]
fn DMViewContent(props: DMViewContentProps) -> Element {
    let navigator = use_navigator();
    let platform = use_platform();
    let session_state = use_context::<SessionState>();
    let game_state = use_context::<GameState>();
    let dialogue_state = use_context::<DialogueState>();
    let lore_state = use_context::<LoreState>();

    let connection_status = *session_state.connection_status().read();

    rsx! {
        div {
            class: "dm-view-content h-full flex flex-col bg-dark-bg",

            // Header with DM tabs, back button, and connection status
            DMViewHeader {
                world_id: props.world_id.clone(),
                dm_mode: props.dm_mode,
                connection_status: connection_status,
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

            // Main content
            main {
                class: "flex-1 overflow-hidden relative z-[1]",

                crate::presentation::views::dm_view::DMView {
                    world_id: props.world_id.clone(),
                    active_mode: props.dm_mode,
                    creator_subtab: props.creator_subtab.clone(),
                    settings_subtab: props.settings_subtab.clone(),
                    story_arc_subtab: props.story_arc_subtab.clone(),
                }
            }
        }
    }
}

/// Header component for DM View - contains title, tabs, back button, and connection status
#[derive(Props, Clone, PartialEq)]
struct DMViewHeaderProps {
    world_id: String,
    dm_mode: DMMode,
    connection_status: ConnectionStatus,
    on_back: EventHandler<()>,
}

#[component]
fn DMViewHeader(props: DMViewHeaderProps) -> Element {
    let indicator_color = props.connection_status.indicator_color();
    let status_text = props.connection_status.display_text();

    rsx! {
        header {
            class: "dm-view-header py-3 px-4 bg-dark-surface text-white flex justify-between items-center border-b border-[#2d2d44] relative z-[100]",

            // Left side: title and DM tabs
            div {
                class: "flex items-center gap-6 relative z-[101]",

                // Title
                h1 {
                    class: "m-0 text-xl font-['Cinzel',serif] text-[#d4af37]",
                    "WrldBldr"
                }

                // DM tabs - use router Links for navigation
                div {
                    class: "flex gap-1 relative z-[102]",

                    DMHeaderTabLink {
                        label: "Director",
                        tab: "director",
                        world_id: props.world_id.clone(),
                        active: props.dm_mode == DMMode::Director,
                    }
                    DMHeaderTabLink {
                        label: "Creator",
                        tab: "creator",
                        world_id: props.world_id.clone(),
                        active: props.dm_mode == DMMode::Creator,
                    }
                    DMHeaderTabLink {
                        label: "Story Arc",
                        tab: "story-arc",
                        world_id: props.world_id.clone(),
                        active: props.dm_mode == DMMode::StoryArc,
                    }
                    DMHeaderTabLink {
                        label: "Settings",
                        tab: "settings",
                        world_id: props.world_id.clone(),
                        active: props.dm_mode == DMMode::Settings,
                    }
                }
            }

            // Right side: back button and connection status
            div {
                class: "flex items-center gap-4",

                // Back button
                button {
                    onclick: move |e| {
                        e.stop_propagation();
                        props.on_back.call(());
                    },
                    class: "py-1.5 px-3 bg-transparent text-gray-400 border border-gray-700 rounded-md cursor-pointer text-sm transition-all duration-150",
                    "← Back"
                }

                // Connection status
                div {
                    class: "connection-status flex items-center gap-2 text-sm",

                    span {
                        class: "status-indicator w-2 h-2 rounded-full",
                        style: "background: {indicator_color};",
                    }
                    span {
                        class: "text-gray-400",
                        "{status_text}"
                    }
                }
            }
        }
    }
}

/// Header tab link for DM View - uses router navigation
/// Links directly to the appropriate subtab route to avoid redirect race conditions
#[component]
fn DMHeaderTabLink(
    label: &'static str,
    tab: &'static str,
    world_id: String,
    active: bool,
) -> Element {
    // Get generation state for queue badge (only for Creator tab)
    let generation_state = if tab == "creator" {
        Some(crate::presentation::state::use_generation_state())
    } else {
        None
    };

    let queue_badge_count = generation_state
        .as_ref()
        .map(|gs| gs.active_count() + gs.active_suggestion_count())
        .unwrap_or(0);

    // Determine the correct route based on tab - link directly to subtab routes
    // to avoid use_effect redirect race conditions
    let route = match tab {
        "director" => Route::DMViewTabRoute {
            world_id: world_id.clone(),
            tab: "director".to_string(),
        },
        "creator" => Route::DMCreatorSubTabRoute {
            world_id: world_id.clone(),
            subtab: "characters".to_string(),
        },
        "story-arc" => Route::DMStoryArcSubTabRoute {
            world_id: world_id.clone(),
            subtab: "timeline".to_string(),
        },
        "settings" => Route::DMSettingsSubTabRoute {
            world_id: world_id.clone(),
            subtab: "workflows".to_string(),
        },
        _ => Route::DMViewTabRoute {
            world_id: world_id.clone(),
            tab: tab.to_string(),
        },
    };

    rsx! {
        Link {
            to: route,
            class: format!(
                "py-1.5 px-3 {} {} border-none rounded-md cursor-pointer text-sm {} transition-all duration-150 relative z-[103] pointer-events-auto no-underline inline-flex items-center gap-2",
                if active { "bg-blue-500" } else { "bg-transparent" },
                if active { "text-white" } else { "text-gray-400" },
                if active { "font-medium" } else { "font-normal" }
            ),
            "{label}"
            if tab == "creator" && queue_badge_count > 0 {
                span {
                    class: "bg-amber-500 text-white rounded-xl py-0.5 px-1.5 text-[0.625rem] font-bold min-w-5 text-center",
                    "{queue_badge_count}"
                }
            }
        }
    }
}
