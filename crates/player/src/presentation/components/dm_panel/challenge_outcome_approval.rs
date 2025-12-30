//! Challenge Outcome Approval Component (P3.3/P3.4)
//!
//! DM approval card for pending challenge outcomes. Displays roll results
//! and allows DM to accept, edit, or request LLM suggestions.

use dioxus::prelude::*;
use crate::presentation::state::PendingChallengeOutcome;
use crate::application::dto::websocket_messages::ChallengeOutcomeDecisionData;

/// Props for ChallengeOutcomeApprovalCard
#[derive(Props, Clone, PartialEq)]
pub struct ChallengeOutcomeApprovalCardProps {
    /// The pending outcome to display
    pub outcome: PendingChallengeOutcome,
    /// Callback when DM makes a decision
    pub on_decision: EventHandler<(String, ChallengeOutcomeDecisionData)>,
}

/// Card for approving a challenge outcome (P3.3/P3.4)
#[component]
pub fn ChallengeOutcomeApprovalCard(props: ChallengeOutcomeApprovalCardProps) -> Element {
    // Clone outcome data to avoid lifetime issues
    let outcome = props.outcome.clone();
    let resolution_id = outcome.resolution_id.clone();
    let outcome_description = outcome.outcome_description.clone();
    let original_description = outcome_description.clone(); // For cancel button reset

    // Editing state
    let mut is_editing = use_signal(|| false);
    let mut edited_description = use_signal(move || outcome_description.clone());
    let mut show_suggestions = use_signal(|| false);

    // Determine border color based on outcome type
    let border_color = match outcome.outcome_type.as_str() {
        "critical_success" => "border-yellow-400",
        "success" => "border-green-500",
        "failure" => "border-red-500",
        "critical_failure" => "border-red-700",
        _ => "border-amber-500",
    };

    // Outcome type display
    let outcome_display = match outcome.outcome_type.as_str() {
        "critical_success" => ("CRITICAL SUCCESS", "text-yellow-400"),
        "success" => ("SUCCESS", "text-green-500"),
        "failure" => ("FAILURE", "text-red-500"),
        "critical_failure" => ("CRITICAL FAILURE", "text-red-700"),
        _ => ("RESULT", "text-amber-500"),
    };

    rsx! {
        div {
            class: "bg-dark-bg rounded-lg border-2 {border_color} p-4 mb-3",

            // Header with challenge name and outcome
            div {
                class: "flex justify-between items-start mb-3",

                div {
                    h4 {
                        class: "text-white font-semibold m-0",
                        "{outcome.challenge_name}"
                    }
                    p {
                        class: "text-gray-400 text-sm m-0",
                        "by {outcome.character_name}"
                    }
                }

                span {
                    class: "text-xs font-bold uppercase {outcome_display.1}",
                    "{outcome_display.0}"
                }
            }

            // Roll breakdown
            div {
                class: "bg-black/30 rounded p-3 mb-3",

                div {
                    class: "flex justify-between text-sm",
                    span { class: "text-gray-400", "Roll: {outcome.roll}" }
                    span { class: "text-gray-400", "Mod: {outcome.modifier}" }
                    span { class: "text-white font-bold", "Total: {outcome.total}" }
                }

                if let Some(breakdown) = &outcome.roll_breakdown {
                    p {
                        class: "text-gray-500 text-xs mt-2 m-0 font-mono",
                        "{breakdown}"
                    }
                }
            }

            // Outcome description (editable)
            if *is_editing.read() {
                div {
                    class: "mb-3",

                    textarea {
                        class: "w-full p-3 bg-black/30 border border-amber-500/50 rounded text-white text-sm resize-none min-h-[100px] box-border",
                        value: "{edited_description}",
                        oninput: move |e| edited_description.set(e.value().to_string()),
                    }

                    div {
                        class: "flex justify-end gap-2 mt-2",

                        button {
                            class: "px-3 py-1.5 bg-transparent border border-gray-600 text-gray-400 rounded text-sm cursor-pointer hover:border-gray-500",
                            onclick: {
                                let original_description = original_description.clone();
                                move |_| {
                                    is_editing.set(false);
                                    edited_description.set(original_description.clone());
                                }
                            },
                            "Cancel"
                        }

                        button {
                            class: "px-3 py-1.5 bg-amber-500 text-white rounded text-sm cursor-pointer hover:bg-amber-400",
                            onclick: {
                                let resolution_id = resolution_id.clone();
                                move |_| {
                                    let description = edited_description.read().clone();
                                    props.on_decision.call((
                                        resolution_id.clone(),
                                        ChallengeOutcomeDecisionData::Edit { modified_description: description }
                                    ));
                                }
                            },
                            "Save & Approve"
                        }
                    }
                }
            } else {
                div {
                    class: "bg-black/20 rounded p-3 mb-3",

                    p {
                        class: "text-gray-300 text-sm m-0 leading-relaxed italic",
                        "{outcome.outcome_description}"
                    }
                }
            }

            // LLM Suggestions (if available)
            if let Some(suggestions) = &outcome.suggestions {
                if *show_suggestions.read() {
                    div {
                        class: "mb-3 border border-purple-500/30 rounded p-3",

                        h5 {
                            class: "text-purple-400 text-xs uppercase m-0 mb-2",
                            "LLM Suggestions"
                        }

                        for (idx, suggestion) in suggestions.iter().enumerate() {
                            button {
                                key: "{idx}",
                                class: "w-full text-left p-2 bg-black/30 rounded mb-2 text-gray-300 text-sm border border-transparent hover:border-purple-500/50 cursor-pointer",
                                onclick: {
                                    let suggestion = suggestion.clone();
                                    move |_| {
                                        edited_description.set(suggestion.clone());
                                        is_editing.set(true);
                                        show_suggestions.set(false);
                                    }
                                },
                                "{suggestion}"
                            }
                        }

                        button {
                            class: "text-gray-500 text-xs underline cursor-pointer bg-transparent border-none",
                            onclick: move |_| show_suggestions.set(false),
                            "Hide suggestions"
                        }
                    }
                }
            }

            // Suggestion request (if generating)
            if outcome.is_generating_suggestions {
                div {
                    class: "flex items-center gap-2 text-purple-400 text-sm mb-3",

                    div {
                        class: "w-4 h-4 border-2 border-purple-500 border-t-transparent rounded-full animate-spin",
                    }
                    span { "Generating suggestions..." }
                }
            }

            // Action buttons
            if !*is_editing.read() {
                div {
                    class: "flex gap-2",

                    // Accept button
                    button {
                        class: "flex-1 py-2 bg-green-600 text-white rounded text-sm font-semibold cursor-pointer hover:bg-green-500 border-none",
                        onclick: {
                            let resolution_id = resolution_id.clone();
                            move |_| {
                                props.on_decision.call((
                                    resolution_id.clone(),
                                    ChallengeOutcomeDecisionData::Accept
                                ));
                            }
                        },
                        "Accept"
                    }

                    // Edit button
                    button {
                        class: "flex-1 py-2 bg-amber-600 text-white rounded text-sm font-semibold cursor-pointer hover:bg-amber-500 border-none",
                        onclick: move |_| {
                            is_editing.set(true);
                        },
                        "Edit"
                    }

                    // Suggest button
                    if outcome.suggestions.is_some() {
                        button {
                            class: "flex-1 py-2 bg-purple-600 text-white rounded text-sm font-semibold cursor-pointer hover:bg-purple-500 border-none",
                            onclick: move |_| {
                                show_suggestions.set(true);
                            },
                            "View"
                        }
                    } else if !outcome.is_generating_suggestions {
                        button {
                            class: "flex-1 py-2 bg-purple-600 text-white rounded text-sm font-semibold cursor-pointer hover:bg-purple-500 border-none",
                            onclick: {
                                let resolution_id = resolution_id.clone();
                                move |_| {
                                    props.on_decision.call((
                                        resolution_id.clone(),
                                        ChallengeOutcomeDecisionData::Suggest { guidance: None }
                                    ));
                                }
                            },
                            "Suggest"
                        }
                    }
                }
            }
        }
    }
}

/// Section showing all pending challenge outcomes (P3.3/P3.4)
#[component]
pub fn ChallengeOutcomesSection(
    pending_outcomes: Vec<PendingChallengeOutcome>,
    on_decision: EventHandler<(String, ChallengeOutcomeDecisionData)>,
) -> Element {
    if pending_outcomes.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "challenge-outcomes-section mb-4",

            h4 {
                class: "text-amber-500 text-xs uppercase mb-2 flex items-center gap-2",
                span {
                    class: "inline-flex items-center justify-center w-5 h-5 bg-amber-500 text-dark-bg rounded-full text-xs font-bold",
                    "{pending_outcomes.len()}"
                }
                "Challenge Results"
            }

            for outcome in pending_outcomes.iter() {
                ChallengeOutcomeApprovalCard {
                    key: "{outcome.resolution_id}",
                    outcome: outcome.clone(),
                    on_decision: move |args| on_decision.call(args),
                }
            }
        }
    }
}
