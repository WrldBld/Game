//! Character Perspective Viewer - DM tool to see any character's perspective

use dioxus::prelude::*;

use crate::presentation::services::{use_character_service, use_player_character_service};

/// Props for CharacterPerspectiveViewer
#[derive(Props, Clone, PartialEq)]
pub struct CharacterPerspectiveViewerProps {
    pub session_id: String,
    pub world_id: String,
    pub on_view_as: EventHandler<String>,
}

/// Character Perspective Viewer component
#[component]
pub fn CharacterPerspectiveViewer(props: CharacterPerspectiveViewerProps) -> Element {
    let pc_service = use_player_character_service();
    let character_service = use_character_service();
    let mut pcs: Signal<Vec<wrldbldr_player_app::application::services::PlayerCharacterData>> = use_signal(Vec::new);
    let mut npcs: Signal<Vec<wrldbldr_player_app::application::services::CharacterSummary>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Load PCs and NPCs on mount
    {
        let session_id = props.session_id.clone();
        let world_id = props.world_id.clone();
        let pc_svc = pc_service.clone();
        let char_svc = character_service.clone();
        use_effect(move || {
            let sid = session_id.clone();
            let wid = world_id.clone();
            let pc_svc_clone = pc_svc.clone();
            let char_svc_clone = char_svc.clone();
            loading.set(true);
            spawn(async move {
                // Load PCs
                let pc_result = pc_svc_clone.list_pcs(&sid).await;
                
                // Load NPCs
                let npc_result = char_svc_clone.list_characters(&wid).await;

                match (pc_result, npc_result) {
                    (Ok(pc_list), Ok(npc_list)) => {
                        pcs.set(pc_list);
                        npcs.set(npc_list);
                        loading.set(false);
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        error.set(Some(format!("Failed to load characters: {}", e)));
                        loading.set(false);
                    }
                }
            });
        });
    }

    let pcs_list = pcs.read().clone();
    let npcs_list = npcs.read().clone();

    rsx! {
        div {
            class: "flex flex-col gap-4 p-4 bg-dark-surface rounded-lg",

            h3 {
                class: "m-0 text-white text-lg",
                "Character Perspective"
            }

            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-red-500 bg-opacity-10 border border-red-500 rounded-lg text-red-500 text-sm",
                    "{err}"
                }
            }

            if *loading.read() {
                div {
                    class: "p-8 text-center text-gray-400",
                    "Loading characters..."
                }
            } else {
                // Player Characters section
                if !pcs_list.is_empty() {
                    div {
                        h4 {
                            class: "m-0 mb-3 text-gray-400 text-sm uppercase",
                            "Player Characters"
                        }
                        div {
                            class: "flex flex-col gap-2",
                            {pcs_list.iter().map(|pc| {
                                let pc_id = pc.id.clone();
                                rsx! {
                                    CharacterCard {
                                        name: pc.name.clone(),
                                        id: pc_id.clone(),
                                        location_id: pc.current_location_id.clone(),
                                        on_view_as: move |_| props.on_view_as.call(pc_id.clone()),
                                    }
                                }
                            })}
                        }
                    }
                }

                // NPCs section
                if !npcs_list.is_empty() {
                    div {
                        h4 {
                            class: "m-0 mt-4 mb-3 text-gray-400 text-sm uppercase",
                            "NPCs"
                        }
                        div {
                            class: "flex flex-col gap-2",
                            {npcs_list.iter().map(|npc| {
                                let npc_id = npc.id.clone();
                                rsx! {
                                    CharacterCard {
                                        name: npc.name.clone(),
                                        id: npc_id.clone(),
                                        location_id: "unknown".to_string(),
                                        on_view_as: move |_| props.on_view_as.call(npc_id.clone()),
                                    }
                                }
                            })}
                        }
                    }
                }

                if pcs_list.is_empty() && npcs_list.is_empty() {
                    div {
                        class: "p-8 text-center text-gray-400",
                        "No characters available"
                    }
                }
            }
        }
    }
}

/// Character Card component
#[derive(Props, Clone, PartialEq)]
struct CharacterCardProps {
    name: String,
    id: String,
    location_id: String,
    on_view_as: EventHandler<()>,
}

#[component]
fn CharacterCard(props: CharacterCardProps) -> Element {
    rsx! {
        div {
            class: "p-3 bg-dark-bg rounded-lg border border-gray-700 flex justify-between items-center",

            div {
                div {
                    class: "text-white text-sm font-medium",
                    "{props.name}"
                }
                div {
                    class: "text-gray-400 text-xs",
                    "Location: {props.location_id}"
                }
            }
            button {
                onclick: move |_| props.on_view_as.call(()),
                class: "py-2 px-4 bg-blue-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                "View as"
            }
        }
    }
}

