use std::sync::Arc;

use wrldbldr_player_app::application::api::Api;
use wrldbldr_player_ports::{
    config::RunnerConfig,
    outbound::{GameConnectionPort, Platform, RawApiPort},
};

pub struct RunnerDeps {
    pub platform: Platform,
    pub api: Api,
    pub raw_api: Arc<dyn RawApiPort>,
    pub connection: Arc<dyn GameConnectionPort>,
    pub config: RunnerConfig,
}

pub fn run(deps: RunnerDeps) {
    let RunnerDeps {
        platform,
        api,
        raw_api,
        connection,
        config,
    } = deps;

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
        .with_context(config)
        .with_context(wrldbldr_player_ui::presentation::Services::new(
            api, raw_api, connection,
        ))
        .launch(wrldbldr_player_ui::app);
}

#[cfg(not(target_arch = "wasm32"))]
fn load_player_css() -> String {
    const FALLBACK_CSS: &str = "";

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    let css_path = repo_root.join("crates/player-ui/assets/css/output.css");
    std::fs::read_to_string(css_path).unwrap_or_else(|_| FALLBACK_CSS.to_string())
}
