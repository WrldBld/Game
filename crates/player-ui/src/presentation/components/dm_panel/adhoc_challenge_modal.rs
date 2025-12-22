//! Ad-hoc Challenge Modal Component
//!
//! Allows DM to create custom challenges on-the-fly without LLM involvement.
//! The DM specifies all challenge details including outcomes.

use dioxus::prelude::*;
use wrldbldr_protocol::AdHocOutcomes;
use wrldbldr_protocol::CharacterData as SceneCharacterState;

/// Data for an ad-hoc challenge creation
#[derive(Debug, Clone, PartialEq)]
pub struct AdHocChallengeData {
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty: String,
    pub target_pc_id: String,
    pub outcomes: AdHocOutcomes,
}

/// Props for AdHocChallengeModal
#[derive(Props, Clone, PartialEq)]
pub struct AdHocChallengeModalProps {
    /// List of PCs that can be targeted
    pub player_characters: Vec<SceneCharacterState>,
    /// Called when the challenge is created
    pub on_create: EventHandler<AdHocChallengeData>,
    /// Called when modal should close
    pub on_close: EventHandler<()>,
}

/// AdHocChallengeModal component
///
/// Allows DM to create custom challenges with:
/// - Challenge name
/// - Skill being tested
/// - Difficulty (e.g., "DC 15", "Hard")
/// - Target PC selection
/// - Custom outcomes (success, failure, optional criticals)
#[component]
pub fn AdHocChallengeModal(props: AdHocChallengeModalProps) -> Element {
    let mut challenge_name = use_signal(|| String::new());
    let mut skill_name = use_signal(|| String::new());
    let mut difficulty = use_signal(|| String::new());
    let mut selected_pc = use_signal(|| String::new());
    let mut success_outcome = use_signal(|| String::new());
    let mut failure_outcome = use_signal(|| String::new());
    let mut critical_success = use_signal(|| String::new());
    let mut critical_failure = use_signal(|| String::new());
    let mut show_criticals = use_signal(|| false);

    let player_characters = props.player_characters.clone();

    // Validation: all required fields must be filled
    let is_valid = !challenge_name.read().is_empty()
        && !skill_name.read().is_empty()
        && !difficulty.read().is_empty()
        && !selected_pc.read().is_empty()
        && !success_outcome.read().is_empty()
        && !failure_outcome.read().is_empty();

    let is_expanded = *show_criticals.read();
    let rotation_class = if is_expanded { "rotate-90" } else { "rotate-0" };
    let create_btn_bg = if is_valid { "bg-purple-600" } else { "bg-gray-500" };
    let create_btn_cursor = if is_valid { "cursor-pointer" } else { "cursor-not-allowed" };

    rsx! {
        // Modal overlay
        div {
            id: "adhoc-overlay",
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000] overflow-y-auto p-4",
            onclick: move |_| {
                props.on_close.call(());
            },

            // Modal content
            div {
                id: "adhoc-modal",
                class: "bg-gradient-to-br from-dark-surface to-dark-bg p-8 rounded-2xl max-w-[600px] w-[90%] border-2 border-purple-600 max-h-[90vh] overflow-y-auto",
                onclick: move |evt| evt.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center mb-6",

                    h2 {
                        class: "text-purple-600 m-0 text-2xl",
                        "Create Ad-hoc Challenge"
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-0 text-gray-400 cursor-pointer text-2xl p-0",
                        "x"
                    }
                }

                // Challenge details section
                div {
                    class: "grid grid-cols-2 gap-4 mb-6",

                    // Challenge name
                    div {
                        label {
                            class: "block text-gray-400 text-xs uppercase mb-1",
                            "Challenge Name *"
                        }
                        input {
                            r#type: "text",
                            value: "{challenge_name}",
                            placeholder: "e.g., Negotiate Price",
                            oninput: move |e| challenge_name.set(e.value()),
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm box-border",
                        }
                    }

                    // Skill name
                    div {
                        label {
                            class: "block text-gray-400 text-xs uppercase mb-1",
                            "Skill Being Tested *"
                        }
                        input {
                            r#type: "text",
                            value: "{skill_name}",
                            placeholder: "e.g., Persuasion",
                            oninput: move |e| skill_name.set(e.value()),
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm box-border",
                        }
                    }

                    // Difficulty
                    div {
                        label {
                            class: "block text-gray-400 text-xs uppercase mb-1",
                            "Difficulty *"
                        }
                        input {
                            r#type: "text",
                            value: "{difficulty}",
                            placeholder: "e.g., DC 15, Hard",
                            oninput: move |e| difficulty.set(e.value()),
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm box-border",
                        }
                    }

                    // Target PC
                    div {
                        label {
                            class: "block text-gray-400 text-xs uppercase mb-1",
                            "Target PC *"
                        }
                        select {
                            value: "{selected_pc}",
                            onchange: move |e| selected_pc.set(e.value()),
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white cursor-pointer text-sm box-border",

                            option {
                                value: "",
                                disabled: true,
                                selected: true,
                                "Select PC..."
                            }

                            for character in player_characters.iter() {
                                option {
                                    key: "{character.id}",
                                    value: "{character.id}",
                                    "{character.name}"
                                }
                            }
                        }
                    }
                }

                // Outcomes section header
                div {
                    class: "border-t border-gray-700 pt-6 mb-4",

                    h3 {
                        class: "text-amber-500 m-0 mb-2 text-lg",
                        "Outcomes"
                    }
                    p {
                        class: "text-gray-500 text-xs m-0",
                        "Define what happens when the player succeeds or fails the challenge."
                    }
                }

                // Success outcome
                div {
                    class: "mb-4",

                    label {
                        class: "block text-green-500 text-xs uppercase mb-1",
                        "Success Outcome *"
                    }
                    textarea {
                        value: "{success_outcome}",
                        placeholder: "What happens when the player succeeds...",
                        oninput: move |e| success_outcome.set(e.value()),
                        class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm min-h-[80px] resize-y box-border",
                    }
                }

                // Failure outcome
                div {
                    class: "mb-4",

                    label {
                        class: "block text-red-500 text-xs uppercase mb-1",
                        "Failure Outcome *"
                    }
                    textarea {
                        value: "{failure_outcome}",
                        placeholder: "What happens when the player fails...",
                        oninput: move |e| failure_outcome.set(e.value()),
                        class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm min-h-[80px] resize-y box-border",
                    }
                }

                // Toggle for critical outcomes
                div {
                    class: "mb-4",

                    button {
                        onclick: move |_| {
                            let current = *show_criticals.read();
                            show_criticals.set(!current);
                        },
                        class: "bg-transparent border-0 text-purple-600 cursor-pointer text-sm p-0 flex items-center gap-2",

                        span {
                            class: "transition-transform duration-200 {rotation_class}",
                            ">"
                        }
                        "Add Critical Outcomes (optional)"
                    }
                }

                // Critical outcomes (collapsible)
                if *show_criticals.read() {
                    div {
                        class: "pl-4 border-l-2 border-purple-600",

                        // Critical success
                        div {
                            class: "mb-4",

                            label {
                                class: "block text-amber-400 text-xs uppercase mb-1",
                                "Critical Success (optional)"
                            }
                            textarea {
                                value: "{critical_success}",
                                placeholder: "What happens on a critical success (e.g., nat 20)...",
                                oninput: move |e| critical_success.set(e.value()),
                                class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm min-h-[60px] resize-y box-border",
                            }
                        }

                        // Critical failure
                        div {
                            class: "mb-4",

                            label {
                                class: "block text-red-600 text-xs uppercase mb-1",
                                "Critical Failure (optional)"
                            }
                            textarea {
                                value: "{critical_failure}",
                                placeholder: "What happens on a critical failure (e.g., nat 1)...",
                                oninput: move |e| critical_failure.set(e.value()),
                                class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm min-h-[60px] resize-y box-border",
                            }
                        }
                    }
                }

                // Action buttons
                div {
                    class: "flex gap-3 mt-6",

                    button {
                        onclick: move |_| {
                            if is_valid {
                                let data = AdHocChallengeData {
                                    challenge_name: challenge_name.read().clone(),
                                    skill_name: skill_name.read().clone(),
                                    difficulty: difficulty.read().clone(),
                                    target_pc_id: selected_pc.read().clone(),
                                    outcomes: AdHocOutcomes {
                                        success: success_outcome.read().clone(),
                                        failure: failure_outcome.read().clone(),
                                        critical_success: if critical_success.read().is_empty() {
                                            None
                                        } else {
                                            Some(critical_success.read().clone())
                                        },
                                        critical_failure: if critical_failure.read().is_empty() {
                                            None
                                        } else {
                                            Some(critical_failure.read().clone())
                                        },
                                    },
                                };
                                props.on_create.call(data);
                            }
                        },
                        disabled: !is_valid,
                        class: "flex-1 p-3 {create_btn_bg} text-white border-0 rounded-lg {create_btn_cursor} font-semibold",

                        "Create Challenge"
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
