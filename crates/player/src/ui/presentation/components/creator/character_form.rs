//! Character Form - Create and edit characters

use dioxus::prelude::*;
use std::collections::{BTreeMap, HashMap};

use super::asset_gallery::AssetGallery;
use super::expression_config_editor::ExpressionConfigEditor;
use super::motivations_tab::MotivationsTab;
use super::suggestion_button::{SuggestionButton, SuggestionType};
use crate::application::dto::CharacterSheetSchema;
use crate::application::services::CharacterFormData;
use crate::application::services::SuggestionContext;
use crate::infrastructure::spawn_task;
use crate::presentation::components::common::FormField;
use crate::presentation::components::schema_character_sheet::SchemaCharacterSheet;
use crate::presentation::services::{use_character_service, use_world_service};
use crate::use_platform;
use wrldbldr_domain::{ExpressionConfig, MoodState};
use wrldbldr_protocol::character_sheet::CharacterSheetValues;
use wrldbldr_protocol::character_sheet::SheetValue;

/// Character archetypes
const ARCHETYPES: &[&str] = &[
    "Hero",
    "Mentor",
    "Threshold Guardian",
    "Herald",
    "Shapeshifter",
    "Shadow",
    "Ally",
    "Trickster",
];

/// Character form for creating/editing characters
#[component]
pub fn CharacterForm(
    character_id: String,
    world_id: String,
    characters_signal: Signal<
        Vec<crate::application::services::character_service::CharacterSummary>,
    >,
    on_close: EventHandler<()>,
) -> Element {
    let is_new = character_id.is_empty();
    let platform = use_platform();
    let char_service = use_character_service();
    let world_service = use_world_service();

    // Form state
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut archetype = use_signal(|| "Hero".to_string());
    let mut wants = use_signal(String::new);
    let mut fears = use_signal(String::new);
    let mut backstory = use_signal(String::new);
    let mut is_loading = use_signal(|| !is_new);
    let mut is_saving = use_signal(|| false);
    let mut success_message: Signal<Option<String>> = use_signal(|| None);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    // Sheet schema state (uses CharacterSheetSchema with SheetValue)
    let mut sheet_schema: Signal<Option<CharacterSheetSchema>> = use_signal(|| None);
    let mut sheet_values: Signal<HashMap<String, SheetValue>> = use_signal(HashMap::new);
    let mut show_sheet_section = use_signal(|| true);

    // Expression config state (Tier 2 & 3 of emotional model)
    let mut expression_config = use_signal(ExpressionConfig::default);
    let mut default_mood = use_signal(|| MoodState::Calm);
    let mut show_expression_section = use_signal(|| false);

    // Load sheet schema on mount
    {
        let world_svc = world_service.clone();
        let plat = platform.clone();
        let world_id_for_schema = world_id.clone();
        use_effect(move || {
            let svc = world_svc.clone();
            let platform = plat.clone();
            let world_id_clone = world_id_for_schema.clone();
            spawn_task(async move {
                match svc.get_sheet_template(&world_id_clone).await {
                    Ok(schema) => {
                        sheet_schema.set(Some(schema));
                    }
                    Err(_e) => {
                        // Schema fetch failure is not critical - sheet section just won't appear
                        platform.log_warn(&format!("Failed to load sheet schema: {}", _e));
                    }
                }
            });
        });
    }

    // Load character data if editing existing character
    {
        let char_id_for_effect = character_id.clone();
        let char_svc = char_service.clone();
        use_effect(move || {
            let char_id = char_id_for_effect.clone();
            let svc = char_svc.clone();
            if !char_id.is_empty() {
                spawn_task(async move {
                    match svc.get_character(&char_id).await {
                        Ok(char_data) => {
                            name.set(char_data.name);
                            description.set(char_data.description.unwrap_or_default());
                            archetype
                                .set(char_data.archetype.unwrap_or_else(|| "Hero".to_string()));
                            wants.set(char_data.wants.unwrap_or_default());
                            fears.set(char_data.fears.unwrap_or_default());
                            backstory.set(char_data.backstory.unwrap_or_default());
                            if let Some(data) = char_data.sheet_data {
                                sheet_values.set(data.values.into_iter().collect());
                            }
                            is_loading.set(false);
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to load character: {}", e)));
                            is_loading.set(false);
                        }
                    }
                });
            }
        });
    }

    rsx! {
        div {
            class: "character-form flex flex-col h-full bg-dark-surface rounded-lg overflow-hidden",

            // Header
            div {
                class: "form-header flex justify-between items-center p-4 border-b border-gray-700",

                h2 {
                    class: "text-white m-0 text-xl",
                    if is_new { "New Character" } else { "Edit Character" }
                }

                button {
                    onclick: move |_| on_close.call(()),
                    class: "px-2 py-1 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                    "×"
                }
            }

            // Error/Success messages
            if let Some(msg) = error_message.read().as_ref() {
                div {
                    class: "px-4 py-3 bg-red-500/10 border-b border-red-500/30 text-red-500 text-sm",
                    "{msg}"
                }
            }
            if let Some(msg) = success_message.read().as_ref() {
                div {
                    class: "px-4 py-3 bg-green-500/10 border-b border-green-500/30 text-green-500 text-sm",
                    "{msg}"
                }
            }

            // Form content (scrollable)
            div {
                class: "form-content flex-1 overflow-y-auto p-4 flex flex-col gap-4",

                if *is_loading.read() {
                    div {
                        class: "flex items-center justify-center p-8 text-gray-500",
                        "Loading character data..."
                    }
                } else {

                // Name field with suggest button
                FormField {
                    label: "Name",
                    required: true,
                    children: rsx! {
                        div { class: "flex gap-2",
                            input {
                                r#type: "text",
                                value: "{name}",
                                oninput: move |e| name.set(e.value()),
                                placeholder: "Enter character name...",
                                class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            }
                            SuggestionButton {
                                suggestion_type: SuggestionType::CharacterName,
                                world_id: world_id.clone(),
                                context: SuggestionContext {
                                    hints: Some(archetype.read().clone()),
                                    ..Default::default()
                                },
                                on_select: move |value| name.set(value),
                            }
                        }
                    }
                }

                // Archetype dropdown
                FormField {
                    label: "Archetype",
                    required: false,
                    children: rsx! {
                        select {
                            value: "{archetype}",
                            onchange: move |e| archetype.set(e.value()),
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",

                            for arch in ARCHETYPES {
                                option { value: "{arch}", "{arch}" }
                            }
                        }
                    }
                }

                // Description field
                FormField {
                    label: "Description",
                    required: false,
                    children: rsx! {
                        div { class: "flex flex-col gap-2",
                            textarea {
                                value: "{description}",
                                oninput: move |e| description.set(e.value()),
                                placeholder: "Physical appearance, mannerisms, voice...",
                                class: "w-full min-h-[80px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                            }
                            div { class: "flex justify-end",
                                SuggestionButton {
                                    suggestion_type: SuggestionType::CharacterDescription,
                                    world_id: world_id.clone(),
                                    context: SuggestionContext {
                                        entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                        hints: Some(archetype.read().clone()),
                                        ..Default::default()
                                    },
                                    on_select: move |value| description.set(value),
                                }
                            }
                        }
                    }
                }

                // Wants field
                FormField {
                    label: "Wants",
                    required: false,
                    children: rsx! {
                        div { class: "flex gap-2",
                            input {
                                r#type: "text",
                                value: "{wants}",
                                oninput: move |e| wants.set(e.value()),
                                placeholder: "What does this character desire?",
                                class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            }
                            SuggestionButton {
                                suggestion_type: SuggestionType::CharacterWants,
                                world_id: world_id.clone(),
                                context: SuggestionContext {
                                    entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                    hints: Some(archetype.read().clone()),
                                    additional_context: if description.read().is_empty() { None } else { Some(description.read().clone()) },
                                    ..Default::default()
                                },
                                on_select: move |value| wants.set(value),
                            }
                        }
                    }
                }

                // Fears field
                FormField {
                    label: "Fears",
                    required: false,
                    children: rsx! {
                        div { class: "flex gap-2",
                            input {
                                r#type: "text",
                                value: "{fears}",
                                oninput: move |e| fears.set(e.value()),
                                placeholder: "What does this character fear?",
                                class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            }
                            SuggestionButton {
                                suggestion_type: SuggestionType::CharacterFears,
                                world_id: world_id.clone(),
                                context: SuggestionContext {
                                    entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                    hints: Some(archetype.read().clone()),
                                    additional_context: if wants.read().is_empty() { None } else { Some(wants.read().clone()) },
                                    ..Default::default()
                                },
                                on_select: move |value| fears.set(value),
                            }
                        }
                    }
                }

                // Backstory field
                FormField {
                    label: "Backstory",
                    required: false,
                    children: rsx! {
                        div { class: "flex flex-col gap-2",
                            textarea {
                                value: "{backstory}",
                                oninput: move |e| backstory.set(e.value()),
                                placeholder: "Background, history, key events...",
                                class: "w-full min-h-[100px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                            }
                            div { class: "flex justify-end",
                                SuggestionButton {
                                    suggestion_type: SuggestionType::CharacterBackstory,
                                    world_id: world_id.clone(),
                                    context: SuggestionContext {
                                        entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                        hints: Some(archetype.read().clone()),
                                        additional_context: if wants.read().is_empty() { None } else { Some(wants.read().clone()) },
                                        world_setting: if fears.read().is_empty() { None } else { Some(fears.read().clone()) },
                                        ..Default::default()
                                    },
                                    on_select: move |value| backstory.set(value),
                                }
                            }
                        }
                    }
                }

                    // Expression Config section (Tier 2 & 3 of emotional model)
                    div {
                        class: "expression-section mt-6 border-t border-gray-700 pt-4",

                        // Section header with collapse toggle
                        div {
                            class: "flex justify-between items-center mb-4 cursor-pointer",
                            onclick: move |_| {
                                let current = *show_expression_section.read();
                                show_expression_section.set(!current);
                            },

                            h3 {
                                class: "text-gray-400 text-sm uppercase m-0",
                                "Expressions & Mood"
                            }

                            span {
                                class: "text-gray-500 text-sm",
                                if *show_expression_section.read() { "[-]" } else { "[+]" }
                            }
                        }

                        if *show_expression_section.read() {
                            ExpressionConfigEditor {
                                config: expression_config.read().clone(),
                                default_mood: *default_mood.read(),
                                on_config_change: move |config| {
                                    expression_config.set(config);
                                },
                                on_mood_change: move |mood| {
                                    default_mood.set(mood);
                                },
                            }
                        } else {
                            // Compact summary when collapsed
                            div {
                                class: "flex flex-wrap gap-2 text-xs",

                                // Mood badge
                                span {
                                    class: "px-2 py-1 bg-amber-900/50 text-amber-200 rounded",
                                    "{default_mood.read().emoji()} {default_mood.read().display_name()}"
                                }

                                // Expression count
                                span {
                                    class: "px-2 py-1 bg-purple-900/50 text-purple-200 rounded",
                                    "{expression_config.read().expressions().len()} expressions"
                                }

                                // Action count
                                if !expression_config.read().actions().is_empty() {
                                    span {
                                        class: "px-2 py-1 bg-gray-700 text-gray-300 rounded",
                                        "{expression_config.read().actions().len()} actions"
                                    }
                                }
                            }
                        }
                    }

                    // Character Sheet section (if schema available)
                    if let Some(schema) = sheet_schema.read().as_ref() {
                        div {
                            class: "sheet-section mt-6 border-t border-gray-700 pt-4",

                            // Section header with collapse toggle
                            div {
                                class: "flex justify-between items-center mb-4 cursor-pointer",
                                onclick: move |_| {
                                    let current = *show_sheet_section.read();
                                    show_sheet_section.set(!current);
                                },

                                h3 {
                                    class: "text-gray-400 text-sm uppercase m-0",
                                    "Character Sheet ({schema.system_name})"
                                }

                                span {
                                    class: "text-gray-500 text-sm",
                                    if *show_sheet_section.read() { "[-]" } else { "[+]" }
                                }
                            }

                            if *show_sheet_section.read() {
                                SchemaCharacterSheet {
                                    schema: schema.clone(),
                                    values: sheet_values,
                                    show_header: false,
                                }
                            }
                        }
                    }

                    // Motivations section (only for existing NPCs)
                    if !is_new {
                        div {
                            class: "motivations-section mt-6 border-t border-gray-700 pt-4",

                            h3 { class: "text-gray-400 text-sm uppercase mb-3", "Motivations (Actantial Model)" }

                            MotivationsTab {
                                character_id: character_id.clone(),
                                world_id: world_id.clone(),
                                character_name: name.read().clone(),
                            }
                        }
                    }

                    // Asset Gallery section
                    div {
                        class: "assets-section mt-4",

                        h3 { class: "text-gray-400 text-sm uppercase mb-3", "Assets" }

                        AssetGallery {
                            world_id: world_id.clone(),
                            entity_type: "character".to_string(),
                            entity_id: character_id.clone(),
                        }
                    }
                }
            }

            // Footer with action buttons
            div {
                class: "form-footer flex justify-end gap-2 p-4 border-t border-gray-700",

                button {
                    onclick: move |_| on_close.call(()),
                    class: "px-4 py-2 bg-transparent text-gray-400 border border-gray-700 rounded cursor-pointer",
                    disabled: *is_saving.read(),
                    "Cancel"
                }

                button {
                    class: format!(
                        "px-4 py-2 bg-green-500 text-white border-none rounded cursor-pointer font-medium {}",
                        if *is_saving.read() { "opacity-60" } else { "opacity-100" }
                    ),
                    disabled: *is_saving.read(),
                    onclick: {
                        let char_svc = char_service.clone();
                        move |_| {
                            let char_name = name.read().clone();
                            if char_name.is_empty() {
                                error_message.set(Some("Character name is required".to_string()));
                                return;
                            }

                            error_message.set(None);
                            success_message.set(None);
                            is_saving.set(true);

                            let char_id = character_id.clone();
                            let on_close = on_close;
                            let svc = char_svc.clone();
                            let world_id_clone = world_id.clone();

                            spawn_task(async move {
                                    // Get sheet values
                                    let sheet_data_to_save = {
                                        let values = sheet_values.read().clone();
                                        if values.is_empty() {
                                            None
                                        } else {
                                            Some(CharacterSheetValues {
                                                values: values.into_iter().collect::<BTreeMap<_, _>>(),
                                                last_updated: None,
                                            })
                                        }
                                    };

                                    let char_data = CharacterFormData {
                                        id: if is_new { None } else { Some(char_id.clone()) },
                                        name: name.read().clone(),
                                        description: {
                                            let desc = description.read().clone();
                                            if desc.is_empty() { None } else { Some(desc) }
                                        },
                                        archetype: {
                                            let arch = archetype.read().clone();
                                            if arch.is_empty() { None } else { Some(arch) }
                                        },
                                        wants: {
                                            let w = wants.read().clone();
                                            if w.is_empty() { None } else { Some(w) }
                                        },
                                        fears: {
                                            let f = fears.read().clone();
                                            if f.is_empty() { None } else { Some(f) }
                                        },
                                        backstory: {
                                            let b = backstory.read().clone();
                                            if b.is_empty() { None } else { Some(b) }
                                        },
                                        sprite_asset: None,
                                        portrait_asset: None,
                                        sheet_data: sheet_data_to_save,
                                    };

                                    match if is_new {
                                        svc.create_character(&world_id_clone, &char_data).await
                                    } else {
                                        svc.update_character(&char_id, &char_data).await
                                    } {
                                        Ok(saved_character) => {
                                            // Update the characters signal reactively
                                            if is_new {
                                                // Add new character to list
                                                let summary = crate::application::services::character_service::CharacterSummary {
                                                    id: saved_character.id.clone().unwrap_or_default(),
                                                    name: saved_character.name.clone(),
                                                    archetype: saved_character.archetype.clone(),
                                                };
                                                characters_signal.write().push(summary);
                                            } else {
                                                // Update existing character in list
                                                if let Some(id) = &saved_character.id {
                                                    let mut chars = characters_signal.write();
                                                    if let Some(existing) = chars.iter_mut().find(|c| c.id == *id) {
                                                        existing.name = saved_character.name.clone();
                                                        existing.archetype = saved_character.archetype.clone();
                                                    }
                                                }
                                            }

                                            success_message.set(Some(if is_new {
                                                "Character created successfully".to_string()
                                            } else {
                                                "Character saved successfully".to_string()
                                            }));
                                            is_saving.set(false);
                                            // Close form - let the user see the success message
                                            on_close.call(());
                                        }
                                        Err(e) => {
                                            error_message.set(Some(format!("Save failed: {}", e)));
                                            is_saving.set(false);
                                        }
                                }
                            });
                        }
                    },
                    if *is_saving.read() { "Saving..." } else { if is_new { "Create" } else { "Save" } }
                }
            }
        }
    }
}
