//! Event Chains view - main component for managing event chains

use dioxus::prelude::*;

use crate::presentation::components::story_arc::event_chain_editor::EventChainEditor;
use crate::presentation::components::story_arc::event_chain_list::{ChainFilter, EventChainList};
use crate::presentation::components::story_arc::event_chain_visualizer::EventChainVisualizer;
use crate::presentation::services::use_event_chain_service;
use crate::application::services::EventChainData;

/// Event Chains view - main component for managing event chains
#[component]
pub fn EventChainsView(world_id: String) -> Element {
    let event_chain_service = use_event_chain_service();
    let mut selected_chain: Signal<Option<String>> = use_signal(|| None);
    let mut selected_chain_data: Signal<Option<EventChainData>> = use_signal(|| None);
    let mut show_editor: Signal<bool> = use_signal(|| false);
    let mut editing_chain: Signal<Option<EventChainData>> = use_signal(|| None);
    let filter: Signal<ChainFilter> = use_signal(|| ChainFilter::All);

    // Load chain data when selected
    let chain_id = selected_chain.read().clone();
    let service = event_chain_service.clone();
    use_effect(move || {
        if let Some(id) = chain_id.as_ref() {
            let svc = service.clone();
            let id_clone = id.clone();
            spawn(async move {
                if let Ok(chain) = svc.get_chain(&id_clone).await {
                    selected_chain_data.set(Some(chain));
                }
            });
        } else {
            selected_chain_data.set(None);
        }
    });

    let chain_data = selected_chain_data.read().clone();

    rsx! {
        div {
            class: "h-full flex flex-col gap-4",

            // Header with create button
            div {
                class: "flex justify-between items-center",
                h2 {
                    class: "text-white m-0 text-2xl",
                    "Event Chains"
                }
                button {
                    onclick: move |_| {
                        editing_chain.set(None);
                        show_editor.set(true);
                    },
                    class: "py-2 px-4 bg-purple-500 text-white border-0 rounded cursor-pointer text-sm",
                    "+ Create Chain"
                }
            }

            // Main content area
            if *show_editor.read() {
                EventChainEditor {
                    chain: editing_chain.read().clone(),
                    world_id: world_id.clone(),
                    on_save: move |chain: EventChainData| {
                        selected_chain.set(Some(chain.id.clone()));
                        selected_chain_data.set(Some(chain.clone()));
                        show_editor.set(false);
                        editing_chain.set(None);
                    },
                    on_cancel: move |_| {
                        show_editor.set(false);
                        editing_chain.set(None);
                    },
                }
            } else if let Some(chain) = chain_data.as_ref() {
                // Show visualizer for selected chain
                div {
                    class: "flex gap-4 h-full",
                    // Sidebar with chain list
                    div {
                        class: "w-[300px] flex-shrink-0 overflow-y-auto",
                        EventChainList {
                            world_id: world_id.clone(),
                            filter: *filter.read(),
                            on_select_chain: move |id| selected_chain.set(Some(id)),
                        }
                    }
                    // Visualizer
                    div {
                        class: "flex-1 min-h-0",
                        EventChainVisualizer {
                            chain: chain.clone(),
                            world_id: world_id.clone(),
                            on_select_event: move |_event_id| {
                                // TODO (Phase 17H): Navigate to event details modal/panel
                            },
                        }
                    }
                }
            } else {
                EventChainList {
                    world_id: world_id.clone(),
                    filter: *filter.read(),
                    on_select_chain: move |id| selected_chain.set(Some(id)),
                }
            }
        }
    }
}
