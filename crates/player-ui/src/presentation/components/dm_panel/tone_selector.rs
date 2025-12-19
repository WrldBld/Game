//! Tone selector component
//!
//! Dropdown selector for scene tone/mood influencing NPC behavior.

use dioxus::prelude::*;

/// Available tone options for the scene
const TONE_OPTIONS: &[&str] = &[
    "Serious",
    "Lighthearted",
    "Tense",
    "Mysterious",
    "Comedic",
    "Romantic",
    "Tragic",
    "Suspenseful",
    "Custom",
];

/// Props for the ToneSelector component
#[derive(Props, Clone, PartialEq)]
pub struct ToneSelectorProps {
    /// Currently selected tone
    pub selected: String,
    /// Handler called when tone is changed
    pub on_change: EventHandler<String>,
    /// Optional custom tone description
    #[props(default)]
    pub custom_tone: Option<String>,
}

/// Get a color for a tone for visual feedback
fn get_tone_color(tone: &str) -> &'static str {
    match tone {
        "Serious" => "#ef4444",
        "Lighthearted" => "#fbbf24",
        "Tense" => "#f97316",
        "Mysterious" => "#8b5cf6",
        "Comedic" => "#ec4899",
        "Romantic" => "#ec4899",
        "Tragic" => "#6366f1",
        "Suspenseful" => "#dc2626",
        _ => "#3b82f6",
    }
}

/// ToneSelector component - Select scene tone/atmosphere
///
/// Provides a dropdown to select the current scene's tone,
/// which influences how NPCs should behave and respond.
#[component]
pub fn ToneSelector(props: ToneSelectorProps) -> Element {
    let tone_color = get_tone_color(&props.selected);

    rsx! {
        div {
            class: "tone-selector flex flex-col",

            // Label
            label {
                class: "text-gray-400 text-xs uppercase mb-2 font-semibold",
                "Scene Tone"
            }

            // Dropdown with color indicator
            div {
                class: "flex items-center gap-3",

                // Color indicator
                div {
                    class: "w-3 h-3 rounded-full shrink-0",
                    style: format!("background: {};", tone_color),
                }

                // Select dropdown
                select {
                    value: "{props.selected}",
                    onchange: move |e| props.on_change.call(e.value()),
                    class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm cursor-pointer transition-colors",

                    for tone in TONE_OPTIONS.iter() {
                        option {
                            value: "{tone}",
                            "{tone}"
                        }
                    }
                }
            }

            // Tone description
            div {
                class: "mt-2 p-2 bg-black bg-opacity-20 rounded-md text-gray-400 text-xs",

                match props.selected.as_str() {
                    "Serious" => rsx! { "A grave and solemn atmosphere. NPCs are thoughtful and careful." },
                    "Lighthearted" => rsx! { "A playful and fun atmosphere. NPCs are cheerful and relaxed." },
                    "Tense" => rsx! { "An edge-of-your-seat atmosphere. NPCs are suspicious and alert." },
                    "Mysterious" => rsx! { "A puzzle-like atmosphere. NPCs are secretive and cryptic." },
                    "Comedic" => rsx! { "A humorous atmosphere. NPCs are witty and absurd." },
                    "Romantic" => rsx! { "An emotional atmosphere. NPCs are passionate and genuine." },
                    "Tragic" => rsx! { "A mournful atmosphere. NPCs are sorrowful and resigned." },
                    "Suspenseful" => rsx! { "A thrilling atmosphere. NPCs are nervous and guarded." },
                    _ => {
                        if let Some(custom) = &props.custom_tone {
                            rsx! { "{custom}" }
                        } else {
                            rsx! { "Custom tone selected." }
                        }
                    }
                }
            }
        }
    }
}
