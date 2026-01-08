//! Pending Events Widget - Small widget for Director view showing relevant narrative events

use dioxus::prelude::*;

use crate::application::dto::NarrativeEventData;
use crate::presentation::services::use_narrative_event_service;

#[derive(Props, Clone, PartialEq)]
pub struct PendingEventsWidgetProps {
    pub world_id: String,
    #[props(default = 5)]
    pub max_events: usize,
    pub on_view_story_arc: EventHandler<()>,
}

#[component]
pub fn PendingEventsWidget(props: PendingEventsWidgetProps) -> Element {
    let mut events: Signal<Vec<NarrativeEventData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);

    // Get narrative event service
    let narrative_event_service = use_narrative_event_service();

    // Load active/pending events
    let world_id = props.world_id.clone();
    use_effect(move || {
        let world_id = world_id.clone();
        let service = narrative_event_service.clone();
        spawn(async move {
            is_loading.set(true);
            if let Ok(loaded) = service.list_pending_events(&world_id).await {
                events.set(loaded);
            }
            is_loading.set(false);
        });
    });

    // Take only the top N events by priority
    let display_events: Vec<NarrativeEventData> = {
        let mut all = events.read().clone();
        all.sort_by(|a, b| b.priority.cmp(&a.priority));
        all.into_iter()
            .filter(|e| e.is_active && !e.is_triggered)
            .take(props.max_events)
            .collect()
    };

    rsx! {
        div {
            class: "pending-events-widget bg-dark-surface rounded-lg p-4",

            // Header
            div {
                class: "flex justify-between items-center mb-3",

                h3 {
                    class: "text-gray-400 m-0 text-sm uppercase",
                    "⭐ Pending Events"
                }

                button {
                    onclick: move |_| props.on_view_story_arc.call(()),
                    class: "bg-transparent border-none text-blue-400 cursor-pointer text-xs",
                    "View All →"
                }
            }

            // Content
            if *is_loading.read() {
                div {
                    class: "text-gray-500 text-sm text-center p-4",
                    "Loading..."
                }
            } else if display_events.is_empty() {
                div {
                    class: "text-gray-500 text-sm text-center p-4",
                    "No pending events"
                }
            } else {
                div {
                    class: "flex flex-col gap-2",

                    for event in display_events.iter() {
                        PendingEventItem {
                            key: "{event.id}",
                            event: event.clone(),
                        }
                    }

                    // Show count if more events exist
                    {
                        let total_active = events.read().iter().filter(|e| e.is_active && !e.is_triggered).count();
                        let max = props.max_events;
                        if total_active > max {
                            let extra = total_active - max;
                            rsx! {
                                div {
                                    class: "text-gray-500 text-xs text-center mt-2",
                                    "+{extra} more"
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
struct PendingEventItemProps {
    event: NarrativeEventData,
}

impl PartialEq for PendingEventItemProps {
    fn eq(&self, other: &Self) -> bool {
        self.event.id == other.event.id
    }
}

#[component]
fn PendingEventItem(props: PendingEventItemProps) -> Element {
    let event = &props.event;

    let priority_color_class = match event.priority {
        p if p >= 8 => "bg-red-500",
        p if p >= 5 => "bg-amber-500",
        p if p >= 3 => "bg-blue-500",
        _ => "bg-gray-500",
    };

    rsx! {
        div {
            class: "flex items-center gap-2 p-2 bg-dark-bg rounded-md",

            // Priority indicator
            div {
                class: "w-1.5 h-1.5 rounded-full {priority_color_class} flex-shrink-0",
            }

            // Event info
            div {
                class: "flex-1 min-w-0",

                p {
                    class: "text-white m-0 text-[0.8125rem] overflow-hidden text-ellipsis whitespace-nowrap",
                    "{event.name}"
                }

                div {
                    class: "flex gap-2 text-gray-500 text-[0.6875rem]",

                    span { "⚡ {event.trigger_condition_count} triggers" }

                    if event.is_favorite {
                        span { class: "text-amber-500", "⭐" }
                    }
                }
            }
        }
    }
}
