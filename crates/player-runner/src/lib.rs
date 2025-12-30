use std::sync::Arc;

use wrldbldr_player_adapters::Platform;
use wrldbldr_player_app::application::api::Api;
use wrldbldr_player_ports::outbound::{GameConnectionPort, PlatformPort, RawApiPort};

/// Configuration types for the player runner.
pub mod config {
    use std::str::FromStr;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ShellKind {
        Desktop,
        Mobile,
    }

    impl Default for ShellKind {
        fn default() -> Self {
            Self::Desktop
        }
    }

    impl FromStr for ShellKind {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.trim().to_ascii_lowercase().as_str() {
                "desktop" => Ok(Self::Desktop),
                "mobile" => Ok(Self::Mobile),
                other => Err(format!("unknown shell kind: {other}")),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct RunnerConfig {
        pub shell: ShellKind,
    }

    impl Default for RunnerConfig {
        fn default() -> Self {
            Self {
                shell: ShellKind::default(),
            }
        }
    }
}

use config::RunnerConfig;

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

    // Wrap Platform in Arc<dyn PlatformPort> for UI layer abstraction
    let platform_port: Arc<dyn PlatformPort> = Arc::new(platform);

    let mut builder = dioxus::LaunchBuilder::new();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let css = load_player_css();
        let head = format!("<style>{}</style>", css);
        let cfg = dioxus_desktop::Config::new().with_custom_head(head);
        builder = builder.with_cfg(cfg);
    }

    // Convert runner's ShellKind to player-ui's ShellKind for context
    let ui_shell = match config.shell {
        config::ShellKind::Desktop => wrldbldr_player_ui::ShellKind::Desktop,
        config::ShellKind::Mobile => wrldbldr_player_ui::ShellKind::Mobile,
    };

    builder
        .with_context(platform_port)
        .with_context(ui_shell)
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
