//! Settings components - Application configuration interface
//!
//! Components for the Settings view, providing workflow configuration,
//! ComfyUI integration settings, skills management, and general application preferences.

pub mod app_settings;
pub mod game_settings;
pub mod skills_panel;
pub mod workflow_slot_list;
pub mod workflow_config_editor;
pub mod workflow_upload_modal;

// Re-export the game settings panel for easy access
pub use game_settings::GameSettingsPanel;

use dioxus::prelude::*;
use crate::routes::Route;

/// Props for SettingsView
#[derive(Props, Clone, PartialEq)]
pub struct SettingsViewProps {
    /// World ID for skills management and routing
    pub world_id: String,
    /// Selected sub-tab from route (workflows, skills)
    #[props(default)]
    pub selected_tab: Option<String>,
}

/// The main Settings container component with route-based tabs
#[component]
pub fn SettingsView(props: SettingsViewProps) -> Element {
    // Determine active tab from route, default to workflows
    let active_tab = props.selected_tab.as_deref().unwrap_or("workflows");

    rsx! {
        div {
            class: "settings-view h-full flex flex-col bg-dark-bg",

            // Tab bar using router Links
            div {
                class: "settings-tabs flex gap-1 py-3 px-4 bg-dark-surface border-b border-gray-700",

                SettingsTabLink {
                    label: "Asset Workflows",
                    subtab: "workflows",
                    world_id: props.world_id.clone(),
                    active: active_tab == "workflows",
                }
                SettingsTabLink {
                    label: "Skills Management",
                    subtab: "skills",
                    world_id: props.world_id.clone(),
                    active: active_tab == "skills",
                }
                SettingsTabLink {
                    label: "World Settings",
                    subtab: "world-settings",
                    world_id: props.world_id.clone(),
                    active: active_tab == "world-settings",
                }
                SettingsTabLink {
                    label: "App Settings",
                    subtab: "app-settings",
                    world_id: props.world_id.clone(),
                    active: active_tab == "app-settings",
                }
            }

            // Tab content
            div {
                class: "settings-content flex-1 overflow-hidden",

                match active_tab {
                    "skills" => rsx! {
                        SkillsManagementTab { world_id: props.world_id.clone() }
                    },
                    "world-settings" => rsx! {
                        div {
                            class: "p-4",
                            game_settings::GameSettingsPanel { world_id: props.world_id.clone() }
                        }
                    },
                    "app-settings" => rsx! {
                        app_settings::AppSettingsPanel {}
                    },
                    _ => rsx! {
                        AssetWorkflowsTab {}
                    },
                }
            }
        }
    }
}

/// Tab link component using router navigation
#[derive(Props, Clone, PartialEq)]
struct SettingsTabLinkProps {
    label: &'static str,
    subtab: &'static str,
    world_id: String,
    active: bool,
}

#[component]
fn SettingsTabLink(props: SettingsTabLinkProps) -> Element {
    let class_str = if props.active {
        "py-2 px-4 bg-blue-500 text-white border-0 rounded-md cursor-pointer text-sm font-medium transition-all no-underline"
    } else {
        "py-2 px-4 bg-transparent text-gray-400 border-0 rounded-md cursor-pointer text-sm font-normal transition-all no-underline"
    };

    rsx! {
        Link {
            to: Route::DMSettingsSubTabRoute {
                world_id: props.world_id.clone(),
                subtab: props.subtab.to_string(),
            },
            class: "{class_str}",
            "{props.label}"
        }
    }
}

