//! Sheet Field Input - Dynamic input rendering for character sheet fields

use dioxus::prelude::*;
use std::collections::HashMap;

use wrldbldr_player_app::application::dto::{
    FieldType, FieldValue, SheetField, SheetSection, SheetTemplate,
};

/// Props for the sheet section renderer
#[derive(Props, Clone, PartialEq)]
pub struct SheetSectionProps {
    pub section: SheetSection,
    pub values: HashMap<String, FieldValue>,
    pub on_change: EventHandler<(String, FieldValue)>,
    #[props(default = false)]
    pub read_only: bool,
}

/// Renders a section of the character sheet
#[component]
pub fn SheetSectionInput(props: SheetSectionProps) -> Element {
    let mut is_collapsed = use_signal(|| props.section.collapsed_by_default);

    let section_class = match props.section.layout {
        wrldbldr_player_app::application::dto::SectionLayout::Vertical => {
            "flex flex-col gap-3"
        }
        wrldbldr_player_app::application::dto::SectionLayout::Grid { columns } => {
            let _cols = columns.min(4).max(1);
            "grid gap-3"
        }
        wrldbldr_player_app::application::dto::SectionLayout::TwoColumn => {
            "grid grid-cols-2 gap-3"
        }
        wrldbldr_player_app::application::dto::SectionLayout::Flow => {
            "flex flex-wrap gap-2"
        }
    };

    // Sort fields by order
    let mut sorted_fields = props.section.fields.clone();
    sorted_fields.sort_by_key(|f| f.order);

    rsx! {
        div {
            class: "sheet-section bg-black/20 rounded-lg overflow-hidden",

            // Section header
            div {
                class: "flex justify-between items-center px-4 py-3 bg-black/30 cursor-pointer",
                onclick: move |_| {
                    if props.section.collapsible {
                        let current = *is_collapsed.read();
                        is_collapsed.set(!current);
                    }
                },

                h4 {
                    class: "text-gray-200 text-sm m-0 font-semibold",
                    "{props.section.name}"
                }

                if props.section.collapsible {
                    span {
                        class: "text-gray-500 text-xs",
                        if *is_collapsed.read() { "+" } else { "-" }
                    }
                }
            }

            // Section content
            if !*is_collapsed.read() {
                div {
                    class: "p-4 {section_class}",

                    for field in sorted_fields {
                        {
                            let field_id = field.id.clone();
                            let current_value = props.values.get(&field_id).cloned();
                            let on_change = props.on_change.clone();
                            rsx! {
                                SheetFieldInput {
                                    key: "{field_id}",
                                    field: field,
                                    value: current_value,
                                    on_change: move |value| on_change.call((field_id.clone(), value)),
                                    read_only: props.read_only,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for an individual field input
#[derive(Props, Clone, PartialEq)]
pub struct SheetFieldInputProps {
    pub field: SheetField,
    pub value: Option<FieldValue>,
    pub on_change: EventHandler<FieldValue>,
    #[props(default = false)]
    pub read_only: bool,
}

/// Renders a single field input based on its type
#[component]
pub fn SheetFieldInput(props: SheetFieldInputProps) -> Element {
    let is_read_only = props.read_only || props.field.read_only;

    rsx! {
        div {
            class: "sheet-field flex flex-col",

            // Label
            label {
                class: "text-gray-400 text-xs mb-1 block",
                "{props.field.name}"
                if props.field.required {
                    span { class: "text-red-500 ml-1", "*" }
                }
            }

            // Input based on field type
            match &props.field.field_type {
                FieldType::Number { min, max, default } => {
                    let current = match &props.value {
                        Some(FieldValue::Number(n)) => *n,
                        _ => default.unwrap_or(0),
                    };
                    let min_val = *min;
                    let max_val = *max;
                    let on_change = props.on_change.clone();

                    rsx! {
                        input {
                            r#type: "number",
                            value: "{current}",
                            min: min_val.map(|v| v.to_string()),
                            max: max_val.map(|v| v.to_string()),
                            disabled: is_read_only,
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                            oninput: move |e| {
                                if let Ok(n) = e.value().parse::<i32>() {
                                    on_change.call(FieldValue::Number(n));
                                }
                            },
                        }
                    }
                }

                FieldType::Text { multiline, max_length: _ } => {
                    let current = match &props.value {
                        Some(FieldValue::Text(s)) => s.clone(),
                        _ => String::new(),
                    };
                    let is_multiline = *multiline;
                    let on_change = props.on_change.clone();

                    if is_multiline {
                        rsx! {
                            textarea {
                                value: "{current}",
                                disabled: is_read_only,
                                class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border min-h-[60px] resize-y",
                                oninput: move |e| {
                                    on_change.call(FieldValue::Text(e.value()));
                                },
                            }
                        }
                    } else {
                        rsx! {
                            input {
                                r#type: "text",
                                value: "{current}",
                                disabled: is_read_only,
                                class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                                oninput: move |e| {
                                    on_change.call(FieldValue::Text(e.value()));
                                },
                            }
                        }
                    }
                }

                FieldType::Checkbox { default } => {
                    let current = match &props.value {
                        Some(FieldValue::Boolean(b)) => *b,
                        _ => *default,
                    };
                    let on_change = props.on_change.clone();

                    rsx! {
                        input {
                            r#type: "checkbox",
                            checked: current,
                            disabled: is_read_only,
                            class: "w-auto my-1",
                            onchange: move |e| {
                                on_change.call(FieldValue::Boolean(e.checked()));
                            },
                        }
                    }
                }

                FieldType::Select { options } => {
                    let current = match &props.value {
                        Some(FieldValue::Text(s)) => s.clone(),
                        _ => options.first().map(|o| o.value.clone()).unwrap_or_default(),
                    };
                    let opts = options.clone();
                    let on_change = props.on_change.clone();

                    rsx! {
                        select {
                            value: "{current}",
                            disabled: is_read_only,
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                            onchange: move |e| {
                                on_change.call(FieldValue::Text(e.value()));
                            },

                            for opt in opts {
                                option {
                                    value: "{opt.value}",
                                    "{opt.label}"
                                }
                            }
                        }
                    }
                }

                FieldType::Resource { max_field: _, default_max } => {
                    let (current, max) = match &props.value {
                        Some(FieldValue::Resource { current, max }) => (*current, *max),
                        _ => (default_max.unwrap_or(10), default_max.unwrap_or(10)),
                    };
                    let on_change = props.on_change.clone();
                    let on_change2 = props.on_change.clone();

                    rsx! {
                        div {
                            class: "flex items-center gap-2",

                            input {
                                r#type: "number",
                                value: "{current}",
                                min: "0",
                                max: "{max}",
                                disabled: is_read_only,
                                class: "w-[60px] p-2 bg-dark-bg border border-gray-700 rounded text-white text-center",
                                oninput: move |e| {
                                    if let Ok(n) = e.value().parse::<i32>() {
                                        on_change.call(FieldValue::Resource { current: n, max });
                                    }
                                },
                            }

                            span { class: "text-gray-500", "/" }

                            input {
                                r#type: "number",
                                value: "{max}",
                                min: "1",
                                disabled: is_read_only,
                                class: "w-[60px] p-2 bg-dark-bg border border-gray-700 rounded text-white text-center",
                                oninput: move |e| {
                                    if let Ok(n) = e.value().parse::<i32>() {
                                        on_change2.call(FieldValue::Resource { current, max: n });
                                    }
                                },
                            }
                        }
                    }
                }

                FieldType::Derived { formula: _, depends_on: _ } => {
                    // Derived fields are read-only and calculated
                    let display = match &props.value {
                        Some(FieldValue::Number(n)) => n.to_string(),
                        Some(FieldValue::Text(s)) => s.clone(),
                        _ => "—".to_string(),
                    };

                    rsx! {
                        div {
                            class: "p-2 bg-black/30 border border-gray-700 rounded text-gray-400",
                            "{display}"
                            span {
                                class: "text-gray-500 text-xs ml-2",
                                "(calculated)"
                            }
                        }
                    }
                }

                FieldType::SkillReference { categories: _, show_attribute: _ } => {
                    // Display stored skill reference or placeholder
                    let display = match &props.value {
                        Some(FieldValue::Text(s)) => s.clone(),
                        Some(FieldValue::SkillEntry { skill_id, proficient, bonus }) => {
                            let prof = if *proficient { " (proficient)" } else { "" };
                            let sign = if *bonus >= 0 { "+" } else { "" };
                            format!("{}{} {}{}", skill_id, prof, sign, bonus)
                        }
                        _ => "No skill selected".to_string(),
                    };
                    rsx! {
                        div {
                            class: "p-2 bg-black/20 border border-gray-700 rounded text-gray-300 text-sm",
                            "{display}"
                        }
                    }
                }

                FieldType::ItemList { item_type: _, max_items } => {
                    // Display stored item list
                    let items = match &props.value {
                        Some(FieldValue::List(list)) => list.clone(),
                        _ => Vec::new(),
                    };
                    let max_display = max_items.map(|m| format!(" (max: {})", m)).unwrap_or_default();
                    rsx! {
                        div {
                            class: "p-2 bg-black/20 border border-gray-700 rounded",

                            if items.is_empty() {
                                span {
                                    class: "text-gray-500 text-sm",
                                    "No items{max_display}"
                                }
                            } else {
                                div {
                                    class: "flex flex-col gap-1",
                                    for item in items.iter() {
                                        div {
                                            class: "text-gray-300 text-sm py-1",
                                            "• {item}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                FieldType::SkillList { show_modifier, show_proficiency } => {
                    // Display stored skill list
                    let skills = match &props.value {
                        Some(FieldValue::List(list)) => list.clone(),
                        _ => Vec::new(),
                    };
                    let show_mod = *show_modifier;
                    let show_prof = *show_proficiency;
                    rsx! {
                        div {
                            class: "p-2 bg-black/20 border border-gray-700 rounded",

                            if skills.is_empty() {
                                span {
                                    class: "text-gray-500 text-sm",
                                    "No skills configured"
                                }
                            } else {
                                div {
                                    class: "flex flex-col gap-1",
                                    for skill in skills.iter() {
                                        div {
                                            class: "flex items-center gap-2 text-gray-300 text-sm py-1",
                                            span { "{skill}" }
                                            if show_prof {
                                                span {
                                                    class: "text-gray-500 text-xs",
                                                    "(prof)"
                                                }
                                            }
                                            if show_mod {
                                                span {
                                                    class: "text-gray-400 text-xs",
                                                    "+0"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Description/help text
            if let Some(desc) = &props.field.description {
                p {
                    class: "text-gray-500 text-xs mt-1 mb-0",
                    "{desc}"
                }
            }
        }
    }
}

/// Props for the full sheet template renderer
#[derive(Props, Clone, PartialEq)]
pub struct CharacterSheetFormProps {
    pub template: SheetTemplate,
    pub values: HashMap<String, FieldValue>,
    #[props(default)]
    pub on_change: EventHandler<(String, FieldValue)>,
    #[props(default)]
    pub on_values_change: EventHandler<HashMap<String, FieldValue>>,
    #[props(default = false)]
    pub read_only: bool,
}

/// Renders the entire character sheet form based on a template
#[component]
pub fn CharacterSheetForm(props: CharacterSheetFormProps) -> Element {
    // Sort sections by order
    let mut sorted_sections = props.template.sections.clone();
    sorted_sections.sort_by_key(|s| s.order);

    rsx! {
        div {
            class: "character-sheet-form flex flex-col gap-4",

            for section in sorted_sections {
                {
                    let section_id = section.id.clone();
                    rsx! {
                        SheetSectionInput {
                            key: "{section_id}",
                            section: section,
                            values: props.values.clone(),
                            on_change: props.on_change.clone(),
                            read_only: props.read_only,
                        }
                    }
                }
            }
        }
    }
}
