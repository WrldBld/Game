//! ComfyUI Connection Status Banner
//!
//! Displays a banner when ComfyUI is disconnected or experiencing issues.

use dioxus::prelude::*;

use crate::presentation::state::use_session_state;

#[derive(Props, Clone, PartialEq)]
pub struct ComfyUIBannerProps {
    pub state: String, // "connected", "degraded", "disconnected", "circuit_open"
    pub message: Option<String>,
    pub retry_in_seconds: Option<u32>,
}

/// Banner component showing ComfyUI connection status
#[component]
pub fn ComfyUIBanner(props: ComfyUIBannerProps) -> Element {
    let session_state = use_session_state();

    // Only show banner if not connected
    if props.state == "connected" {
        return rsx! {};
    }

    let (icon, bg_class) = match props.state.as_str() {
        "degraded" => ("⚠️", "bg-amber-500"),
        "disconnected" | "circuit_open" => ("🔴", "bg-red-500"),
        _ => ("❓", "bg-gray-500"),
    };

    rsx! {
        div {
            class: format!("{} text-white py-3 px-4 flex items-center justify-between rounded-md mb-4", bg_class),
            div {
                class: "flex items-center gap-2",
                span { class: "text-xl", "{icon}" }
                div {
                    div {
                        class: "font-medium text-sm",
                        if let Some(msg) = props.message.as_ref() {
                            "{msg}"
                        } else {
                            "ComfyUI Connection Issue"
                        }
                    }
                    if let Some(seconds) = props.retry_in_seconds {
                        div {
                            class: "text-xs opacity-90 mt-1",
                            "Reconnecting in {seconds}s..."
                        }
                    }
                }
            }
            button {
                onclick: move |_| {
                    if let Some(client) = session_state.engine_client().read().as_ref() {
                        let _ = client.check_comfyui_health();
                    }
                },
                class: "bg-white bg-opacity-20 border border-white border-opacity-30 text-white py-1.5 px-3 rounded cursor-pointer text-xs",
                "Retry Now"
            }
        }
    }
}