/// Asset Workflows tab content (original SettingsView content)
#[component]
fn AssetWorkflowsTab() -> Element {
    // Track the selected workflow slot for editing
    let mut selected_slot: Signal<Option<String>> = use_signal(|| None);
    // Track whether the upload modal is open
    let mut show_upload_modal = use_signal(|| false);
    // Track which slot we're uploading for
    let mut upload_target_slot: Signal<Option<String>> = use_signal(|| None);

    rsx! {
        div {
            class: "asset-workflows-tab h-full grid gap-4 p-4",
            style: "grid-template-columns: 320px 1fr;",

            // Left panel - Workflow slots list
            div {
                class: "left-panel flex flex-col gap-4 overflow-hidden",

                workflow_slot_list::WorkflowSlotList {
                    selected_slot: selected_slot.read().clone(),
                    on_select: move |slot: String| selected_slot.set(Some(slot)),
                    on_configure: move |slot: String| {
                        upload_target_slot.set(Some(slot.clone()));
                        show_upload_modal.set(true);
                    },
                }
            }

            // Right panel - Configuration editor
            div {
                class: "editor-panel flex flex-col gap-4 overflow-hidden",

                if let Some(slot) = selected_slot.read().clone() {
                    workflow_config_editor::WorkflowConfigEditor {
                        slot: slot.clone(),
                        on_close: move |_| selected_slot.set(None),
                        on_reconfigure: move |_| {
                            upload_target_slot.set(Some(slot.clone()));
                            show_upload_modal.set(true);
                        },
                        on_deleted: move |_| {
                            // Deselect the slot and refresh the list
                            selected_slot.set(None);
                        },
                    }
                } else {
                    WorkflowEmptyStatePanel {}
                }
            }

            // Upload modal overlay
            if *show_upload_modal.read() {
                if let Some(slot) = upload_target_slot.read().clone() {
                    workflow_upload_modal::WorkflowUploadModal {
                        slot: slot.clone(),
                        on_close: move |_| {
                            show_upload_modal.set(false);
                            upload_target_slot.set(None);
                        },
                        on_save: move |_| {
                            show_upload_modal.set(false);
                            // Select the slot we just configured
                            selected_slot.set(upload_target_slot.read().clone());
                            upload_target_slot.set(None);
                        },
                    }
                }
            }
        }
    }
}

/// Skills Management tab content - embedded skills panel
#[derive(Props, Clone, PartialEq)]
struct SkillsManagementTabProps {
    world_id: String,
}

