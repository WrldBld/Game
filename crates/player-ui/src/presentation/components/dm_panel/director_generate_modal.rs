//! Director Generate Modal - Quick asset generation from Director Mode
//!
//! Pre-populates prompts from character description and provides
//! quick access to asset generation without switching to Creator Mode.

use dioxus::prelude::*;

use wrldbldr_player_app::application::services::{Asset, GenerateRequest};
use crate::presentation::services::use_asset_service;

/// Props for DirectorGenerateModal
#[derive(Props, Clone, PartialEq)]
pub struct DirectorGenerateModalProps {
    /// World ID for the generation request
    pub world_id: String,
    /// Entity type (e.g., "character")
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Asset type to generate
    pub asset_type: String,
    /// Character name for display
    pub character_name: String,
    /// Initial prompt (pre-populated from character description)
    pub initial_prompt: String,
    /// Handler called when modal should close
    pub on_close: EventHandler<()>,
}

/// Director Mode generation modal with pre-populated prompt
#[component]
pub fn DirectorGenerateModal(props: DirectorGenerateModalProps) -> Element {
    let asset_service = use_asset_service();
    let mut prompt = use_signal(|| props.initial_prompt.clone());
    let mut negative_prompt = use_signal(|| String::new());
    let mut count = use_signal(|| 4u8);
    let mut workflow_slot = use_signal(|| String::new());
    let mut is_generating = use_signal(|| false);
    let mut style_reference_id: Signal<Option<String>> = use_signal(|| None);
    let mut style_reference_label: Signal<Option<String>> = use_signal(|| None);
    let mut show_style_selector = use_signal(|| false);
    let mut available_assets: Signal<Vec<Asset>> = use_signal(Vec::new);

    // Load available assets for style reference selection
    let entity_type_for_assets = props.entity_type.clone();
    let entity_id_for_assets = props.entity_id.clone();
    let asset_service_for_effect = asset_service.clone();
    use_effect(move || {
        let et = entity_type_for_assets.clone();
        let ei = entity_id_for_assets.clone();
        let svc = asset_service_for_effect.clone();
        spawn(async move {
            if let Ok(assets) = svc.get_assets(&et, &ei).await {
                available_assets.set(assets);
            }
        });
    });

    let button_text = if *is_generating.read() { "Generating..." } else { "Generate" };

    rsx! {
        div {
            class: "modal-overlay fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl p-6 w-[90%] max-w-[500px] max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                div {
                    class: "flex justify-between items-center mb-4",
                    h3 { class: "text-white m-0", "Generate {props.asset_type} for {props.character_name}" }
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-2 py-1 bg-transparent text-gray-400 border-0 cursor-pointer text-xl",
                        "×"
                    }
                }

                // Workflow slot field (optional)
                div { class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Workflow Slot (optional)" }
                    input {
                        r#type: "text",
                        value: "{workflow_slot}",
                        oninput: move |e| workflow_slot.set(e.value()),
                        placeholder: "Leave empty for default workflow...",
                        class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                    }
                }

                // Style Reference field
                div { class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Style Reference (optional)" }
                    if let Some(ref_id) = style_reference_id.read().as_ref() {
                        div {
                            class: "flex items-center gap-2 p-2 bg-dark-bg border border-gray-700 rounded",
                            span {
                                class: "flex-1 text-white text-sm",
                                if let Some(label) = style_reference_label.read().as_ref() {
                                    "{label}"
                                } else {
                                    "Selected asset"
                                }
                            }
                            button {
                                onclick: move |_| {
                                    style_reference_id.set(None);
                                    style_reference_label.set(None);
                                },
                                class: "px-2 py-1 bg-red-500 text-white border-0 rounded cursor-pointer text-xs",
                                "Clear"
                            }
                        }
                    } else {
                        div {
                            class: "flex gap-2",
                            button {
                                onclick: move |_| show_style_selector.set(true),
                                class: "flex-1 p-2 bg-gray-700 text-white border-0 rounded cursor-pointer text-sm",
                                "Select from Gallery..."
                            }
                        }
                    }
                }

                // Style reference selector modal
                if *show_style_selector.read() {
                    div {
                        class: "modal-overlay fixed inset-0 bg-black/90 flex items-center justify-center z-[1001]",
                        onclick: move |_| show_style_selector.set(false),
                        div {
                            class: "bg-dark-surface rounded-xl p-6 w-[90%] max-w-[600px] max-h-[80vh] overflow-y-auto",
                            onclick: move |e| e.stop_propagation(),
                            h3 { class: "text-white m-0 mb-4", "Select Style Reference" }
                            div {
                                class: "grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-3",
                                for asset in available_assets.read().iter() {
                                    button {
                                        onclick: {
                                            let asset_id = asset.id.clone();
                                            let asset_label = asset.label.clone().or_else(|| Some(asset.id.clone()));
                                            move |_| {
                                                style_reference_id.set(Some(asset_id.clone()));
                                                style_reference_label.set(asset_label.clone());
                                                show_style_selector.set(false);
                                            }
                                        },
                                        class: "flex flex-col items-center p-2 bg-dark-bg border border-gray-700 rounded cursor-pointer transition-all",
                                        div {
                                            class: "w-20 h-20 bg-gray-700 rounded mb-2 flex items-center justify-center",
                                            span { class: "text-gray-400 text-xs", "📷" }
                                        }
                                        span {
                                            class: "text-white text-xs text-center overflow-hidden text-ellipsis whitespace-nowrap w-full",
                                            "{asset.label.as_ref().unwrap_or(&asset.id)}"
                                        }
                                    }
                                }
                            }
                            if available_assets.read().is_empty() {
                                div {
                                    class: "text-gray-500 text-center p-8",
                                    "No assets available for style reference"
                                }
                            }
                        }
                    }
                }

                // Prompt field (pre-populated)
                div { class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Prompt" }
                    textarea {
                        value: "{prompt}",
                        oninput: move |e| prompt.set(e.value()),
                        placeholder: "Describe the {props.asset_type} you want to generate...",
                        class: "w-full min-h-[100px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                    }
                }

                // Negative prompt field
                div { class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Negative Prompt (optional)" }
                    input {
                        r#type: "text",
                        value: "{negative_prompt}",
                        oninput: move |e| negative_prompt.set(e.value()),
                        placeholder: "Things to avoid...",
                        class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                    }
                }

                // Variation count
                div { class: "mb-6",
                    label { class: "block text-gray-400 text-sm mb-1", "Variations: {count}" }
                    input {
                        r#type: "range",
                        min: "1",
                        max: "8",
                        value: "{count}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<u8>() {
                                count.set(v);
                            }
                        },
                        class: "w-full",
                    }
                }

                // Action buttons
                div { class: "flex justify-end gap-2",
                    button {
                        onclick: move |_| props.on_close.call(()),
                        disabled: *is_generating.read(),
                        class: "px-4 py-2 bg-transparent text-gray-400 border border-gray-700 rounded cursor-pointer",
                        "Cancel"
                    }
                    button {
                        onclick: {
                            let world_id = props.world_id.clone();
                            let entity_type = props.entity_type.clone();
                            let entity_id = props.entity_id.clone();
                            let asset_type = props.asset_type.clone();
                            let svc = asset_service.clone();
                            move |_| {
                                is_generating.set(true);
                                let req = GenerateRequest {
                                    world_id: world_id.clone(),
                                    entity_type: entity_type.clone(),
                                    entity_id: entity_id.clone(),
                                    asset_type: asset_type.clone(),
                                    prompt: prompt.read().clone(),
                                    negative_prompt: if negative_prompt.read().is_empty() {
                                        None
                                    } else {
                                        Some(negative_prompt.read().clone())
                                    },
                                    count: *count.read(),
                                    style_reference_id: style_reference_id.read().clone(),
                                };
                                let svc_clone = svc.clone();
                                spawn(async move {
                                    if let Err(e) = svc_clone.generate_assets(&req).await {
                                        tracing::error!("Failed to queue generation: {}", e);
                                    }
                                });
                                props.on_close.call(());
                                is_generating.set(false);
                            }
                        },
                        disabled: *is_generating.read(),
                        class: "px-4 py-2 bg-purple-500 text-white border-0 rounded cursor-pointer font-medium",
                        "{button_text}"
                    }
                }
            }
        }
    }
}

