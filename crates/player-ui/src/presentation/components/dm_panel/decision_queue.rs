//! Decision Queue Panel - Shows pending approvals and recent decisions for the DM

use dioxus::prelude::*;

use wrldbldr_protocol::ChallengeOutcomeDecisionData;
use crate::presentation::components::dm_panel::challenge_outcome_approval::ChallengeOutcomesSection;
use crate::presentation::state::use_session_state;

/// Compact decision queue view for Director mode
#[component]
pub fn DecisionQueuePanel() -> Element {
    let session_state = use_session_state();

    let pending = session_state.pending_approvals().read().clone();
    let pending_outcomes = session_state.pending_challenge_outcomes().read().clone();
    let history = session_state.get_approval_history();

    let mut show_history_only: Signal<bool> = use_signal(|| false);

    let has_pending = !pending.is_empty();
    let has_pending_outcomes = !pending_outcomes.is_empty();
    let has_history = !history.is_empty();

    rsx! {
        div {
            class: "decision-queue-panel bg-dark-surface rounded-lg p-3 flex flex-col gap-2",

            // Header with toggle
            div {
                class: "flex justify-between items-center",

                h3 {
                    class: "text-gray-400 m-0 text-xs uppercase",
                    "Decision Queue"
                }

                if has_history {
                    label {
                        class: "inline-flex items-center gap-1 text-gray-400 text-xs",
                        input {
                            r#type: "checkbox",
                            checked: *show_history_only.read(),
                            onchange: move |_| {
                                let current = *show_history_only.read();
                                show_history_only.set(!current);
                            },
                        }
                        span { "Show history only" }
                    }
                }
            }

            // Content
            if !has_pending && !has_pending_outcomes && !has_history {
                div {
                    class: "text-gray-500 text-sm text-center p-2",
                    "No decisions yet"
                }
            } else {
                // Challenge outcome approvals (P3.3/P3.4)
                if has_pending_outcomes && !*show_history_only.read() {
                    ChallengeOutcomesSection {
                        pending_outcomes: pending_outcomes.clone(),
                        on_decision: move |(resolution_id, decision): (String, ChallengeOutcomeDecisionData)| {
                            // Send the decision to the Engine via WebSocket
                            if let Some(client) = session_state.engine_client().read().as_ref() {
                                if let Err(e) = client.send_challenge_outcome_decision(&resolution_id, decision) {
                                    tracing::error!("Failed to send challenge outcome decision: {}", e);
                                }
                            }
                        },
                    }
                }

                // Pending approvals list
                if has_pending && !*show_history_only.read() {
                    div {
                        class: "flex flex-col gap-1.5 mb-1",
                        for approval in pending.iter() {
                            div {
                                key: "{approval.request_id}",
                                class: "flex flex-col gap-0.5 py-1.5 px-2 bg-dark-bg rounded-md",

                                div {
                                    class: "flex justify-between items-center",
                                    span { class: "text-white text-sm", "{approval.npc_name}" }
                                    span { class: "text-amber-500 text-xs", "Pending" }
                                }

                                if let Some(challenge) = &approval.challenge_suggestion {
                                    div {
                                        class: "text-gray-400 text-xs",
                                        "Challenge: {challenge.challenge_name}"
                                    }
                                } else if let Some(narrative) = &approval.narrative_event_suggestion {
                                    div {
                                        class: "text-gray-400 text-xs",
                                        "Narrative: {narrative.event_name}"
                                    }
                                } else {
                                    div {
                                        class: "text-gray-400 text-xs overflow-hidden text-ellipsis whitespace-nowrap",
                                        "{approval.proposed_dialogue}"
                                    }
                                }
                            }
                        }
                    }
                }

                // History list
                if has_history {
                    div {
                        class: "border-t border-gray-700 mt-1 pt-1 flex flex-col gap-1",

                        // Caption
                        div {
                            class: "text-gray-500 text-xs",
                            "Recent decisions ({history.len()})"
                        }

                        for entry in history.iter().rev().take(10) {
                            div {
                                key: "{entry.request_id}-{entry.timestamp}",
                                class: "flex justify-between items-center py-1 px-1.5 bg-dark-bg rounded",

                                div {
                                    class: "flex flex-col gap-0.5",
                                    span {
                                        class: "text-white text-sm",
                                        "{entry.npc_name}"
                                    }
                                    span {
                                        class: "text-gray-500 text-xs",
                                        "#{entry.request_id}"
                                    }
                                }

                                span {
                                    class: "text-blue-300 text-xs capitalize",
                                    "{entry.outcome}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


