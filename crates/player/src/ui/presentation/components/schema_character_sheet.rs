//! Schema-based character sheet components
//!
//! Renders character sheets based on CharacterSheetSchema definitions.
//! Used by both PC creation and DM character management.

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::application::dto::{CharacterSheetSchema, SchemaFieldDefinition, SchemaFieldType, SchemaResourceColor, SchemaSection as SchemaSectionDto, SchemaSectionType};

/// Schema-based character sheet form
///
/// Renders a complete character sheet based on a CharacterSheetSchema,
/// using JSON values for flexibility across game systems.
#[derive(Props, Clone, PartialEq)]
pub struct SchemaCharacterSheetProps {
    /// The schema defining the character sheet structure
    pub schema: CharacterSheetSchema,
    /// Signal for field values - child components read/write directly
    pub values: Signal<HashMap<String, serde_json::Value>>,
    /// Whether to show the system name header (default true)
    #[props(default = true)]
    pub show_header: bool,
    /// Force all fields to be read-only (for viewer mode)
    #[props(default = false)]
    pub read_only: bool,
}

#[component]
pub fn SchemaCharacterSheet(props: SchemaCharacterSheetProps) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-6",
            // System header
            if props.show_header {
                div {
                    class: "pb-4 border-b border-gray-700",
                    span {
                        class: "text-sm text-gray-400",
                        "Game System: "
                    }
                    span {
                        class: "text-sm text-blue-400 font-medium",
                        "{props.schema.system_name}"
                    }
                }
            }
            // Render each section
            {props.schema.sections.iter().cloned().map(|section| {
                let section_id = section.id.clone();
                rsx! {
                    SchemaSection {
                        key: "{section_id}",
                        section,
                        values: props.values,
                        read_only: props.read_only,
                    }
                }
            })}
        }
    }
}

/// Render a single schema section
#[derive(Props, Clone, PartialEq)]
pub struct SchemaSectionProps {
    pub section: SchemaSectionDto,
    /// Signal for field values - child components read/write directly
    pub values: Signal<HashMap<String, serde_json::Value>>,
    /// Force all fields to be read-only
    #[props(default = false)]
    pub read_only: bool,
}

#[component]
pub fn SchemaSection(props: SchemaSectionProps) -> Element {
    // Determine grid layout based on section type
    let grid_class = match props.section.section_type {
        SchemaSectionType::AbilityScores => "grid grid-cols-3 gap-4 md:grid-cols-6",
        SchemaSectionType::Skills => "grid grid-cols-2 gap-3 md:grid-cols-3",
        SchemaSectionType::Combat => "grid grid-cols-2 gap-4 md:grid-cols-4",
        _ => "grid grid-cols-1 gap-4 md:grid-cols-2",
    };

    rsx! {
        div {
            class: "p-4 bg-dark-surface rounded-lg border border-gray-700",
            h3 {
                class: "mb-4 text-lg font-medium text-white border-b border-gray-600 pb-2",
                "{props.section.label}"
            }
            if let Some(desc) = props.section.description.as_ref() {
                p {
                    class: "text-sm text-gray-400 mb-4",
                    "{desc}"
                }
            }
            div {
                class: "{grid_class}",
                {props.section.fields.iter().cloned().map(|field| {
                    let field_id = field.id.clone();
                    rsx! {
                        SchemaField {
                            key: "{field_id}",
                            field,
                            field_id,
                            values: props.values,
                            read_only: props.read_only,
                        }
                    }
                })}
            }
        }
    }
}

/// Render a single schema field
#[derive(Props, Clone, PartialEq)]
pub struct SchemaFieldProps {
    pub field: SchemaFieldDefinition,
    /// The field ID for this field
    pub field_id: String,
    /// Signal for field values - writes directly on change
    pub values: Signal<HashMap<String, serde_json::Value>>,
    /// Force field to be read-only
    #[props(default = false)]
    pub read_only: bool,
}

