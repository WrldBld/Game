//! Trigger Challenge Modal Component
//!
//! Allows DM to select and trigger a challenge for a specific character.

use crate::application::dto::{ChallengeData, CharacterData as SceneCharacterState};
use dioxus::prelude::*;

/// Props for TriggerChallengeModal
#[derive(Props, Clone, PartialEq)]
pub struct TriggerChallengeModalProps {
    /// List of available challenges
    pub challenges: Vec<ChallengeData>,
    /// List of characters in the current scene to target
    pub scene_characters: Vec<SceneCharacterState>,
    /// Called when a challenge is triggered
    pub on_trigger: EventHandler<(String, String)>, // (challenge_id, character_id)
    /// Called when modal should close
    pub on_close: EventHandler<()>,
}

/// TriggerChallengeModal component
///
/// Allows DM to:
/// - Select a challenge from available challenges
/// - Select a target character
/// - Trigger the challenge
#[component]
pub fn TriggerChallengeModal(props: TriggerChallengeModalProps) -> Element {
    let mut selected_challenge = use_signal(String::new);
    let mut selected_character = use_signal(String::new);

    let challenges = props.challenges.clone();
    let scene_characters = props.scene_characters.clone();

    let is_both_selected =
        !selected_challenge.read().is_empty() && !selected_character.read().is_empty();
    let trigger_btn_bg = if is_both_selected {
        "bg-green-500"
    } else {
        "bg-gray-500"
    };
    let trigger_btn_cursor = if is_both_selected {
        "cursor-pointer"
    } else {
        "cursor-not-allowed"
    };

    rsx! {
        // Modal overlay
        div {
            id: "trigger-overlay",
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| {
                props.on_close.call(());
            },

            // Modal content
            div {
                id: "trigger-modal",
                class: "bg-gradient-to-br from-dark-surface to-dark-bg p-8 rounded-2xl max-w-[500px] w-[90%] border-2 border-amber-500",
                onclick: move |evt| evt.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center mb-6",

                    h2 {
                        class: "text-amber-500 m-0 text-2xl",
                        "Trigger Challenge"
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-0 text-gray-400 cursor-pointer text-2xl p-0",
                        "×"
                    }
                }

                // Challenge selection
                div {
                    class: "mb-6",

                    label {
                        class: "block text-gray-400 text-sm uppercase mb-2",
                        "Select Challenge"
                    }

                    select {
                        value: "{selected_challenge}",
                        onchange: move |e| selected_challenge.set(e.value()),
                        class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white cursor-pointer text-sm",

                        option {
                            value: "",
                            disabled: true,
                            selected: true,
                            "Choose a challenge..."
                        }

                        for challenge in challenges.iter() {
                            option {
                                key: "{challenge.id}",
                                value: "{challenge.id}",
                                "{challenge.name}"
                            }
                        }
                    }
                }

                // Challenge preview
                if !selected_challenge.read().is_empty() {
                    {
                        let selected_id = selected_challenge.read().clone();
                        if let Some(challenge) = challenges.iter().find(|c| c.id == selected_id) {
                            rsx! {
                                div {
                                    class: "mb-6 p-4 bg-black/30 rounded-lg border-l-3 border-l-amber-500",

                                    p {
                                        class: "text-gray-400 text-xs uppercase m-0 mb-2",
                                        "Challenge Preview"
                                    }

                                    p {
                                        class: "text-white m-0 mb-2 leading-normal",
                                        "{challenge.description}"
                                    }

                                    div {
                                        class: "flex gap-4 text-sm",

                                        span { class: "text-gray-400",
                                            "Type: "
                                            span { class: "text-blue-500", "{challenge.challenge_type:?}" }
                                        }

                                        span { class: "text-gray-400",
                                            "Difficulty: "
                                            span { class: "text-amber-500", "{challenge.difficulty:?}" }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { div {} }
                        }
                    }
                }

                // Character selection
                div {
                    class: "mb-6",

                    label {
                        class: "block text-gray-400 text-sm uppercase mb-2",
                        "Target Character"
                    }

                    select {
                        value: "{selected_character}",
                        onchange: move |e| selected_character.set(e.value()),
                        class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white cursor-pointer text-sm",

                        option {
                            value: "",
                            disabled: true,
                            selected: true,
                            "Choose a character..."
                        }

                        for character in scene_characters.iter() {
                            option {
                                key: "{character.id}",
                                value: "{character.id}",
                                "{character.name}"
                            }
                        }
                    }
                }

                // Character preview
                if !selected_character.read().is_empty() {
                    {
                        let selected_char_id = selected_character.read().clone();
                        if let Some(character) = scene_characters.iter().find(|c| c.id == selected_char_id) {
                            rsx! {
                                div {
                                    class: "mb-6 p-4 bg-blue-500/10 rounded-lg border-l-3 border-l-blue-500",

                                    p {
                                        class: "text-white m-0 font-bold",
                                        "Target: {character.name}"
                                    }
                                }
                            }
                        } else {
                            rsx! { div {} }
                        }
                    }
                }

                // Action buttons
                div {
                    class: "flex gap-3",

                    button {
                        onclick: move |_| {
                            let challenge_id = selected_challenge.read().clone();
                            let character_id = selected_character.read().clone();
                            if !challenge_id.is_empty() && !character_id.is_empty() {
                                props.on_trigger.call((challenge_id, character_id));
                            }
                        },
                        disabled: selected_challenge.read().is_empty() || selected_character.read().is_empty(),
                        class: "flex-1 p-3 {trigger_btn_bg} text-white border-0 rounded-lg {trigger_btn_cursor} font-semibold",

                        "Trigger Challenge"
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "flex-1 p-3 bg-gray-700 text-white border-0 rounded-lg cursor-pointer font-semibold",
                        "Cancel"
                    }
                }
            }
        }
    }
}
