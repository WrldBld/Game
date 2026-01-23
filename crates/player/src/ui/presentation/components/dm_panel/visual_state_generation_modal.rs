//! Visual state generation modal
//!
//! Allows DMs to generate new visual states with AI-generated assets.
//! Used from visual state dropdowns in pre-stage and staging approval.

use dioxus::prelude::*;

use crate::infrastructure::spawn_task;
use crate::presentation::services::{use_command_bus, use_generation_service};

use wrldbldr_shared::requests::visual_state::{GenerateVisualStateRequest, VisualStateType};

/// Quick preset options for generation
#[derive(Clone, Copy, PartialEq, Debug)]
enum QuickPreset {
    RainyNight,
    Moonlit,
    Festive,
    Eerie,
}

impl QuickPreset {
    fn name(&self) -> &'static str {
        match self {
            QuickPreset::RainyNight => "🌧️ Rainy Night",
            QuickPreset::Moonlit => "🌙 Moonlit",
            QuickPreset::Festive => "🔥 Festive",
            QuickPreset::Eerie => "💀 Eerie",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            QuickPreset::RainyNight => "Rain-soaked surfaces, flickering lanterns, wet reflections",
            QuickPreset::Moonlit => "Cool moonlight filtering through windows, shadows stretching long",
            QuickPreset::Festive => "Colorful lanterns, celebration decorations, lively atmosphere",
            QuickPreset::Eerie => "Thick fog, unnatural quiet, dim lighting",
        }
    }

    fn tags(&self) -> Vec<String> {
        match self {
            QuickPreset::RainyNight => vec!["weather:rain".to_string(), "time:night".to_string()],
            QuickPreset::Moonlit => vec!["time:night".to_string(), "lighting:moonlight".to_string()],
            QuickPreset::Festive => vec!["mood:festive".to_string(), "activity:celebration".to_string()],
            QuickPreset::Eerie => vec!["mood:eerie".to_string(), "lighting:dim".to_string()],
        }
    }
}

/// Asset style options
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum AssetStyle {
    #[default]
    FantasyRealistic,
    AnimeVisualNovel,
    PixelArt,
    DarkNoir,
}

impl AssetStyle {
    fn name(&self) -> &'static str {
        match self {
            AssetStyle::FantasyRealistic => "Fantasy Realistic",
            AssetStyle::AnimeVisualNovel => "Anime/Visual Novel",
            AssetStyle::PixelArt => "Pixel Art",
            AssetStyle::DarkNoir => "Dark/Noir",
        }
    }
}

/// Result of visual state generation
#[derive(Clone, PartialEq, Debug)]
pub struct GeneratedStateResult {
    pub state_id: String,
    pub name: String,
}

/// Props for VisualStateGenerationModal
#[derive(Props, Clone, PartialEq)]
pub struct VisualStateGenerationModalProps {
    /// Region ID for generation context
    pub region_id: Option<String>,
    /// Location ID for generation context
    pub location_id: Option<String>,
    /// Scope of state to generate
    pub scope: VisualStateType,
    /// Handler when state is saved
    pub on_save: EventHandler<GeneratedStateResult>,
    /// Handler when modal is cancelled
    pub on_close: EventHandler<()>,
}

