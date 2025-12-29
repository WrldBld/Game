//! World selection and role selection route handlers

use super::Route;
use dioxus::prelude::*;
use wrldbldr_player_adapters::Platform;
use wrldbldr_player_app::application::dto::ParticipantRole;
use wrldbldr_player_ports::outbound::storage_keys;

/// Role selection route
#[component]
pub fn RoleSelectRoute() -> Element {
    let navigator = use_navigator();
    let platform = use_context::<Platform>();

    // Clone platform for different closures
    let platform_title = platform.clone();
    let platform_storage = platform.clone();

    // Set page title for this view
    use_effect(move || {
        platform_title.set_page_title("Select Role");
    });

    rsx! {
            crate::presentation::views::role_select::RoleSelect {
            on_select_role: move |role: ParticipantRole| {
                // Save selected role preference
                let role_str = format!("{:?}", role);
                platform_storage.storage_save(storage_keys::ROLE, &role_str);
                navigator.push(Route::WorldSelectRoute {});
            }
        }
    }
}

/// World selection route
///
/// Connection is NOT initiated here - destination views handle connection
/// via their ensure_*_connection functions. This prevents race conditions
/// between spawned async tasks and navigation.
#[component]
pub fn WorldSelectRoute() -> Element {
    let navigator = use_navigator();
    let platform = use_context::<Platform>();

    // Clone platform for different closures
    let platform_title = platform.clone();
    let platform_handler = platform.clone();

    // Set page title for this view
    use_effect(move || {
        platform_title.set_page_title("Select World");
    });

    // Load selected role from localStorage
    let role = load_role_from_storage(&platform);

    // Ensure an anonymous user ID exists early in the flow
    let _ = platform.get_user_id();

    rsx! {
        crate::presentation::views::world_select::WorldSelectView {
            role: role,
            on_world_selected: {
                let role = role;
                let platform_storage = platform_handler.clone();
                move |world_id: String| {
                    // Save last accessed world - the destination view will use this
                    platform_storage.storage_save(storage_keys::LAST_WORLD, &world_id);

                    // Navigate to the appropriate view based on role
                    // Connection will be established by the destination view's ensure_*_connection
                    match role {
                        ParticipantRole::DungeonMaster => {
                            navigator.push(Route::DMViewRoute { world_id });
                        }
                        ParticipantRole::Player => {
                            navigator.push(Route::PCViewRoute { world_id });
                        }
                        ParticipantRole::Spectator => {
                            navigator.push(Route::SpectatorViewRoute { world_id });
                        }
                    }
                }
            },
            on_back: move |_| {
                navigator.push(Route::RoleSelectRoute {});
            },
        }
    }
}

/// Load user role from localStorage, defaults to Player
fn load_role_from_storage(platform: &Platform) -> ParticipantRole {
    if let Some(role_str) = platform.storage_load(storage_keys::ROLE) {
        match role_str.as_str() {
            "DungeonMaster" => return ParticipantRole::DungeonMaster,
            "Player" => return ParticipantRole::Player,
            "Spectator" => return ParticipantRole::Spectator,
            _ => {}
        }
    }
    ParticipantRole::Player
}
