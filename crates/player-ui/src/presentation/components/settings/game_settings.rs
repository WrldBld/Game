//! Game Settings Panel - Per-world settings management
//!
//! This component provides a form interface for viewing and editing
//! world-specific settings. It's designed for use during active gameplay
//! where DMs can tune settings for the current world/session.

use crate::presentation::services::use_settings_service;
use dioxus::prelude::*;
use wrldbldr_player_app::application::dto::{AppSettings, BatchQueueFailurePolicy};

/// Props for the Game Settings Panel
#[derive(Props, Clone, PartialEq)]
pub struct GameSettingsPanelProps {
    /// The world ID for per-world settings
    pub world_id: String,
}

/// Game Settings Panel component for per-world settings
///
/// Loads settings for a specific world and allows editing. Settings are
/// saved per-world, overriding global defaults.
#[component]
pub fn GameSettingsPanel(props: GameSettingsPanelProps) -> Element {
    let settings_service = use_settings_service();

    // State for the form fields
    let mut settings = use_signal(AppSettings::default);
    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    // Clone for closures
    let world_id_for_load = props.world_id.clone();
    let world_id_for_save = props.world_id.clone();
    let world_id_for_reset = props.world_id.clone();
    let service_for_load = settings_service.clone();
    let service_for_save = settings_service.clone();
    let service_for_reset = settings_service.clone();

    // Load settings on mount or world_id change
    use_effect(move || {
        let svc = service_for_load.clone();
        let wid = world_id_for_load.clone();
        spawn(async move {
            is_loading.set(true);
            error.set(None);

            match svc.get_for_world(&wid).await {
                Ok(loaded_settings) => {
                    settings.set(loaded_settings);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load world settings: {}", e)));
                    is_loading.set(false);
                }
            }
        });
    });

    // Handler for saving settings
    let handle_save = move |_| {
        let svc = service_for_save.clone();
        let wid = world_id_for_save.clone();
        let current_settings = settings.read().clone();
        spawn(async move {
            is_saving.set(true);
            error.set(None);
            success_message.set(None);

            match svc.update_for_world(&wid, &current_settings).await {
                Ok(updated_settings) => {
                    settings.set(updated_settings);
                    success_message.set(Some("World settings saved!".to_string()));
                    is_saving.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to save settings: {}", e)));
                    is_saving.set(false);
                }
            }
        });
    };

    // Handler for resetting to global defaults
    let handle_reset = move |_| {
        let svc = service_for_reset.clone();
        let wid = world_id_for_reset.clone();
        spawn(async move {
            is_saving.set(true);
            error.set(None);
            success_message.set(None);

            match svc.reset_for_world(&wid).await {
                Ok(reset_settings) => {
                    settings.set(reset_settings);
                    success_message.set(Some("Reset to global defaults!".to_string()));
                    is_saving.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to reset settings: {}", e)));
                    is_saving.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "game-settings-panel h-full flex flex-col",

            // Header
            div {
                class: "flex justify-between items-center mb-4",

                div {
                    h2 {
                        class: "text-white text-xl font-medium mb-1",
                        "World Settings"
                    }
                    p {
                        class: "text-gray-500 text-sm",
                        "Configure settings for this world. These override global defaults."
                    }
                }

                div {
                    class: "flex gap-2",

                    button {
                        class: "px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm",
                        onclick: handle_reset,
                        disabled: *is_loading.read() || *is_saving.read(),
                        "Reset to Global"
                    }

                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm",
                        onclick: handle_save,
                        disabled: *is_loading.read() || *is_saving.read(),
                        if *is_saving.read() { "Saving..." } else { "Save" }
                    }
                }
            }

            // Success/Error messages
            if let Some(msg) = success_message.read().as_ref() {
                div {
                    class: "mb-4 p-3 bg-green-900 bg-opacity-30 text-green-400 rounded-md text-sm",
                    "{msg}"
                }
            }

            if let Some(err) = error.read().as_ref() {
                div {
                    class: "mb-4 p-3 bg-red-900 bg-opacity-30 text-red-400 rounded-md text-sm",
                    "{err}"
                }
            }

            // Loading state
            if *is_loading.read() {
                div {
                    class: "flex-1 flex items-center justify-center text-gray-400",
                    "Loading world settings..."
                }
            } else {
                // Settings form - focused on gameplay-relevant settings
                div {
                    class: "flex-1 overflow-y-auto bg-gray-900 rounded-lg p-4 space-y-6",

                    // Conversation Settings
                    SettingsSection {
                        title: "Conversation",
                        description: "LLM context and conversation history settings",

                        NumberField {
                            label: "LLM Context History Turns",
                            description: "Number of conversation turns to include in LLM prompts",
                            value: settings.read().conversation_history_turns,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.conversation_history_turns = val);
                                success_message.set(None);
                            }
                        }

                        NumberField {
                            label: "Max Stored Turns",
                            description: "Maximum conversation turns to keep in session memory",
                            value: settings.read().max_conversation_turns,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.max_conversation_turns = val);
                                success_message.set(None);
                            }
                        }
                    }

                    // Challenge Settings
                    SettingsSection {
                        title: "Challenges",
                        description: "Challenge resolution and outcome generation",

                        BoundedNumberField {
                            label: "Outcome Branches",
                            description: "Options generated per challenge result",
                            value: settings.read().outcome_branch_count,
                            min: settings.read().outcome_branch_min,
                            max: settings.read().outcome_branch_max,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.outcome_branch_count = val);
                                success_message.set(None);
                            }
                        }

                        NumberField {
                            label: "Tokens per Branch",
                            description: "Max tokens when generating LLM suggestions",
                            value: settings.read().suggestion_tokens_per_branch as usize,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.suggestion_tokens_per_branch = val as u32);
                                success_message.set(None);
                            }
                        }
                    }

                    // Asset Generation Settings
                    SettingsSection {
                        title: "Asset Generation",
                        description: "Behavior when queueing batch prompts",

                        SelectField {
                            label: "Batch Queue Failure Policy",
                            description: "All-or-nothing fails the batch on any queue error; best-effort continues and only fails if nothing queued",
                            value: match settings.read().batch_queue_failure_policy {
                                BatchQueueFailurePolicy::AllOrNothing => "all_or_nothing",
                                BatchQueueFailurePolicy::BestEffort => "best_effort",
                            },
                            options: vec![
                                ("all_or_nothing", "All-or-nothing"),
                                ("best_effort", "Best effort"),
                            ],
                            onchange: move |val: String| {
                                let parsed = match val.as_str() {
                                    "best_effort" => BatchQueueFailurePolicy::BestEffort,
                                    _ => BatchQueueFailurePolicy::AllOrNothing,
                                };
                                settings.with_mut(|s| s.batch_queue_failure_policy = parsed);
                                success_message.set(None);
                            }
                        }
                    }

                    // Animation Settings
                    SettingsSection {
                        title: "Text Animation",
                        description: "Typewriter effect timing",

                        NumberField {
                            label: "Character Delay (ms)",
                            description: "Delay between characters",
                            value: settings.read().typewriter_char_delay_ms as usize,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.typewriter_char_delay_ms = val as u64);
                                success_message.set(None);
                            }
                        }

                        NumberField {
                            label: "Sentence Delay (ms)",
                            description: "Delay after completing a sentence",
                            value: settings.read().typewriter_sentence_delay_ms as usize,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.typewriter_sentence_delay_ms = val as u64);
                                success_message.set(None);
                            }
                        }
                    }

                    // LLM Context Budget (collapsible for advanced users)
                    CollapsibleSettingsSection {
                        title: "Advanced: LLM Token Budgets",
                        description: "Fine-tune context allocation for LLM prompts",
                        initially_open: false,

                        NumberField {
                            label: "Total Budget",
                            description: "Maximum total tokens for system prompt",
                            value: settings.read().context_budget.total_budget_tokens,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.context_budget.total_budget_tokens = val);
                                success_message.set(None);
                            }
                        }

                        NumberField {
                            label: "Scene Context",
                            description: "Tokens for scene/location details",
                            value: settings.read().context_budget.scene_tokens,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.context_budget.scene_tokens = val);
                                success_message.set(None);
                            }
                        }

                        NumberField {
                            label: "Character Context",
                            description: "Tokens for NPC personality/motivations",
                            value: settings.read().context_budget.character_tokens,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.context_budget.character_tokens = val);
                                success_message.set(None);
                            }
                        }

                        NumberField {
                            label: "History Tokens",
                            description: "Tokens for conversation history",
                            value: settings.read().context_budget.conversation_history_tokens,
                            onchange: move |val: usize| {
                                settings.with_mut(|s| s.context_budget.conversation_history_tokens = val);
                                success_message.set(None);
                            }
                        }

                        BooleanField {
                            label: "Auto-Summarization",
                            description: "Summarize when over budget",
                            value: settings.read().context_budget.enable_summarization,
                            onchange: move |val: bool| {
                                settings.with_mut(|s| s.context_budget.enable_summarization = val);
                                success_message.set(None);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Settings section component - groups related settings
#[derive(Props, Clone, PartialEq)]
struct SettingsSectionProps {
    title: &'static str,
    description: &'static str,
    children: Element,
}

#[component]
fn SettingsSection(props: SettingsSectionProps) -> Element {
    rsx! {
        div {
            class: "settings-section",

            div {
                class: "mb-3",

                h3 {
                    class: "text-white font-medium text-base mb-1",
                    "{props.title}"
                }

                p {
                    class: "text-gray-500 text-xs",
                    "{props.description}"
                }
            }

            div {
                class: "space-y-3",
                {props.children}
            }
        }
    }
}

/// Collapsible settings section for advanced options
#[derive(Props, Clone, PartialEq)]
struct CollapsibleSettingsSectionProps {
    title: &'static str,
    description: &'static str,
    #[props(default = false)]
    initially_open: bool,
    children: Element,
}

#[component]
fn CollapsibleSettingsSection(props: CollapsibleSettingsSectionProps) -> Element {
    let mut is_open = use_signal(|| props.initially_open);

    let current_open = *is_open.read();

    rsx! {
        div {
            class: "settings-section border border-gray-700 rounded-lg",

            // Header (clickable to toggle)
            button {
                class: "w-full flex justify-between items-center p-3 text-left hover:bg-gray-800 rounded-t-lg transition-colors",
                onclick: move |_| is_open.set(!current_open),

                div {
                    h3 {
                        class: "text-gray-400 font-medium text-sm mb-0.5",
                        "{props.title}"
                    }

                    p {
                        class: "text-gray-600 text-xs",
                        "{props.description}"
                    }
                }

                // Chevron indicator
                span {
                    class: "text-gray-500 text-lg transition-transform",
                    style: if current_open { "transform: rotate(180deg)" } else { "" },
                    "▼"
                }
            }

            // Content (collapsible)
            if current_open {
                div {
                    class: "p-3 pt-0 space-y-3 border-t border-gray-700",
                    {props.children}
                }
            }
        }
    }
}

/// Number input field component
#[derive(Props, Clone, PartialEq)]
struct NumberFieldProps {
    label: &'static str,
    description: &'static str,
    value: usize,
    onchange: EventHandler<usize>,
}

/// Simple select field component
#[derive(Props, Clone, PartialEq)]
struct SelectFieldProps {
    label: &'static str,
    description: &'static str,
    value: &'static str,
    options: Vec<(&'static str, &'static str)>,
    onchange: EventHandler<String>,
}

#[component]
fn SelectField(props: SelectFieldProps) -> Element {
    rsx! {
        div {
            class: "select-field",

            label {
                class: "block",

                div {
                    class: "flex justify-between items-baseline mb-1",

                    span {
                        class: "text-gray-300 text-xs font-medium",
                        "{props.label}"
                    }

                    span {
                        class: "text-gray-600 text-xs",
                        "{props.description}"
                    }
                }

                select {
                    class: "w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500",
                    value: "{props.value}",
                    onchange: move |evt| props.onchange.call(evt.value()),

                    for (opt_value, opt_label) in props.options.iter() {
                        option {
                            value: "{opt_value}",
                            "{opt_label}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NumberField(props: NumberFieldProps) -> Element {
    let value_str = format!("{}", props.value);

    rsx! {
        div {
            class: "number-field flex items-center gap-3",

            div {
                class: "flex-1",

                span {
                    class: "text-gray-300 text-sm",
                    "{props.label}"
                }

                span {
                    class: "text-gray-600 text-xs ml-2",
                    "({props.description})"
                }
            }

            input {
                r#type: "number",
                class: "w-24 px-2 py-1 bg-gray-800 border border-gray-700 rounded text-white text-sm focus:outline-none focus:ring-1 focus:ring-blue-500",
                value: "{value_str}",
                oninput: move |evt| {
                    if let Ok(val) = evt.value().parse::<usize>() {
                        props.onchange.call(val);
                    }
                }
            }
        }
    }
}

/// Bounded number input field component
#[derive(Props, Clone, PartialEq)]
struct BoundedNumberFieldProps {
    label: &'static str,
    description: &'static str,
    value: usize,
    min: usize,
    max: usize,
    onchange: EventHandler<usize>,
}

#[component]
fn BoundedNumberField(props: BoundedNumberFieldProps) -> Element {
    let value_str = format!("{}", props.value);
    let min_str = format!("{}", props.min);
    let max_str = format!("{}", props.max);

    rsx! {
        div {
            class: "number-field flex items-center gap-3",

            div {
                class: "flex-1",

                span {
                    class: "text-gray-300 text-sm",
                    "{props.label}"
                }

                span {
                    class: "text-gray-600 text-xs ml-2",
                    "({props.description}, {props.min}-{props.max})"
                }
            }

            input {
                r#type: "number",
                class: "w-24 px-2 py-1 bg-gray-800 border border-gray-700 rounded text-white text-sm focus:outline-none focus:ring-1 focus:ring-blue-500",
                value: "{value_str}",
                min: "{min_str}",
                max: "{max_str}",
                oninput: move |evt| {
                    if let Ok(val) = evt.value().parse::<usize>() {
                        let clamped = val.clamp(props.min, props.max);
                        props.onchange.call(clamped);
                    }
                }
            }
        }
    }
}

/// Boolean toggle field component
#[derive(Props, Clone, PartialEq)]
struct BooleanFieldProps {
    label: &'static str,
    description: &'static str,
    value: bool,
    onchange: EventHandler<bool>,
}

#[component]
fn BooleanField(props: BooleanFieldProps) -> Element {
    rsx! {
        div {
            class: "boolean-field flex items-center gap-3",

            div {
                class: "flex-1",

                span {
                    class: "text-gray-300 text-sm",
                    "{props.label}"
                }

                span {
                    class: "text-gray-600 text-xs ml-2",
                    "({props.description})"
                }
            }

            label {
                class: "relative inline-flex items-center cursor-pointer",

                input {
                    r#type: "checkbox",
                    class: "sr-only peer",
                    checked: props.value,
                    onchange: move |evt| {
                        props.onchange.call(evt.checked());
                    }
                }

                div {
                    class: "w-9 h-5 bg-gray-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600"
                }
            }
        }
    }
}
