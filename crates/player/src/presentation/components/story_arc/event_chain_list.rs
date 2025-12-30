//! Event Chain List - Display all event chains with progress indicators

use dioxus::prelude::*;
use tracing::info;
use crate::application::services::EventChainData;
use crate::presentation::services::use_event_chain_service;

#[derive(Props, Clone, PartialEq)]
pub struct EventChainListProps {
    pub world_id: String,
    #[props(default)]
    pub filter: ChainFilter,
    pub on_select_chain: EventHandler<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ChainFilter {
    #[default]
    All,
    Active,
    Favorites,
}

/// Event chain list component showing all chains with progress
#[component]
pub fn EventChainList(props: EventChainListProps) -> Element {
    let event_chain_service = use_event_chain_service();
    let mut chains: Signal<Vec<EventChainData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut selected_chain: Signal<Option<String>> = use_signal(|| None);

    // Load chains based on filter
    let world_id = props.world_id.clone();
    let filter = props.filter;
    let service_for_effect = event_chain_service.clone();
    use_effect(move || {
        let world_id = world_id.clone();
        let service = service_for_effect.clone();
        let filter_val = filter;
        spawn(async move {
            is_loading.set(true);
            error.set(None);
            let result = match filter_val {
                ChainFilter::All => service.list_chains(&world_id).await,
                ChainFilter::Active => service.list_active(&world_id).await,
                ChainFilter::Favorites => service.list_favorites(&world_id).await,
            };
            match result {
                Ok(loaded) => {
                    chains.set(loaded);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
            is_loading.set(false);
        });
    });

    rsx! {
        div {
            class: "event-chain-list flex flex-col gap-4 h-full",

            // Filter tabs
            div {
                class: "flex gap-2 border-b border-gray-700",
                FilterTab {
                    label: "All",
                    is_active: filter == ChainFilter::All,
                    onclick: move |_| {
                        // Filter change handled by parent
                    },
                }
                FilterTab {
                    label: "Active",
                    is_active: filter == ChainFilter::Active,
                    onclick: move |_| {
                        // Filter change handled by parent
                    },
                }
                FilterTab {
                    label: "Favorites",
                    is_active: filter == ChainFilter::Favorites,
                    onclick: move |_| {
                        // Filter change handled by parent
                    },
                }
            }

            // Export/Import buttons (simplified - copy to clipboard for export)
            div {
                class: "flex gap-2 mb-2",
                button {
                    onclick: {
                        let chains_signal = chains;
                        move |_| {
                            let chains_data = chains_signal.read().clone();
                            if let Ok(json) = serde_json::to_string_pretty(&chains_data) {
                                // Copy to clipboard (simplified export)
                                #[cfg(target_arch = "wasm32")]
                                {
                                    // In WASM, we'd use web_sys to copy to clipboard
                                    // For now, just log - can be enhanced later
                                    info!("Export JSON:\n{}", json);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    info!("Export JSON:\n{}", json);
                                }
                            }
                        }
                    },
                    class: "px-3 py-1.5 bg-gray-700 text-white border-none rounded cursor-pointer text-xs",
                    "📥 Export JSON"
                }
            }

            // Content
            if *is_loading.read() {
                div {
                    class: "text-gray-500 text-sm text-center p-8",
                    "Loading chains..."
                }
            } else if let Some(err) = error.read().as_ref() {
                div {
                    class: "text-red-500 text-sm p-4 bg-gray-800 rounded-md",
                    "Error: {err}"
                }
            } else if chains.read().is_empty() {
                div {
                    class: "text-gray-500 text-sm text-center p-8",
                    "No event chains found"
                }
            } else {
                div {
                    class: "flex-1 overflow-y-auto flex flex-col gap-3",
                    for chain in chains.read().iter() {
                        EventChainCard {
                            key: "{chain.id}",
                            chain: chain.clone(),
                            is_selected: selected_chain.read().as_ref() == Some(&chain.id),
                            on_select: move |chain_id: String| {
                                selected_chain.set(Some(chain_id.clone()));
                                props.on_select_chain.call(chain_id);
                            },
                            on_toggle_favorite: {
                                let chain_id = chain.id.clone();
                                let service = event_chain_service.clone();
                                let chains_signal = chains;
                                move |_| {
                                    let cid = chain_id.clone();
                                    let svc = service.clone();
                                    let mut chains_state = chains_signal;
                                    spawn(async move {
                                        if let Ok(new_favorite) = svc.toggle_favorite(&cid).await {
                                            // Update local state
                                            let mut chains_list = chains_state.write();
                                            if let Some(chain) = chains_list.iter_mut().find(|c| c.id == cid) {
                                                chain.is_favorite = new_favorite;
                                            }
                                        }
                                    });
                                }
                            },
                            on_toggle_active: {
                                let chain_id = chain.id.clone();
                                let service = event_chain_service.clone();
                                let chains_signal = chains;
                                move |is_active| {
                                    let cid = chain_id.clone();
                                    let svc = service.clone();
                                    let mut chains_state = chains_signal;
                                    spawn(async move {
                                        if svc.set_active(&cid, is_active).await.is_ok() {
                                            // Update local state
                                            let mut chains_list = chains_state.write();
                                            if let Some(chain) = chains_list.iter_mut().find(|c| c.id == cid) {
                                                chain.is_active = is_active;
                                            }
                                        }
                                    });
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Filter tab component
#[component]
fn FilterTab(
    label: &'static str,
    is_active: bool,
    onclick: EventHandler<()>,
) -> Element {
    let border_class = if is_active { "border-purple-500" } else { "border-transparent" };
    let text_class = if is_active { "text-white" } else { "text-gray-400" };

    rsx! {
        button {
            onclick: move |_| onclick.call(()),
            class: "px-4 py-2 bg-transparent border-none border-b-2 {border_class} {text_class} text-sm cursor-pointer transition-all duration-200",
            "{label}"
        }
    }
}

/// Individual event chain card
#[component]
fn EventChainCard(
    chain: EventChainData,
    is_selected: bool,
    on_select: EventHandler<String>,
    on_toggle_favorite: EventHandler<()>,
    on_toggle_active: EventHandler<bool>,
) -> Element {
    let progress_color_class = if chain.is_complete {
        "bg-green-500"
    } else if chain.is_active {
        "bg-blue-500"
    } else {
        "bg-gray-500"
    };

    let bg_class = if is_selected { "bg-dark-surface" } else { "bg-dark-bg" };
    let border_class = if is_selected { "border-purple-500" } else { "border-gray-700" };

    rsx! {
        div {
            onclick: move |_| on_select.call(chain.id.clone()),
            class: "p-4 {bg_class} border-2 {border_class} rounded-lg cursor-pointer transition-all duration-200",

            // Header
            div {
                class: "flex justify-between items-start mb-3",
                div {
                    class: "flex-1",
                    h3 {
                        class: "text-white m-0 mb-1 text-base font-medium",
                        "{chain.name}"
                        if chain.is_favorite {
                            span { class: "text-amber-500 ml-2", "⭐" }
                        }
                    }
                    if !chain.description.is_empty() {
                        p {
                            class: "text-gray-400 m-0 text-sm",
                            "{chain.description}"
                        }
                    }
                }
                div {
                    class: "flex gap-2",
                    {
                        let favorite_color = if chain.is_favorite { "text-amber-500" } else { "text-gray-500" };
                        rsx! {
                            button {
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    on_toggle_favorite.call(());
                                },
                                class: "px-2 py-1 bg-transparent border border-gray-700 rounded {favorite_color} cursor-pointer text-xs",
                                "⭐"
                            }
                        }
                    }
                    {
                        let active_bg = if chain.is_active { "bg-green-500" } else { "bg-gray-500" };
                        let active_text = if chain.is_active { "Active" } else { "Inactive" };
                        rsx! {
                            button {
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    on_toggle_active.call(!chain.is_active);
                                },
                                class: "px-2 py-1 {active_bg} border-none rounded text-white cursor-pointer text-xs",
                                "{active_text}"
                            }
                        }
                    }
                }
            }

            // Progress bar
            div {
                class: "mb-2",
                div {
                    class: "flex justify-between items-center mb-1",
                    span {
                        class: "text-gray-400 text-xs",
                        "{chain.completed_events.len()} / {chain.events.len()} events"
                    }
                    {
                        let progress_text_color = if chain.is_complete {
                            "text-green-500"
                        } else if chain.is_active {
                            "text-blue-500"
                        } else {
                            "text-gray-500"
                        };
                        rsx! {
                            span {
                                class: "{progress_text_color} text-xs font-medium",
                                "{chain.progress_percent}%"
                            }
                        }
                    }
                }
                div {
                    class: "w-full h-2 bg-gray-700 rounded overflow-hidden",
                    div {
                        class: "h-full {progress_color_class} transition-all duration-300",
                        style: "width: {chain.progress_percent}%",
                    }
                }
            }

            // Tags
            if !chain.tags.is_empty() {
                div {
                    class: "flex flex-wrap gap-1",
                    for tag in chain.tags.iter() {
                        span {
                            class: "px-1.5 py-0.5 bg-gray-700 text-gray-400 rounded text-[0.625rem]",
                            "{tag}"
                        }
                    }
                }
            }
        }
    }
}

