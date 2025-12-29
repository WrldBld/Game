//! Asset Gallery - Display and manage entity assets

use dioxus::prelude::*;

use crate::presentation::services::{use_asset_service, use_settings_service};
use wrldbldr_player_app::application::services::{Asset, GenerateRequest};

/// Asset types that can be generated
const ASSET_TYPES: &[(&str, &str)] = &[
    ("portrait", "Portrait"),
    ("sprite", "Sprite"),
    ("backdrop", "Backdrop"),
    ("emotion_sheet", "Emotions"),
];

/// Asset gallery for an entity
#[component]
pub fn AssetGallery(world_id: String, entity_type: String, entity_id: String) -> Element {
    let asset_service = use_asset_service();
    let settings_service = use_settings_service();
    let mut selected_asset_type = use_signal(|| "portrait".to_string());
    let mut show_generate_modal = use_signal(|| false);
    let mut assets: Signal<Vec<Asset>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut world_style_reference_id: Signal<Option<String>> = use_signal(|| None);

    // Fetch assets and world settings on mount
    {
        let entity_type_clone = entity_type.clone();
        let entity_id_clone = entity_id.clone();
        let world_id_clone = world_id.clone();
        let asset_svc = asset_service.clone();
        let settings_svc = settings_service.clone();

        use_effect(move || {
            let et = entity_type_clone.clone();
            let ei = entity_id_clone.clone();
            let wid = world_id_clone.clone();
            let asset_svc = asset_svc.clone();
            let settings_svc = settings_svc.clone();
            spawn(async move {
                // Fetch world settings to get current style reference
                if let Ok(settings) = settings_svc.get_for_world(&wid).await {
                    world_style_reference_id.set(settings.style_reference_asset_id);
                }

                // Skip API call if entity_id is empty (new entity being created)
                if ei.is_empty() {
                    assets.set(Vec::new());
                    is_loading.set(false);
                    return;
                }

                match asset_svc.get_assets(&et, &ei).await {
                    Ok(fetched_assets) => {
                        assets.set(fetched_assets);
                        is_loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        is_loading.set(false);
                    }
                }
            });
        });
    }

    // Filter assets by selected type
    let selected_type = selected_asset_type.read().clone();
    let filtered_assets: Vec<Asset> = assets
        .read()
        .iter()
        .filter(|a| a.asset_type == selected_type)
        .cloned()
        .collect();

    rsx! {
        div {
            class: "asset-gallery bg-dark-bg rounded-lg p-3",

            // Error display
            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-red-500 bg-opacity-10 rounded text-red-500 text-sm mb-3",
                    "Error: {err}"
                }
            }

            // Asset type tabs
            div {
                class: "asset-tabs flex gap-1 mb-3",

                for (type_id, type_label) in ASSET_TYPES {
                    {
                        let btn_class = if *selected_asset_type.read() == *type_id {
                            "p-1 px-2 text-xs rounded cursor-pointer border-0 bg-blue-500 text-white"
                        } else {
                            "p-1 px-2 text-xs rounded cursor-pointer border-0 bg-transparent text-gray-400"
                        };
                        rsx! {
                            button {
                                onclick: {
                                    let type_id = type_id.to_string();
                                    move |_| selected_asset_type.set(type_id.clone())
                                },
                                class: "{btn_class}",
                                "{type_label}"
                            }
                        }
                    }
                }
            }

            // Asset grid
            div {
                class: "asset-grid flex flex-wrap gap-2 min-h-20",

                if entity_id.is_empty() {
                    // New entity - show message about generating assets after creation
                    div {
                        class: "w-full text-center text-gray-500 text-sm p-4 bg-purple-500 bg-opacity-10 rounded border border-dashed border-purple-500",
                        "Save the {entity_type} first to generate assets"
                    }
                } else if *is_loading.read() {
                    div {
                        class: "w-full text-center text-gray-500 text-sm p-4",
                        "Loading assets..."
                    }
                } else if filtered_assets.is_empty() {
                    div {
                        class: "w-full text-center text-gray-500 text-sm p-4",
                        "No {selected_asset_type} assets yet"
                    }
                } else {
                    for asset in filtered_assets {
                        {
                            let entity_type_activate = entity_type.clone();
                            let entity_id_activate = entity_id.clone();
                            let entity_type_delete = entity_type.clone();
                            let entity_id_delete = entity_id.clone();
                            let world_id_for_style = world_id.clone();
                            let asset_svc_activate = asset_service.clone();
                            let asset_svc_delete = asset_service.clone();
                            let settings_svc_style = settings_service.clone();
                            let is_world_style_ref = world_style_reference_id.read().as_ref() == Some(&asset.id);
                            rsx! {
                                AssetThumbnail {
                                    id: asset.id.clone(),
                                    label: asset.label.clone(),
                                    is_active: asset.is_active,
                                    is_world_style_reference: is_world_style_ref,
                                    style_reference_id: asset.style_reference_id.clone(),
                                    on_activate: move |id: String| {
                                        let entity_type = entity_type_activate.clone();
                                        let entity_id = entity_id_activate.clone();
                                        let svc = asset_svc_activate.clone();
                                        spawn(async move {
                                            if let Err(e) = svc.activate_asset(&entity_type, &entity_id, &id).await {
                                                tracing::error!("Failed to activate asset: {}", e);
                                            }
                                        });
                                    },
                                    on_delete: move |id: String| {
                                        let entity_type = entity_type_delete.clone();
                                        let entity_id = entity_id_delete.clone();
                                        let svc = asset_svc_delete.clone();
                                        spawn(async move {
                                            if let Err(e) = svc.delete_asset(&entity_type, &entity_id, &id).await {
                                                tracing::error!("Failed to delete asset: {}", e);
                                            }
                                        });
                                    },
                                    on_use_as_reference: move |id: String| {
                                        let wid = world_id_for_style.clone();
                                        let svc = settings_svc_style.clone();
                                        spawn(async move {
                                            // Fetch current settings, update style reference, save
                                            match svc.get_for_world(&wid).await {
                                                Ok(mut settings) => {
                                                    settings.style_reference_asset_id = Some(id.clone());
                                                    if let Err(e) = svc.update_for_world(&wid, &settings).await {
                                                        tracing::error!("Failed to save style reference: {}", e);
                                                    } else {
                                                        // Update local state
                                                        world_style_reference_id.set(Some(id));
                                                        tracing::info!("Style reference updated");
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to fetch settings: {}", e);
                                                }
                                            }
                                        });
                                    },
                                }
                            }
                        }
                    }
                }

                // Generate button (only show if entity_id exists)
                if !entity_id.is_empty() {
                button {
                    onclick: move |_| show_generate_modal.set(true),
                    class: "w-16 h-16 flex flex-col items-center justify-center bg-purple-500 bg-opacity-20 border-2 border-dashed border-purple-500 rounded-lg cursor-pointer text-purple-500 text-xs",
                    span { class: "text-2xl", "+" }
                    span { "Generate" }
                    }
                }
            }

            // Generation modal
            if *show_generate_modal.read() {
                GenerateAssetModal {
                    world_id: world_id.clone(),
                    entity_type: entity_type.clone(),
                    entity_id: entity_id.clone(),
                    asset_type: selected_asset_type.read().clone(),
                    world_style_reference_id: world_style_reference_id.read().clone(),
                    on_close: move |_| show_generate_modal.set(false),
                    on_generate: {
                        let asset_svc_gen = asset_service.clone();
                        move |req| {
                            let svc = asset_svc_gen.clone();
                            spawn(async move {
                                if let Err(e) = svc.generate_assets(&req).await {
                                    tracing::error!("Failed to queue generation: {}", e);
                                }
                            });
                            show_generate_modal.set(false);
                        }
                    },
                }
            }
        }
    }
}

