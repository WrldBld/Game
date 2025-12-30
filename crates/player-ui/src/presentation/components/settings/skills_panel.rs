//! Skills Panel - Manage skills for a world
//!
//! Displays skills grouped by category with options to:
//! - View default skills from the rule system
//! - Hide/show default skills
//! - Create custom skills
//! - Edit and delete custom skills

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::presentation::services::use_skill_service;
use wrldbldr_player_app::application::dto::{SkillCategory, SkillData};
use wrldbldr_player_app::application::services::{CreateSkillRequest, UpdateSkillRequest};

/// Props for SkillsPanel
#[derive(Props, Clone, PartialEq)]
pub struct SkillsPanelProps {
    /// The world ID to manage skills for
    pub world_id: String,
    /// Called when the panel should close
    pub on_close: EventHandler<()>,
}

/// Skills Panel component
#[component]
pub fn SkillsPanel(props: SkillsPanelProps) -> Element {
    let mut skills: Signal<Vec<SkillData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut show_hidden = use_signal(|| false);
    let mut show_add_form = use_signal(|| false);
    let mut editing_skill: Signal<Option<String>> = use_signal(|| None);

    // Clone world_id once for all handlers
    let world_id = props.world_id.clone();
    let world_id_for_effect = world_id.clone();
    let world_id_for_rows = world_id.clone();
    let world_id_for_add = world_id.clone();
    let world_id_for_edit = world_id;

    // Get skill service
    let skill_service = use_skill_service();

    // Load skills on mount
    use_effect(move || {
        let world_id = world_id_for_effect.clone();
        let service = skill_service.clone();
        spawn(async move {
            match service.list_skills(&world_id).await {
                Ok(list) => {
                    skills.set(list);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load skills: {}", e)));
                    is_loading.set(false);
                }
            }
        });
    });

    // Group skills by category - clone the data to avoid borrow issues
    let skills_by_category: HashMap<SkillCategory, Vec<SkillData>> = {
        let skills_read = skills.read();
        let show_hidden_val = *show_hidden.read();
        let mut grouped: HashMap<SkillCategory, Vec<SkillData>> = HashMap::new();
        for skill in skills_read.iter() {
            if !skill.is_hidden || show_hidden_val {
                grouped
                    .entry(skill.category)
                    .or_default()
                    .push(skill.clone());
            }
        }
        // Sort skills within each category by order
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

    let handle_skill_created = move |skill: SkillData| {
        skills.write().push(skill);
        show_add_form.set(false);
    };

    let handle_skill_updated = move |skill: SkillData| {
        let mut skills_write = skills.write();
        if let Some(existing) = skills_write.iter_mut().find(|s| s.id == skill.id) {
            *existing = skill;
        }
        editing_skill.set(None);
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-80 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-lg w-11/12 max-w-3xl max-h-screen-90 overflow-hidden flex flex-col",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center py-4 px-6 border-b border-gray-700",

                    h2 { class: "text-white m-0 text-xl", "Skills Management" }

                    div { class: "flex gap-2 items-center",
                        // Show hidden toggle
                        label { class: "flex items-center gap-2 text-gray-400 text-xs cursor-pointer",
                            input {
                                r#type: "checkbox",
                                checked: *show_hidden.read(),
                                onchange: move |e| show_hidden.set(e.checked()),
                            }
                            "Show Hidden"
                        }

                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "p-2 bg-transparent border-0 text-gray-400 cursor-pointer text-xl",
                            "X"
                        }
                    }
                }

                // Error message
                if let Some(err) = error.read().as_ref() {
                    div {
                        class: "py-3 px-6 bg-red-500 bg-opacity-10 text-red-500 text-sm",
                        "{err}"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto py-4 px-6",

                    if *is_loading.read() {
                        div {
                            class: "text-center text-gray-500 py-8",
                            "Loading skills..."
                        }
                    } else if *show_add_form.read() {
                        AddSkillForm {
                            world_id: world_id_for_add.clone(),
                            on_created: handle_skill_created,
                            on_cancel: move |_| show_add_form.set(false),
                        }
                    } else if let Some(skill_id) = editing_skill.read().clone() {
                        {
                            let skill = skills.read().iter().find(|s| s.id == skill_id).cloned();
                            if let Some(skill) = skill {
                                rsx! {
                                    EditSkillForm {
                                        world_id: world_id_for_edit.clone(),
                                        skill: skill,
                                        on_updated: handle_skill_updated,
                                        on_cancel: move |_| editing_skill.set(None),
                                    }
                                }
                            } else {
                                rsx! { div { "Skill not found" } }
                            }
                        }
                    } else {
                        // Add skill button
                        div { class: "mb-4",
                            button {
                                onclick: move |_| show_add_form.set(true),
                                class: "py-2 px-4 bg-purple-500 text-white border-0 rounded cursor-pointer text-sm",
                                "+ Add Custom Skill"
                            }
                        }

                        // Skills by category
                        for category in active_categories.iter() {
                            {
                                let cat_skills = skills_by_category.get(category).cloned().unwrap_or_default();
                                if !cat_skills.is_empty() {
                                    rsx! {
                                        div { class: "mb-6",
                                            h3 { class: "text-gray-400 text-xs uppercase m-0 mb-2",
                                                "{category.display_name()} ({cat_skills.len()})"
                                            }

                                            div { class: "flex flex-col gap-1",
                                                for skill in cat_skills.iter() {
                                                    SkillRow {
                                                        key: "{skill.id}",
                                                        world_id: world_id_for_rows.clone(),
                                                        skill: skill.clone(),
                                                        skills_signal: skills,
                                                        error_signal: error,
                                                        on_edit: move |id| editing_skill.set(Some(id)),
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

                        if skills.read().is_empty() {
                            div {
                                class: "text-center text-gray-500 py-8",
                                p { "No skills defined for this world." }
                                p { class: "text-sm", "Skills are loaded from the rule system preset." }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Individual skill row
#[component]
fn SkillRow(
    world_id: String,
    skill: SkillData,
    skills_signal: Signal<Vec<SkillData>>,
    error_signal: Signal<Option<String>>,
    on_edit: EventHandler<String>,
) -> Element {
    let skill_id_for_toggle = skill.id.clone();
    let skill_id_for_edit = skill.id.clone();
    let skill_id_for_delete = skill.id.clone();
    let is_hidden = skill.is_hidden;
    let is_custom = skill.is_custom;

    let world_id_for_toggle = world_id.clone();
    let _world_id_for_delete = world_id.clone();

    // Get skill service
    let skill_service = use_skill_service();

    // Pre-compute classes based on hidden state
    let row_class = if skill.is_hidden {
        "flex items-center gap-3 py-2 px-3 bg-gray-500 bg-opacity-20 rounded"
    } else {
        "flex items-center gap-3 py-2 px-3 bg-dark-bg rounded"
    };
    let icon_class = if skill.is_hidden {
        "p-1 bg-transparent border-0 text-gray-500 cursor-pointer text-sm"
    } else {
        "p-1 bg-transparent border-0 text-green-500 cursor-pointer text-sm"
    };
    let name_class = if skill.is_hidden {
        "text-gray-500 font-medium"
    } else {
        "text-white font-medium"
    };

    let handle_toggle = {
        let service = skill_service.clone();
        move |_| {
            let _world_id = world_id_for_toggle.clone();
            let skill_id = skill_id_for_toggle.clone();
            let new_hidden = !is_hidden;
            let service = service.clone();
            spawn(async move {
                match service.update_skill_visibility(&skill_id, new_hidden).await {
                    Ok(updated) => {
                        let mut skills_write = skills_signal.write();
                        if let Some(skill) = skills_write.iter_mut().find(|s| s.id == skill_id) {
                            skill.is_hidden = updated.is_hidden;
                        }
                    }
                    Err(e) => {
                        error_signal.set(Some(format!("Failed to update skill: {}", e)));
                    }
                }
            });
        }
    };

    let handle_delete = {
        let service = skill_service.clone();
        move |_| {
            let skill_id = skill_id_for_delete.clone();
            let service = service.clone();
            spawn(async move {
                match service.delete_skill(&skill_id).await {
                    Ok(()) => {
                        skills_signal.write().retain(|s| s.id != skill_id);
                    }
                    Err(e) => {
                        error_signal.set(Some(format!("Failed to delete skill: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "{row_class}",

            // Visibility toggle
            button {
                onclick: handle_toggle,
                class: "{icon_class}",
                title: if is_hidden { "Show skill" } else { "Hide skill" },
                if is_hidden { "👁" } else { "👁" }
            }

            // Skill info
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-2",
                    span { class: "{name_class}", "{skill.name}" }
                    if let Some(attr) = &skill.base_attribute {
                        span { class: "text-purple-500 text-xs bg-purple-500 bg-opacity-10 py-0.5 px-1.5 rounded",
                            "{attr}"
                        }
                    }
                    if is_custom {
                        span { class: "text-amber-500 text-xs bg-amber-500 bg-opacity-10 py-0.5 px-1.5 rounded",
                            "Custom"
                        }
                    }
                }
                if !skill.description.is_empty() {
                    p { class: "text-gray-500 text-xs mt-1 mb-0 whitespace-nowrap overflow-hidden text-ellipsis",
                        "{skill.description}"
                    }
                }
            }

            // Actions
            div { class: "flex gap-1",
                if is_custom {
                    button {
                        onclick: move |_| on_edit.call(skill_id_for_edit.clone()),
                        class: "py-1 px-2 bg-gray-700 text-white border-0 rounded cursor-pointer text-xs",
                        "Edit"
                    }
                    button {
                        onclick: handle_delete,
                        class: "py-1 px-2 bg-red-500 text-white border-0 rounded cursor-pointer text-xs",
                        "X"
                    }
                }
            }
        }
    }
}

/// Form to add a new custom skill
#[component]
fn AddSkillForm(
    world_id: String,
    on_created: EventHandler<SkillData>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut category = use_signal(|| SkillCategory::Custom);
    let mut base_attribute = use_signal(String::new);
    let mut is_creating = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Get skill service
    let skill_service = use_skill_service();

    let handle_create = move |_| {
        let name_val = name.read().clone();
        if name_val.trim().is_empty() {
            error.set(Some("Skill name is required".to_string()));
            return;
        }

        let world_id = world_id.clone();
        let desc = description.read().clone();
        let cat = *category.read();
        let attr = base_attribute.read().clone();
        let service = skill_service.clone();

        spawn(async move {
            is_creating.set(true);
            error.set(None);

            let request = CreateSkillRequest {
                name: name_val,
                description: desc,
                category: cat,
                base_attribute: if attr.is_empty() { None } else { Some(attr) },
            };

            match service.create_skill(&world_id, &request).await {
                Ok(skill) => {
                    on_created.call(skill);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to create skill: {}", e)));
                    is_creating.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "p-4 bg-dark-bg rounded-lg",

            h3 { class: "text-white m-0 mb-4 text-base", "Add Custom Skill" }

            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-2 bg-red-500 bg-opacity-10 text-red-500 text-sm mb-4 rounded",
                    "{err}"
                }
            }

            // Name
            div { class: "mb-3",
                label { class: "block text-gray-400 text-xs mb-1", "Name *" }
                input {
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                    disabled: *is_creating.read(),
                    placeholder: "Skill name",
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white box-border",
                }
            }

            // Description
            div { class: "mb-3",
                label { class: "block text-gray-400 text-xs mb-1", "Description" }
                textarea {
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                    disabled: *is_creating.read(),
                    placeholder: "What this skill is used for...",
                    class: "w-full min-h-15 p-2 bg-dark-surface border border-gray-700 rounded text-white resize-y box-border",
                }
            }

            // Category
            div { class: "mb-3",
                label { class: "block text-gray-400 text-xs mb-1", "Category" }
                select {
                    value: "{category.read().display_name()}",
                    onchange: move |e| {
                        let cat = match e.value().as_str() {
                            "Physical" => SkillCategory::Physical,
                            "Mental" => SkillCategory::Mental,
                            "Social" => SkillCategory::Social,
                            "Interpersonal" => SkillCategory::Interpersonal,
                            "Investigation" => SkillCategory::Investigation,
                            "Academic" => SkillCategory::Academic,
                            "Practical" => SkillCategory::Practical,
                            "Combat" => SkillCategory::Combat,
                            "Approach" => SkillCategory::Approach,
                            "Aspect" => SkillCategory::Aspect,
                            "Other" => SkillCategory::Other,
                            _ => SkillCategory::Custom,
                        };
                        category.set(cat);
                    },
                    disabled: *is_creating.read(),
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white",
                    for cat in SkillCategory::all() {
                        option { value: "{cat.display_name()}", "{cat.display_name()}" }
                    }
                }
            }

            // Base Attribute
            div { class: "mb-4",
                label { class: "block text-gray-400 text-xs mb-1", "Base Attribute (optional)" }
                input {
                    r#type: "text",
                    value: "{base_attribute}",
                    oninput: move |e| base_attribute.set(e.value()),
                    disabled: *is_creating.read(),
                    placeholder: "e.g., STR, DEX, INT",
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white box-border",
                }
            }

            // Buttons
            div { class: "flex gap-2",
                button {
                    onclick: handle_create,
                    disabled: *is_creating.read(),
                    class: "flex-1 p-2 bg-purple-500 text-white border-0 rounded cursor-pointer",
                    if *is_creating.read() { "Creating..." } else { "Create Skill" }
                }
                button {
                    onclick: move |_| on_cancel.call(()),
                    disabled: *is_creating.read(),
                    class: "py-2 px-4 bg-gray-700 text-white border-0 rounded cursor-pointer",
                    "Cancel"
                }
            }
        }
    }
}

/// Form to edit an existing skill
#[component]
fn EditSkillForm(
    world_id: String,
    skill: SkillData,
    on_updated: EventHandler<SkillData>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| skill.name.clone());
    let mut description = use_signal(|| skill.description.clone());
    let mut category = use_signal(|| skill.category);
    let mut base_attribute = use_signal(|| skill.base_attribute.clone().unwrap_or_default());
    let mut is_saving = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let skill_id = skill.id.clone();

    // Get skill service
    let skill_service = use_skill_service();

    let handle_save = move |_| {
        let name_val = name.read().clone();
        if name_val.trim().is_empty() {
            error.set(Some("Skill name is required".to_string()));
            return;
        }

        let _world_id = world_id.clone();
        let skill_id = skill_id.clone();
        let desc = description.read().clone();
        let cat = *category.read();
        let attr = base_attribute.read().clone();
        let service = skill_service.clone();

        spawn(async move {
            is_saving.set(true);
            error.set(None);

            let request = UpdateSkillRequest {
                name: Some(name_val),
                description: Some(desc),
                category: Some(cat),
                base_attribute: if attr.is_empty() { None } else { Some(attr) },
                is_hidden: None,
            };

            match service.update_skill(&skill_id, &request).await {
                Ok(skill) => {
                    on_updated.call(skill);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to update skill: {}", e)));
                    is_saving.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "p-4 bg-dark-bg rounded-lg",

            h3 { class: "text-white m-0 mb-4 text-base", "Edit Skill" }

            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-2 bg-red-500 bg-opacity-10 text-red-500 text-sm mb-4 rounded",
                    "{err}"
                }
            }

            // Name
            div { class: "mb-3",
                label { class: "block text-gray-400 text-xs mb-1", "Name *" }
                input {
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                    disabled: *is_saving.read(),
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white box-border",
                }
            }

            // Description
            div { class: "mb-3",
                label { class: "block text-gray-400 text-xs mb-1", "Description" }
                textarea {
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                    disabled: *is_saving.read(),
                    class: "w-full min-h-15 p-2 bg-dark-surface border border-gray-700 rounded text-white resize-y box-border",
                }
            }

            // Category
            div { class: "mb-3",
                label { class: "block text-gray-400 text-xs mb-1", "Category" }
                select {
                    value: "{category.read().display_name()}",
                    onchange: move |e| {
                        let cat = match e.value().as_str() {
                            "Physical" => SkillCategory::Physical,
                            "Mental" => SkillCategory::Mental,
                            "Social" => SkillCategory::Social,
                            "Interpersonal" => SkillCategory::Interpersonal,
                            "Investigation" => SkillCategory::Investigation,
                            "Academic" => SkillCategory::Academic,
                            "Practical" => SkillCategory::Practical,
                            "Combat" => SkillCategory::Combat,
                            "Approach" => SkillCategory::Approach,
                            "Aspect" => SkillCategory::Aspect,
                            "Other" => SkillCategory::Other,
                            _ => SkillCategory::Custom,
                        };
                        category.set(cat);
                    },
                    disabled: *is_saving.read(),
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white",
                    for cat in SkillCategory::all() {
                        option { value: "{cat.display_name()}", "{cat.display_name()}" }
                    }
                }
            }

            // Base Attribute
            div { class: "mb-4",
                label { class: "block text-gray-400 text-xs mb-1", "Base Attribute (optional)" }
                input {
                    r#type: "text",
                    value: "{base_attribute}",
                    oninput: move |e| base_attribute.set(e.value()),
                    disabled: *is_saving.read(),
                    placeholder: "e.g., STR, DEX, INT",
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white box-border",
                }
            }

            // Buttons
            div { class: "flex gap-2",
                button {
                    onclick: handle_save,
                    disabled: *is_saving.read(),
                    class: "flex-1 p-2 bg-purple-500 text-white border-0 rounded cursor-pointer",
                    if *is_saving.read() { "Saving..." } else { "Save Changes" }
                }
                button {
                    onclick: move |_| on_cancel.call(()),
                    disabled: *is_saving.read(),
                    class: "py-2 px-4 bg-gray-700 text-white border-0 rounded cursor-pointer",
                    "Cancel"
                }
            }
        }
    }
}
