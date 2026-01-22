//! Prompt Template Editor Component
//!
//! Settings UI for viewing and editing LLM prompt template overrides.
//! Focuses on dialogue.response_format as specified in US-DLG-010.

use crate::application::services::prompt_template_service::{
    use_prompt_template_service, ResolvedPromptTemplate, SavePromptTemplateRequest,
};
use crate::infrastructure::spawn_task;
use dioxus::prelude::*;

/// Props for PromptTemplateEditor component
#[derive(Props, Clone, PartialEq)]
pub struct PromptTemplateEditorProps {
    /// World ID for world-specific overrides
    pub world_id: String,
    /// Template key to edit (default: dialogue.response_format)
    #[props(default = "dialogue.response_format".to_string())]
    pub template_key: String,
}

/// Prompt template override editor component
///
/// Displays the resolved template value and allows editing a world-specific override.
/// Provides save and reset buttons for managing overrides.
#[component]
pub fn PromptTemplateEditor(props: PromptTemplateEditorProps) -> Element {
    let service = use_prompt_template_service();

    // State hooks at component root (CRITICAL: all hooks must be unconditional)
    let template_value = use_signal(String::new);
    let is_override = use_signal(|| false);
    let default_value = use_signal(String::new);

    let is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut success_message = use_signal(|| Option::<String>::None);

    // Clone for handlers - clone BEFORE moving into closures
    let world_id = props.world_id.clone();
    let world_id_for_save = world_id.clone();
    let world_id_for_reset = world_id.clone();
    let template_key = props.template_key.clone();
    let template_key_for_save = template_key.clone();
    let template_key_for_reset = template_key.clone();
    let mut template_value_for_change = template_value.clone();
    let world_id_for_effect = world_id.clone();
    let service_for_effect = service.clone();
    let service_for_save = service.clone();
    let service_for_reset = service.clone();

    // Load template on mount
    use_effect(move || {
        let world_id = world_id_for_effect.clone();
        let template_key = template_key.clone();
        let svc = service_for_effect.clone();
        // Clone signals into async closure (Dioxus: cloned signals are read-write)
        let mut template_value_clone = template_value.clone();
        let mut is_override_clone = is_override.clone();
        let mut default_value_clone = default_value.clone();
        let mut is_loading_clone = is_loading.clone();
        let mut error_clone = error.clone();

        spawn_task(async move {
            match svc.get_template(&world_id, &template_key).await {
                Ok(resolved) => {
                    template_value_clone.set(resolved.value);
                    is_override_clone.set(resolved.is_override);
                    default_value_clone.set(resolved.default_value);
                    is_loading_clone.set(false);
                }
                Err(e) => {
                    // If endpoint doesn't exist (404), show helpful message
                    error_clone.set(Some(format!("Backend endpoint not available: {}", e)));
                    is_loading_clone.set(false);
                }
            }
        });
    });

    // Event handlers (closures capturing signals)
    let handle_save = move |_| {
        let world_id = world_id_for_save.clone();
        let template_key = template_key_for_save.clone();
        let _current_value = template_value.read().clone();
        let svc = service_for_save.clone();
        // Clone signals into async closure (Dioxus: cloned signals are read-write)
        let mut template_value_clone = template_value.clone();
        let mut is_override_clone = is_override.clone();
        let mut is_saving_clone = is_saving.clone();
        let mut error_clone = error.clone();
        let mut success_message_clone = success_message.clone();

        is_saving.set(true);
        error.set(None);
        success_message.set(None);

        spawn_task(async move {
            match svc
                .save_template(
                    &world_id,
                    &template_key,
                    SavePromptTemplateRequest {
                        value: _current_value,
                    },
                )
                .await
            {
                Ok(resolved) => {
                    // Use type annotation to avoid unused import warning
                    let _resolved: ResolvedPromptTemplate = resolved;
                    template_value_clone.set(_resolved.value);
                    is_override_clone.set(true);
                    is_saving_clone.set(false);
                    success_message_clone.set(Some("Override saved successfully".to_string()));
                }
                Err(e) => {
                    error_clone.set(Some(format!("Failed to save: {}", e)));
                    is_saving_clone.set(false);
                }
            }
        });
    };

    let handle_reset = move |_| {
        let world_id = world_id_for_reset.clone();
        let template_key = template_key_for_reset.clone();
        let _default = default_value.read().clone();
        let svc = service_for_reset.clone();
        // Clone signals into async closure (Dioxus: cloned signals are read-write)
        let mut template_value_clone = template_value.clone();
        let mut is_override_clone = is_override.clone();
        let mut is_saving_clone = is_saving.clone();
        let mut error_clone = error.clone();
        let mut success_message_clone = success_message.clone();
        let mut default_value_clone = default_value.clone();

        is_saving.set(true);
        error.set(None);
        success_message.set(None);

        spawn_task(async move {
            match svc.reset_template(&world_id, &template_key).await {
                Ok(resolved) => {
                    // Use type annotation to avoid unused import warning
                    let _resolved: ResolvedPromptTemplate = resolved;
                    template_value_clone.set(_resolved.value);
                    is_override_clone.set(_resolved.is_override);
                    default_value_clone.set(_resolved.default_value);
                    is_saving_clone.set(false);
                    success_message_clone.set(Some("Reset to default".to_string()));
                }
                Err(e) => {
                    error_clone.set(Some(format!("Failed to reset: {}", e)));
                    is_saving_clone.set(false);
                }
            }
        });
    };

    let handle_value_change = move |e: FormEvent| {
        template_value_for_change.set(e.value());
    };

    // Render
    rsx! {
        div {
            class: "prompt-template-editor flex flex-col gap-4",

            // Header with template info
            div {
                class: "flex items-center justify-between",

                div {
                    class: "flex flex-col",

                    h3 {
                        class: "text-white font-semibold m-0",
                        "dialogue.response_format"
                    }

                    p {
                        class: "text-gray-400 text-sm m-0",
                        "Instructions shown to the LLM for how to format NPC dialogue responses"
                    }
                }

                // Override badge
                if *is_override.read() {
                    span {
                        class: "px-2 py-1 bg-amber-500 bg-opacity-20 text-amber-400 text-xs rounded",
                        "World Override"
                    }
                }
            }

            // Error message
            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-red-500 bg-opacity-10 text-red-500 text-sm rounded-md",
                    "{err}"
                }
            }

            // Success message
            if let Some(msg) = success_message.read().as_ref() {
                div {
                    class: "p-3 bg-green-500 bg-opacity-10 text-green-500 text-sm rounded-md",
                    "{msg}"
                }
            }

            // Loading state
            if *is_loading.read() {
                div {
                    class: "text-center text-gray-500 py-8",
                    "Loading template..."
                }
            } else {
                // Editor
                div {
                    class: "flex-1 flex flex-col gap-4",

                    // Textarea editor
                    div {
                        class: "flex-1 flex flex-col gap-2",

                        div {
                            class: "flex items-center justify-between",

                            label {
                                class: "text-gray-400 text-sm",
                                "Template Value"
                            }

                            span {
                                class: "text-gray-600 text-xs",
                                "{template_value.read().len()} characters"
                            }
                        }

                        textarea {
                            class: "w-full flex-1 bg-dark-bg text-white border border-gray-700 rounded p-3 font-mono text-sm resize-none",
                            rows: 20,
                            value: "{&template_value.read()}",
                            oninput: handle_value_change,
                        }
                    }

                    // Action buttons
                    div {
                        class: "flex items-center gap-3",

                        // Save button
                        button {
                            onclick: handle_save,
                            disabled: *is_saving.read(),
                            class: if *is_saving.read() {
                                "px-4 py-2 bg-blue-600 bg-opacity-50 text-white border-0 rounded cursor-not-allowed text-sm font-medium"
                            } else {
                                "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white border-0 rounded cursor-pointer text-sm font-medium transition-colors"
                            },
                            if *is_saving.read() { "Saving..." } else { "Save Override" }
                        }

                        // Reset button (only show if there's an override)
                        if *is_override.read() {
                            button {
                                onclick: handle_reset,
                                disabled: *is_saving.read(),
                                class: if *is_saving.read() {
                                    "px-4 py-2 bg-gray-700 bg-opacity-50 text-gray-400 border-0 rounded cursor-not-allowed text-sm font-medium"
                                } else {
                                    "px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white border-0 rounded cursor-pointer text-sm font-medium transition-colors"
                                },
                                "Reset to Default"
                            }
                        }

                        // Spacer
                        div {
                            class: "flex-1"
                        }

                        // Help text
                        span {
                            class: "text-gray-500 text-xs",
                            "Changes apply to this world only. Resetting restores the global default."
                        }
                    }
                }
            }
        }
    }
}
