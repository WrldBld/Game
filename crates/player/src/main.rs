//! WrldBldr Player - unified composition root binary.

#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use wrldbldr_player::ports::outbound::PlatformPort;

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

    // Platform
    let platform = wrldbldr_player::infrastructure::platform::create_platform();
    let platform: std::sync::Arc<dyn PlatformPort> = std::sync::Arc::new(platform);

    // HTTP
    let raw_api =
        std::sync::Arc::new(wrldbldr_player::infrastructure::http_client::ApiAdapter::new());
    let api = wrldbldr_player::application::api::Api::new(raw_api.clone());

    // Shell kind (desktop vs mobile layout)
    let shell = {
        #[cfg(target_arch = "wasm32")]
        {
            let width = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(1024.0);

            if width < 768.0 {
                wrldbldr_player::ui::ShellKind::Mobile
            } else {
                wrldbldr_player::ui::ShellKind::Desktop
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::var("WRLDBLDR_SHELL")
                .ok()
                .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
                    "desktop" => Some(wrldbldr_player::ui::ShellKind::Desktop),
                    "mobile" => Some(wrldbldr_player::ui::ShellKind::Mobile),
                    _ => None,
                })
                .unwrap_or_default()
        }
    };

    // Engine WS URL
    // Prefer the legacy env var used by docker/dev scripts; fall back to the newer name.
    let ws_url = std::env::var("ENGINE_WS_URL")
        .or_else(|_| std::env::var("WRLDBLDR_ENGINE_WS_URL"))
        .unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());

    // Create initial connection to get CommandBus
    // Note: The connection is established immediately but the session
    // (world join) happens later when navigating to a world.
    // IMPORTANT: We must keep the connection handle alive for the duration of the app,
    // otherwise the bridge task will disconnect immediately when the handle is dropped.
    let connection = wrldbldr_player::infrastructure::websocket::create_connection(&ws_url);
    let command_bus = connection.command_bus.clone();
    // Leak the connection to keep the handle alive for the app's lifetime.
    // This is intentional - the connection lives as long as the app.
    std::mem::forget(connection);

    // Launch Dioxus
    #[allow(unused_mut)]
    let mut builder = dioxus::LaunchBuilder::new();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let css = load_player_css();
        let head = format!("<style>{}</style>", css);
        let cfg = dioxus_desktop::Config::new().with_custom_head(head);
        builder = builder.with_cfg(cfg);
    }

    builder
        .with_context(platform)
        .with_context(shell)
        .with_context(wrldbldr_player::ui::presentation::Services::new(
            api, raw_api, command_bus,
        ))
        .launch(wrldbldr_player::ui::app);
}

#[cfg(not(target_arch = "wasm32"))]
fn load_player_css() -> String {
    const FALLBACK_CSS: &str = "";

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    // New unified crate path
    let css_path = repo_root.join("crates/player/assets/css/output.css");
    std::fs::read_to_string(css_path).unwrap_or_else(|_| FALLBACK_CSS.to_string())
}
