//! Workflow Upload Modal Component
//!
//! Modal for uploading or pasting a ComfyUI workflow JSON,
//! analyzing it, and configuring prompt mappings.

use dioxus::prelude::*;

use crate::presentation::services::use_workflow_service;
use crate::application::services::AnalyzeWorkflowResponse;

/// Props for the WorkflowUploadModal component
#[derive(Props, Clone, PartialEq)]
pub struct WorkflowUploadModalProps {
    /// The slot to configure
    pub slot: String,
    /// Callback when modal is closed
    pub on_close: EventHandler<()>,
    /// Callback when workflow is saved
    pub on_save: EventHandler<()>,
}

/// Workflow analysis result
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkflowAnalysisResult {
    pub is_valid: bool,
    pub node_count: usize,
    pub input_count: usize,
    pub text_inputs: Vec<TextInputInfo>,
    pub suggested_primary: Option<TextInputInfo>,
    pub suggested_negative: Option<TextInputInfo>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputInfo {
    pub node_id: String,
    pub node_title: Option<String>,
    pub input_name: String,
}

/// Upload modal wizard step
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum UploadStep {
    #[default]
    Upload,
    Configure,
    Review,
}

/// Workflow upload modal
#[component]
pub fn WorkflowUploadModal(props: WorkflowUploadModalProps) -> Element {
    let workflow_service = use_workflow_service();

    // Track wizard step
    let mut current_step = use_signal(|| UploadStep::Upload);
    // Store the workflow name
    let mut workflow_name = use_signal(|| "".to_string());
    // Store the pasted JSON
    let mut workflow_json = use_signal(|| "".to_string());
    // Store analysis result
    let mut analysis: Signal<Option<WorkflowAnalysisResult>> = use_signal(|| None);
    // Track loading state
    let mut is_analyzing = use_signal(|| false);
    let mut is_saving = use_signal(|| false);
    // Track error
    let mut error: Signal<Option<String>> = use_signal(|| None);
    // Store selected mappings
    let mut primary_mapping: Signal<Option<TextInputInfo>> = use_signal(|| None);
    let mut negative_mapping: Signal<Option<TextInputInfo>> = use_signal(|| None);

    let workflow_service_for_analyze = workflow_service.clone();
    // Analyze workflow
    let do_analyze = move |_| {
        let json_text = workflow_json.read().clone();
        let svc = workflow_service_for_analyze.clone();

        spawn(async move {
            is_analyzing.set(true);
            error.set(None);

            // Parse the JSON first
            let workflow_json_value = match serde_json::from_str::<serde_json::Value>(&json_text) {
                Ok(v) => v,
                Err(e) => {
                    error.set(Some(format!("Invalid JSON: {}", e)));
                    is_analyzing.set(false);
                    return;
                }
            };

            match svc.analyze_workflow(workflow_json_value).await {
                Ok(analysis_result) => {
                    // Convert to local type and auto-select suggested mappings
                    let result = WorkflowAnalysisResult::from(analysis_result);
                    primary_mapping.set(result.suggested_primary.clone());
                    negative_mapping.set(result.suggested_negative.clone());
                    analysis.set(Some(result));
                    current_step.set(UploadStep::Configure);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }

            is_analyzing.set(false);
        });
    };

    // Save workflow configuration
    let slot_for_save = props.slot.clone();
    let on_save_handler = props.on_save.clone();
    let workflow_service_for_save = workflow_service.clone();
    let do_save = move |_| {
        let json_text = workflow_json.read().clone();
        let name = workflow_name.read().clone();
        let primary = primary_mapping.read().clone();
        let negative = negative_mapping.read().clone();
        let slot = slot_for_save.clone();
        let on_save = on_save_handler.clone();
        let svc = workflow_service_for_save.clone();

        spawn(async move {
            is_saving.set(true);
            error.set(None);

            // Parse JSON
            let workflow_json_value = match serde_json::from_str::<serde_json::Value>(&json_text) {
                Ok(v) => v,
                Err(e) => {
                    error.set(Some(format!("Invalid JSON: {}", e)));
                    is_saving.set(false);
                    return;
                }
            };

            // Build prompt mappings
            let mut prompt_mappings = Vec::new();
            if let Some(p) = primary {
                prompt_mappings.push(serde_json::json!({
                    "node_id": p.node_id,
                    "input_name": p.input_name,
                    "mapping_type": "primary"
                }));
            }
            if let Some(n) = negative {
                prompt_mappings.push(serde_json::json!({
                    "node_id": n.node_id,
                    "input_name": n.input_name,
                    "mapping_type": "negative"
                }));
            }

            match svc.save_workflow_config(&slot, &name, workflow_json_value, prompt_mappings, vec![], vec![]).await {
                Ok(_) => {
                    on_save.call(());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }

            is_saving.set(false);
        });
    };

    rsx! {
        // Modal backdrop
        div {
            class: "modal-backdrop fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),

            // Modal content
            div {
                class: "modal-content bg-dark-surface rounded-xl w-11/12 max-w-2xl max-h-screen-80 flex flex-col overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between py-4 px-6 border-b border-gray-700",

                    div {
                        h2 {
                            class: "text-white text-xl m-0",
                            "Configure Workflow"
                        }
                        p {
                            class: "text-gray-500 text-sm mt-1 mb-0",
                            "Slot: {props.slot}"
                        }
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-0 text-gray-500 text-2xl cursor-pointer p-1",
                        "×"
                    }
                }

                // Progress indicator
                div {
                    class: "flex gap-2 py-4 px-6 bg-black bg-opacity-20",

                    StepIndicator {
                        number: 1,
                        label: "Upload",
                        is_active: *current_step.read() == UploadStep::Upload,
                        is_complete: *current_step.read() != UploadStep::Upload,
                    }
                    StepIndicator {
                        number: 2,
                        label: "Configure",
                        is_active: *current_step.read() == UploadStep::Configure,
                        is_complete: *current_step.read() == UploadStep::Review,
                    }
                    StepIndicator {
                        number: 3,
                        label: "Review",
                        is_active: *current_step.read() == UploadStep::Review,
                        is_complete: false,
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-6",

                    // Error display
                    if let Some(err) = error.read().as_ref() {
                        div {
                            class: "py-3 px-4 bg-red-500 bg-opacity-10 border border-red-500 rounded-lg text-red-500 mb-4",
                            "{err}"
                        }
                    }

                    match *current_step.read() {
                        UploadStep::Upload => rsx! {
                            UploadStepContent {
                                workflow_name: workflow_name.read().clone(),
                                workflow_json: workflow_json.read().clone(),
                                on_name_change: move |name| workflow_name.set(name),
                                on_json_change: move |json| workflow_json.set(json),
                            }
                        },

                        UploadStep::Configure => rsx! {
                            if let Some(result) = analysis.read().as_ref() {
                                ConfigureStepContent {
                                    analysis: result.clone(),
                                    primary_mapping: primary_mapping.read().clone(),
                                    negative_mapping: negative_mapping.read().clone(),
                                    on_primary_change: move |mapping| primary_mapping.set(mapping),
                                    on_negative_change: move |mapping| negative_mapping.set(mapping),
                                }
                            }
                        },

                        UploadStep::Review => rsx! {
                            ReviewStepContent {
                                workflow_name: workflow_name.read().clone(),
                                analysis: analysis.read().clone(),
                                primary_mapping: primary_mapping.read().clone(),
                                negative_mapping: negative_mapping.read().clone(),
                            }
                        },
                    }
                }

                // Footer
                div {
                    class: "flex justify-between py-4 px-6 border-t border-gray-700",

                    // Back button
                    {
                        let step = *current_step.read();
                        if step != UploadStep::Upload {
                            rsx! {
                                button {
                                    onclick: move |_| {
                                        let current = *current_step.read();
                                        match current {
                                            UploadStep::Configure => current_step.set(UploadStep::Upload),
                                            UploadStep::Review => current_step.set(UploadStep::Configure),
                                            _ => {}
                                        }
                                    },
                                    class: "py-2 px-4 bg-gray-700 text-white border-0 rounded-lg cursor-pointer",
                                    "← Back"
                                }
                            }
                        } else {
                            rsx! { div {} } // Spacer
                        }
                    }

                    // Next/Save button
                    {
                        let step = *current_step.read();
                        match step {
                            UploadStep::Upload => {
                                let analyzing = *is_analyzing.read();
                                let name_empty = workflow_name.read().is_empty();
                                let json_empty = workflow_json.read().is_empty();
                                rsx! {
                                    button {
                                        onclick: do_analyze,
                                        disabled: name_empty || json_empty || analyzing,
                                        class: "py-2 px-6 bg-blue-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                                        if analyzing { "Analyzing..." } else { "Analyze Workflow →" }
                                    }
                                }
                            },

                            UploadStep::Configure => rsx! {
                                button {
                                    onclick: move |_| current_step.set(UploadStep::Review),
                                    class: "py-2 px-6 bg-blue-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                                    "Review →"
                                }
                            },

                            UploadStep::Review => {
                                let saving = *is_saving.read();
                                rsx! {
                                    button {
                                        onclick: do_save,
                                        disabled: saving,
                                        class: "py-2 px-6 bg-green-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                                        if saving { "Saving..." } else { "Save Configuration" }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Step indicator component
#[component]
fn StepIndicator(number: u8, label: &'static str, is_active: bool, is_complete: bool) -> Element {
    let bg_class = if is_active || is_complete {
        "w-6 h-6 rounded-full bg-blue-500 flex items-center justify-center text-xs text-white font-semibold"
    } else {
        "w-6 h-6 rounded-full bg-gray-700 flex items-center justify-center text-xs text-white font-semibold"
    };
    let text_class = if is_active {
        "text-white text-sm"
    } else if is_complete {
        "text-gray-400 text-sm"
    } else {
        "text-gray-500 text-sm"
    };

    rsx! {
        div {
            class: "flex items-center gap-2",

            div {
                class: "{bg_class}",
                if is_complete { "✓" } else { "{number}" }
            }

            span {
                class: "{text_class}",
                "{label}"
            }

            if number < 3 {
                div {
                    class: "w-10 h-0.5 bg-gray-700 mx-2",
                }
            }
        }
    }
}

/// Upload step content
#[derive(Props, Clone, PartialEq)]
struct UploadStepContentProps {
    workflow_name: String,
    workflow_json: String,
    on_name_change: EventHandler<String>,
    on_json_change: EventHandler<String>,
}

#[component]
fn UploadStepContent(props: UploadStepContentProps) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-4",

            // Workflow name input
            div {
                label {
                    class: "block text-gray-400 text-sm mb-2",
                    "Workflow Name"
                }
                input {
                    r#type: "text",
                    value: "{props.workflow_name}",
                    oninput: move |e| props.on_name_change.call(e.value()),
                    placeholder: "e.g., SD1.5 Portrait Generator",
                    class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-base box-border",
                }
            }

            // JSON textarea
            div {
                label {
                    class: "block text-gray-400 text-sm mb-2",
                    "Workflow JSON (API Format)"
                }
                p {
                    class: "text-gray-500 text-xs mb-2",
                    "In ComfyUI, use 'Save (API Format)' from the menu to export the workflow in the correct format."
                }
                textarea {
                    value: "{props.workflow_json}",
                    oninput: move |e| props.on_json_change.call(e.value()),
                    placeholder: "Paste your ComfyUI workflow JSON here...",
                    class: "w-full h-75 p-3 bg-dark-bg border border-gray-700 rounded-lg text-white font-mono text-sm resize-y box-border",
                }
            }
        }
    }
}

/// Configure step content
#[derive(Props, Clone, PartialEq)]
struct ConfigureStepContentProps {
    analysis: WorkflowAnalysisResult,
    primary_mapping: Option<TextInputInfo>,
    negative_mapping: Option<TextInputInfo>,
    on_primary_change: EventHandler<Option<TextInputInfo>>,
    on_negative_change: EventHandler<Option<TextInputInfo>>,
}

#[component]
fn ConfigureStepContent(props: ConfigureStepContentProps) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-6",

            // Analysis summary
            div {
                class: "flex gap-4 p-4 bg-green-500 bg-opacity-10 rounded-lg",

                div {
                    class: "text-green-500 text-2xl",
                    "✓"
                }
                div {
                    h3 { class: "text-green-500 m-0 mb-1 text-base", "Valid Workflow" }
                    p { class: "text-gray-400 m-0 text-sm",
                        "{props.analysis.node_count} nodes, {props.analysis.input_count} configurable inputs"
                    }
                }
            }

            // Primary prompt mapping
            div {
                h4 { class: "text-white text-sm mb-2", "Primary Prompt Mapping" }
                p { class: "text-gray-500 text-xs mb-2",
                    "Select the text input that will receive the main generation prompt."
                }
                TextInputSelector {
                    inputs: props.analysis.text_inputs.clone(),
                    selected: props.primary_mapping.clone(),
                    on_select: props.on_primary_change.clone(),
                }
            }

            // Negative prompt mapping
            div {
                h4 { class: "text-white text-sm mb-2", "Negative Prompt Mapping (Optional)" }
                p { class: "text-gray-500 text-xs mb-2",
                    "Select the text input for negative prompts, if applicable."
                }
                TextInputSelector {
                    inputs: props.analysis.text_inputs.clone(),
                    selected: props.negative_mapping.clone(),
                    on_select: props.on_negative_change.clone(),
                }
            }
        }
    }
}

/// Text input selector dropdown
#[derive(Props, Clone, PartialEq)]
struct TextInputSelectorProps {
    inputs: Vec<TextInputInfo>,
    selected: Option<TextInputInfo>,
    on_select: EventHandler<Option<TextInputInfo>>,
}

#[component]
fn TextInputSelector(props: TextInputSelectorProps) -> Element {
    let selected_key = props.selected.as_ref().map(|i| format!("{}:{}", i.node_id, i.input_name));

    rsx! {
        select {
            value: selected_key.clone().unwrap_or_default(),
            onchange: move |e| {
                let value = e.value();
                if value.is_empty() {
                    props.on_select.call(None);
                } else {
                    let input = props.inputs.iter().find(|i| {
                        format!("{}:{}", i.node_id, i.input_name) == value
                    }).cloned();
                    props.on_select.call(input);
                }
            },
            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm",

            option { value: "", "(None)" }

            for input in props.inputs.iter() {
                {
                    let node_label = input.node_title.clone().unwrap_or_else(|| format!("Node {}", input.node_id));
                    let key = format!("{}:{}", input.node_id, input.input_name);
                    let display = format!("{} → {}", node_label, input.input_name);
                    rsx! {
                        option {
                            value: "{key}",
                            "{display}"
                        }
                    }
                }
            }
        }
    }
}

/// Review step content
#[derive(Props, Clone, PartialEq)]
struct ReviewStepContentProps {
    workflow_name: String,
    analysis: Option<WorkflowAnalysisResult>,
    primary_mapping: Option<TextInputInfo>,
    negative_mapping: Option<TextInputInfo>,
}

#[component]
fn ReviewStepContent(props: ReviewStepContentProps) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-4",

            h3 { class: "text-white m-0 text-lg", "Review Configuration" }

            div {
                class: "flex flex-col gap-3 p-4 bg-black bg-opacity-20 rounded-lg",

                ReviewRow { label: "Workflow Name", value: props.workflow_name.clone() }

                if let Some(analysis) = &props.analysis {
                    ReviewRow { label: "Node Count", value: analysis.node_count.to_string() }
                    ReviewRow { label: "Configurable Inputs", value: analysis.input_count.to_string() }
                }

                ReviewRow {
                    label: "Primary Prompt",
                    value: props.primary_mapping.as_ref()
                        .map(|m| format!("{} → {}", m.node_title.clone().unwrap_or_else(|| format!("Node {}", m.node_id)), m.input_name))
                        .unwrap_or_else(|| "(Not set)".to_string())
                }

                ReviewRow {
                    label: "Negative Prompt",
                    value: props.negative_mapping.as_ref()
                        .map(|m| format!("{} → {}", m.node_title.clone().unwrap_or_else(|| format!("Node {}", m.node_id)), m.input_name))
                        .unwrap_or_else(|| "(Not set)".to_string())
                }
            }

            p {
                class: "text-gray-500 text-sm",
                "Click 'Save Configuration' to save this workflow. You can edit input defaults after saving."
            }
        }
    }
}

#[component]
fn ReviewRow(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            class: "flex justify-between",

            span { class: "text-gray-400", "{label}" }
            span { class: "text-white", "{value}" }
        }
    }
}

impl From<AnalyzeWorkflowResponse> for WorkflowAnalysisResult {
    fn from(resp: AnalyzeWorkflowResponse) -> Self {
        let text_inputs: Vec<TextInputInfo> = resp.analysis.text_inputs.iter().map(|i| TextInputInfo {
            node_id: i.node_id.clone(),
            node_title: i.node_title.clone(),
            input_name: i.input_name.clone(),
        }).collect();

        let suggested_primary = resp.suggested_prompt_mappings.iter()
            .find(|m| m.mapping_type == "primary")
            .and_then(|m| {
                text_inputs.iter().find(|i| i.node_id == m.node_id && i.input_name == m.input_name).cloned()
            });

        let suggested_negative = resp.suggested_prompt_mappings.iter()
            .find(|m| m.mapping_type == "negative")
            .and_then(|m| {
                text_inputs.iter().find(|i| i.node_id == m.node_id && i.input_name == m.input_name).cloned()
            });

        Self {
            is_valid: resp.is_valid,
            node_count: resp.analysis.node_count,
            input_count: resp.analysis.inputs.len(),
            text_inputs,
            suggested_primary,
            suggested_negative,
            errors: resp.errors,
        }
    }
}
