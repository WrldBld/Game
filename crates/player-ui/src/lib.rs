use dioxus::prelude::*;

pub mod routes;
pub mod presentation;

pub use routes::Route;

pub fn app() -> Element {
    // Provided by `wrldbldr-player-runner`.
    let config = use_context::<wrldbldr_player_ports::config::RunnerConfig>();

    match config.shell {
        wrldbldr_player_ports::config::ShellKind::Desktop => rsx! {
            DesktopShell {
                Router::<routes::Route> {}
            }
        },
        wrldbldr_player_ports::config::ShellKind::Mobile => rsx! {
            MobileShell {
                Router::<routes::Route> {}
            }
        },
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
