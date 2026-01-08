//! PC Management Panel - DM view of all player characters

use dioxus::prelude::*;

use crate::application::services::PlayerCharacterData;
use crate::presentation::services::use_player_character_service;

/// Props for PCManagementPanel
#[derive(Props, Clone, PartialEq)]
pub struct PCManagementPanelProps {
    pub world_id: String,
    pub on_view_as_character: EventHandler<String>,
}

/// PC Management Panel component
#[component]
pub fn PCManagementPanel(props: PCManagementPanelProps) -> Element {
    let pc_service = use_player_character_service();
    let mut pcs: Signal<Vec<PlayerCharacterData>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Load PCs on mount
    {
        let world_id = props.world_id.clone();
        let pc_svc = pc_service.clone();
        use_effect(move || {
            let wid = world_id.clone();
            let svc = pc_svc.clone();
            loading.set(true);
            spawn(async move {
                match svc.list_pcs(&wid).await {
                    Ok(pc_list) => {
                        pcs.set(pc_list);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load PCs: {}", e)));
                        loading.set(false);
                    }
                }
            });
        });
    }

    rsx! {
        div {
            class: "flex flex-col gap-4 p-4 bg-dark-surface rounded-lg",

            h3 {
                class: "m-0 text-white text-lg",
                "Player Characters"
            }

            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-red-500/10 border border-red-500 rounded-lg text-red-500 text-sm",
                    "{err}"
                }
            }

            if *loading.read() {
                div {
                    class: "p-8 text-center text-gray-400",
                    "Loading player characters..."
                }
            } else if pcs.read().is_empty() {
                div {
                    class: "p-8 text-center text-gray-400",
                    "No player characters in this world"
                }
            } else {
                {
                    let pcs_list = pcs.read().clone();
                    rsx! {
                        div {
                            class: "flex flex-col gap-3",
                            {pcs_list.into_iter().map(|pc| {
                                let pc_id = pc.id.clone();
                                rsx! {
                                    PCManagementCard {
                                        pc,
                                        on_view_as: move |_| props.on_view_as_character.call(pc_id.clone()),
                                    }
                                }
                            })}
                        }
                    }
                }
            }
        }
    }
}

/// PC Management Card component
#[derive(Props, Clone, PartialEq)]
struct PCManagementCardProps {
    pc: PlayerCharacterData,
    on_view_as: EventHandler<()>,
}

#[component]
fn PCManagementCard(props: PCManagementCardProps) -> Element {
    rsx! {
        div {
            class: "p-4 bg-dark-bg rounded-lg border border-gray-700",

            div {
                class: "flex justify-between items-start mb-3",
                div {
                    h4 {
                        class: "m-0 mb-1 text-white text-base",
                        "{props.pc.name}"
                    }
                    div {
                        class: "text-gray-400 text-xs",
                        "User: {props.pc.user_id}"
                    }
                }
                button {
                    onclick: move |_| props.on_view_as.call(()),
                    class: "px-4 py-2 bg-blue-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                    "View as"
                }
            }

            div {
                class: "flex flex-col gap-2",
                div {
                    class: "flex gap-4",
                    div {
                        class: "flex-1",
                        div {
                            class: "text-gray-400 text-xs mb-1",
                            "Current Location"
                        }
                        div {
                            class: "text-white text-sm",
                            "Location ID: {props.pc.current_location_id}"
                        }
                    }
                    div {
                        class: "flex-1",
                        div {
                            class: "text-gray-400 text-xs mb-1",
                            "Last Active"
                        }
                        div {
                            class: "text-white text-sm",
                            "{props.pc.last_active_at}"
                        }
                    }
                }
            }
        }
    }
}

/// Props for PCLocationsWidget
#[derive(Props, Clone, PartialEq)]
pub struct PCLocationsWidgetProps {
    pub world_id: String,
    pub on_manage: EventHandler<()>,
}

/// PC Locations Widget - Shows PC locations summary in Director View
#[component]
pub fn PCLocationsWidget(props: PCLocationsWidgetProps) -> Element {
    let pc_service = use_player_character_service();
    let mut pcs: Signal<Vec<PlayerCharacterData>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);

    // Load PCs on mount
    {
        let world_id = props.world_id.clone();
        let pc_svc = pc_service.clone();
        use_effect(move || {
            let wid = world_id.clone();
            let svc = pc_svc.clone();
            loading.set(true);
            spawn(async move {
                match svc.list_pcs(&wid).await {
                    Ok(pc_list) => {
                        pcs.set(pc_list);
                        loading.set(false);
                    }
                    Err(_) => {
                        loading.set(false);
                    }
                }
            });
        });
    }

    // Group PCs by location
    let location_counts: Vec<(String, usize)> = {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for pc in pcs.read().iter() {
            *counts.entry(pc.current_location_id.clone()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    };

    let is_split = location_counts.len() > 1;

    rsx! {
        div {
            class: "panel-section bg-dark-surface rounded-lg p-4",

            h3 {
                class: "text-gray-400 mb-3 text-sm uppercase",
                "PC Locations"
            }

            if *loading.read() {
                div {
                    class: "text-gray-400 text-sm",
                    "Loading..."
                }
            } else if pcs.read().is_empty() {
                div {
                    class: "text-gray-400 text-sm",
                    "No player characters"
                }
            } else {
                div {
                    class: "flex flex-col gap-2",

                    if is_split {
                        div {
                            class: "p-2 bg-amber-500/10 border border-amber-500 rounded-lg text-amber-500 text-sm font-medium",
                            "⚠️ Party split across {location_counts.len()} locations"
                        }
                    }

                    for (location_id, count) in location_counts.iter() {
                        div {
                            class: "flex justify-between items-center p-2 bg-dark-bg rounded-lg",
                            div {
                                class: "text-white text-sm",
                                "Location: {location_id}"
                            }
                            div {
                                class: "text-gray-400 text-sm",
                                if *count > 1 {
                                    "{count} PCs"
                                } else {
                                    "{count} PC"
                                }
                            }
                        }
                    }

                    button {
                        onclick: move |_| props.on_manage.call(()),
                        class: "mt-2 p-2 bg-blue-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                        "Manage PCs"
                    }
                }
            }
        }
    }
}
