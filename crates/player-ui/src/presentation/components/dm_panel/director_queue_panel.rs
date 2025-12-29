//! Director Queue Panel - Minimal generation queue view for Director Mode
//!
//! Shows active generation batches and suggestions in a compact format
//! suitable for Director Mode overlay.

use dioxus::prelude::*;

use crate::presentation::state::{use_generation_state, BatchStatus, SuggestionStatus};

/// Props for DirectorQueuePanel
#[derive(Props, Clone, PartialEq)]
pub struct DirectorQueuePanelProps {
    /// Handler called when panel should close
    pub on_close: EventHandler<()>,
}

/// Minimal queue panel for Director Mode
#[component]
pub fn DirectorQueuePanel(props: DirectorQueuePanelProps) -> Element {
    let generation_state = use_generation_state();
    let batches = generation_state.get_batches();
    let suggestions = generation_state.get_suggestions();

    // Filter to only active items
    let active_batches: Vec<_> = batches
        .iter()
        .filter(|b| {
            matches!(
                b.status,
                BatchStatus::Queued { .. } | BatchStatus::Generating { .. }
            )
        })
        .collect();
    let active_suggestions: Vec<_> = suggestions
        .iter()
        .filter(|s| {
            matches!(
                s.status,
                SuggestionStatus::Queued | SuggestionStatus::Processing
            )
        })
        .collect();

    rsx! {
        div {
            class: "director-queue-panel fixed top-0 right-0 bottom-0 w-[400px] bg-dark-surface border-l border-gray-700 z-[1000] flex flex-col shadow-[-4px_0_6px_rgba(0,0,0,0.3)]",

            // Header
            div {
                class: "flex justify-between items-center p-4 border-b border-gray-700",
                h3 { class: "text-white m-0 text-base", "Generation Queue" }
                button {
                    onclick: move |_| props.on_close.call(()),
                    class: "py-1 px-2 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                    "×"
                }
            }

            // Content
            div {
                class: "flex-1 overflow-y-auto p-4",
                if active_batches.is_empty() && active_suggestions.is_empty() {
                    div {
                        class: "text-center text-gray-500 p-8",
                        "No active generations"
                    }
                } else {
                    div {
                        class: "flex flex-col gap-3",

                        // Active batches
                        for batch in active_batches.iter() {
                            div {
                                key: "{batch.batch_id}",
                                class: "p-3 bg-dark-bg rounded-lg border-l-[3px] border-l-purple-500",
                                div {
                                    class: "flex justify-between items-start mb-2",
                                    div {
                                        class: "flex-1",
                                        div {
                                            class: "text-white text-sm font-medium",
                                            "{batch.entity_type} - {batch.asset_type}"
                                        }
                                        div {
                                            class: "text-gray-400 text-xs mt-1",
                                            match &batch.status {
                                                BatchStatus::Queued { position } => format!("#{} in queue", position),
                                                BatchStatus::Generating { progress } => format!("Generating... {}%", progress),
                                                _ => String::new(),
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Active suggestions
                        for suggestion in active_suggestions.iter() {
                            div {
                                key: "{suggestion.request_id}",
                                class: "p-3 bg-dark-bg rounded-lg border-l-[3px] border-l-blue-500",
                                div {
                                    class: "flex justify-between items-start",
                                    div {
                                        class: "flex-1",
                                        div {
                                            class: "text-white text-sm font-medium",
                                            "{suggestion.field_type} suggestion"
                                        }
                                        div {
                                            class: "text-gray-400 text-xs mt-1",
                                            match &suggestion.status {
                                                SuggestionStatus::Queued => "Queued",
                                                SuggestionStatus::Processing => "Processing...",
                                                _ => "",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
