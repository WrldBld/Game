//! Dialogue box component for visual novel scenes
//!
//! Displays dialogue with speaker name, text, and choices.

use dioxus::prelude::*;

use crate::application::dto::DialogueChoice;

use super::choice_menu::{ChoiceMenu, ContinuePrompt};

/// Props for the DialogueBox component
#[derive(Props, Clone, PartialEq)]
pub struct DialogueBoxProps {
    /// Speaker name
    pub speaker_name: String,
    /// Dialogue text to display (may be partial during typewriter)
    pub dialogue_text: String,
    /// Whether typewriter is still animating
    #[props(default = false)]
    pub is_typing: bool,
    /// Available dialogue choices
    #[props(default)]
    pub choices: Vec<DialogueChoice>,
    /// Handler for when a choice is selected
    pub on_choice_selected: EventHandler<String>,
    /// Handler for custom text input
    pub on_custom_input: EventHandler<String>,
    /// Handler for advancing dialogue (clicking to continue)
    pub on_advance: EventHandler<()>,
    /// Whether NPC is currently thinking (LLM processing)
    #[props(default = false)]
    pub is_llm_processing: bool,
}

/// Dialogue box component - displays dialogue with typewriter effect
///
/// Uses `.vn-dialogue-box`, `.vn-character-name`, `.vn-dialogue-text` Tailwind classes.
#[component]
pub fn DialogueBox(props: DialogueBoxProps) -> Element {
    let has_speaker = !props.speaker_name.is_empty();
    let has_choices = !props.choices.is_empty();
    let show_continue = !props.is_typing && !has_choices;

    rsx! {
        div {
            class: "vn-dialogue-box",

            // Speaker name plate
            if has_speaker {
                div {
                    class: "vn-character-name",
                    "{props.speaker_name}"
                }
            }

            // Dialogue text with typewriter cursor or loading indicator
            div {
                class: "dialogue-text-container min-h-[60px]",
                onclick: move |_| {
                    if props.is_typing && !props.is_llm_processing {
                        props.on_advance.call(());
                    }
                },

                if props.is_llm_processing {
                    p {
                        class: "vn-dialogue-text text-gray-400 italic",

                        "NPC is thinking"

                        // Animated ellipsis
                        span {
                            class: "animate-ellipsis",
                            "..."
                        }
                    }
                } else {
                    p {
                        class: "vn-dialogue-text",

                        "{props.dialogue_text}"

                        // Blinking cursor during typing
                        if props.is_typing {
                            span {
                                class: "typewriter-cursor animate-blink ml-0.5",
                                "▌"
                            }
                        }
                    }
                }
            }

            // Choice menu or continue prompt (disabled while processing)
            if !props.is_typing && !props.is_llm_processing {
                if has_choices {
                    ChoiceMenu {
                        choices: props.choices.clone(),
                        on_select: props.on_choice_selected,
                        on_custom_input: props.on_custom_input,
                    }
                } else if show_continue {
                    ContinuePrompt {
                        on_continue: props.on_advance,
                    }
                }
            }
        }
    }
}

/// Minimal dialogue box for narration (no speaker name)
#[derive(Props, Clone, PartialEq)]
pub struct NarrationBoxProps {
    /// Narration text
    pub text: String,
    /// Whether typewriter is animating
    #[props(default = false)]
    pub is_typing: bool,
    /// Click handler to advance
    pub on_advance: EventHandler<()>,
}

#[component]
pub fn NarrationBox(props: NarrationBoxProps) -> Element {
    rsx! {
        div {
            class: "vn-dialogue-box narration text-center",
            onclick: move |_| props.on_advance.call(()),

            p {
                class: "vn-dialogue-text italic text-gray-300",

                "{props.text}"

                if props.is_typing {
                    span {
                        class: "typewriter-cursor animate-blink ml-0.5",
                        "▌"
                    }
                }
            }

            if !props.is_typing {
                div {
                    class: "text-gray-500 text-xs mt-2",
                    "Click to continue"
                }
            }
        }
    }
}

/// Empty dialogue box placeholder
#[component]
pub fn EmptyDialogueBox() -> Element {
    rsx! {
        div {
            class: "vn-dialogue-box opacity-50",

            p {
                class: "vn-dialogue-text text-gray-500 italic",
                "..."
            }
        }
    }
}
