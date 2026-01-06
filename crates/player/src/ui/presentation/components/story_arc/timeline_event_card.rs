//! Timeline Event Card - Display a single story event

use dioxus::prelude::*;

use crate::presentation::components::story_arc::timeline_view::get_event_type_icon;
use crate::application::application::dto::{StoryEventData, StoryEventTypeData};

#[derive(Props, Clone)]
pub struct TimelineEventCardProps {
    pub event: StoryEventData,
    pub on_click: EventHandler<()>,
    pub on_toggle_visibility: EventHandler<()>,
}

impl PartialEq for TimelineEventCardProps {
    fn eq(&self, other: &Self) -> bool {
        self.event.id == other.event.id
    }
}

#[component]
pub fn TimelineEventCard(props: TimelineEventCardProps) -> Element {
    let event = &props.event;
    let icon = get_event_type_icon(&event.event_type);
    let type_color = get_event_type_color(&event.event_type);

    // Parse and format timestamp
    let formatted_time = format_timestamp(&event.timestamp);

    let opacity_class = if event.is_hidden {
        "opacity-50"
    } else {
        "opacity-100"
    };

    rsx! {
        div {
            class: "timeline-event-card bg-dark-surface rounded-lg p-4 cursor-pointer transition-all duration-200 border-l-[3px] {opacity_class}",
            style: "border-left-color: {type_color}",
            onclick: move |_| props.on_click.call(()),

            // Top row: Icon, summary, timestamp
            div {
                class: "flex items-start gap-3",

                // Icon
                div {
                    class: "text-xl p-2 rounded-lg",
                    style: "background-color: {type_color}20",
                    "{icon}"
                }

                // Content
                div {
                    class: "flex-1 min-w-0",

                    // Summary
                    p {
                        class: "text-white m-0 mb-1 text-[0.9375rem] leading-normal",
                        "{event.summary}"
                    }

                    // Metadata row
                    div {
                        class: "flex flex-wrap gap-2 items-center",

                        // Timestamp
                        span {
                            class: "text-gray-500 text-xs",
                            "{formatted_time}"
                        }

                        // Game time if available
                        if let Some(ref game_time) = event.game_time {
                            span {
                                class: "text-purple-500 text-xs",
                                "🕐 {game_time}"
                            }
                        }

                        // Tags
                        for tag in event.tags.iter().take(3) {
                            span {
                                class: "bg-gray-700 text-gray-400 px-1.5 py-0.5 rounded text-[0.6875rem]",
                                "#{tag}"
                            }
                        }
                        if event.tags.len() > 3 {
                            {
                                let extra = event.tags.len() - 3;
                                rsx! {
                                    span {
                                        class: "text-gray-500 text-[0.6875rem]",
                                        "+{extra}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Actions
                div {
                    class: "flex gap-1",

                    // Visibility toggle
                    button {
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_toggle_visibility.call(());
                        },
                        class: "bg-transparent border-none cursor-pointer p-1 text-gray-500",
                        title: if event.is_hidden { "Show in timeline" } else { "Hide from timeline" },
                        if event.is_hidden { "👁️‍🗨️" } else { "👁️" }
                    }
                }
            }

            // Event-specific preview content
            match &event.event_type {
                StoryEventTypeData::DialogueExchange { npc_name, npc_response, .. } => rsx! {
                    div {
                        class: "mt-3 p-3 bg-black bg-opacity-20 rounded-md",
                        p {
                            class: "text-blue-400 text-[0.8125rem] m-0 mb-1",
                            "{npc_name}:"
                        }
                        p {
                            class: "text-gray-400 text-[0.8125rem] m-0 italic overflow-hidden text-ellipsis line-clamp-2",
                            "\"{npc_response}\""
                        }
                    }
                },
                StoryEventTypeData::ChallengeAttempted { challenge_name, outcome, roll_result, .. } => {
                    let outcome_bg = get_outcome_color(outcome);
                    rsx! {
                        div {
                            class: "mt-2 flex gap-3 items-center",
                            span {
                                class: "text-gray-400 text-[0.8125rem]",
                                "{challenge_name}"
                            }
                            if let Some(roll) = roll_result {
                                span {
                                    class: "bg-gray-700 text-white px-2 py-1 rounded text-xs font-mono",
                                    "🎲 {roll}"
                                }
                            }
                            span {
                                class: "px-2 py-1 rounded text-xs text-white",
                                style: "background-color: {outcome_bg}",
                                "{outcome}"
                            }
                        }
                    }
                },
                StoryEventTypeData::DmMarker { title, importance, .. } => {
                    let importance_bg = get_importance_color(importance);
                    rsx! {
                        div {
                            class: "mt-2 flex items-center gap-2",
                            span {
                                class: "px-2 py-1 rounded text-xs text-white",
                                style: "background-color: {importance_bg}",
                                "{importance}"
                            }
                            span {
                                class: "text-gray-400 text-[0.8125rem]",
                                "{title}"
                            }
                        }
                    }
                },
                StoryEventTypeData::RelationshipChanged { previous_sentiment, new_sentiment, .. } => {
                    let change = new_sentiment - previous_sentiment.unwrap_or(0.0);
                    let change_icon = if change > 0.0 { "📈" } else { "📉" };
                    let change_color_class = if change > 0.0 { "text-green-500" } else { "text-red-500" };
                    let change_str = format!("{:+.1}", change);
                    let sentiment_str = format!("{:.1}", new_sentiment);
                    rsx! {
                        div {
                            class: "mt-2 flex items-center gap-2",
                            span { "{change_icon}" }
                            span {
                                class: "{change_color_class} text-[0.8125rem]",
                                "{change_str}"
                            }
                            span {
                                class: "text-gray-500 text-[0.8125rem]",
                                "→ {sentiment_str}"
                            }
                        }
                    }
                },
                _ => rsx! {}
            }
        }
    }
}

/// Get a color for each event type
fn get_event_type_color(event_type: &StoryEventTypeData) -> &'static str {
    match event_type {
        StoryEventTypeData::LocationChange { .. } => "#22c55e",
        StoryEventTypeData::DialogueExchange { .. } => "#3b82f6",
        StoryEventTypeData::CombatEvent { .. } => "#ef4444",
        StoryEventTypeData::ChallengeAttempted { .. } => "#f59e0b",
        StoryEventTypeData::ItemAcquired { .. } => "#a855f7",
        StoryEventTypeData::RelationshipChanged { .. } => "#ec4899",
        StoryEventTypeData::SceneTransition { .. } => "#06b6d4",
        StoryEventTypeData::InformationRevealed { .. } => "#eab308",
        StoryEventTypeData::DmMarker { .. } => "#8b5cf6",
        StoryEventTypeData::NarrativeEventTriggered { .. } => "#14b8a6",
        StoryEventTypeData::SessionStarted { .. } => "#22c55e",
        StoryEventTypeData::SessionEnded { .. } => "#6b7280",
        StoryEventTypeData::Custom { .. } => "#9ca3af",
    }
}

/// Get color for challenge outcome
fn get_outcome_color(outcome: &str) -> &'static str {
    match outcome.to_lowercase().as_str() {
        "critical_success" | "criticalsuccess" => "#22c55e",
        "success" => "#3b82f6",
        "partial_success" | "partialsuccess" => "#f59e0b",
        "failure" => "#ef4444",
        "critical_failure" | "criticalfailure" => "#dc2626",
        _ => "#6b7280",
    }
}

/// Get color for DM marker importance
fn get_importance_color(importance: &str) -> &'static str {
    match importance.to_lowercase().as_str() {
        "critical" => "#ef4444",
        "major" => "#f59e0b",
        "normal" => "#3b82f6",
        "minor" => "#6b7280",
        _ => "#6b7280",
    }
}

/// Format a timestamp string for display
fn format_timestamp(timestamp: &str) -> String {
    // Try to parse as RFC3339 and format nicely
    // If that fails, just return the original string
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%b %d, %H:%M").to_string()
    } else {
        timestamp.to_string()
    }
}
