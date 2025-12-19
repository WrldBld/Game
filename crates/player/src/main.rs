//! WrldBldr Player - TTRPG gameplay client
//!
//! This crate is the *composition root* for the player application.
//! The UI lives in `wrldbldr-player-ui` and infrastructure adapters live in
//! `wrldbldr-player-adapters`.

use dioxus::prelude::*;
use std::sync::Arc;

use wrldbldr_player_adapters::infrastructure::{http_client::ApiAdapter, platform};
use wrldbldr_player_app::application::api::Api;
use wrldbldr_player_ports::outbound::RawApiPort;
use wrldbldr_player_ui::{
    presentation::state::{DialogueState, GameState, GenerationState, SessionState},
    Route,
};

#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
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
    dioxus::launch(app);
}

#[component]
fn app() -> Element {
    // Platform is used throughout UI via `use_context::<Platform>()`.
    let platform = platform::create_platform();
    use_context_provider(|| platform);

    // Global UI state.
    use_context_provider(GameState::new);
    use_context_provider(SessionState::new);
    use_context_provider(DialogueState::new);
    use_context_provider(GenerationState::new);

    // Concrete adapter chosen here only.
    let raw_api: Arc<dyn RawApiPort> = Arc::new(ApiAdapter::new());
    let api = Api::new(raw_api);

    // UI consumes a typed `ApiPort` (the `Api` wrapper around `RawApiPort`).
    use_context_provider(|| wrldbldr_player_ui::presentation::Services::new(api));

    rsx! {
        div {
            style: "width: 100vw; height: 100vh; overflow: hidden;",
            Router::<Route> {}
        }
    }
}
