//! WrldBldr Player - TTRPG gameplay client
//!
//! The Player app provides two views:
//! - PC View: Visual novel gameplay experience for players
//! - DM View: Directorial control panel for running the game

mod application;
mod domain;
mod infrastructure;
mod presentation;
mod routes;

use dioxus::prelude::*;
use presentation::state::{DialogueState, GameState, GenerationState, SessionState};
use presentation::Services;
use routes::Route;

// Composition root: Define concrete service types using infrastructure implementations
use crate::infrastructure::http_client::ApiAdapter;
pub type ConcreteServices = Services<ApiAdapter>;

#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    // Initialize logging (desktop only - WASM uses tracing-wasm)
    #[cfg(not(target_arch = "wasm32"))]
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wrldbldr_player=debug,dioxus=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        tracing_wasm::set_as_global_default();
    }

    tracing::info!("Starting WrldBldr Player");

    // Launch the Dioxus application
    dioxus::launch(App);
}

/// Root application component with state providers and router
#[component]
fn App() -> Element {
    // Provide platform services via context
    let platform = infrastructure::platform::create_platform();

    use_context_provider(|| platform);

    // Provide global state via context
    use_context_provider(GameState::new);
    use_context_provider(SessionState::new);
    use_context_provider(DialogueState::new);
    use_context_provider(GenerationState::new);

    // Infrastructure instantiation happens HERE only (composition root)
    let api = infrastructure::http_client::ApiAdapter::new();

    // Provide application services via context with the API adapter
    use_context_provider(|| presentation::Services::new(api));

    // Non-DM routes show a simple header, DM routes use their own layout
    // Router handles all view switching
    // Wrapper provides full viewport height for child views using height: 100%
    rsx! {
        div {
            style: "width: 100vw; height: 100vh; overflow: hidden;",
            Router::<Route> {}
        }
    }
}

/// User role in the game session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    DungeonMaster,
    Player,
    Spectator,
}