/// Props for AssetThumbnail
#[derive(Props, Clone, PartialEq)]
struct AssetThumbnailProps {
    id: String,
    label: Option<String>,
    is_active: bool,
    /// Whether this asset is the world's default style reference
    is_world_style_reference: bool,
    style_reference_id: Option<String>,
    on_activate: EventHandler<String>,
    on_delete: EventHandler<String>,
    on_use_as_reference: EventHandler<String>,
}

/// Individual asset thumbnail
#[component]
fn AssetThumbnail(props: AssetThumbnailProps) -> Element {
    let mut show_menu = use_signal(|| false);

    let border_class = if props.is_active {
        "border-2 border-green-500"
    } else if props.is_world_style_reference {
        "border-2 border-purple-500"
    } else {
        "border-2 border-transparent"
    };

    let id_for_activate = props.id.clone();
    let id_for_menu_activate = props.id.clone();
    let id_for_style_ref = props.id.clone();
    let id_for_delete = props.id.clone();

    rsx! {
        div {
            class: format!("w-16 h-16 bg-dark-surface {} rounded-lg cursor-pointer relative overflow-hidden", border_class),
            oncontextmenu: move |e| {
                e.prevent_default();
                show_menu.toggle();
            },

            // Thumbnail click to activate
            div {
                onclick: {
                    let id = id_for_activate.clone();
                    let on_activate = props.on_activate.clone();
                    move |_| {
                        on_activate.call(id.clone());
                        show_menu.set(false);
                    }
                },
                class: "w-full h-full flex items-center justify-center bg-gradient-to-br from-gray-700 to-gray-800",

                // Active indicator (green dot)
                if props.is_active {
                    div {
                        class: "absolute top-0.5 right-0.5 w-2 h-2 bg-green-500 rounded-full",
                    }
                }

                // Style reference indicator (purple star)
                if props.is_world_style_reference {
                    div {
                        class: "absolute top-0.5 left-0.5 text-purple-400 text-xs",
                        title: "World default style reference",
                        "*"
                    }
                }
            }

            // Label
            if let Some(label) = &props.label {
                div {
                    class: "absolute bottom-0 left-0 right-0 p-0.5 bg-black bg-opacity-70 text-white text-xs text-center overflow-hidden text-ellipsis whitespace-nowrap",
                    "{label}"
                }
            }

            // Context menu
            if *show_menu.read() {
                div {
                    class: "absolute top-full left-0 right-0 bg-gray-800 border border-gray-700 rounded z-100 shadow-lg",

                    if !props.is_active {
                        button {
                            onclick: {
                                let id = id_for_menu_activate.clone();
                                let on_activate = props.on_activate.clone();
                                move |_| {
                                    on_activate.call(id.clone());
                                    show_menu.set(false);
                                }
                            },
                            class: "block w-full p-2 text-left bg-transparent text-white border-0 cursor-pointer text-xs border-b border-gray-700",
                            "Activate"
                        }
                    }

                    // Style reference button - show different text if already the reference
                    button {
                        onclick: {
                            let id = id_for_style_ref.clone();
                            let handler = props.on_use_as_reference.clone();
                            move |_| {
                                handler.call(id.clone());
                                show_menu.set(false);
                            }
                        },
                        class: if props.is_world_style_reference {
                            "block w-full p-2 text-left bg-transparent text-purple-300 border-0 cursor-pointer text-xs border-b border-gray-700"
                        } else {
                            "block w-full p-2 text-left bg-transparent text-purple-500 border-0 cursor-pointer text-xs border-b border-gray-700"
                        },
                        if props.is_world_style_reference {
                            "Current Style Reference"
                        } else {
                            "Use as Style Reference"
                        }
                    }

                    button {
                        onclick: {
                            let id = id_for_delete.clone();
                            let on_delete = props.on_delete.clone();
                            move |_| {
                                on_delete.call(id.clone());
                                show_menu.set(false);
                            }
                        },
                        class: "block w-full p-2 text-left bg-transparent text-red-500 border-0 cursor-pointer text-xs",
                        "Delete"
                    }
                }
            }
        }
    }
}

