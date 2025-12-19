//! Backdrop component for visual novel scenes
//!
//! Displays the background image for the current scene.

use dioxus::prelude::*;

/// Props for the Backdrop component
#[derive(Props, Clone, PartialEq)]
pub struct BackdropProps {
    /// URL or asset path for the backdrop image
    #[props(default)]
    pub image_url: Option<String>,
    /// Whether to show fade transition animation
    #[props(default = false)]
    pub transitioning: bool,
    /// Optional children to render on top of the backdrop
    #[props(default)]
    pub children: Element,
}

/// Backdrop component - displays the scene background
///
/// Uses the `.vn-backdrop` Tailwind class for styling.
/// Falls back to a gradient if no image is provided.
#[component]
pub fn Backdrop(props: BackdropProps) -> Element {
    // Extract conditionals BEFORE rsx! block (CRITICAL for Dioxus)
    let (bg_class, bg_style) = match &props.image_url {
        Some(url) => (
            "bg-cover bg-center",
            format!("background-image: url('{}');", url)
        ),
        None => (
            "bg-gradient-to-b from-dark-surface to-dark-purple-end",
            String::new()
        ),
    };

    rsx! {
        div {
            class: "vn-backdrop absolute inset-0 {bg_class}",
            style: if !bg_style.is_empty() { "{bg_style}" } else { "" },

            // Fade overlay for scene transitions
            if props.transitioning {
                div {
                    class: "backdrop-fade absolute inset-0 bg-black animate-fadeOut",
                }
            }

            // Vignette effect
            div {
                class: "backdrop-vignette absolute inset-0 pointer-events-none shadow-[inset_0_0_150px_rgba(0,0,0,0.5)]",
            }

            // Children (character sprites, etc.)
            {props.children}
        }
    }
}

/// A simple loading backdrop shown while assets are loading
#[component]
pub fn LoadingBackdrop() -> Element {
    rsx! {
        div {
            class: "vn-backdrop absolute inset-0 flex items-center justify-center bg-gradient-to-b from-dark-surface to-dark-purple-end",

            div {
                class: "text-center text-gray-400",

                div {
                    class: "text-2xl mb-4",
                    "Loading..."
                }

                div {
                    class: "loading-spinner w-10 h-10 border-[3px] border-gray-700 border-t-[#d4af37] rounded-full animate-spin mx-auto",
                }
            }
        }
    }
}
