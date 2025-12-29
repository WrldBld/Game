//! Main menu route handler

use super::Route;
use dioxus::prelude::*;
use wrldbldr_player_adapters::Platform;
use wrldbldr_player_app::application::services::DEFAULT_ENGINE_URL;
use wrldbldr_player_ports::outbound::storage_keys;

/// Main menu route - automatically redirects to role selection
#[component]
pub fn MainMenuRoute() -> Element {
    let navigator = use_navigator();
    let platform = use_context::<Platform>();

    // On web, automatically connect to the default (or last-used) server and
    // skip the manual "Connect to Server" modal. This keeps the flow:
    // MainMenu → RoleSelect → WorldSelect, without an extra click.
    let platform_for_effect = platform.clone();
    let navigator_for_effect = navigator.clone();
    use_effect(move || {
        // Load last-used server URL or fall back to the default WS URL
        let server_url = platform_for_effect
            .storage_load(storage_keys::SERVER_URL)
            .unwrap_or_else(|| DEFAULT_ENGINE_URL.to_string());

        // Persist it so subsequent screens can read it
        platform_for_effect.storage_save(storage_keys::SERVER_URL, &server_url);

        // Configure Engine HTTP base URL from the WebSocket URL
        platform_for_effect.configure_engine_url(&server_url);

        // Go straight to role selection
        navigator_for_effect.push(Route::RoleSelectRoute {});
    });

    // Minimal placeholder while the effect redirects
    rsx! {
        div {
            class: "flex items-center justify-center h-full text-white bg-dark-bg",
            "Loading WrldBldr..."
        }
    }
}