#[component]
pub fn SchemaField(props: SchemaFieldProps) -> Element {
    let field = &props.field;
    let field_id = props.field_id.clone();
    let mut values = props.values;
    let is_derived = field.derived_from.is_some();
    let is_readonly = props.read_only || !field.editable || is_derived;

    // Read current value from signal
    let current_value = values.read().get(&field_id).cloned();

    rsx! {
        div {
            class: "flex flex-col gap-1",
            label {
                class: "text-sm text-gray-400",
                "{field.label}"
                if field.required {
                    span { class: "text-red-400 ml-1", "*" }
                }
            }
            match &field.field_type {
                SchemaFieldType::Text { multiline, .. } => {
                    let fid = field_id.clone();
                    if *multiline {
                        rsx! {
                            textarea {
                                class: "w-full p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm resize-y",
                                readonly: is_readonly,
                                placeholder: field.placeholder.as_deref().unwrap_or(""),
                                value: current_value.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                                oninput: move |e| {
                                    values.write().insert(fid.clone(), serde_json::json!(e.value()));
                                },
                            }
                        }
                    } else {
                        rsx! {
                            input {
                                r#type: "text",
                                class: "w-full p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm",
                                readonly: is_readonly,
                                placeholder: field.placeholder.as_deref().unwrap_or(""),
                                value: current_value.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                                oninput: move |e| {
                                    values.write().insert(fid.clone(), serde_json::json!(e.value()));
                                },
                            }
                        }
                    }
                }
                SchemaFieldType::Integer { min, max, show_modifier } => {
                    let fid = field_id.clone();
                    let current = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    rsx! {
                        div {
                            class: "flex items-center gap-2",
                            input {
                                r#type: "number",
                                class: "w-20 p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm text-center",
                                readonly: is_readonly,
                                min: min.map(|m| format!("{}", m)),
                                max: max.map(|m| format!("{}", m)),
                                value: "{current}",
                                oninput: move |e| {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        values.write().insert(fid.clone(), serde_json::json!(val));
                                    }
                                },
                            }
                            if *show_modifier {
                                span {
                                    class: if current >= 0 { "text-green-400 text-sm" } else { "text-red-400 text-sm" },
                                    if current >= 0 { "+{current}" } else { "{current}" }
                                }
                            }
                        }
                    }
                }
                SchemaFieldType::AbilityScore { min, max } => {
                    let fid = field_id.clone();
                    let score = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(10) as i32;
                    let modifier = (score - 10) / 2;
                    rsx! {
                        div {
                            class: "flex flex-col items-center p-3 bg-dark-bg border border-gray-600 rounded",
                            input {
                                r#type: "number",
                                class: "w-16 p-1 bg-transparent border-0 text-white text-xl text-center font-bold",
                                readonly: is_readonly,
                                min: min.map(|m| format!("{}", m)),
                                max: max.map(|m| format!("{}", m)),
                                value: "{score}",
                                oninput: move |e| {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        values.write().insert(fid.clone(), serde_json::json!(val));
                                    }
                                },
                            }
                            span {
                                class: if modifier >= 0 { "text-green-400 text-sm" } else { "text-red-400 text-sm" },
                                if modifier >= 0 { "+{modifier}" } else { "{modifier}" }
                            }
                        }
                    }
                }
                SchemaFieldType::Boolean { checked_label, unchecked_label } => {
                    let fid = field_id.clone();
                    let checked = current_value.as_ref().and_then(|v| v.as_bool()).unwrap_or(false);
                    let label = if checked {
                        checked_label.as_deref().unwrap_or("Yes")
                    } else {
                        unchecked_label.as_deref().unwrap_or("No")
                    };
                    rsx! {
                        label {
                            class: "flex items-center gap-2 cursor-pointer",
                            input {
                                r#type: "checkbox",
                                class: "w-4 h-4",
                                disabled: is_readonly,
                                checked,
                                onchange: move |e| {
                                    values.write().insert(fid.clone(), serde_json::json!(e.checked()));
                                },
                            }
                            span {
                                class: "text-sm text-gray-300",
                                "{label}"
                            }
                        }
                    }
                }
                SchemaFieldType::Select { options, .. } => {
                    let fid = field_id.clone();
                    let current = current_value.as_ref().and_then(|v| v.as_str()).unwrap_or("");
                    rsx! {
                        select {
                            class: "w-full p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm",
                            disabled: is_readonly,
                            value: "{current}",
                            onchange: move |e| {
                                values.write().insert(fid.clone(), serde_json::json!(e.value()));
                            },
                            option { value: "", "Select..." }
                            {options.iter().map(|opt| rsx! {
                                option {
                                    key: "{opt.value}",
                                    value: "{opt.value}",
                                    "{opt.label}"
                                }
                            })}
                        }
                    }
                }
                SchemaFieldType::ResourceBar { max_field, color } => {
                    let fid = field_id.clone();
                    let current = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let color_class = match color {
                        SchemaResourceColor::Red => "bg-red-500",
                        SchemaResourceColor::Blue => "bg-blue-500",
                        SchemaResourceColor::Green => "bg-green-500",
                        SchemaResourceColor::Purple => "bg-purple-500",
                        SchemaResourceColor::Orange => "bg-orange-500",
                        SchemaResourceColor::Gray => "bg-gray-500",
                    };
                    rsx! {
                        div {
                            class: "flex items-center gap-2",
                            input {
                                r#type: "number",
                                class: "w-16 p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm text-center",
                                min: "0",
                                value: "{current}",
                                oninput: move |e| {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        values.write().insert(fid.clone(), serde_json::json!(val));
                                    }
                                },
                            }
                            span { class: "text-gray-400", "/" }
                            span { class: "text-gray-300 text-sm", "{max_field}" }
                            div {
                                class: "w-4 h-4 rounded-full {color_class}",
                            }
                        }
                    }
                }
                SchemaFieldType::DicePool { max_dice, .. } => {
                    let current = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(0) as u8;
                    rsx! {
                        div {
                            class: "flex gap-1",
                            {(0..*max_dice).map(|i| {
                                let fid = field_id.clone();
                                let filled = i < current;
                                let class = if filled {
                                    "w-6 h-6 rounded border-2 border-white bg-white cursor-pointer"
                                } else {
                                    "w-6 h-6 rounded border-2 border-gray-500 bg-transparent cursor-pointer"
                                };
                                rsx! {
                                    div {
                                        key: "{i}",
                                        class: "{class}",
                                        onclick: move |_| {
                                            let new_val = if filled { i } else { i + 1 };
                                            values.write().insert(fid.clone(), serde_json::json!(new_val));
                                        },
                                    }
                                }
                            })}
                        }
                    }
                }
                SchemaFieldType::PercentileSkill { show_derived } => {
                    let fid = field_id.clone();
                    let current = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let half = current / 2;
                    let fifth = current / 5;
                    rsx! {
                        div {
                            class: "flex items-center gap-2",
                            input {
                                r#type: "number",
                                class: "w-16 p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm text-center",
                                min: "0",
                                max: "100",
                                value: "{current}",
                                oninput: move |e| {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        values.write().insert(fid.clone(), serde_json::json!(val));
                                    }
                                },
                            }
                            span { class: "text-gray-400", "%" }
                            if *show_derived {
                                span { class: "text-xs text-gray-500", "(1/2: {half}, 1/5: {fifth})" }
                            }
                        }
                    }
                }
                SchemaFieldType::LadderRating { min, max, labels } => {
                    let fid = field_id.clone();
                    let current = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(*min as i64) as i32;
                    let label = labels.iter().find(|l| l.value == current).map(|l| l.label.as_str()).unwrap_or("");
                    rsx! {
                        div {
                            class: "flex items-center gap-2",
                            input {
                                r#type: "range",
                                class: "flex-1",
                                min: "{min}",
                                max: "{max}",
                                value: "{current}",
                                oninput: move |e| {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        values.write().insert(fid.clone(), serde_json::json!(val));
                                    }
                                },
                            }
                            span { class: "text-sm text-gray-300 min-w-[60px]", "{current} {label}" }
                        }
                    }
                }
                SchemaFieldType::Clock { segments } => {
                    let current = current_value.as_ref().and_then(|v| v.as_i64()).unwrap_or(0) as u8;
                    rsx! {
                        div {
                            class: "flex gap-1",
                            {(0..*segments).map(|i| {
                                let fid = field_id.clone();
                                let filled = i < current;
                                let class = if filled {
                                    "w-6 h-6 rounded-full border-2 border-blue-500 bg-blue-500 cursor-pointer"
                                } else {
                                    "w-6 h-6 rounded-full border-2 border-gray-500 bg-transparent cursor-pointer"
                                };
                                rsx! {
                                    div {
                                        key: "{i}",
                                        class: "{class}",
                                        onclick: move |_| {
                                            let new_val = if filled { i } else { i + 1 };
                                            values.write().insert(fid.clone(), serde_json::json!(new_val));
                                        },
                                    }
                                }
                            })}
                        }
                    }
                }
                _ => {
                    // Fallback for unsupported field types
                    let fid = field_id.clone();
                    let current = current_value.as_ref().and_then(|v| v.as_str()).unwrap_or("");
                    rsx! {
                        input {
                            r#type: "text",
                            class: "w-full p-2 bg-dark-bg border border-gray-600 rounded text-white text-sm",
                            readonly: is_readonly,
                            value: "{current}",
                            oninput: move |e| {
                                values.write().insert(fid.clone(), serde_json::json!(e.value()));
                            },
                        }
                    }
                }
            }
            if let Some(desc) = field.description.as_ref() {
                p {
                    class: "text-xs text-gray-500 mt-1",
                    "{desc}"
                }
            }
        }
    }
}
