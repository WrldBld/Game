//! Visual state generation modal
//!
//! Allows DMs to generate new visual states with AI-generated assets.
//! Used from visual state dropdowns in pre-stage and staging approval.

use dioxus::prelude::*;

use crate::application::services::WorkflowSlotStatus;
use crate::infrastructure::spawn_task;
use crate::presentation::services::{use_visual_state_service, use_workflow_service};

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
            QuickPreset::Moonlit => {
                "Cool moonlight filtering through windows, shadows stretching long"
            }
            QuickPreset::Festive => "Colorful lanterns, celebration decorations, lively atmosphere",
            QuickPreset::Eerie => "Thick fog, unnatural quiet, dim lighting",
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
    let visual_state_service = use_visual_state_service();
    let workflow_service = use_workflow_service();

    // Form fields
    let mut name = use_signal(|| "New State".to_string());
    let mut description = use_signal(String::new);
    let mut atmosphere_guidance = use_signal(String::new);
    let mut generate_backdrop = use_signal(|| true);
    let mut selected_workflow = use_signal(|| "backdrop_v2".to_string());
    let mut pose_asset_id = use_signal(String::new);

    // Workflow list state
    let mut workflow_slots = use_signal(Vec::<WorkflowSlotStatus>::new);
    let mut is_loading_workflows = use_signal(|| true);

    // Generation state
    let mut is_generating = use_signal(|| false);
    let mut generation_error: Signal<Option<String>> = use_signal(|| None);
    let mut generated_result: Signal<Option<GeneratedStateResult>> = use_signal(|| None);

    // Load workflow slots on mount
    let workflow_service_for_effect = workflow_service.clone();
    use_effect(move || {
        let svc = workflow_service_for_effect.clone();
        spawn_task(async move {
            is_loading_workflows.set(true);
            match svc.list_workflows().await {
                Ok(response) => {
                    // Flatten categories into a list of configured slots
                    let mut slots: Vec<WorkflowSlotStatus> = Vec::new();
                    for category in response.categories {
                        for slot in category.slots {
                            if slot.configured {
                                slots.push(slot);
                            }
                        }
                    }
                    workflow_slots.set(slots);
                    is_loading_workflows.set(false);
                }
                Err(e) => {
                    tracing::warn!("Failed to load workflow slots: {}", e);
                    is_loading_workflows.set(false);
                }
            }
        });
    });

    // Quick presets
    let mut handle_apply_preset = move |preset: QuickPreset| {
        description.set(preset.description().to_string());
        atmosphere_guidance.set(String::new());
    };

    // ═══════════════════════════════════════════════════════════════
    // SECTION 2: EVENT HANDLERS
    // ═══════════════════════════════════════════════════════════════
    let handle_generate = move |_| {
        generation_error.set(None);
        let desc = description.read();
        if desc.trim().is_empty() {
            generation_error.set(Some("Please enter a description".to_string()));
            return;
        }

        // Compute composed prompt length (description + optional atmosphere guidance)
        let atmosphere = atmosphere_guidance.read();
        let composed_prompt = if atmosphere.trim().is_empty() {
            desc.clone()
        } else {
            format!("{} - {}", desc, atmosphere)
        };
        let prompt_char_count = composed_prompt.chars().count();

        if prompt_char_count > 5000 {
            generation_error.set(Some(
                "Prompt must be 5000 characters or less".to_string(),
            ));
            return;
        }

        is_generating.set(true);

        // Clone for use in closure
        let scope_clone = scope;
        let location_id_clone = location_id_for_closure.clone();
        let region_id_clone = region_id_for_closure.clone();
        let name_clone = name.read().clone();
        let desc_clone = desc.clone();
        let generate_backdrop_clone = *generate_backdrop.read();
        let workflow_clone = selected_workflow.read().clone();
        let _pose_asset_clone = pose_asset_id.read().clone();
        let service = visual_state_service.clone();

        // Reuse the composed prompt we already validated
        let prompt_clone = composed_prompt;

        spawn_task(async move {
            // Build generation request using the pre-composed prompt
            let prompt = prompt_clone;

            let request = GenerateVisualStateRequest {
                state_type: scope_clone,
                location_id: location_id_clone
                    .as_ref()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok()),
                region_id: region_id_clone
                    .as_ref()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok()),
                name: name_clone.clone(),
                description: desc_clone,
                prompt,
                workflow: workflow_clone,
                map_workflow: None,
                negative_prompt: Some("blurry, low quality, ugly, distorted".to_string()),
                generate_backdrop: generate_backdrop_clone,
                generate_map: false, // Maps not generated for regions
                activation_rules: None,
                activation_logic: None,
                priority: 10,
                is_default: false,
            };

            // Call the visual state service to generate
            match service.generate_state(request).await {
                Ok(generated_data) => {
                    // Extract state info from the response
                    if let Some(location_state) = generated_data.location_state {
                        generated_result.set(Some(GeneratedStateResult {
                            state_id: location_state.id.to_string(),
                            name: location_state.name,
                        }));
                    } else if let Some(region_state) = generated_data.region_state {
                        generated_result.set(Some(GeneratedStateResult {
                            state_id: region_state.id.to_string(),
                            name: region_state.name,
                        }));
                    } else {
                        generation_error.set(Some(
                            "Generation failed: missing state data".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    generation_error.set(Some(format!("Generation failed: {}", e)));
                }
            }

            is_generating.set(false);
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
                            {
                                let desc = description.read();
                                let atm = atmosphere_guidance.read();
                                let composed_len = if atm.trim().is_empty() {
                                    desc.chars().count()
                                } else {
                                    format!("{} - {}", desc, atm).chars().count()
                                };
                                format!("{}/5000", composed_len)
                            }
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

                    // Workflow selection
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Workflow"
                        }
                        if *is_loading_workflows.read() {
                            div {
                                class: "text-gray-500 text-sm",
                                "Loading workflows..."
                            }
                        } else {
                            select {
                                class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white",
                                value: "{selected_workflow}",
                                onchange: move |e| selected_workflow.set(e.value()),
                                {
                                    let slots = workflow_slots.read();
                                    if slots.is_empty() {
                                        rsx! {
                                            option {
                                                value: "backdrop_v2",
                                                "backdrop_v2 (default)"
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            for slot in slots.iter() {
                                                option {
                                                    value: "{slot.slot}",
                                                    "{slot.display_name}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        p {
                            class: "text-gray-600 text-xs mt-1",
                            "Select a ComfyUI workflow for generation"
                        }
                    }

                    // Pose asset selection
                    div {
                        class: "mb-4",
                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Pose Asset (optional)"
                        }
                        input {
                            r#type: "text",
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white",
                            value: "{pose_asset_id}",
                            oninput: move |e| pose_asset_id.set(e.value()),
                            placeholder: "Enter asset ID for pose reference"
                        }
                        p {
                            class: "text-gray-600 text-xs mt-1",
                            "Asset ID for ControlNet pose injection (if workflow supports it)"
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
                    } else if let Some(result) = generated_result.read().as_ref() {
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
