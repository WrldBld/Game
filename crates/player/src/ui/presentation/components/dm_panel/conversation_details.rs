//! Conversation Details Panel - DM panel for viewing conversation details
//!
//! This component displays detailed information about a specific conversation,
//! including all dialogue turns and participant information.

use dioxus::prelude::*;

#[component]
pub fn ConversationDetailsPanel(
    details: Option<wrldbldr_shared::ConversationFullDetails>,
    loading: bool,
    on_close: EventHandler<()>,
    on_end: EventHandler<()>,
    on_view_history: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| on_close.call(()),
            div {
                class: "bg-dark-surface rounded-lg w-[90%] max-w-[900px] max-h-[90vh] overflow-hidden flex flex-col",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center p-4 border-b border-gray-700",

                    h2 {
                        class: "m-0 text-white text-xl",
                        "Conversation Details"
                    }

                    div {
                        class: "flex gap-2",
                        button {
                            onclick: move |_| on_view_history.call(()),
                            class: "px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm",
                            "View History"
                        }
                        button {
                            onclick: move |_| on_end.call(()),
                            class: "px-3 py-1 bg-red-600 hover:bg-red-700 text-white rounded text-sm",
                            "End Conversation"
                        }
                        button {
                            onclick: move |_| on_close.call(()),
                            class: "px-2 py-1 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                            "×"
                        }
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4",
                    if loading {
                        div { class: "flex items-center justify-center p-8", "Loading conversation details..." }
                    } else if let Some(ref details) = details {
                        // Conversation info
                        div {
                            class: "bg-gray-700 rounded p-4 mb-4 border border-gray-600",
                            div {
                                class: "text-sm text-gray-400 mb-2",
                                "ID: {details.conversation_id}"
                            }
                            if let Some(ref topic) = details.topic_hint {
                                p {
                                    class: "font-semibold mb-2",
                                    "{topic}"
                                }
                            }
                            div {
                                class: "flex justify-between text-sm text-gray-400 mb-2",
                                div {
                                    "Turns: {details.turn_count}"
                                }
                                if details.pending_approval {
                                    span {
                                        class: "bg-yellow-600 text-xs px-2 py-1 rounded",
                                        "Pending Approval"
                                    }
                                }
                            }
                            div {
                                class: "text-xs text-gray-500",
                                "Started: {format_timestamp(&details.started_at)}"
                            }
                            div {
                                class: "text-xs text-gray-500 mb-2",
                                "Last Updated: {format_timestamp(&details.last_updated_at)}"
                            }
                            // Location and scene context
                            if let Some(ref location) = details.location {
                                p {
                                    class: "text-sm text-gray-300",
                                    "Location: {location.location_name} ({location.region_name})"
                                }
                            }
                            if let Some(ref scene) = details.scene {
                                p {
                                    class: "text-sm text-gray-300",
                                    "Scene: {scene.scene_name}"
                                }
                            }
                        }

                        // Participants
                        h3 {
                            class: "font-semibold mb-2",
                            "Participants"
                        }
                        div {
                            class: "space-y-2 mb-4",
                            for participant in details.participants.iter() {
                                div {
                                    class: "bg-gray-700 rounded p-2 border border-gray-600",
                                    div {
                                        class: "flex justify-between",
                                        span {
                                            class: "font-semibold",
                                            "{participant.name}"
                                        }
                                        span {
                                            class: format!("text-xs px-2 py-1 rounded {}",
                                                match participant.participant_type {
                                                    wrldbldr_shared::ParticipantType::Pc => "bg-blue-600",
                                                    wrldbldr_shared::ParticipantType::Npc => "bg-purple-600",
                                                    wrldbldr_shared::ParticipantType::Unknown => "bg-gray-600",
                                                }
                                            ),
                                            match participant.participant_type {
                                                wrldbldr_shared::ParticipantType::Pc => "PC",
                                                wrldbldr_shared::ParticipantType::Npc => "NPC",
                                                wrldbldr_shared::ParticipantType::Unknown => "?",
                                            }
                                        }
                                    }
                                    div {
                                        class: "text-sm text-gray-400",
                                        "Turns: {participant.turn_count}"
                                    }
                                    if let Some(ref spoke_at) = participant.last_spoke_at {
                                        div {
                                            class: "text-xs text-gray-500",
                                            "Last spoke: {format_timestamp(spoke_at.as_str())}"
                                        }
                                    }
                                    if let Some(ref want) = participant.want {
                                        p {
                                            class: "text-sm text-gray-300 mt-2",
                                            "Want: {want}"
                                        }
                                    }
                                    if let Some(ref relationship) = participant.relationship {
                                        p {
                                            class: "text-sm text-gray-300",
                                            "Relationship: {relationship}"
                                        }
                                    }
                                }
                            }
                        }

                        // Recent turns
                        h3 {
                            class: "font-semibold mb-2",
                            "Recent Dialogue"
                        }
                        if details.recent_turns.is_empty() {
                            p {
                                class: "text-gray-400",
                                "No dialogue turns recorded"
                            }
                        } else {
                            div {
                                class: "space-y-2",
                                for turn in details.recent_turns.iter() {
                                    div {
                                        class: "bg-gray-700 rounded p-3 border border-gray-600",
                                        div {
                                            class: "flex justify-between items-start mb-1",
                                            span {
                                                class: "font-semibold",
                                                "{turn.speaker_name}"
                                            }
                                            if turn.is_dm_override {
                                                span {
                                                    class: "bg-red-600 text-xs px-2 py-1 rounded",
                                                    "DM Override"
                                                }
                                            }
                                        }
                                        p {
                                            class: "text-gray-300",
                                            "{turn.text}"
                                        }
                                        div {
                                            class: "text-xs text-gray-500",
                                            "{format_timestamp(&turn.timestamp)}"
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        p {
                            class: "text-gray-400",
                            "No conversation details loaded"
                        }
                    }
                }
            }
        }
    }
}

/// Format an ISO 8601 timestamp to a readable display format
fn format_timestamp(iso_timestamp: &str) -> String {
    // Simple display of timestamp - could be enhanced with proper parsing
    iso_timestamp.to_string()
}
