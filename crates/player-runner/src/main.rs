//! WrldBldr Player Runner - composition root binary
//!
//! The UI lives in `wrldbldr-player-ui`.

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

    let platform = wrldbldr_player_adapters::infrastructure::platform::create_platform();
    let raw_api: std::sync::Arc<dyn wrldbldr_player_ports::outbound::RawApiPort> =
        std::sync::Arc::new(wrldbldr_player_adapters::infrastructure::http_client::ApiAdapter::new());

    let api = wrldbldr_player_app::application::api::Api::new(raw_api.clone());

    let shell = {
        #[cfg(target_arch = "wasm32")]
        {
            // On web, pick a shell based on screen size.
            // We keep the default conservative and treat small widths as mobile.
            let width = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(1024.0);

            if width < 768.0 {
                wrldbldr_player_ports::config::ShellKind::Mobile
            } else {
                wrldbldr_player_ports::config::ShellKind::Desktop
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::var("WRLDBLDR_SHELL")
                .ok()
                .and_then(|s| s.parse::<wrldbldr_player_ports::config::ShellKind>().ok())
                .unwrap_or_default()
        }
    };

    let config = wrldbldr_player_ports::config::RunnerConfig { shell };

    // Get WebSocket URL from environment or use default
    let ws_url = std::env::var("WRLDBLDR_ENGINE_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:3456/ws".to_string());
    let connection = wrldbldr_player_adapters::infrastructure::ConnectionFactory::create_game_connection(&ws_url);

    wrldbldr_player_runner::run(wrldbldr_player_runner::RunnerDeps {
        platform,
        api,
        raw_api,
        connection,
        config,
    });
}
