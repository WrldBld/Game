//! Conversation log component
//!
//! Displays a scrollable log of dialogue turns with speakers and timestamps.

use dioxus::prelude::*;

/// Challenge result information for special rendering
#[derive(Clone, PartialEq)]
pub struct ChallengeResultInfo {
    /// Name of the challenge
    pub challenge_name: String,
    /// Character who performed the challenge
    pub character_name: String,
    /// The roll value
    pub roll: i32,
    /// The modifier applied
    pub modifier: i32,
    /// Total (roll + modifier)
    pub total: i32,
    /// Outcome type (success, failure, critical_success, critical_failure)
    pub outcome_type: String,
    /// Description of the outcome
    pub outcome_description: String,
}

/// A single turn in the conversation
#[derive(Clone, PartialEq)]
pub struct ConversationTurn {
    /// Who is speaking (character name or "System")
    pub speaker: String,
    /// The spoken text
    pub text: String,
    /// Timestamp of the turn
    pub timestamp: String,
    /// Whether this is a system message
    pub is_system: bool,
    /// Optional challenge result data for special rendering
    pub challenge_result: Option<ChallengeResultInfo>,
}

/// Props for the ConversationLog component
#[derive(Props, Clone, PartialEq)]
pub struct ConversationLogProps {
    /// History of conversation turns
    pub turns: Vec<ConversationTurn>,
    /// Optional CSS class for styling
    #[props(default = "")]
    pub class: &'static str,
}

/// ConversationLog component - Scrollable dialogue history
///
/// Shows a log of all dialogue exchanges with speaker names and timestamps.
/// Useful for reviewing what has been said during the session.
#[component]
pub fn ConversationLog(props: ConversationLogProps) -> Element {
    // Auto-scroll to bottom when new turns are added (CSS scroll-behavior handles this)
    use_effect({
        let _turn_count = props.turns.len();
        move || {
            // Future: trigger scroll to bottom when new turns are added
        }
    });

    rsx! {
        div {
            class: "conversation-log {props.class} flex flex-col h-full bg-dark-surface rounded-lg overflow-hidden",

            // Header
            div {
                class: "py-3 px-4 border-b border-gray-700",

                h3 {
                    class: "text-gray-400 text-sm uppercase m-0",
                    "Conversation Log"
                }
            }

            // Log entries (scrollable)
            div {
                class: "log-entries flex-1 overflow-y-auto p-4 flex flex-col gap-3",

                // Empty state
                if props.turns.is_empty() {
                    div {
                        class: "flex items-center justify-center h-full text-gray-500 text-sm",
                        "Waiting for dialogue..."
                    }
                } else {
                    for turn in props.turns.iter() {
                        ConversationEntry {
                            turn: turn.clone(),
                        }
                    }
                }
            }
        }
    }
}

/// Individual conversation entry
#[component]
fn ConversationEntry(turn: ConversationTurn) -> Element {
    // Check if this is a challenge result
    if let Some(ref result) = turn.challenge_result {
        return rsx! { ChallengeResultEntry { result: result.clone(), timestamp: turn.timestamp.clone() } };
    }

    let (bg_class, text_class, border_class, speaker_class) = if turn.is_system {
        (
            "bg-blue-500 bg-opacity-10",
            "text-blue-400",
            "border-blue-400",
            "text-blue-400",
        )
    } else {
        (
            "bg-black bg-opacity-30",
            "text-white",
            "border-blue-500",
            "text-blue-500",
        )
    };

    rsx! {
        div {
            class: "log-entry p-3 rounded-md border-l-2 {bg_class} {border_class}",

            // Header with speaker and timestamp
            div {
                class: "flex items-center gap-2 mb-1",

                span {
                    class: "font-semibold text-sm {speaker_class}",
                    "{turn.speaker}"
                }

                span {
                    class: "text-gray-500 text-xs ml-auto",
                    "{turn.timestamp}"
                }
            }

            // Message text
            p {
                class: "text-sm leading-snug m-0 {text_class}",
                "{turn.text}"
            }
        }
    }
}

/// Challenge result entry with special amber styling (P3.3/P3.4)
#[component]
fn ChallengeResultEntry(result: ChallengeResultInfo, timestamp: String) -> Element {
    // Determine colors based on outcome type
    let (border_color, outcome_color, outcome_label) = match result.outcome_type.as_str() {
        "critical_success" => ("border-yellow-400", "text-yellow-400", "CRITICAL SUCCESS"),
        "success" => ("border-green-500", "text-green-500", "SUCCESS"),
        "failure" => ("border-red-500", "text-red-500", "FAILURE"),
        "critical_failure" => ("border-red-700", "text-red-700", "CRITICAL FAILURE"),
        _ => ("border-amber-500", "text-amber-500", "RESULT"),
    };

    // Format modifier with sign
    let modifier_display = if result.modifier >= 0 {
        format!("+{}", result.modifier)
    } else {
        format!("{}", result.modifier)
    };

    rsx! {
        div {
            class: "log-entry p-3 rounded-md border-l-2 bg-amber-500 bg-opacity-10 {border_color}",

            // Header with challenge name and timestamp
            div {
                class: "flex items-center justify-between mb-2",

                div {
                    span {
                        class: "font-semibold text-sm text-amber-500",
                        "{result.challenge_name}"
                    }
                    span {
                        class: "text-gray-400 text-xs ml-2",
                        "by {result.character_name}"
                    }
                }

                span {
                    class: "text-gray-500 text-xs",
                    "{timestamp}"
                }
            }

            // Roll breakdown
            div {
                class: "flex items-center gap-4 bg-black/30 rounded px-3 py-2 mb-2",

                span {
                    class: "text-gray-400 text-sm",
                    "Roll: {result.roll}"
                }
                span {
                    class: "text-gray-400 text-sm",
                    "Mod: {modifier_display}"
                }
                span {
                    class: "text-white text-sm font-bold",
                    "Total: {result.total}"
                }
                span {
                    class: "text-xs font-bold uppercase ml-auto {outcome_color}",
                    "{outcome_label}"
                }
            }

            // Outcome description
            p {
                class: "text-sm leading-snug m-0 text-gray-300 italic",
                "{result.outcome_description}"
            }
        }
    }
}