#[component]
fn SkillsManagementTab(props: SkillsManagementTabProps) -> Element {
    use wrldbldr_player_app::application::services::SkillCategory;
    use crate::presentation::services::use_skill_service;
    use std::collections::HashMap;

    let skill_service = use_skill_service();

    let mut skills: Signal<Vec<wrldbldr_player_app::application::services::SkillData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut show_hidden = use_signal(|| false);
    let mut show_add_form = use_signal(|| false);
    let _editing_skill: Signal<Option<String>> = use_signal(|| None);

    // Clone world_id for handlers
    let world_id = props.world_id.clone();
    let world_id_for_effect = world_id.clone();

    // Load skills on mount
    use_effect(move || {
        let world_id = world_id_for_effect.clone();
        let svc = skill_service.clone();
        spawn(async move {
            match svc.list_skills(&world_id).await {
                Ok(list) => {
                    skills.set(list);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    is_loading.set(false);
                }
            }
        });
    });

    // Group skills by category
    let skills_by_category: HashMap<SkillCategory, Vec<wrldbldr_player_app::application::services::SkillData>> = {
        let skills_read = skills.read();
        let show_hidden_val = *show_hidden.read();
        let mut grouped: HashMap<SkillCategory, Vec<wrldbldr_player_app::application::services::SkillData>> = HashMap::new();
        for skill in skills_read.iter() {
            if !skill.is_hidden || show_hidden_val {
                grouped.entry(skill.category).or_default().push(skill.clone());
            }
        }
        for skills_vec in grouped.values_mut() {
            skills_vec.sort_by_key(|s| s.order);
        }
        grouped
    };

    // Get categories that have skills
    let mut active_categories: Vec<SkillCategory> = skills_by_category.keys().cloned().collect();
    active_categories.sort_by_key(|c| match c {
        SkillCategory::Physical => 0,
        SkillCategory::Mental => 1,
        SkillCategory::Social => 2,
        SkillCategory::Interpersonal => 3,
        SkillCategory::Investigation => 4,
        SkillCategory::Academic => 5,
        SkillCategory::Practical => 6,
        SkillCategory::Combat => 7,
        SkillCategory::Approach => 8,
        SkillCategory::Aspect => 9,
        SkillCategory::Other => 10,
        SkillCategory::Custom => 11,
    });

    rsx! {
        div {
            class: "skills-management-tab h-full flex flex-col p-4",

            // Header with controls
            div {
                class: "flex justify-between items-center mb-4",

                h2 {
                    class: "text-white m-0 text-xl",
                    "Skills Management"
                }

                div {
                    class: "flex gap-4 items-center",

                    // Show hidden toggle
                    label {
                        class: "flex items-center gap-2 text-gray-400 text-sm cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: *show_hidden.read(),
                            onchange: move |e| show_hidden.set(e.checked()),
                        }
                        "Show Hidden"
                    }

                    // Add skill button
                    button {
                        onclick: move |_| show_add_form.set(true),
                        class: "py-2 px-4 bg-purple-500 text-white border-0 rounded-md cursor-pointer text-sm",
                        "+ Add Custom Skill"
                    }
                }
            }

            // Error message
            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-red-500 bg-opacity-10 text-red-500 text-sm rounded-md mb-4",
                    "{err}"
                }
            }

            // Content area
            div {
                class: "flex-1 overflow-y-auto bg-dark-surface rounded-lg p-4",

                if *is_loading.read() {
                    div {
                        class: "text-center text-gray-500 py-8",
                        "Loading skills..."
                    }
                } else if *show_add_form.read() {
                    // Add form would go here - simplified for now
                    div {
                        class: "p-4 bg-dark-bg rounded-lg",
                        p { class: "text-gray-400", "Add skill form (coming soon)" }
                        button {
                            onclick: move |_| show_add_form.set(false),
                            class: "py-2 px-4 bg-gray-700 text-white border-0 rounded cursor-pointer",
                            "Cancel"
                        }
                    }
                } else if skills.read().is_empty() {
                    div {
                        class: "text-center text-gray-500 py-8",
                        div { class: "text-5xl mb-4 opacity-50", "📚" }
                        p { class: "text-gray-400 mb-2", "No skills defined for this world." }
                        p { class: "text-sm", "Skills are loaded from the rule system preset." }
                    }
                } else {
                    // Skills by category
                    for category in active_categories.iter() {
                        {
                            let cat_skills = skills_by_category.get(category).cloned().unwrap_or_default();
                            if !cat_skills.is_empty() {
                                rsx! {
                                    div {
                                        key: "{category:?}",
                                        class: "mb-6",

                                        h3 {
                                            class: "text-gray-400 text-xs uppercase m-0 mb-2",
                                            "{category.display_name()} ({cat_skills.len()})"
                                        }

                                        div {
                                            class: "flex flex-col gap-1",
                                            for skill in cat_skills.iter() {
                                                SkillRowInline {
                                                    key: "{skill.id}",
                                                    skill: skill.clone(),
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Inline skill row for Skills Management tab
#[derive(Props, Clone, PartialEq)]
struct SkillRowInlineProps {
    skill: wrldbldr_player_app::application::services::SkillData,
}

#[component]
fn SkillRowInline(props: SkillRowInlineProps) -> Element {
    let row_class = if props.skill.is_hidden {
        "flex items-center gap-3 py-2 px-3 bg-gray-500 bg-opacity-20 rounded"
    } else {
        "flex items-center gap-3 py-2 px-3 bg-dark-bg rounded"
    };
    let name_color = if props.skill.is_hidden { "text-gray-500" } else { "text-white" };

    rsx! {
        div {
            class: "{row_class}",

            // Visibility indicator
            div {
                class: if props.skill.is_hidden {
                    "w-2 h-2 rounded-full bg-gray-500"
                } else {
                    "w-2 h-2 rounded-full bg-green-500"
                }
            }

            // Skill info
            div {
                class: "flex-1 min-w-0",

                div {
                    class: "flex items-center gap-2",

                    span {
                        class: "{name_color} font-medium",
                        "{props.skill.name}"
                    }

                    if let Some(attr) = &props.skill.base_attribute {
                        span {
                            class: "text-purple-500 text-xs bg-purple-500 bg-opacity-10 py-0.5 px-1.5 rounded",
                            "{attr}"
                        }
                    }

                    if props.skill.is_custom {
                        span {
                            class: "text-amber-500 text-xs bg-amber-500 bg-opacity-10 py-0.5 px-1.5 rounded",
                            "Custom"
                        }
                    }
                }

                if !props.skill.description.is_empty() {
                    p {
                        class: "text-gray-500 text-xs mt-1 mb-0 whitespace-nowrap overflow-hidden text-ellipsis",
                        "{props.skill.description}"
                    }
                }
            }
        }
    }
}


/// Empty state panel when no workflow is selected
#[component]
fn WorkflowEmptyStatePanel() -> Element {
    rsx! {
        div {
            class: "empty-state flex-1 flex flex-col items-center justify-center bg-dark-surface rounded-lg text-gray-500",

            div {
                class: "text-center max-w-xs",

                // Gear icon
                div {
                    class: "text-5xl mb-4 opacity-50",
                    "⚙️"
                }

                h2 {
                    class: "text-gray-400 mb-2 text-xl",
                    "Workflow Configuration"
                }

                p {
                    class: "text-gray-500 text-sm leading-relaxed",
                    "Select a workflow slot from the left panel to view or edit its configuration. Click 'Configure' to upload a new ComfyUI workflow."
                }
            }
        }
    }
}
