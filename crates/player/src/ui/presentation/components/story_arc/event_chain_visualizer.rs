//! Event Chain Visualizer - Flowchart view of event chains

use crate::application::dto::NarrativeEventData;
use crate::application::services::EventChainData;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct EventChainVisualizerProps {
    pub chain: EventChainData,
    pub world_id: String,
    pub on_select_event: EventHandler<String>,
}

/// Event chain visualizer component showing flowchart of events
#[component]
pub fn EventChainVisualizer(props: EventChainVisualizerProps) -> Element {
    let mut events: Signal<Vec<NarrativeEventData>> = use_signal(Vec::new);
    let is_loading = use_signal(|| false);
    let mut zoom_level: Signal<f32> = use_signal(|| 1.0);
    let mut pan_x: Signal<f32> = use_signal(|| 0.0);
    let mut pan_y: Signal<f32> = use_signal(|| 0.0);

    // Initialize with event placeholders based on the chain's event IDs
    {
        let chain_events = props.chain.events.clone();
        use_effect(move || {
            let placeholder_events: Vec<NarrativeEventData> = chain_events
                .iter()
                .map(|id| NarrativeEventData {
                    id: id.clone(),
                    world_id: String::new(),
                    name: format!("Event: {}", id),
                    description: String::new(),
                    scene_direction: String::new(),
                    suggested_opening: None,
                    trigger_count: 0,
                    is_active: false,
                    is_triggered: false,
                    triggered_at: None,
                    selected_outcome: None,
                    is_repeatable: false,
                    delay_turns: 0,
                    expires_after_turns: None,
                    priority: 0,
                    is_favorite: false,
                    tags: Vec::new(),
                    scene_id: None,
                    location_id: None,
                    act_id: None,
                    chain_id: Some(props.chain.id.clone()),
                    chain_position: None,
                    outcome_count: 0,
                    trigger_condition_count: 0,
                    created_at: String::new(),
                    updated_at: String::new(),
                })
                .collect();
            events.set(placeholder_events);
        });
    }

    let events_list = events.read().clone();

    rsx! {
        div {
            class: "event-chain-visualizer relative w-full h-full overflow-hidden bg-dark-bg rounded-lg",

            // Controls
            div {
                class: "absolute top-4 right-4 z-10 flex gap-2",
                button {
                    onclick: move |_| {
                        let current = *zoom_level.read();
                        zoom_level.set((current * 1.2).min(2.0));
                    },
                    class: "p-2 bg-dark-surface border border-gray-700 rounded text-white cursor-pointer",
                    "+"
                }
                button {
                    onclick: move |_| {
                        let current = *zoom_level.read();
                        zoom_level.set((current / 1.2).max(0.5));
                    },
                    class: "p-2 bg-dark-surface border border-gray-700 rounded text-white cursor-pointer",
                    "-"
                }
                button {
                    onclick: move |_| {
                        zoom_level.set(1.0);
                        pan_x.set(0.0);
                        pan_y.set(0.0);
                    },
                    class: "p-2 bg-dark-surface border border-gray-700 rounded text-white cursor-pointer",
                    "Reset"
                }
            }

            // Flowchart canvas
            div {
                style: format!(
                    "position: relative; width: 100%; height: 100%; transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
                    *pan_x.read(),
                    *pan_y.read(),
                    *zoom_level.read()
                ),
                if *is_loading.read() {
                    div {
                        class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-gray-500",
                        "Loading events..."
                    }
                } else {
                    div {
                        class: "flex flex-col items-center gap-8 p-8 min-w-fit",
                        for (index, event) in events_list.iter().enumerate() {
                            div {
                                key: "{event.id}",
                                class: "flex flex-col items-center gap-4",
                                // Connection line (except for first event)
                                if index > 0 {
                                    div {
                                        class: "w-0.5 h-8 bg-gray-700",
                                    }
                                }
                                // Event node
                                EventNode {
                                    event: event.clone(),
                                    is_completed: props.chain.completed_events.contains(&event.id),
                                    is_current: props.chain.current_position as usize == index,
                                    on_click: move |event_id| props.on_select_event.call(event_id),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Individual event node in the flowchart
#[component]
fn EventNode(
    event: NarrativeEventData,
    is_completed: bool,
    is_current: bool,
    on_click: EventHandler<String>,
) -> Element {
    let bg_color_class = if is_completed {
        "bg-green-500"
    } else if is_current {
        "bg-blue-500"
    } else {
        "bg-gray-700"
    };

    let border_color_class = if is_current {
        "border-purple-500"
    } else {
        "border-gray-500"
    };

    rsx! {
        div {
            onclick: move |_| on_click.call(event.id.clone()),
            class: "w-[200px] p-4 {bg_color_class} border-2 {border_color_class} rounded-lg cursor-pointer transition-all",
            onmouseenter: move |_| {
                // Could add hover effect
            },
            h4 {
                class: "text-white m-0 mb-2 text-sm font-medium text-center",
                "{event.name}"
            }
            if !event.description.is_empty() {
                p {
                    class: "text-white/70 m-0 text-xs text-center overflow-hidden text-ellipsis line-clamp-2",
                    "{event.description}"
                }
            }
            div {
                class: "flex justify-center gap-2 mt-2",
                if is_completed {
                    span { class: "text-white text-xs", "✅" }
                }
                if is_current {
                    span { class: "text-white text-xs", "📍" }
                }
            }
        }
    }
}
