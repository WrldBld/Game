//! Dungeon Master View - Directorial control panel and Creator mode

use dioxus::prelude::*;

use crate::presentation::components::creator::CreatorMode;
use crate::presentation::components::dm_panel::adhoc_challenge_modal::{
    AdHocChallengeData, AdHocChallengeModal,
};
use crate::presentation::components::settings::SettingsView;
use crate::presentation::views::director::DirectorModeContent;
use crate::presentation::views::story_arc::StoryArcContent;

// Re-export for backward compatibility

/// The active tab/mode in the DM View
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DMMode {
    #[default]
    Director,
    Creator,
    StoryArc,
    Settings,
}

/// Props for DMView - receives active mode from parent
#[derive(Props, Clone, PartialEq)]
pub struct DMViewProps {
    /// World ID from the route
    pub world_id: String,
    /// Currently active DM mode/tab
    pub active_mode: DMMode,
    /// Optional Creator sub-tab (characters, locations, items, maps)
    #[props(default)]
    pub creator_subtab: Option<String>,
    /// Optional Settings sub-tab (workflows, skills)
    #[props(default)]
    pub settings_subtab: Option<String>,
    /// Optional Story Arc sub-tab (timeline, events, chains)
    #[props(default)]
    pub story_arc_subtab: Option<String>,
}

#[component]
pub fn DMView(props: DMViewProps) -> Element {
    // Local UI state for ad-hoc challenge modal visibility
    let mut show_adhoc_modal = use_signal(|| false);

    rsx! {
        div {
            class: "dm-view h-full flex flex-col bg-dark-bg",

            // Content area - no header, tabs are in main AppHeader
            div {
                class: "dm-content flex-1 overflow-hidden",

                match props.active_mode {
                    DMMode::Director => rsx! {
                        DirectorModeContent {}
                    },
                    DMMode::Creator => rsx! {
                        CreatorMode {
                            world_id: props.world_id.clone(),
                            selected_tab: props.creator_subtab.clone(),
                        }
                    },
                    DMMode::StoryArc => rsx! {
                        StoryArcContent {
                            world_id: props.world_id.clone(),
                            selected_tab: props.story_arc_subtab.clone(),
                        }
                    },
                    DMMode::Settings => rsx! {
                        SettingsView {
                            world_id: props.world_id.clone(),
                            selected_tab: props.settings_subtab.clone(),
                        }
                    },
                }
            }
            // Global ad-hoc challenge modal overlay
            if *show_adhoc_modal.read() {
                AdHocChallengeEntryPoint {
                    on_close: move || show_adhoc_modal.set(false),
                }
            }
        }
    }
}

/// Thin wrapper that wires the AdHocChallengeModal to the SessionCommandService
/// and current session state.
#[component]
fn AdHocChallengeEntryPoint(on_close: EventHandler<()>) -> Element {
    let mut session_state = crate::presentation::state::use_session_state();
    let game_state = use_context::<crate::presentation::state::GameState>();
    let platform = use_context::<wrldbldr_player_ports::outbound::Platform>();

    let player_characters = game_state.scene_characters.read().clone();

    rsx! {
        AdHocChallengeModal {
            player_characters: player_characters,
            on_create: move |data: AdHocChallengeData| {
                // Get the engine client directly from session state
                if let Some(client) = session_state.engine_client().read().as_ref() {
                    match client.create_adhoc_challenge(
                        &data.challenge_name,
                        &data.skill_name,
                        &data.difficulty,
                        &data.target_pc_id,
                        data.outcomes.clone(),
                    ) {
                        Ok(()) => {
                            tracing::info!(
                                "Ad-hoc challenge '{}' sent for PC {}",
                                data.challenge_name,
                                data.target_pc_id
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to create ad-hoc challenge: {}", e);
                        }
                    }
                } else {
                    tracing::warn!("No Engine client available for ad-hoc challenge");
                }

                // Add a quick log entry for instant feedback
                session_state.add_log_entry(
                    "System".to_string(),
                    format!(
                        "Ad-hoc challenge '{}' created for PC {}",
                        data.challenge_name, data.target_pc_id
                    ),
                    true,
                    &platform,
                );

                on_close.call(());
            },
            on_close: move |_| on_close.call(()),
        }
    }
}
