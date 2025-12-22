//! Player and spectator route handlers

use dioxus::prelude::*;
use wrldbldr_player_app::application::services::ParticipantRolePort as ParticipantRole;
use crate::presentation::state::SessionState;
use super::world_session_layout::WorldSessionLayout;
use super::Route;

/// PC (Player Character) view route
#[component]
pub fn PCViewRoute(world_id: String) -> Element {
    let navigator = use_navigator();
    let session_state = use_context::<SessionState>();
    let pc_service = crate::presentation::services::use_player_character_service();

    // Check for existing PC on mount - redirect to creation if none exists
    {
        let session_id = session_state.session_id().read().clone();
        let world_id_clone = world_id.clone();
        let nav = navigator.clone();
        let pc_svc = pc_service.clone();
        use_effect(move || {
            if let Some(sid) = session_id.as_ref() {
                let sid_clone = sid.clone();
                let wid = world_id_clone.clone();
                let nav_clone = nav.clone();
                let pc_svc_clone = pc_svc.clone();
                spawn(async move {
                    match pc_svc_clone.get_my_pc(&sid_clone).await {
                        Ok(Some(_pc)) => {
                            // PC exists, continue to PC View
                        }
                        Ok(None) => {
                            // No PC, redirect to creation
                            nav_clone.push(Route::PCCreationRoute { world_id: wid });
                        }
                        Err(e) => {
                            tracing::warn!("Failed to check for PC: {}", e);
                            // On error, still try to show PC view (might be a transient error)
                        }
                    }
                });
            }
        });
    }

    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::Player,
            page_title: "Playing",

            PCViewContent {}
        }
    }
}

/// PCViewContent - inner content for PC view
#[component]
fn PCViewContent() -> Element {
    rsx! {
        crate::presentation::views::pc_view::PCView {}
    }
}

/// Spectator view route
#[component]
pub fn SpectatorViewRoute(world_id: String) -> Element {
    rsx! {
        WorldSessionLayout {
            world_id: world_id.clone(),
            role: ParticipantRole::Spectator,
            page_title: "Watching",

            SpectatorViewContent {}
        }
    }
}

/// SpectatorViewContent - inner content for spectator view
#[component]
fn SpectatorViewContent() -> Element {
    rsx! {
        crate::presentation::views::spectator_view::SpectatorView {}
    }
}