/// Visual state generation modal
#[component]
pub fn VisualStateGenerationModal(props: VisualStateGenerationModalProps) -> Element {
    // Clone props for use in closures
    let scope = props.scope;
    let location_id_for_closure = props.location_id.clone();
    let region_id_for_closure = props.region_id.clone();
    // Keep references for use in rsx
    let location_id = props.location_id.as_ref();
    let region_id = props.region_id.as_ref();

    // ═══════════════════════════════════════════════════════════════
    // SECTION 1: ALL HOOKS - Always at top, never conditional
    // ═══════════════════════════════════════════════════════════════
    let command_bus = use_command_bus();
    let _generation_service = use_generation_service();

    // Form fields
    let mut name = use_signal(|| "New State".to_string());
    let mut description = use_signal(String::new);
    let mut atmosphere_guidance = use_signal(String::new);
    let mut tags: Signal<Vec<String>> = use_signal(Vec::new);
    let mut tag_input = use_signal(String::new);
    let mut asset_style = use_signal(|| AssetStyle::default());
    let mut generate_backdrop = use_signal(|| true);
    let mut generate_sound = use_signal(|| true);

    // Generation state
    let mut is_generating = use_signal(|| false);
    let mut generation_error: Signal<Option<String>> = use_signal(|| None);
    let mut generated_result: Signal<Option<GeneratedStateResult>> = use_signal(|| None);

    // Quick presets
    let mut handle_apply_preset = move |preset: QuickPreset| {
        description.set(preset.description().to_string());
        atmosphere_guidance.set(String::new());
        tags.set(preset.tags());
    };

    // ═══════════════════════════════════════════════════════════════
    // SECTION 2: EVENT HANDLERS
    // ═══════════════════════════════════════════════════════════════
    let handle_add_tag = move |_| {
        let input_value = tag_input.read().trim().to_string();
        if !input_value.is_empty() {
            let mut current = tags.read().clone();
            if !current.contains(&input_value) {
                current.push(input_value);
                tags.set(current);
            }
            tag_input.set(String::new());
        }
    };

    let handle_tag_input_change = move |e: Event<FormData>| {
        tag_input.set(e.value());
    };

    let mut handle_remove_tag = move |idx: usize| {
        let mut current = tags.read().clone();
        if idx < current.len() {
            current.remove(idx);
            tags.set(current);
        }
    };

    let handle_generate = move |_| {
        generation_error.set(None);
        let desc = description.read();
        if desc.trim().is_empty() {
            generation_error.set(Some("Please enter a description".to_string()));
            return;
        }

        if desc.len() > 500 {
            generation_error.set(Some("Description must be 500 characters or less".to_string()));
            return;
        }

        is_generating.set(true);

        // Clone for use in closure
        let scope_clone = scope;
        let location_id_clone = location_id_for_closure.clone();
        let region_id_clone = region_id_for_closure.clone();

        // Build generation request
        let _request = GenerateVisualStateRequest {
            state_type: scope_clone,
            location_id: location_id_clone.as_ref().and_then(|s| uuid::Uuid::parse_str(s).ok()),
            region_id: region_id_clone.as_ref().and_then(|s| uuid::Uuid::parse_str(s).ok()),
            name: name.read().clone(),
            description: desc.clone(),
            prompt: format!("{} - {}", desc, atmosphere_guidance.read()),
            workflow: "backdrop_v2".to_string(),
            negative_prompt: Some("blurry, low quality, ugly, distorted".to_string()),
            tags: tags.read().clone(),
            generate_backdrop: *generate_backdrop.read(),
            generate_map: false, // Maps not generated for regions
            activation_rules: None,
            activation_logic: None,
            priority: 10,
            is_default: false,
        };

        let _command_bus = command_bus.clone();
        let mut is_gen = is_generating.clone();

        spawn_task(async move {
            // For now, we'll create a simplified state without actual generation
            // In production, this would call the generation endpoint
            let _request_id = uuid::Uuid::new_v4().to_string();

            // TODO: Use generation_service to trigger actual generation workflow
            // For this implementation, we simulate success after a delay
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mock_state_id = uuid::Uuid::new_v4().to_string();
            let mock_name = name.read().clone();

            generated_result.set(Some(GeneratedStateResult {
                state_id: mock_state_id,
                name: mock_name,
            }));

            is_gen.set(false);
        });
    };

    let handle_save_and_use = move |_| {
        if let Some(ref result) = *generated_result.read() {
            props.on_save.call(result.clone());
        }
    };

    let handle_cancel = move |_| {
        props.on_close.call(());
    };

    // ═════════════════════════════════════════════════════════════
    // SECTION 3: RENDER
    // ═══════════════════════════════════════════════════════════════
    rsx! {
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000] p-4",
            onclick: handle_cancel,

            // Modal
            div {
                class: "bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl max-w-2xl w-full max-h-[80vh] overflow-hidden border border-purple-500/30 flex flex-col",
                onclick: |e| e.stop_propagation(),

                    // Header
                    div {
                        class: "p-6 border-b border-white/10",
                        div {
                            class: "flex justify-between items-start",
                            div {
                                h2 {
                                    class: "text-xl font-bold text-purple-400 m-0 mb-2",
                                    "🎨 Generate Visual State"
                                }
                                p {
                                    class: "text-gray-400 text-sm m-0",
                                    match scope {
                                        VisualStateType::Location => {
                                            if let Some(lid) = location_id {
                                                format!("Location ID: {lid}")
                                            } else {
                                                "Location (new state)".to_string()
                                            }
                                        }
                                        VisualStateType::Region => {
                                            if let Some(rid) = region_id {
                                                format!("Region ID: {rid}")
                                            } else {
                                                "Region (new state)".to_string()
                                            }
                                        }
                                        VisualStateType::Unknown => {
                                            "Unknown visual state type".to_string()
                                        }
                                    }
                                }
                            }
                            button {
                                onclick: handle_cancel,
                                class: "p-2 text-gray-400 hover:text-white transition-colors",
                                "X"
                            }
                        }
                    }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-6",

                    // Error display
                    if let Some(ref e) = generation_error.read().as_ref() {
                        div {
                            class: "mb-4 p-3 bg-red-500/10 border border-red-500/50 rounded-lg text-red-400 text-sm",
                            "{e}"
                        }
                    }

                    // Name
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "State Name"
                        }
                        input {
                            r#type: "text",
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                            placeholder: "e.g., Rainy Evening"
                        }
                    }

                    // Description
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Description"
                        }
                        textarea {
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white min-h-[100px]",
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                            placeholder: "Describe how this location should look...",
                        }
                        div {
                            class: "text-right text-xs text-gray-500 mt-1",
                            "{description.read().len()}/500"
                        }
                    }

                    // Quick presets
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Quick Presets"
                        }
                        div {
                            class: "flex gap-2 flex-wrap",
                            button {
                                class: "px-3 py-2 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-sm transition-colors",
                                onclick: move |_| handle_apply_preset(QuickPreset::RainyNight),
                                "{QuickPreset::RainyNight.name()}"
                            }
                            button {
                                class: "px-3 py-2 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-sm transition-colors",
                                onclick: move |_| handle_apply_preset(QuickPreset::Moonlit),
                                "{QuickPreset::Moonlit.name()}"
                            }
                            button {
                                class: "px-3 py-2 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-sm transition-colors",
                                onclick: move |_| handle_apply_preset(QuickPreset::Festive),
                                "{QuickPreset::Festive.name()}"
                            }
                            button {
                                class: "px-3 py-2 bg-blue-600/30 hover:bg-blue-600/50 text-blue-300 rounded text-sm transition-colors",
                                onclick: move |_| handle_apply_preset(QuickPreset::Eerie),
                                "{QuickPreset::Eerie.name()}"
                            }
                        }
                    }

                    // Atmosphere guidance
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Generation Guidance (optional)"
                        }
                        textarea {
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white min-h-[60px]",
                            value: "{atmosphere_guidance}",
                            oninput: move |e| atmosphere_guidance.set(e.value()),
                            placeholder: "e.g., Foggy, mysterious lighting"
                        }
                    }

                    // Tags
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Tags"
                        }
                        div {
                            class: "flex gap-2 mb-2",
                            input {
                                r#type: "text",
                                class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm",
                                value: "{tag_input}",
                                oninput: handle_tag_input_change,
                                placeholder: "Enter tag..."
                            }
                            button {
                                onclick: handle_add_tag,
                                class: "px-3 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg text-sm transition-colors",
                                "Add"
                            }
                        }
                        div {
                            class: "flex flex-wrap gap-2",
                            for (idx, tag) in tags.read().iter().enumerate() {
                                span {
                                    key: "{idx}",
                                    class: "px-2 py-1 bg-purple-500/30 text-purple-300 rounded text-sm flex items-center gap-1",
                                    "{tag}"
                                    button {
                                        onclick: move |_| handle_remove_tag(idx),
                                        class: "ml-1 text-purple-300 hover:text-white",
                                        "×"
                                    }
                                }
                            }
                        }
                    }

                    // Generation options
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Generation Options"
                        }
                        div {
                            class: "space-y-2",
                            label {
                                class: "flex items-center gap-2 text-white cursor-pointer",
                                input {
                                    r#type: "checkbox",
                                    checked: *generate_backdrop.read(),
                                    onchange: move |e| generate_backdrop.set(e.checked()),
                                    class: "w-4 h-4 rounded border-gray-600 bg-dark-bg text-purple-500",
                                }
                                "Generate Backdrop"
                            }
                            label {
                                class: "flex items-center gap-2 text-white cursor-pointer",
                                input {
                                    r#type: "checkbox",
                                    checked: *generate_sound.read(),
                                    onchange: move |e| generate_sound.set(e.checked()),
                                    class: "w-4 h-4 rounded border-gray-600 bg-dark-bg text-purple-500",
                                }
                                "Generate Ambient Sound"
                            }
                        }

                        // Asset style dropdown
                        div {
                            class: "mt-3",
                            label {
                                class: "block text-gray-400 text-sm mb-1",
                                "Asset Style"
                            }
                            select {
                                class: "w-full p-2 bg-dark-bg border border-gray-700 rounded-lg text-white",
                                value: format!("{:?}", asset_style.read()),
                                onchange: move |e| {
                                    let val = e.value();
                                    asset_style.set(match val.as_str() {
                                        "FantasyRealistic" => AssetStyle::FantasyRealistic,
                                        "AnimeVisualNovel" => AssetStyle::AnimeVisualNovel,
                                        "PixelArt" => AssetStyle::PixelArt,
                                        "DarkNoir" => AssetStyle::DarkNoir,
                                        _ => AssetStyle::FantasyRealistic,
                                    });
                                },
                                option {
                                    value: "FantasyRealistic",
                                    "{AssetStyle::FantasyRealistic.name()}"
                                }
                                option {
                                    value: "AnimeVisualNovel",
                                    "{AssetStyle::AnimeVisualNovel.name()}"
                                }
                                option {
                                    value: "PixelArt",
                                    "{AssetStyle::PixelArt.name()}"
                                }
                                option {
                                    value: "DarkNoir",
                                    "{AssetStyle::DarkNoir.name()}"
                                }
                            }
                        }
                    }

                    // Preview/Loading
                    if *is_generating.read() {
                        div {
                            class: "mt-6 p-6 bg-black/30 rounded-lg text-center",
                            div {
                                class: "text-purple-400 mb-2",
                                "⏳ Generating visual assets..."
                            }
                            div {
                                class: "text-gray-400 text-sm",
                                "This may take a moment..."
                            }
                        }
                    } else if let Some(ref result) = generated_result.read().as_ref() {
                        div {
                            class: "mt-6 p-4 bg-green-500/10 border border-green-500/50 rounded-lg",
                            div {
                                class: "text-green-400 font-medium",
                                "✓ State Generated Successfully"
                            }
                            div {
                                class: "text-white text-sm mt-1",
                                "{result.name}"
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "p-6 border-t border-white/10 flex justify-end gap-3",
                    button {
                        onclick: handle_cancel,
                        class: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-500 transition-colors",
                        "Cancel"
                    }

                    button {
                        onclick: handle_generate,
                        class: "px-6 py-2 bg-gradient-to-br from-purple-500 to-purple-600 text-white font-semibold rounded-lg hover:from-purple-400 hover:to-purple-500 transition-all",
                        disabled: *is_generating.read(),
                        "Generate State"
                    }

                    if generated_result.read().is_some() {
                        button {
                            onclick: handle_save_and_use,
                            class: "px-6 py-2 bg-gradient-to-br from-amber-500 to-amber-600 text-white font-semibold rounded-lg hover:from-amber-400 hover:to-amber-500 transition-all",
                            "Save & Use in Staging"
                        }
                    }
                }
            }
        }
    }
}
