//! Event Chain Editor - Edit chain info and manage events

use crate::presentation::services::use_event_chain_service;
use dioxus::prelude::*;
use wrldbldr_player_app::application::services::{
    CreateEventChainRequest, EventChainData, UpdateEventChainRequest,
};

#[derive(Props, Clone, PartialEq)]
pub struct EventChainEditorProps {
    pub chain: Option<EventChainData>,
    pub world_id: String,
    pub on_save: EventHandler<EventChainData>,
    pub on_cancel: EventHandler<()>,
}

/// Event chain editor component for creating/editing chains
#[component]
pub fn EventChainEditor(props: EventChainEditorProps) -> Element {
    let event_chain_service = use_event_chain_service();
    let is_editing = props.chain.is_some();

    let mut name = use_signal(|| {
        props
            .chain
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default()
    });
    let mut description = use_signal(|| {
        props
            .chain
            .as_ref()
            .map(|c| c.description.clone())
            .unwrap_or_default()
    });
    let mut tags = use_signal(|| {
        props
            .chain
            .as_ref()
            .map(|c| c.tags.clone())
            .unwrap_or_default()
    });
    let mut color = use_signal(|| props.chain.as_ref().and_then(|c| c.color.clone()));
    let mut is_active = use_signal(|| props.chain.as_ref().map(|c| c.is_active).unwrap_or(true));
    let mut new_tag = use_signal(String::new);
    let is_saving = use_signal(|| false);
    let error: Signal<Option<String>> = use_signal(|| None);

    let save_handler = {
        let world_id = props.world_id.clone();
        let service = event_chain_service.clone();
        let chain_id = props.chain.as_ref().map(|c| c.id.clone());
        move || {
            let world_id = world_id.clone();
            let service = service.clone();
            let chain_id = chain_id.clone();
            let name_val = name.read().clone();
            let desc_val = description.read().clone();
            let tags_val = tags.read().clone();
            let color_val = color.read().clone();
            let active_val = *is_active.read();
            let mut saving = is_saving;
            let mut err = error;
            let on_save_handler = props.on_save.clone();
            spawn(async move {
                saving.set(true);
                err.set(None);
                let result = if let Some(id) = chain_id {
                    // Update existing
                    let request = UpdateEventChainRequest {
                        name: Some(name_val),
                        description: Some(desc_val),
                        tags: Some(tags_val),
                        color: color_val,
                        is_active: Some(active_val),
                        events: None,
                        act_id: None,
                    };
                    service.update_chain(&id, &request).await
                } else {
                    // Create new
                    let request = CreateEventChainRequest {
                        name: name_val,
                        description: desc_val,
                        events: Vec::new(),
                        act_id: None,
                        tags: tags_val,
                        color: color_val,
                        is_active: active_val,
                    };
                    service.create_chain(&world_id, &request).await
                };
                match result {
                    Ok(chain) => {
                        on_save_handler.call(chain);
                    }
                    Err(e) => {
                        err.set(Some(e.to_string()));
                    }
                }
                saving.set(false);
            });
        }
    };

    rsx! {
        div {
            class: "event-chain-editor bg-dark-surface rounded-lg p-6 max-w-[600px]",

            h2 {
                class: "text-white m-0 mb-6 text-xl",
                if is_editing { "Edit Event Chain" } else { "Create Event Chain" }
            }

            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-gray-800 border-l-[3px] border-red-500 rounded mb-4",
                    div {
                        class: "text-red-500 text-sm",
                        "Error: {err}"
                    }
                }
            }

            // Form fields
            div {
                class: "flex flex-col gap-4",

                // Name
                div {
                    label {
                        class: "block text-gray-400 text-sm mb-2",
                        "Chain Name"
                    }
                    input {
                        r#type: "text",
                        value: "{name.read()}",
                        oninput: move |evt| name.set(evt.value()),
                        placeholder: "Enter chain name",
                        class: "w-full px-2 py-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                    }
                }

                // Description
                div {
                    label {
                        class: "block text-gray-400 text-sm mb-2",
                        "Description"
                    }
                    textarea {
                        value: "{description.read()}",
                        oninput: move |evt| description.set(evt.value()),
                        placeholder: "Enter chain description",
                        rows: 3,
                        class: "w-full px-2 py-2 bg-dark-bg border border-gray-700 rounded text-white text-sm resize-y",
                    }
                }

                // Color
                div {
                    label {
                        class: "block text-gray-400 text-sm mb-2",
                        "Color (hex)"
                    }
                    input {
                        r#type: "text",
                        value: "{color.read().as_ref().map(|c| c.as_str()).unwrap_or(\"\")}",
                        oninput: move |evt| color.set(if evt.value().is_empty() { None } else { Some(evt.value()) }),
                        placeholder: "#8b5cf6",
                        class: "w-full px-2 py-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                    }
                }

                // Tags
                div {
                    label {
                        class: "block text-gray-400 text-sm mb-2",
                        "Tags"
                    }
                    div {
                        class: "flex flex-wrap gap-2 mb-2",
                        for tag in tags.read().iter() {
                            span {
                                class: "inline-flex items-center gap-1 px-2 py-1 bg-gray-700 text-white rounded text-xs",
                                "{tag}"
                                button {
                                    onclick: {
                                        let tag_to_remove = tag.clone();
                                        move |_| {
                                            let mut tags_list = tags.write();
                                            tags_list.retain(|t| t != &tag_to_remove);
                                        }
                                    },
                                    class: "bg-transparent border-none text-white cursor-pointer p-0 ml-1",
                                    "×"
                                }
                            }
                        }
                    }
                    div {
                        class: "flex gap-2",
                        input {
                            r#type: "text",
                            value: "{new_tag.read()}",
                            oninput: move |evt| new_tag.set(evt.value()),
                            onkeydown: move |evt| {
                                if evt.key().to_string() == "Enter" {
                                    evt.prevent_default();
                                    let tag = new_tag.read().trim().to_string();
                                    if !tag.is_empty() && !tags.read().contains(&tag) {
                                        tags.write().push(tag);
                                        new_tag.set(String::new());
                                    }
                                }
                            },
                            placeholder: "Add tag (press Enter)",
                            class: "flex-1 px-2 py-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                        }
                        button {
                            onclick: move |_| {
                                let tag = new_tag.read().trim().to_string();
                                if !tag.is_empty() && !tags.read().contains(&tag) {
                                    tags.write().push(tag);
                                    new_tag.set(String::new());
                                }
                            },
                            class: "px-4 py-2 bg-purple-500 text-white border-none rounded cursor-pointer text-sm",
                            "Add"
                        }
                    }
                }

                // Active toggle
                div {
                    class: "flex items-center gap-2",
                    input {
                        r#type: "checkbox",
                        checked: *is_active.read(),
                        onchange: move |evt| is_active.set(evt.checked()),
                    }
                    label {
                        class: "text-gray-400 text-sm cursor-pointer",
                        "Active"
                    }
                }
            }

            // Actions
            div {
                class: "flex justify-end gap-3 mt-6",
                button {
                    onclick: move |_| props.on_cancel.call(()),
                    disabled: *is_saving.read(),
                    class: "px-6 py-2 bg-gray-700 text-white border-none rounded cursor-pointer text-sm",
                    "Cancel"
                }
                {
                    let is_disabled = *is_saving.read() || name.read().trim().is_empty();
                    let save_bg = if is_disabled { "bg-gray-500" } else { "bg-purple-500" };
                    let save_cursor = if is_disabled { "cursor-not-allowed" } else { "cursor-pointer" };
                    let save_text = if *is_saving.read() { "Saving..." } else { "Save" };
                    rsx! {
                        button {
                            onclick: move |_| save_handler(),
                            disabled: is_disabled,
                            class: "px-6 py-2 {save_bg} text-white border-none rounded {save_cursor} text-sm",
                            "{save_text}"
                        }
                    }
                }
            }
        }
    }
}
