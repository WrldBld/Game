//! Active Conversations Panel - DM panel for viewing active conversations
//!
//! This component displays all active conversations in the world, allowing
//! the DM to monitor and manage them.

use dioxus::prelude::*;

/// Render a single conversation card
fn render_conversation_card(
    conversation: &wrldbldr_shared::ConversationInfo,
    on_view_details: EventHandler<String>,
) -> Element {
    let conv_id = conversation.conversation_id.clone();
    rsx! {
        div {
            class: "bg-gray-700 rounded p-3 border border-gray-600 hover:bg-gray-600 cursor-pointer",
            onclick: move |_| on_view_details.call(conv_id.clone()),
            // Conversation header
            div {
                class: "flex justify-between items-start mb-2",
                div {
                    class: "font-semibold",
                    "{conversation.conversation_id.clone()}"
                }
                if conversation.pending_approval {
                    span {
                        class: "bg-yellow-600 text-xs px-2 py-1 rounded",
                        "Pending Approval"
                    }
                }
            }
            // Topic hint
            if let Some(ref topic) = conversation.topic_hint {
                p {
                    class: "text-sm text-gray-300 mb-2",
                    "{topic}"
                }
            }
            // Participants
            div {
                class: "text-sm text-gray-400 mb-2",
                for participant in conversation.participants.iter() {
                    span {
                        class: "mr-2",
                        "{participant.name}"
                        match participant.participant_type {
                            wrldbldr_shared::ParticipantType::Pc => " (PC)",
                            wrldbldr_shared::ParticipantType::Npc => " (NPC)",
                            wrldbldr_shared::ParticipantType::Unknown => " (?)",
                        }
                    }
                }
            }
            // Location context
            if let Some(ref location) = conversation.location {
                p {
                    class: "text-xs text-gray-500",
                    "{location.location_name} - {location.region_name}"
                }
            }
            // Turn count
            div {
                class: "text-xs text-gray-500",
                "{conversation.turn_count} turns"
            }
            // Timestamps
            div {
                class: "text-xs text-gray-500",
                "Started: {format_timestamp(&conversation.started_at)}"
            }
        }
    }
}

#[component]
pub fn ActiveConversationsPanel(
    conversations: Vec<wrldbldr_shared::ConversationInfo>,
    loading: bool,
    on_refresh: EventHandler<()>,
    on_view_details: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "active-conversations-panel p-4",
            if loading {
                div { class: "flex items-center justify-center p-8", "Loading conversations..." }
            } else if conversations.is_empty() {
                p { class: "text-gray-400", "No active conversations" }
            } else {
                div {
                    class: "flex justify-between items-center mb-4",
                    h2 { class: "text-lg font-bold", "Active Conversations" }
                    button {
                        onclick: move |_| on_refresh.call(()),
                        class: "px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm",
                        "Refresh"
                    }
                }
                div {
                    class: "space-y-2",
                    for conversation in &conversations {
                        { render_conversation_card(conversation, on_view_details) }
                    }
                }
            }
        }
    }
}

/// Format an ISO 8601 timestamp to a readable display format
fn format_timestamp(iso_timestamp: &str) -> String {
    // Simple display of the timestamp - could be enhanced with proper parsing
    iso_timestamp.to_string()
}
