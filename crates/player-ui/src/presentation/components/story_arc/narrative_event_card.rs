//! Narrative Event Card - Display a single narrative event

use dioxus::prelude::*;

use wrldbldr_player_app::application::dto::NarrativeEventData;

#[derive(Props, Clone)]
pub struct NarrativeEventCardProps {
    pub event: NarrativeEventData,
    pub on_click: EventHandler<()>,
    pub on_toggle_favorite: EventHandler<()>,
    pub on_toggle_active: EventHandler<()>,
}

impl PartialEq for NarrativeEventCardProps {
    fn eq(&self, other: &Self) -> bool {
        self.event.id == other.event.id
            && self.event.is_favorite == other.event.is_favorite
            && self.event.is_active == other.event.is_active
            && self.event.is_triggered == other.event.is_triggered
    }
}

#[component]
pub fn NarrativeEventCard(props: NarrativeEventCardProps) -> Element {
    let event = &props.event;

    // Determine status color and label
    let (status_label, status_color_class) = if event.is_triggered {
        ("Triggered", "text-green-500")
    } else if event.is_active {
        ("Active", "text-blue-500")
    } else {
        ("Inactive", "text-gray-500")
    };

    // Priority indicator
    let priority_color_class = match event.priority {
        p if p >= 8 => "bg-red-500",
        p if p >= 5 => "bg-amber-500",
        p if p >= 3 => "bg-blue-500",
        _ => "bg-gray-500",
    };

    // Conditional classes for card
    let card_border_class = if event.is_active { "border-gray-700" } else { "border-gray-800" };
    let card_opacity_class = if event.is_active { "opacity-100" } else { "opacity-70" };

    // Favorite button conditional classes
    let favorite_color_class = if event.is_favorite { "text-amber-500" } else { "text-gray-500 opacity-50" };

    // Active toggle conditional classes
    let active_toggle_class = if event.is_active { "text-green-500" } else { "text-gray-500" };

    rsx! {
        div {
            class: "narrative-event-card bg-dark-surface rounded-lg p-4 cursor-pointer transition-all border {card_border_class} {card_opacity_class}",
            onclick: move |_| props.on_click.call(()),

            // Header row
            div {
                class: "flex justify-between items-start mb-3",

                // Title and priority
                div {
                    class: "flex-1 min-w-0",

                    div {
                        class: "flex items-center gap-2 mb-1",

                        // Priority indicator
                        div {
                            class: "w-2 h-2 rounded-full {priority_color_class}",
                            title: "Priority: {event.priority}",
                        }

                        h3 {
                            class: "text-white m-0 text-base overflow-hidden text-ellipsis whitespace-nowrap",
                            "{event.name}"
                        }
                    }

                    // Status badge
                    span {
                        class: "inline-block px-2 py-0.5 rounded text-[0.6875rem] bg-opacity-20 {status_color_class}",
                        "{status_label}"
                    }
                }

                // Actions
                div {
                    class: "flex gap-1",

                    // Favorite button
                    button {
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_toggle_favorite.call(());
                        },
                        class: "bg-transparent border-none cursor-pointer p-1 text-base {favorite_color_class}",
                        title: if event.is_favorite { "Remove from favorites" } else { "Add to favorites" },
                        "⭐"
                    }

                    // Active toggle
                    button {
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_toggle_active.call(());
                        },
                        class: "bg-transparent border-none cursor-pointer p-1 text-sm {active_toggle_class}",
                        title: if event.is_active { "Deactivate" } else { "Activate" },
                        if event.is_active { "●" } else { "○" }
                    }
                }
            }

            // Description
            if !event.description.is_empty() {
                p {
                    class: "text-gray-400 text-[0.8125rem] m-0 mb-3 overflow-hidden text-ellipsis line-clamp-2",
                    "{event.description}"
                }
            }

            // Stats row
            div {
                class: "flex gap-4 text-gray-500 text-xs",

                // Trigger count
                span {
                    title: "Times triggered",
                    "🎯 {event.trigger_count}"
                }

                // Outcome count
                span {
                    title: "Possible outcomes",
                    "🔀 {event.outcome_count}"
                }

                // Trigger conditions
                span {
                    title: "Trigger conditions",
                    "⚡ {event.trigger_condition_count}"
                }

                // Chain indicator
                if event.chain_id.is_some() {
                    span {
                        class: "text-purple-500",
                        title: "Part of an event chain",
                        "🔗 Chain"
                    }
                }
            }

            // Tags
            if !event.tags.is_empty() {
                div {
                    class: "flex flex-wrap gap-1 mt-3",

                    for tag in event.tags.iter().take(4) {
                        span {
                            class: "bg-gray-700 text-gray-400 px-1.5 py-0.5 rounded text-[0.6875rem]",
                            "#{tag}"
                        }
                    }
                    if event.tags.len() > 4 {
                        {
                            let extra = event.tags.len() - 4;
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

            // Triggered timestamp
            if event.is_triggered {
                if let Some(ref triggered_at) = event.triggered_at {
                    div {
                        class: "mt-3 pt-2 border-t border-gray-700 text-green-500 text-xs",
                        "✓ Triggered: {triggered_at}"
                    }
                }
            }
        }
    }
}
