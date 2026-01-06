//! Workflow Configuration Editor Component
//!
//! Displays and allows editing of a configured workflow's settings,
//! including prompt mappings, input defaults, and locked inputs.
//!
//! US-AST-010: Advanced Workflow Parameter Editor
//! - Editable prompt mappings with add/remove
//! - Locked inputs toggle section
//! - Style reference (IPAdapter) detection
//! - Raw JSON viewer

use dioxus::prelude::*;

use crate::presentation::services::use_workflow_service;
use crate::application::application::services::{
    InputDefault, PromptMapping, TestWorkflowResponse, WorkflowConfig, WorkflowInput,
};

/// Props for the WorkflowConfigEditor component
#[derive(Props, Clone, PartialEq)]
pub struct WorkflowConfigEditorProps {
    /// The slot ID to show configuration for
    pub slot: String,
    /// Callback when close is clicked
    pub on_close: EventHandler<()>,
    /// Callback when reconfigure is clicked
    pub on_reconfigure: EventHandler<()>,
    /// Callback when workflow is deleted
    pub on_deleted: EventHandler<()>,
}

// Type aliases for service types to minimize changes
type WorkflowConfigFull = WorkflowConfig;
type WorkflowInputData = WorkflowInput;
type PromptMappingData = PromptMapping;
type InputDefaultData = InputDefault;
type WorkflowTestResult = TestWorkflowResponse;

