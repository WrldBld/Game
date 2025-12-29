//! Character Sheet Viewer - Read-only display of character stats for players

use dioxus::prelude::*;
use std::collections::HashMap;

use wrldbldr_player_app::application::dto::{
    FieldType, FieldValue, SheetField, SheetSection, SheetTemplate,
};

/// Props for the character sheet viewer
#[derive(Props, Clone, PartialEq)]
pub struct CharacterSheetViewerProps {
    /// The character's name
    pub character_name: String,
    /// The sheet template
    pub template: SheetTemplate,
    /// The character's values
    pub values: HashMap<String, FieldValue>,
    /// Handler for closing the viewer
    pub on_close: EventHandler<()>,
}

/// Character Sheet Viewer - modal overlay showing character stats
#[component]
pub fn CharacterSheetViewer(props: CharacterSheetViewerProps) -> Element {
    // Sort sections by order
    let mut sorted_sections = props.template.sections.clone();
    sorted_sections.sort_by_key(|s| s.order);

    rsx! {
        // Overlay background
        div {
            class: "character-sheet-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-8",
            onclick: move |_| props.on_close.call(()),

            // Sheet container (prevent click propagation)
            div {
                class: "character-sheet-modal bg-gradient-to-br from-dark-surface to-dark-gradient-end rounded-2xl w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col shadow-2xl",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "sheet-header flex justify-between items-center p-6 border-b-2 border-white/10 bg-black/20",

                    div {
                        h2 {
                            class: "text-gray-100 text-2xl m-0 font-semibold",
                            "{props.character_name}"
                        }
                        p {
                            class: "text-gray-400 text-sm mt-1 mb-0",
                            "{props.template.name}"
                        }
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "w-9 h-9 bg-white/10 border-0 rounded-lg text-gray-400 cursor-pointer text-xl flex items-center justify-center hover:bg-white/20",
                        "×"
                    }
                }

                // Scrollable content
                div {
                    class: "sheet-content flex-1 overflow-y-auto p-6",

                    div {
                        class: "flex flex-col gap-6",

                        for section in sorted_sections {
                            SheetSectionViewer {
                                key: "{section.id}",
                                section: section,
                                values: props.values.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for a section viewer
#[derive(Props, Clone, PartialEq)]
struct SheetSectionViewerProps {
    section: SheetSection,
    values: HashMap<String, FieldValue>,
}

/// Renders a read-only section of the character sheet
#[component]
fn SheetSectionViewer(props: SheetSectionViewerProps) -> Element {
    let mut is_collapsed = use_signal(|| props.section.collapsed_by_default);

    // CRITICAL: Extract layout classes BEFORE rsx! block - no inline conditionals in class strings
    let content_layout_class = match props.section.layout {
        wrldbldr_player_app::application::dto::SectionLayout::Vertical => "flex flex-col gap-2",
        wrldbldr_player_app::application::dto::SectionLayout::Grid { columns: _ } => {
            "grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3"
        }
        wrldbldr_player_app::application::dto::SectionLayout::TwoColumn => "grid grid-cols-2 gap-3",
        wrldbldr_player_app::application::dto::SectionLayout::Flow => "flex flex-wrap gap-3",
    };

    // Sort fields by order
    let mut sorted_fields = props.section.fields.clone();
    sorted_fields.sort_by_key(|f| f.order);

    rsx! {
        div {
            class: "sheet-section bg-black/20 rounded-xl overflow-hidden",

            // Section header
            div {
                class: "flex justify-between items-center px-4 py-3 bg-black/30 cursor-pointer",
                onclick: move |_| {
                    if props.section.collapsible {
                        let current = *is_collapsed.read();
                        is_collapsed.set(!current);
                    }
                },

                h3 {
                    class: "text-gray-200 text-sm m-0 font-semibold uppercase tracking-wide",
                    "{props.section.name}"
                }

                if props.section.collapsible {
                    span {
                        class: "text-gray-500 text-sm",
                        if *is_collapsed.read() { "+" } else { "−" }
                    }
                }
            }

            // Section content
            if !*is_collapsed.read() {
                div {
                    class: "p-4 {content_layout_class}",

                    for field in sorted_fields {
                        SheetFieldViewer {
                            key: "{field.id}",
                            field: field.clone(),
                            value: props.values.get(&field.id).cloned(),
                        }
                    }
                }
            }
        }
    }
}

/// Props for a field viewer
#[derive(Props, Clone, PartialEq)]
struct SheetFieldViewerProps {
    field: SheetField,
    value: Option<FieldValue>,
}

/// Renders a single read-only field
#[component]
fn SheetFieldViewer(props: SheetFieldViewerProps) -> Element {
    let value_display = format_field_value(&props.field.field_type, &props.value);

    // Check if this is a "large" field type
    let is_resource = matches!(props.field.field_type, FieldType::Resource { .. });

    rsx! {
        div {
            class: "sheet-field flex flex-col",

            // Label
            span {
                class: "text-gray-500 text-xs mb-1",
                "{props.field.name}"
            }

            // Value display
            if is_resource {
                // Resource bar display
                {
                    if let Some(FieldValue::Resource { current, max }) = &props.value {
                        let percentage = if *max > 0 { (*current as f32 / *max as f32) * 100.0 } else { 0.0 };
                        // CRITICAL: Extract bar color class BEFORE rsx! - no inline if in class strings
                        let bar_color_class = if percentage > 50.0 {
                            "bg-green-500"
                        } else if percentage > 25.0 {
                            "bg-amber-500"
                        } else {
                            "bg-red-500"
                        };
                        let width_style = format!("width: {}%", percentage);

                        rsx! {
                            div {
                                class: "flex flex-col gap-1",

                                // Text value
                                span {
                                    class: "text-gray-100 text-base font-medium",
                                    "{current} / {max}"
                                }

                                // Progress bar
                                div {
                                    class: "h-1.5 bg-black/30 rounded overflow-hidden",

                                    div {
                                        class: "h-full {bar_color_class} transition-all duration-300",
                                        style: "{width_style}",
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            span {
                                class: "text-gray-500 text-base",
                                "—"
                            }
                        }
                    }
                }
            } else {
                // Regular value display
                span {
                    class: "text-gray-100 text-lg font-medium",
                    "{value_display}"
                }
            }
        }
    }
}

/// Format a field value for display
fn format_field_value(field_type: &FieldType, value: &Option<FieldValue>) -> String {
    match (field_type, value) {
        (FieldType::Number { .. }, Some(FieldValue::Number(n))) => {
            // Format with sign for modifiers
            if *n >= 0 {
                format!("+{}", n)
            } else {
                n.to_string()
            }
        }
        (FieldType::Number { default, .. }, None) => default
            .map(|d| format!("+{}", d))
            .unwrap_or_else(|| "0".to_string()),
        (FieldType::Text { .. }, Some(FieldValue::Text(s))) => s.clone(),
        (FieldType::Text { .. }, None) => "—".to_string(),
        (FieldType::Checkbox { .. }, Some(FieldValue::Boolean(b))) => {
            if *b { "Yes" } else { "No" }.to_string()
        }
        (FieldType::Checkbox { default }, None) => if *default { "Yes" } else { "No" }.to_string(),
        (FieldType::Select { options }, Some(FieldValue::Text(s))) => {
            // Find the label for the selected value
            options
                .iter()
                .find(|o| o.value == *s)
                .map(|o| o.label.clone())
                .unwrap_or_else(|| s.clone())
        }
        (FieldType::Select { options }, None) => options
            .first()
            .map(|o| o.label.clone())
            .unwrap_or_else(|| "—".to_string()),
        (FieldType::Resource { .. }, _) => {
            // Handled specially in the component
            "—".to_string()
        }
        (FieldType::Derived { .. }, Some(FieldValue::Number(n))) => n.to_string(),
        (FieldType::Derived { .. }, Some(FieldValue::Text(s))) => s.clone(),
        (FieldType::Derived { .. }, _) => "—".to_string(),
        (FieldType::SkillReference { .. }, _) => "—".to_string(),
        (FieldType::ItemList { .. }, _) => "—".to_string(),
        (FieldType::SkillList { .. }, _) => "—".to_string(),
        _ => "—".to_string(),
    }
}