/// Modal for generating new assets
#[component]
fn GenerateAssetModal(
    world_id: String,
    entity_type: String,
    entity_id: String,
    asset_type: String,
    /// World's default style reference asset ID (from settings)
    world_style_reference_id: Option<String>,
    on_close: EventHandler<()>,
    on_generate: EventHandler<GenerateRequest>,
) -> Element {
    let asset_service = use_asset_service();
    let mut prompt = use_signal(|| String::new());
    let mut negative_prompt = use_signal(|| String::new());
    let mut count = use_signal(|| 4u8);
    let mut workflow_slot = use_signal(|| String::new());
    let mut is_generating = use_signal(|| false);
    // Pre-populate with world default style reference
    let mut style_reference_id: Signal<Option<String>> = use_signal(|| None);
    let mut style_reference_label: Signal<Option<String>> = use_signal(|| None);
    let mut is_using_world_default = use_signal(|| false);
    let mut show_style_selector = use_signal(|| false);
    let mut available_assets: Signal<Vec<Asset>> = use_signal(Vec::new);

    // Load available assets for style reference selection
    // and pre-populate with world default if available
    let entity_type_for_assets = entity_type.clone();
    let entity_id_for_assets = entity_id.clone();
    let world_default_ref = world_style_reference_id.clone();
    use_effect(move || {
        let et = entity_type_for_assets.clone();
        let ei = entity_id_for_assets.clone();
        let svc = asset_service.clone();
        let default_ref = world_default_ref.clone();
        spawn(async move {
            if let Ok(assets) = svc.get_assets(&et, &ei).await {
                // If world has a default style reference, pre-populate it
                if let Some(ref default_id) = default_ref {
                    // Find the asset label for the default reference
                    let label = assets
                        .iter()
                        .find(|a| &a.id == default_id)
                        .and_then(|a| a.label.clone())
                        .or_else(|| Some(default_id.clone()));
                    style_reference_id.set(Some(default_id.clone()));
                    style_reference_label.set(label);
                    is_using_world_default.set(true);
                }
                available_assets.set(assets);
            }
        });
    });

    rsx! {
        div {
            class: "modal-overlay fixed inset-0 bg-black bg-opacity-80 flex items-center justify-center z-1000",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl p-6 w-11/12 max-w-lg",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-white m-0 mb-4", "Generate {asset_type}" }

                // Workflow slot field (optional hint text)
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
                    if let Some(_ref_id) = style_reference_id.read().as_ref() {
                        div {
                            class: if *is_using_world_default.read() {
                                "flex items-center gap-2 p-2 bg-dark-bg border border-purple-500 rounded"
                            } else {
                                "flex items-center gap-2 p-2 bg-dark-bg border border-gray-700 rounded"
                            },
                            span {
                                class: "flex-1 text-white text-sm",
                                if let Some(label) = style_reference_label.read().as_ref() {
                                    if *is_using_world_default.read() {
                                        "{label} (World Default)"
                                    } else {
                                        "{label}"
                                    }
                                } else {
                                    "Selected asset"
                                }
                            }
                            button {
                                onclick: move |_| show_style_selector.set(true),
                                class: "py-1 px-2 bg-gray-600 text-white border-0 rounded cursor-pointer text-xs",
                                "Change"
                            }
                            button {
                                onclick: move |_| {
                                    style_reference_id.set(None);
                                    style_reference_label.set(None);
                                    is_using_world_default.set(false);
                                },
                                class: "py-1 px-2 bg-red-500 text-white border-0 rounded cursor-pointer text-xs",
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
                        class: "modal-overlay fixed inset-0 bg-black bg-opacity-90 flex items-center justify-center z-1001",
                        onclick: move |_| show_style_selector.set(false),
                        div {
                            class: "bg-dark-surface rounded-xl p-6 w-11/12 max-w-2xl max-h-screen-80 overflow-y-auto",
                            onclick: move |e| e.stop_propagation(),
                            h3 { class: "text-white m-0 mb-4", "Select Style Reference" }
                            div {
                                class: "grid gap-3 grid-cols-[repeat(auto-fill,minmax(120px,1fr))]",
                                for asset in available_assets.read().iter() {
                                    button {
                                        onclick: {
                                            let asset_id = asset.id.clone();
                                            let asset_label = asset.label.clone().or_else(|| Some(asset.id.clone()));
                                            move |_| {
                                                style_reference_id.set(Some(asset_id.clone()));
                                                style_reference_label.set(asset_label.clone());
                                                is_using_world_default.set(false); // User manually selected
                                                show_style_selector.set(false);
                                            }
                                        },
                                        class: "flex flex-col items-center p-2 bg-dark-bg border border-gray-700 rounded cursor-pointer transition-all",
                                        onmouseenter: move |_| {
                                            // Could add hover effect
                                        },
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

                // Prompt field
                div { class: "mb-4",
                    label { class: "block text-gray-400 text-sm mb-1", "Prompt" }
                    textarea {
                        value: "{prompt}",
                        oninput: move |e| prompt.set(e.value()),
                        placeholder: "Describe the {asset_type} you want to generate...",
                        class: "w-full min-h-20 p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
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
                        onclick: move |_| on_close.call(()),
                        disabled: *is_generating.read(),
                        class: "py-2 px-4 bg-transparent text-gray-400 border border-gray-700 rounded cursor-pointer",
                        "Cancel"
                    }
                    button {
                        onclick: {
                            let world_id = world_id.clone();
                            let entity_type = entity_type.clone();
                            let entity_id = entity_id.clone();
                            let asset_type = asset_type.clone();
                            move |_| {
                                is_generating.set(true);
                                on_generate.call(GenerateRequest {
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
                                });
                                is_generating.set(false);
                            }
                        },
                        disabled: *is_generating.read(),
                        class: "py-2 px-4 bg-purple-500 text-white border-0 rounded cursor-pointer font-medium",
                        if *is_generating.read() { "Generating..." } else { "Generate" }
                    }
                }
            }
        }
    }
}