/// Workflow configuration editor
#[component]
pub fn WorkflowConfigEditor(props: WorkflowConfigEditorProps) -> Element {
    let workflow_service = use_workflow_service();

    // Track loading state
    let mut is_loading = use_signal(|| true);
    // Track error state
    let mut error: Signal<Option<String>> = use_signal(|| None);
    // Store the config
    let mut config: Signal<Option<WorkflowConfigFull>> = use_signal(|| None);
    // Track which section is expanded
    let mut expanded_section = use_signal(|| "mappings");
    // Track if we're saving
    let mut is_saving = use_signal(|| false);
    // Track edits to defaults
    let mut edited_defaults: Signal<Vec<InputDefaultData>> = use_signal(Vec::new);
    // Track edits to prompt mappings
    let mut edited_mappings: Signal<Vec<PromptMappingData>> = use_signal(Vec::new);
    // Track edits to locked inputs
    let mut edited_locked: Signal<Vec<String>> = use_signal(Vec::new);
    // Track delete confirmation dialog visibility
    let mut show_delete_confirmation = use_signal(|| false);
    // Track if deleting
    let mut is_deleting = use_signal(|| false);
    // Track test modal visibility
    let mut show_test_modal = use_signal(|| false);
    // Track test state
    let mut test_prompt = use_signal(|| "".to_string());
    let mut is_testing = use_signal(|| false);
    let mut test_result: Signal<Option<WorkflowTestResult>> = use_signal(|| None);
    let mut test_error: Signal<Option<String>> = use_signal(|| None);

    let slot_id = props.slot.clone();
    let slot_id_for_effect = slot_id.clone();
    let workflow_service_for_effect = workflow_service.clone();

    // Fetch config on mount or when slot changes
    use_effect(move || {
        let slot = slot_id_for_effect.clone();
        let svc = workflow_service_for_effect.clone();
        spawn(async move {
            is_loading.set(true);
            error.set(None);

            match svc.get_workflow_config(&slot).await {
                Ok(Some(fetched_config)) => {
                    edited_defaults.set(fetched_config.input_defaults.clone());
                    edited_mappings.set(fetched_config.prompt_mappings.clone());
                    edited_locked.set(fetched_config.locked_inputs.clone());
                    config.set(Some(fetched_config));
                    is_loading.set(false);
                }
                Ok(None) => {
                    config.set(None);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    is_loading.set(false);
                }
            }
        });
    });

    let slot_id_for_save = slot_id.clone();
    let workflow_service_for_save = workflow_service.clone();
    // Save handler
    let save_config = move |_| {
        let slot = slot_id_for_save.clone();
        let defaults = edited_defaults.read().clone();
        let locked = edited_locked.read().clone();
        let current_config = config.read().clone();
        let svc = workflow_service_for_save.clone();

        spawn(async move {
            is_saving.set(true);
            error.set(None);

            if current_config.is_some() {
                match svc.update_workflow_defaults(&slot, defaults, Some(locked)).await {
                    Ok(updated_config) => {
                        config.set(Some(updated_config));
                        tracing::info!("Workflow defaults saved successfully");
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save: {}", e)));
                        tracing::error!("Failed to save workflow defaults: {}", e);
                    }
                }
            }

            is_saving.set(false);
        });
    };

    let slot_id_for_delete = slot_id.clone();
    let workflow_service_for_delete = workflow_service.clone();
    // Delete handler
    let on_deleted = props.on_deleted;
    let do_delete = move |_| {
        let slot = slot_id_for_delete.clone();
        let callback = on_deleted;
        let svc = workflow_service_for_delete.clone();

        spawn(async move {
            is_deleting.set(true);

            match svc.delete_workflow_config(&slot).await {
                Ok(_) => {
                    show_delete_confirmation.set(false);
                    callback.call(());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    is_deleting.set(false);
                    show_delete_confirmation.set(false);
                }
            }
        });
    };

    let slot_id_for_test = slot_id.clone();
    let workflow_service_for_test = workflow_service.clone();
    // Test handler
    let do_test = move |_| {
        let slot = slot_id_for_test.clone();
        let prompt = test_prompt.read().clone();
        let svc = workflow_service_for_test.clone();

        spawn(async move {
            is_testing.set(true);
            test_error.set(None);
            test_result.set(None);

            match svc.test_workflow(&slot, &prompt).await {
                Ok(result) => {
                    test_result.set(Some(result));
                }
                Err(e) => {
                    test_error.set(Some(e.to_string()));
                }
            }

            is_testing.set(false);
        });
    };

    rsx! {
        div {
            class: "workflow-config-editor flex-1 flex flex-col bg-dark-surface rounded-lg overflow-hidden",

            if *is_loading.read() {
                div {
                    class: "flex-1 flex items-center justify-center text-gray-500",
                    "Loading configuration..."
                }
            } else if let Some(err) = error.read().as_ref() {
                div {
                    class: "flex-1 flex flex-col items-center justify-center py-8",

                    div {
                        class: "p-4 bg-red-500 bg-opacity-10 rounded-lg text-red-500 text-sm mb-4",
                        "Error: {err}"
                    }

                    button {
                        onclick: move |_| props.on_reconfigure.call(()),
                        class: "py-2 px-4 bg-blue-500 text-white border-0 rounded-lg cursor-pointer",
                        "Configure Workflow"
                    }
                }
            } else if let Some(cfg) = config.read().as_ref() {
                // Header
                div {
                    class: "p-4 border-b border-gray-700",

                    div {
                        class: "flex items-center justify-between",

                        div {
                            h2 {
                                class: "text-white text-lg m-0 mb-1",
                                "{cfg.slot_display_name}"
                            }
                            p {
                                class: "text-green-500 text-sm m-0",
                                "✓ {cfg.name}"
                            }
                        }

                        div {
                            class: "flex gap-2",

                            button {
                                onclick: move |_| {
                                    test_prompt.set("".to_string());
                                    test_result.set(None);
                                    test_error.set(None);
                                    show_test_modal.set(true);
                                },
                                class: "py-2 px-4 bg-purple-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                                "Test Workflow"
                            }

                            button {
                                onclick: move |_| props.on_reconfigure.call(()),
                                class: "py-2 px-4 bg-gray-700 text-white border-0 rounded-lg cursor-pointer text-sm",
                                "Reconfigure"
                            }

                            button {
                                onclick: move |_| show_delete_confirmation.set(true),
                                class: "py-2 px-4 bg-red-600 text-white border-0 rounded-lg cursor-pointer text-sm",
                                "Delete"
                            }
                        }
                    }
                }

                // Stats bar
                div {
                    class: "flex gap-4 py-3 px-4 bg-black bg-opacity-20 border-b border-gray-700",

                    StatBadge { label: "Nodes", value: cfg.analysis.node_count.to_string() }
                    StatBadge { label: "Inputs", value: cfg.analysis.inputs.len().to_string() }
                    StatBadge { label: "Text Inputs", value: cfg.analysis.text_inputs.len().to_string() }
                }

                // Content sections
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    // Prompt Mappings section (editable)
                    CollapsibleSection {
                        title: "Prompt Mappings",
                        is_expanded: *expanded_section.read() == "mappings",
                        on_toggle: move |_| {
                            if *expanded_section.read() == "mappings" {
                                expanded_section.set("");
                            } else {
                                expanded_section.set("mappings");
                            }
                        },

                        div {
                            class: "flex flex-col gap-2",

                            p {
                                class: "text-gray-500 text-xs mb-2",
                                "Map entity descriptions to workflow text inputs for prompt injection."
                            }

                            // Existing mappings
                            {
                                let mappings = edited_mappings.read().clone();
                                let text_inputs = cfg.analysis.text_inputs.clone();
                                rsx! {
                                    for (idx, mapping) in mappings.iter().enumerate() {
                                        EditablePromptMappingRow {
                                            key: "{idx}",
                                            mapping: mapping.clone(),
                                            inputs: text_inputs.clone(),
                                            on_remove: {
                                                move |_| {
                                                    let mut current = edited_mappings.read().clone();
                                                    if idx < current.len() {
                                                        current.remove(idx);
                                                        edited_mappings.set(current);
                                                    }
                                                }
                                            },
                                            on_change: {
                                                move |new_mapping: PromptMappingData| {
                                                    let mut current = edited_mappings.read().clone();
                                                    if idx < current.len() {
                                                        current[idx] = new_mapping;
                                                        edited_mappings.set(current);
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                            }

                            // Add new mapping button
                            {
                                let text_inputs = cfg.analysis.text_inputs.clone();
                                let has_unmapped = text_inputs.iter().any(|input| {
                                    !edited_mappings.read().iter().any(|m| 
                                        m.node_id == input.node_id && m.input_name == input.input_name
                                    )
                                });
                                
                                if has_unmapped {
                                    rsx! {
                                        button {
                                            r#type: "button",
                                            class: "mt-2 py-2 px-3 bg-blue-500 bg-opacity-20 text-blue-400 border border-blue-500 border-opacity-30 rounded-lg cursor-pointer text-sm hover:bg-opacity-30",
                                            onclick: move |_| {
                                                // Find first unmapped text input
                                                if let Some(input) = text_inputs.iter().find(|input| {
                                                    !edited_mappings.read().iter().any(|m| 
                                                        m.node_id == input.node_id && m.input_name == input.input_name
                                                    )
                                                }) {
                                                    let mut current = edited_mappings.read().clone();
                                                    current.push(PromptMappingData {
                                                        node_id: input.node_id.clone(),
                                                        input_name: input.input_name.clone(),
                                                        mapping_type: "primary".to_string(),
                                                    });
                                                    edited_mappings.set(current);
                                                }
                                            },
                                            "+ Add Mapping"
                                        }
                                    }
                                } else if edited_mappings.read().is_empty() {
                                    rsx! {
                                        div {
                                            class: "text-gray-500 text-sm p-2",
                                            "No text inputs available for prompt mapping"
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }
                        }
                    }

                    // Input Defaults section with lock toggles
                    CollapsibleSection {
                        title: "Input Defaults",
                        is_expanded: *expanded_section.read() == "defaults",
                        on_toggle: move |_| {
                            if *expanded_section.read() == "defaults" {
                                expanded_section.set("");
                            } else {
                                expanded_section.set("defaults");
                            }
                        },

                        div {
                            class: "flex flex-col gap-2",

                            p {
                                class: "text-gray-500 text-xs mb-2",
                                "Set default values for workflow inputs. Lock inputs to prevent changes during generation."
                            }

                            for input in cfg.analysis.inputs.iter() {
                                {
                                    let input_path = format!("{}:{}", input.node_id, input.input_name);
                                    let is_locked = edited_locked.read().contains(&input_path);
                                    let input_for_lock = input.clone();
                                    rsx! {
                                        InputDefaultRowWithLock {
                                            input: input.clone(),
                                            defaults: edited_defaults.read().clone(),
                                            is_locked: is_locked,
                                            on_change: move |new_value: InputDefaultData| {
                                                let mut current = edited_defaults.read().clone();
                                                if let Some(existing) = current.iter_mut().find(|d| {
                                                    d.node_id == new_value.node_id && d.input_name == new_value.input_name
                                                }) {
                                                    existing.default_value = new_value.default_value;
                                                } else {
                                                    current.push(new_value);
                                                }
                                                edited_defaults.set(current);
                                            },
                                            on_lock_toggle: {
                                                let path = format!("{}:{}", input_for_lock.node_id, input_for_lock.input_name);
                                                move |_| {
                                                    let mut current = edited_locked.read().clone();
                                                    if current.contains(&path) {
                                                        current.retain(|p| p != &path);
                                                    } else {
                                                        current.push(path.clone());
                                                    }
                                                    edited_locked.set(current);
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Locked Inputs Summary
                    {
                        let locked_count = edited_locked.read().len();
                        if locked_count > 0 {
                            rsx! {
                                CollapsibleSection {
                                    title: "Locked Inputs",
                                    is_expanded: *expanded_section.read() == "locked",
                                    on_toggle: move |_| {
                                        if *expanded_section.read() == "locked" {
                                            expanded_section.set("");
                                        } else {
                                            expanded_section.set("locked");
                                        }
                                    },

                                    div {
                                        class: "flex flex-col gap-1",

                                        p {
                                            class: "text-gray-500 text-xs mb-2",
                                            "These inputs will always use their default values and won't be shown in generation forms."
                                        }

                                        for path in edited_locked.read().iter() {
                                            div {
                                                class: "flex items-center justify-between py-1.5 px-2 bg-amber-500 bg-opacity-10 rounded-md border border-amber-500 border-opacity-20",
                                                
                                                div {
                                                    class: "flex items-center gap-2",
                                                    span { class: "text-amber-500 text-xs", "🔒" }
                                                    span { class: "text-gray-300 text-sm font-mono", "{path}" }
                                                }
                                                
                                                {
                                                    let path_to_remove = path.clone();
                                                    rsx! {
                                                        button {
                                                            r#type: "button",
                                                            class: "text-gray-500 hover:text-red-400 text-sm px-1",
                                                            onclick: move |_| {
                                                                let mut current = edited_locked.read().clone();
                                                                current.retain(|p| p != &path_to_remove);
                                                                edited_locked.set(current);
                                                            },
                                                            "×"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }

                    // Style Reference Detection section
                    {
                        // Detect style reference nodes based on node types in the analysis
                        let style_nodes: Vec<&WorkflowInputData> = cfg.analysis.inputs.iter()
                            .filter(|i| {
                                let node_type = i.node_type.to_lowercase();
                                node_type.contains("ipadapter") || 
                                node_type.contains("clipvision") ||
                                node_type.contains("prepimage")
                            })
                            .collect();
                        
                        if !style_nodes.is_empty() {
                            rsx! {
                                CollapsibleSection {
                                    title: "Style Reference",
                                    is_expanded: *expanded_section.read() == "style",
                                    on_toggle: move |_| {
                                        if *expanded_section.read() == "style" {
                                            expanded_section.set("");
                                        } else {
                                            expanded_section.set("style");
                                        }
                                    },

                                    div {
                                        class: "flex flex-col gap-2",

                                        div {
                                            class: "flex items-center gap-2 py-2 px-3 bg-green-500 bg-opacity-10 rounded-lg border border-green-500 border-opacity-30",
                                            
                                            span { class: "text-green-400 text-lg", "✓" }
                                            
                                            div {
                                                span { class: "text-green-400 text-sm font-medium", "Style reference nodes detected" }
                                                p { class: "text-gray-400 text-xs mt-1", "This workflow supports style reference images via IPAdapter." }
                                            }
                                        }

                                        div {
                                            class: "text-gray-500 text-xs mt-2 mb-1",
                                            "Detected nodes:"
                                        }

                                        for node in style_nodes.iter() {
                                            div {
                                                class: "flex items-center gap-2 py-1.5 px-2 bg-black bg-opacity-20 rounded-md",
                                                
                                                span { 
                                                    class: "py-0.5 px-1.5 bg-purple-500 bg-opacity-30 text-purple-300 text-xs rounded",
                                                    "{node.node_type}" 
                                                }
                                                
                                                if let Some(title) = &node.node_title {
                                                    span { class: "text-gray-400 text-sm", "{title}" }
                                                }
                                                
                                                span { class: "text-gray-600 text-xs ml-auto", "Node {node.node_id}" }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                CollapsibleSection {
                                    title: "Style Reference",
                                    is_expanded: *expanded_section.read() == "style",
                                    on_toggle: move |_| {
                                        if *expanded_section.read() == "style" {
                                            expanded_section.set("");
                                        } else {
                                            expanded_section.set("style");
                                        }
                                    },

                                    div {
                                        class: "py-2 px-3 bg-gray-700 bg-opacity-30 rounded-lg",
                                        
                                        p { class: "text-gray-400 text-sm", "No IPAdapter nodes detected" }
                                        p { class: "text-gray-500 text-xs mt-1", "Add IPAdapter nodes to your workflow to enable style reference support." }
                                    }
                                }
                            }
                        }
                    }

                    // Workflow Info section
                    CollapsibleSection {
                        title: "Workflow Info",
                        is_expanded: *expanded_section.read() == "info",
                        on_toggle: move |_| {
                            if *expanded_section.read() == "info" {
                                expanded_section.set("");
                            } else {
                                expanded_section.set("info");
                            }
                        },

                        div {
                            class: "flex flex-col gap-2 text-sm",

                            InfoRow { label: "ID", value: cfg.id.clone() }
                            InfoRow { label: "Created", value: cfg.created_at.clone() }
                            InfoRow { label: "Updated", value: cfg.updated_at.clone() }
                        }
                    }

                    // Raw JSON Viewer section
                    CollapsibleSection {
                        title: "Raw Workflow JSON",
                        is_expanded: *expanded_section.read() == "json",
                        on_toggle: move |_| {
                            if *expanded_section.read() == "json" {
                                expanded_section.set("");
                            } else {
                                expanded_section.set("json");
                            }
                        },

                        div {
                            class: "flex flex-col gap-2",

                            p {
                                class: "text-gray-500 text-xs",
                                "The raw ComfyUI API workflow JSON. This is read-only."
                            }

                            div {
                                class: "relative",

                                pre {
                                    class: "p-3 bg-dark-bg rounded-lg text-gray-400 text-xs font-mono overflow-x-auto max-h-[300px] overflow-y-auto border border-gray-700",
                                    // Show node count summary since we don't have the full JSON
                                    "{cfg.analysis.node_count} nodes, {cfg.analysis.inputs.len()} inputs"
                                }
                            }
                        }
                    }
                }

                // Footer with save button
                div {
                    class: "p-4 border-t border-gray-700 flex justify-end gap-2",

                    button {
                        onclick: save_config,
                        disabled: *is_saving.read(),
                        class: "py-2 px-6 bg-green-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                        if *is_saving.read() { "Saving..." } else { "Save Changes" }
                    }
                }
            } else {
                // Not configured
                div {
                    class: "flex-1 flex flex-col items-center justify-center py-8",

                    div {
                        class: "text-center max-w-xs",

                        div {
                            class: "text-3xl mb-4 opacity-50",
                            "📋"
                        }

                        h3 {
                            class: "text-gray-400 mb-2",
                            "Not Configured"
                        }

                        p {
                            class: "text-gray-500 text-sm mb-4",
                            "This workflow slot hasn't been configured yet. Upload a ComfyUI workflow to get started."
                        }

                        button {
                            onclick: move |_| props.on_reconfigure.call(()),
                            class: "py-3 px-6 bg-blue-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                            "Configure Workflow"
                        }
                    }
                }

                // Delete confirmation modal
                if *show_delete_confirmation.read() {
                    ConfirmDeleteModal {
                        slot_name: if let Some(cfg) = config.read().as_ref() { cfg.slot_display_name.clone() } else { "Workflow".to_string() },
                        is_deleting: *is_deleting.read(),
                        on_confirm: do_delete,
                        on_cancel: move |_| show_delete_confirmation.set(false),
                    }
                }

                // Test workflow modal
                if *show_test_modal.read() {
                    TestWorkflowModal {
                        slot: props.slot.clone(),
                        test_prompt: test_prompt.read().clone(),
                        is_testing: *is_testing.read(),
                        test_result: test_result.read().clone(),
                        test_error: test_error.read().clone(),
                        on_prompt_change: move |prompt| test_prompt.set(prompt),
                        on_test: do_test,
                        on_close: move |_| {
                            show_test_modal.set(false);
                            test_prompt.set("".to_string());
                            test_result.set(None);
                            test_error.set(None);
                        },
                    }
                }
            }
        }
    }
}

/// Stat badge component
#[component]
fn StatBadge(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-2",

            span { class: "text-gray-500 text-xs", "{label}:" }
            span { class: "text-white text-sm font-medium", "{value}" }
        }
    }
}

/// Collapsible section component
#[derive(Props, Clone, PartialEq)]
struct CollapsibleSectionProps {
    title: &'static str,
    is_expanded: bool,
    on_toggle: EventHandler<()>,
    children: Element,
}

#[component]
fn CollapsibleSection(props: CollapsibleSectionProps) -> Element {
    rsx! {
        div {
            class: "collapsible-section mb-3 bg-black bg-opacity-20 rounded-lg overflow-hidden",

            button {
                onclick: move |_| props.on_toggle.call(()),
                class: "w-full flex items-center justify-between py-3 px-4 bg-transparent border-0 cursor-pointer text-white",

                span { class: "font-medium text-sm", "{props.title}" }
                span { class: "text-gray-500", if props.is_expanded { "▼" } else { "▶" } }
            }

            if props.is_expanded {
                div {
                    class: "px-4 pb-4",
                    {props.children}
                }
            }
        }
    }
}

/// Prompt mapping row component
#[derive(Props, Clone, PartialEq)]
struct PromptMappingRowProps {
    mapping: PromptMappingData,
    inputs: Vec<WorkflowInputData>,
}

#[component]
fn PromptMappingRow(props: PromptMappingRowProps) -> Element {
    let input_info = props
        .inputs
        .iter()
        .find(|i| i.node_id == props.mapping.node_id && i.input_name == props.mapping.input_name);

    let node_label = input_info
        .and_then(|i| i.node_title.clone())
        .unwrap_or_else(|| format!("Node {}", props.mapping.node_id));

    let type_color = if props.mapping.mapping_type == "primary" {
        "#22c55e"
    } else {
        "#ef4444"
    };

    rsx! {
        div {
            class: "flex items-center gap-3 p-2 bg-black bg-opacity-20 rounded-md",

            span {
                class: format!("py-0.5 px-2 {} text-white text-xs rounded uppercase", if type_color == "#22c55e" { "bg-green-500" } else { "bg-red-500" }),
                "{props.mapping.mapping_type}"
            }

            div {
                class: "flex-1",

                span { class: "text-white text-sm", "{node_label}" }
                span { class: "text-gray-500 text-xs ml-2", "→ {props.mapping.input_name}" }
            }
        }
    }
}

/// Input default row component
#[derive(Props, Clone, PartialEq)]
struct InputDefaultRowProps {
    input: WorkflowInputData,
    defaults: Vec<InputDefaultData>,
    locked: bool,
    on_change: EventHandler<InputDefaultData>,
}

#[component]
fn InputDefaultRow(props: InputDefaultRowProps) -> Element {
    let current_default = props
        .defaults
        .iter()
        .find(|d| d.node_id == props.input.node_id && d.input_name == props.input.input_name);

    let display_value = current_default
        .map(|d| format_json_value(&d.default_value))
        .unwrap_or_else(|| format_json_value(&props.input.current_value));

    let node_label = props
        .input
        .node_title
        .clone()
        .unwrap_or_else(|| format!("Node {}", props.input.node_id));

    let input_for_change = props.input.clone();

    rsx! {
        div {
            class: "flex items-center gap-2 p-2 bg-black bg-opacity-20 rounded-md",

            // Lock indicator
            if props.locked {
                span { class: "text-amber-500 text-xs", "🔒" }
            }

            // Input info
            div {
                class: "flex-1 min-w-0",

                div {
                    class: "flex items-center gap-2",

                    span {
                        class: "text-gray-400 text-xs py-0.5 px-1.5 bg-gray-700 rounded",
                        "{props.input.input_type}"
                    }
                    span { class: "text-white text-sm", "{props.input.input_name}" }
                }

                span { class: "text-gray-500 text-xs", "{node_label}" }
            }

            // Value input
            input {
                r#type: "text",
                value: "{display_value}",
                disabled: props.locked,
                onchange: move |e| {
                    props.on_change.call(InputDefaultData {
                        node_id: input_for_change.node_id.clone(),
                        input_name: input_for_change.input_name.clone(),
                        default_value: parse_input_value(&e.value(), &input_for_change.input_type),
                    });
                },
                class: "w-30 py-1.5 px-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm",
            }
        }
    }
}

/// Info row component
#[component]
fn InfoRow(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            class: "flex justify-between py-1",

            span { class: "text-gray-500", "{label}" }
            span { class: "text-gray-400 font-mono text-xs", "{value}" }
        }
    }
}

/// Editable prompt mapping row component
#[derive(Props, Clone, PartialEq)]
struct EditablePromptMappingRowProps {
    mapping: PromptMappingData,
    inputs: Vec<WorkflowInputData>,
    on_remove: EventHandler<()>,
    on_change: EventHandler<PromptMappingData>,
}

#[component]
fn EditablePromptMappingRow(props: EditablePromptMappingRowProps) -> Element {
    let input_info = props
        .inputs
        .iter()
        .find(|i| i.node_id == props.mapping.node_id && i.input_name == props.mapping.input_name);

    let node_label = input_info
        .and_then(|i| i.node_title.clone())
        .unwrap_or_else(|| format!("Node {}", props.mapping.node_id));

    let is_primary = props.mapping.mapping_type == "primary";
    let mapping_for_type_change = props.mapping.clone();

    rsx! {
        div {
            class: "flex items-center gap-3 p-2 bg-black bg-opacity-20 rounded-md",

            // Type selector
            select {
                value: "{props.mapping.mapping_type}",
                onchange: {
                    move |e| {
                        let mut new_mapping = mapping_for_type_change.clone();
                        new_mapping.mapping_type = e.value();
                        props.on_change.call(new_mapping);
                    }
                },
                class: "py-1 px-2 bg-dark-bg border border-gray-700 rounded text-xs text-white cursor-pointer",

                option { value: "primary", class: "bg-green-600", "Primary" }
                option { value: "negative", class: "bg-red-600", "Negative" }
            }

            // Node info
            div {
                class: "flex-1",

                span { class: "text-white text-sm", "{node_label}" }
                span { class: "text-gray-500 text-xs ml-2", "→ {props.mapping.input_name}" }
            }

            // Type badge indicator
            span {
                class: if is_primary { "py-0.5 px-2 bg-green-500 text-white text-xs rounded uppercase" } else { "py-0.5 px-2 bg-red-500 text-white text-xs rounded uppercase" },
                if is_primary { "+" } else { "−" }
            }

            // Remove button
            button {
                r#type: "button",
                onclick: move |_| props.on_remove.call(()),
                class: "text-gray-500 hover:text-red-400 text-lg px-1 bg-transparent border-none cursor-pointer",
                "×"
            }
        }
    }
}

/// Input default row with lock toggle
#[derive(Props, Clone, PartialEq)]
struct InputDefaultRowWithLockProps {
    input: WorkflowInputData,
    defaults: Vec<InputDefaultData>,
    is_locked: bool,
    on_change: EventHandler<InputDefaultData>,
    on_lock_toggle: EventHandler<()>,
}

#[component]
fn InputDefaultRowWithLock(props: InputDefaultRowWithLockProps) -> Element {
    let current_default = props
        .defaults
        .iter()
        .find(|d| d.node_id == props.input.node_id && d.input_name == props.input.input_name);

    let display_value = current_default
        .map(|d| format_json_value(&d.default_value))
        .unwrap_or_else(|| format_json_value(&props.input.current_value));

    let node_label = props
        .input
        .node_title
        .clone()
        .unwrap_or_else(|| format!("Node {}", props.input.node_id));

    let input_for_change = props.input.clone();
    let input_type_display = match props.input.input_type.as_str() {
        "Text" | "text" => "txt",
        "Integer" | "integer" => "int",
        "Float" | "float" => "flt",
        "Boolean" | "boolean" => "bool",
        other => other,
    };

    rsx! {
        div {
            class: if props.is_locked { 
                "flex items-center gap-2 p-2 bg-amber-500 bg-opacity-10 rounded-md border border-amber-500 border-opacity-20" 
            } else { 
                "flex items-center gap-2 p-2 bg-black bg-opacity-20 rounded-md" 
            },

            // Lock toggle button
            button {
                r#type: "button",
                onclick: move |_| props.on_lock_toggle.call(()),
                class: if props.is_locked {
                    "text-amber-500 text-sm px-1 bg-transparent border-none cursor-pointer hover:text-amber-400"
                } else {
                    "text-gray-600 text-sm px-1 bg-transparent border-none cursor-pointer hover:text-amber-500"
                },
                title: if props.is_locked { "Click to unlock" } else { "Click to lock" },
                if props.is_locked { "🔒" } else { "🔓" }
            }

            // Input info
            div {
                class: "flex-1 min-w-0",

                div {
                    class: "flex items-center gap-2",

                    span {
                        class: "text-gray-400 text-xs py-0.5 px-1.5 bg-gray-700 rounded",
                        "{input_type_display}"
                    }
                    span { class: "text-white text-sm", "{props.input.input_name}" }
                }

                span { class: "text-gray-500 text-xs", "{node_label}" }
            }

            // Value input
            input {
                r#type: "text",
                value: "{display_value}",
                disabled: props.is_locked,
                onchange: move |e| {
                    props.on_change.call(InputDefaultData {
                        node_id: input_for_change.node_id.clone(),
                        input_name: input_for_change.input_name.clone(),
                        default_value: parse_input_value(&e.value(), &input_for_change.input_type),
                    });
                },
                class: if props.is_locked {
                    "w-24 py-1.5 px-2 bg-gray-800 border border-gray-700 rounded-md text-gray-500 text-sm cursor-not-allowed"
                } else {
                    "w-24 py-1.5 px-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm"
                },
            }
        }
    }
}

/// Format JSON value for display
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

/// Parse input value based on type
fn parse_input_value(value: &str, input_type: &str) -> serde_json::Value {
    match input_type {
        "integer" => value
            .parse::<i64>()
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::String(value.to_string())),
        "float" => value
            .parse::<f64>()
            .ok()
            .and_then(|n| serde_json::Number::from_f64(n).map(serde_json::Value::Number))
            .unwrap_or_else(|| serde_json::Value::String(value.to_string())),
        "boolean" => serde_json::Value::Bool(value == "true" || value == "1"),
        _ => serde_json::Value::String(value.to_string()),
    }
}

/// Confirm delete modal component
#[derive(Props, Clone, PartialEq)]
struct ConfirmDeleteModalProps {
    slot_name: String,
    is_deleting: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
}

#[component]
fn ConfirmDeleteModal(props: ConfirmDeleteModalProps) -> Element {
    rsx! {
        div {
            class: "modal-backdrop fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_cancel.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl w-[90%] max-w-[400px] p-6 overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center gap-4 mb-4",

                    div {
                        class: "text-red-600 text-2xl",
                        "!"
                    }

                    h2 {
                        class: "text-red-600 text-lg m-0",
                        "Delete Workflow Configuration"
                    }
                }

                // Message
                p {
                    class: "text-gray-400 my-4",
                    "Are you sure you want to delete the configuration for {props.slot_name}? This action cannot be undone."
                }

                // Buttons
                div {
                    class: "flex gap-3 justify-end mt-6",

                    button {
                        onclick: move |_| props.on_cancel.call(()),
                        disabled: props.is_deleting,
                        class: "px-4 py-2 bg-gray-700 text-white border-none rounded-lg cursor-pointer text-sm",
                        "Cancel"
                    }

                    button {
                        onclick: move |_| props.on_confirm.call(()),
                        disabled: props.is_deleting,
                        class: "px-4 py-2 bg-red-600 text-white border-none rounded-lg cursor-pointer text-sm font-medium",
                        if props.is_deleting { "Deleting..." } else { "Delete Configuration" }
                    }
                }
            }
        }
    }
}

/// Test workflow modal component
#[derive(Props, Clone, PartialEq)]
struct TestWorkflowModalProps {
    slot: String,
    test_prompt: String,
    is_testing: bool,
    test_result: Option<WorkflowTestResult>,
    test_error: Option<String>,
    on_prompt_change: EventHandler<String>,
    on_test: EventHandler<()>,
    on_close: EventHandler<()>,
}

#[component]
fn TestWorkflowModal(props: TestWorkflowModalProps) -> Element {
    let has_result = props.test_result.is_some();

    rsx! {
        div {
            class: "modal-backdrop fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl w-[90%] max-w-[700px] max-h-[80vh] flex flex-col overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between px-6 py-4 border-b border-gray-700",

                    h2 {
                        class: "text-white text-xl m-0",
                        "Test Workflow"
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-none text-gray-500 text-2xl cursor-pointer p-1",
                        "×"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-6",

                    if let Some(err) = &props.test_error {
                        div {
                            class: "py-3 px-4 bg-red-500 bg-opacity-10 border border-red-500 rounded-lg text-red-500 mb-4 text-sm",
                            "{err}"
                        }
                    }

                    if !has_result {
                        div {
                            class: "flex flex-col gap-4",

                            div {
                                label {
                                    class: "block text-gray-400 text-sm mb-2",
                                    "Test Prompt"
                                }
                                p {
                                    class: "text-gray-500 text-xs mb-2",
                                    "Enter a test prompt to generate an image and verify the workflow is working correctly."
                                }
                                textarea {
                                    value: "{props.test_prompt}",
                                    oninput: move |e| props.on_prompt_change.call(e.value()),
                                    placeholder: "Enter your test prompt here...",
                                    disabled: props.is_testing,
                                    class: "w-full h-[120px] p-3 bg-dark-bg border border-gray-700 rounded-lg text-white font-sans text-sm resize-y box-border",
                                }
                            }
                        }
                    } else if let Some(result) = &props.test_result {
                        div {
                            class: "flex flex-col gap-4",

                            // Success message
                            div {
                                class: "flex gap-4 py-3 px-4 bg-green-500 bg-opacity-10 rounded-lg border border-green-500",

                                div { class: "text-green-500 text-2xl", "✓" }

                                div {
                                    div { class: "text-green-500 text-sm font-medium", "Generation Successful" }
                                    div { class: "text-gray-400 text-xs mt-1", "Time: {result.duration_ms}ms" }
                                }
                            }

                            // Generated image
                            div {
                                h3 { class: "text-white text-sm m-0 mb-2", "Generated Image" }

                                img {
                                    src: "{result.image_url}",
                                    class: "w-full rounded-lg border border-gray-700 bg-dark-bg",
                                }
                            }

                            // Prompt display
                            div {
                                h3 { class: "text-white text-sm m-0 mb-2", "Test Prompt" }

                                div {
                                    class: "p-3 bg-dark-bg border border-gray-700 rounded-lg text-gray-400 text-sm break-words",
                                    "{props.test_prompt}"
                                }
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "flex justify-end gap-3 px-6 py-4 border-t border-gray-700",

                    button {
                        onclick: move |_| props.on_close.call(()),
                        disabled: props.is_testing,
                        class: "px-4 py-2 bg-gray-700 text-white border-none rounded-lg cursor-pointer text-sm",
                        "Close"
                    }

                    if !has_result {
                        button {
                            onclick: move |_| props.on_test.call(()),
                            disabled: props.is_testing || props.test_prompt.is_empty(),
                            class: "px-6 py-2 bg-purple-500 text-white border-none rounded-lg cursor-pointer text-sm font-medium",
                            if props.is_testing { "Generating..." } else { "Generate" }
                        }
                    } else {
                        button {
                            onclick: move |_| {
                                // Reset to test again
                                // This is handled by the parent component
                            },
                            class: "px-6 py-2 bg-purple-500 text-white border-none rounded-lg cursor-pointer text-sm font-medium",
                            "Test Again"
                        }
                    }
                }
            }
        }
    }
}
