use crate::ports::outbound::PlatformPort;
use dioxus::prelude::*;
use std::sync::Arc;

pub mod presentation;
pub mod routes;

pub use routes::Route;

/// Shell variant for UI layout selection.
/// This is passed via Dioxus context from the runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShellKind {
    #[default]
    Desktop,
    Mobile,
}

/// Type alias for the platform port used throughout the UI
pub type Platform = Arc<dyn PlatformPort>;

/// Hook to access the Platform from Dioxus context
pub fn use_platform() -> Platform {
    use_context::<Platform>()
}

pub fn app() -> Element {
    rsx! {
        AppRoot {}
    }
}

#[component]
fn AppRoot() -> Element {
    // Provided by the composition root (see `crates/player/src/main.rs`).
    let shell = use_context::<ShellKind>();

    // These must be created inside an active Dioxus runtime.
    use_context_provider(presentation::state::GameState::new);
    use_context_provider(presentation::state::SessionState::new);
    use_context_provider(presentation::state::DialogueState::new);
    use_context_provider(presentation::state::GenerationState::new);
    use_context_provider(presentation::state::LoreState::new);

    rsx! {
        document::Stylesheet {
            href: asset!("assets/css/output.css"),
        }

        {
            match shell {
                ShellKind::Desktop => rsx! {
                    DesktopShell {
                        Router::<routes::Route> {}
                    }
                },
                ShellKind::Mobile => rsx! {
                    MobileShell {
                        Router::<routes::Route> {}
                    }
                },
            }
        }
    }
}

#[component]
fn DesktopShell(children: Element) -> Element {
    rsx! {
        div {
            style: "width: 100vw; height: 100vh; overflow: hidden;",
            {children}
        }
    }
}

#[component]
fn MobileShell(children: Element) -> Element {
    rsx! {
        // For now, mobile uses the same router and layout bounds.
        // Keeping it separate lets us swap in a mobile-first layout later.
        div {
            style: "width: 100vw; height: 100vh; overflow: hidden;",
            {children}
        }
    }
}
