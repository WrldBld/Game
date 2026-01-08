//! Application routing - URL-based navigation for all views
//!
//! This module handles all routing for the WrldBldr Player application.
//!
//! ## Browser History & Deep Linking
//!
//! The application uses Dioxus Router which provides automatic browser history support:
//! - URL updates automatically on navigation (browser address bar updates)
//! - Back/forward buttons work correctly (router handles state restoration)
//! - Deep links work: users can share or bookmark direct URLs to specific views
//! - Missing state redirects: if a user navigates directly to a view without required context,
//!   the application redirects to the appropriate setup step (MainMenu → RoleSelect → WorldSelect)
//!
//! ## Page Titles
//!
//! Each route component sets a dynamic page title visible in the browser tab.
//! This helps users distinguish between different views when multiple tabs are open.
//!
//! ## localStorage Persistence
//!
//! On web platforms (WASM), the application persists user preferences:
//! - Server URL: Remembers the last connected server
//! - Selected Role: Saves the player's role choice
//! - Last World: Remembers the last world they accessed
//!
//! These are loaded on application startup and saved when changed.

mod connection;
mod dm_routes;
mod main_menu;
mod pc_creation;
mod player_routes;
mod world_select;
mod world_session_layout;

pub use dm_routes::{
    DMCreatorSubTabRoute, DMSettingsSubTabRoute, DMStoryArcSubTabRoute, DMViewRoute, DMViewTabRoute,
};
pub use main_menu::MainMenuRoute;
pub use pc_creation::PCCreationRoute;
pub use player_routes::{PCViewRoute, SpectatorViewRoute};
pub use world_select::{RoleSelectRoute, WorldSelectRoute};

use dioxus::prelude::*;

/// Application routes - each URL maps to a view
#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    MainMenuRoute {},

    #[route("/roles")]
    RoleSelectRoute {},

    #[route("/worlds")]
    WorldSelectRoute {},

    // DM view with tab parameter - defaults to "director"
    #[route("/worlds/:world_id/dm")]
    DMViewRoute { world_id: String },

    #[route("/worlds/:world_id/dm/:tab")]
    DMViewTabRoute { world_id: String, tab: String },

    // Creator mode sub-tabs
    #[route("/worlds/:world_id/dm/creator/:subtab")]
    DMCreatorSubTabRoute { world_id: String, subtab: String },

    // Settings sub-tabs
    #[route("/worlds/:world_id/dm/settings/:subtab")]
    DMSettingsSubTabRoute { world_id: String, subtab: String },

    // Story Arc sub-tabs
    #[route("/worlds/:world_id/dm/story-arc/:subtab")]
    DMStoryArcSubTabRoute { world_id: String, subtab: String },

    #[route("/worlds/:world_id/play")]
    PCViewRoute { world_id: String },

    #[route("/worlds/:world_id/play/create-character")]
    PCCreationRoute { world_id: String },

    #[route("/worlds/:world_id/watch")]
    SpectatorViewRoute { world_id: String },

    #[route("/:..route")]
    NotFoundRoute { route: Vec<String> },
}

/// Not Found page
#[component]
pub fn NotFoundRoute(route: Vec<String>) -> Element {
    let navigator = use_navigator();
    let platform = crate::use_platform();

    // Set page title for this view
    use_effect(move || {
        platform.set_page_title("Page Not Found");
    });

    rsx! {
        div {
            class: "flex flex-col items-center justify-center h-full text-white bg-dark-bg",

            h1 {
                class: "text-6xl text-red-500 m-0",
                "404"
            }
            p {
                class: "text-gray-400 my-4 mb-8",
                "Page not found: /{route.join(\"/\")}"
            }

            button {
                onclick: move |_| {
                    navigator.push(Route::MainMenuRoute {});
                },
                class: "py-3 px-6 bg-blue-500 text-white border-none rounded-lg no-underline cursor-pointer text-base",
                "← Back to Main Menu"
            }
        }
    }
}
