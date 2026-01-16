//! Visual Trigger Condition Builder
//!
//! A visual builder for creating narrative event trigger conditions.
//! Allows DMs to build complex trigger logic using dropdowns and forms
//! without writing JSON manually.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::infrastructure::spawn_task;
use crate::presentation::components::common::CharacterPicker;
use crate::presentation::Services;
use wrldbldr_shared::{NarrativeEventRequest, RequestPayload};

// =============================================================================
// Schema Types (mirrors protocol types for local use)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerSchema {
    pub trigger_types: Vec<TriggerTypeSchema>,
    pub logic_options: Vec<TriggerLogicOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerTypeSchema {
    pub type_name: String,
    pub label: String,
    pub description: String,
    pub category: String,
    pub fields: Vec<TriggerFieldSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerFieldSchema {
    pub name: String,
    pub label: String,
    pub field_type: String,
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_value: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerLogicOption {
    pub value: String,
    pub label: String,
    pub description: String,
    pub requires_count: bool,
}

// =============================================================================
// Trigger Condition Data
// =============================================================================

/// A single trigger condition being built
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerCondition {
    /// Unique ID for this condition
    pub id: String,
    /// The trigger type name (e.g., "PlayerEntersLocation")
    pub trigger_type: String,
    /// Field values as JSON object
    pub values: serde_json::Map<String, JsonValue>,
    /// Human-readable description
    pub description: String,
    /// Whether this condition is required (for AtLeast logic)
    pub is_required: bool,
}

impl TriggerCondition {
    pub fn new(trigger_type: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger_type: trigger_type.to_string(),
            values: serde_json::Map::new(),
            description: String::new(),
            is_required: false,
        }
    }

    /// Convert to the domain NarrativeTrigger format
    pub fn to_trigger_json(&self) -> JsonValue {
        let mut trigger_type_obj = serde_json::Map::new();
        trigger_type_obj.insert(
            "type".to_string(),
            JsonValue::String(self.trigger_type.clone()),
        );
        for (key, value) in &self.values {
            trigger_type_obj.insert(key.clone(), value.clone());
        }

        serde_json::json!({
            "triggerId": self.id,
            "triggerType": trigger_type_obj,
            "description": self.description,
            "isRequired": self.is_required
        })
    }
}

// =============================================================================
// Component Props
// =============================================================================

#[derive(Props, Clone, PartialEq)]
pub struct TriggerBuilderProps {
    /// World ID for entity pickers
    pub world_id: String,
    /// Current trigger conditions (as JSON array)
    #[props(default)]
    pub initial_conditions: Option<Vec<JsonValue>>,
    /// Current trigger logic
    #[props(default = "all".to_string())]
    pub initial_logic: String,
    /// AtLeast count (when logic is "atLeast")
    #[props(default = 1)]
    pub initial_at_least_count: u32,
    /// Callback when conditions change
    pub on_change: EventHandler<(Vec<JsonValue>, String, u32)>,
}

// =============================================================================
// Main Component
// =============================================================================

#[component]
pub fn TriggerBuilder(props: TriggerBuilderProps) -> Element {
    let services = use_context::<Services<crate::application::api::Api>>();

    // Schema loaded from server
    let mut schema: Signal<Option<TriggerSchema>> = use_signal(|| None);
    let mut schema_loading = use_signal(|| true);
    let mut schema_error: Signal<Option<String>> = use_signal(|| None);

    // Builder state
    let mut conditions: Signal<Vec<TriggerCondition>> = use_signal(Vec::new);
    let mut logic = use_signal(|| props.initial_logic.clone());
    let mut at_least_count = use_signal(|| props.initial_at_least_count);
    let mut expanded_condition: Signal<Option<String>> = use_signal(|| None);

    // Load schema on mount
    {
        let commands = services.command_bus.clone();
        use_effect(move || {
            let commands = commands.clone();
            spawn_task(async move {
                schema_loading.set(true);
                schema_error.set(None);

                let payload =
                    RequestPayload::NarrativeEvent(NarrativeEventRequest::GetTriggerSchema);
                match commands.request(payload).await {
                    Ok(response) => match response {
                        wrldbldr_shared::ResponseResult::Success { data } => {
                            if let Some(json_data) = data {
                                match serde_json::from_value::<TriggerSchema>(json_data) {
                                    Ok(s) => schema.set(Some(s)),
                                    Err(e) => schema_error
                                        .set(Some(format!("Failed to parse schema: {}", e))),
                                }
                            } else {
                                schema_error.set(Some("Empty response".to_string()));
                            }
                        }
                        wrldbldr_shared::ResponseResult::Error { message, .. } => {
                            schema_error.set(Some(message));
                        }
                        wrldbldr_shared::ResponseResult::Unknown => {
                            schema_error.set(Some("Unknown response type".to_string()));
                        }
                    },
                    Err(e) => schema_error.set(Some(format!("Failed to load schema: {}", e))),
                }
                schema_loading.set(false);
            });
        });
    }

    // Parse initial conditions
    {
        let initial = props.initial_conditions.clone();
        use_effect(move || {
            if let Some(init_conditions) = initial.as_ref() {
                let parsed: Vec<TriggerCondition> = init_conditions
                    .iter()
                    .filter_map(|v| {
                        let trigger_type = v.get("triggerType")?.get("type")?.as_str()?;
                        let mut values = serde_json::Map::new();
                        if let Some(obj) = v.get("triggerType").and_then(|t| t.as_object()) {
                            for (k, val) in obj {
                                if k != "type" {
                                    values.insert(k.clone(), val.clone());
                                }
                            }
                        }
                        Some(TriggerCondition {
                            id: v
                                .get("triggerId")
                                .and_then(|i| i.as_str())
                                .unwrap_or(&uuid::Uuid::new_v4().to_string())
                                .to_string(),
                            trigger_type: trigger_type.to_string(),
                            values,
                            description: v
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string(),
                            is_required: v
                                .get("isRequired")
                                .and_then(|r| r.as_bool())
                                .unwrap_or(false),
                        })
                    })
                    .collect();
                conditions.set(parsed);
            }
        });
    }

    // Notify parent of changes
    let notify_change = {
        let on_change = props.on_change.clone();
        move || {
            let conds: Vec<JsonValue> = conditions
                .read()
                .iter()
                .map(|c| c.to_trigger_json())
                .collect();
            let logic_val = logic.read().clone();
            let count = *at_least_count.read();
            on_change.call((conds, logic_val, count));
        }
    };

    // Add a new condition
    let add_condition = {
        let notify = notify_change.clone();
        move |type_name: String| {
            let mut conds = conditions.write();
            let new_cond = TriggerCondition::new(&type_name);
            let new_id = new_cond.id.clone();
            conds.push(new_cond);
            drop(conds);
            expanded_condition.set(Some(new_id));
            notify();
        }
    };

    // Remove a condition
    let remove_condition = {
        let notify = notify_change.clone();
        move |id: String| {
            let mut conds = conditions.write();
            conds.retain(|c| c.id != id);
            drop(conds);
            notify();
        }
    };

    // Update a condition field
    let update_field = {
        let notify = notify_change.clone();
        move |cond_id: String, field_name: String, value: JsonValue| {
            let mut conds = conditions.write();
            if let Some(cond) = conds.iter_mut().find(|c| c.id == cond_id) {
                cond.values.insert(field_name, value);
            }
            drop(conds);
            notify();
        }
    };

    // Update condition description
    let update_description = {
        let notify = notify_change.clone();
        move |cond_id: String, desc: String| {
            let mut conds = conditions.write();
            if let Some(cond) = conds.iter_mut().find(|c| c.id == cond_id) {
                cond.description = desc;
            }
            drop(conds);
            notify();
        }
    };

    // Toggle is_required
    let toggle_required = {
        let notify = notify_change.clone();
        move |cond_id: String| {
            let mut conds = conditions.write();
            if let Some(cond) = conds.iter_mut().find(|c| c.id == cond_id) {
                cond.is_required = !cond.is_required;
            }
            drop(conds);
            notify();
        }
    };

    // Loading state
    if *schema_loading.read() {
        return rsx! {
            div { class: "flex items-center justify-center p-8",
                div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" }
                span { class: "ml-3 text-gray-400", "Loading trigger schema..." }
            }
        };
    }

    // Error state
    if let Some(err) = schema_error.read().as_ref() {
        return rsx! {
            div { class: "p-4 bg-red-900/20 border border-red-500 rounded-lg",
                p { class: "text-red-400", "{err}" }
            }
        };
    }

    // Get schema
    let schema_data = match schema.read().as_ref() {
        Some(s) => s.clone(),
        None => return rsx! { div { "No schema available" } },
    };

    // Group trigger types by category
    let categories: Vec<String> = {
        let mut cats: Vec<String> = schema_data
            .trigger_types
            .iter()
            .map(|t| t.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    };

    let world_id = props.world_id.clone();

    rsx! {
        div { class: "space-y-4",
            // Logic selector
            div { class: "flex items-center gap-4 p-4 bg-gray-800 rounded-lg",
                label { class: "text-sm font-medium text-gray-300", "Logic:" }
                select {
                    class: "bg-gray-700 border border-gray-600 rounded px-3 py-1.5 text-white",
                    value: "{logic}",
                    onchange: {
                        let notify = notify_change.clone();
                        move |e: Event<FormData>| {
                            logic.set(e.value().clone());
                            notify();
                        }
                    },
                    for opt in &schema_data.logic_options {
                        option {
                            value: "{opt.value}",
                            "{opt.label}"
                        }
                    }
                }

                // AtLeast count input
                if *logic.read() == "atLeast" {
                    input {
                        r#type: "number",
                        class: "w-16 bg-gray-700 border border-gray-600 rounded px-2 py-1.5 text-white",
                        value: "{at_least_count}",
                        min: "1",
                        onchange: {
                            let notify = notify_change.clone();
                            move |e: Event<FormData>| {
                                if let Ok(n) = e.value().parse::<u32>() {
                                    at_least_count.set(n);
                                    notify();
                                }
                            }
                        }
                    }
                    span { class: "text-gray-400 text-sm", "conditions must match" }
                }

                // Logic description
                if let Some(opt) = schema_data.logic_options.iter().find(|o| o.value == *logic.read()) {
                    span { class: "text-gray-500 text-sm italic ml-2", "({opt.description})" }
                }
            }

            // Conditions list
            div { class: "space-y-3",
                for (idx, cond) in conditions.read().iter().enumerate() {
                    {
                        let cond_id = cond.id.clone();
                        let is_expanded = expanded_condition.read().as_ref() == Some(&cond_id);
                        let type_schema = schema_data.trigger_types.iter().find(|t| t.type_name == cond.trigger_type);

                        rsx! {
                            TriggerConditionCard {
                                key: "{cond_id}",
                                index: idx,
                                condition: cond.clone(),
                                type_schema: type_schema.cloned(),
                                world_id: world_id.clone(),
                                is_expanded: is_expanded,
                                on_toggle_expand: move |_| {
                                    if is_expanded {
                                        expanded_condition.set(None);
                                    } else {
                                        expanded_condition.set(Some(cond_id.clone()));
                                    }
                                },
                                on_remove: {
                                    let mut remove = remove_condition.clone();
                                    let id = cond.id.clone();
                                    move |_| remove(id.clone())
                                },
                                on_update_field: {
                                    let mut update = update_field.clone();
                                    let id = cond.id.clone();
                                    move |(field, value): (String, JsonValue)| update(id.clone(), field, value)
                                },
                                on_update_description: {
                                    let mut update = update_description.clone();
                                    let id = cond.id.clone();
                                    move |desc: String| update(id.clone(), desc)
                                },
                                on_toggle_required: {
                                    let mut toggle = toggle_required.clone();
                                    let id = cond.id.clone();
                                    move |_| toggle(id.clone())
                                },
                            }
                        }
                    }
                }
            }

            // Add condition dropdown
            div { class: "mt-4",
                details { class: "group",
                    summary { class: "cursor-pointer list-none",
                        div { class: "inline-flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium transition-colors",
                            svg { class: "w-5 h-5",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M12 4v16m8-8H4"
                                }
                            }
                            "Add Condition"
                        }
                    }

                    div { class: "mt-2 p-4 bg-gray-800 rounded-lg border border-gray-700 max-h-96 overflow-y-auto",
                        for category in &categories {
                            div { class: "mb-4",
                                h4 { class: "text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2",
                                    "{category}"
                                }
                                div { class: "grid grid-cols-2 gap-2",
                                    for trigger_type in schema_data.trigger_types.iter().filter(|t| t.category == *category) {
                                        {
                                            let type_name = trigger_type.type_name.clone();
                                            let mut add = add_condition.clone();
                                            rsx! {
                                                button {
                                                    class: "text-left px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-sm text-gray-200 transition-colors",
                                                    title: "{trigger_type.description}",
                                                    onclick: move |_| add(type_name.clone()),
                                                    "{trigger_type.label}"
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

            // Empty state
            if conditions.read().is_empty() {
                div { class: "text-center py-8 text-gray-500",
                    p { "No trigger conditions defined." }
                    p { class: "text-sm mt-1", "Click \"Add Condition\" to create trigger rules." }
                }
            }
        }
    }
}

// =============================================================================
// Trigger Condition Card
// =============================================================================

#[derive(Props, Clone, PartialEq)]
struct TriggerConditionCardProps {
    index: usize,
    condition: TriggerCondition,
    type_schema: Option<TriggerTypeSchema>,
    world_id: String,
    is_expanded: bool,
    on_toggle_expand: EventHandler<()>,
    on_remove: EventHandler<()>,
    on_update_field: EventHandler<(String, JsonValue)>,
    on_update_description: EventHandler<String>,
    on_toggle_required: EventHandler<()>,
}

#[component]
fn TriggerConditionCard(props: TriggerConditionCardProps) -> Element {
    let type_label = props
        .type_schema
        .as_ref()
        .map(|s| s.label.clone())
        .unwrap_or_else(|| props.condition.trigger_type.clone());

    let type_desc = props
        .type_schema
        .as_ref()
        .map(|s| s.description.clone())
        .unwrap_or_default();

    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
            // Header
            div { class: "flex items-center justify-between px-4 py-3 bg-gray-750 cursor-pointer hover:bg-gray-700 transition-colors",
                onclick: move |_| props.on_toggle_expand.call(()),

                div { class: "flex items-center gap-3",
                    span { class: "text-gray-500 font-mono text-sm", "{props.index + 1}." }
                    div {
                        h4 { class: "font-medium text-white", "{type_label}" }
                        if !props.condition.description.is_empty() {
                            p { class: "text-sm text-gray-400 truncate max-w-md",
                                "{props.condition.description}"
                            }
                        }
                    }
                }

                div { class: "flex items-center gap-2",
                    if props.condition.is_required {
                        span { class: "px-2 py-0.5 bg-yellow-600/30 text-yellow-400 text-xs rounded",
                            "Required"
                        }
                    }

                    // Expand/collapse icon
                    svg {
                        class: if props.is_expanded { "w-5 h-5 text-gray-400 transform rotate-180" } else { "w-5 h-5 text-gray-400" },
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M19 9l-7 7-7-7"
                        }
                    }

                    // Remove button
                    button {
                        class: "p-1 text-gray-500 hover:text-red-400 transition-colors",
                        title: "Remove condition",
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_remove.call(());
                        },
                        svg { class: "w-5 h-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M6 18L18 6M6 6l12 12"
                            }
                        }
                    }
                }
            }

            // Expanded content
            if props.is_expanded {
                div { class: "px-4 py-4 border-t border-gray-700 space-y-4",
                    // Description
                    p { class: "text-sm text-gray-400 mb-4", "{type_desc}" }

                    // Description input
                    div {
                        label { class: "block text-sm font-medium text-gray-400 mb-1",
                            "Description (for DM reference)"
                        }
                        input {
                            r#type: "text",
                            class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white placeholder-gray-500",
                            placeholder: "e.g., \"When player enters the tavern\"",
                            value: "{props.condition.description}",
                            onchange: move |e: Event<FormData>| {
                                props.on_update_description.call(e.value().clone());
                            }
                        }
                    }

                    // Required checkbox
                    label { class: "flex items-center gap-2 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            class: "rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500",
                            checked: props.condition.is_required,
                            onchange: move |_| props.on_toggle_required.call(())
                        }
                        span { class: "text-sm text-gray-300", "Required (must be true even with AtLeast logic)" }
                    }

                    // Fields
                    if let Some(schema) = &props.type_schema {
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mt-4",
                            for field in &schema.fields {
                                TriggerFieldInput {
                                    field: field.clone(),
                                    value: props.condition.values.get(&field.name).cloned(),
                                    world_id: props.world_id.clone(),
                                    on_change: {
                                        let field_name = field.name.clone();
                                        move |value: JsonValue| {
                                            props.on_update_field.call((field_name.clone(), value));
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
}

// =============================================================================
// Trigger Field Input
// =============================================================================

#[derive(Props, Clone, PartialEq)]
struct TriggerFieldInputProps {
    field: TriggerFieldSchema,
    value: Option<JsonValue>,
    world_id: String,
    on_change: EventHandler<JsonValue>,
}

#[component]
fn TriggerFieldInput(props: TriggerFieldInputProps) -> Element {
    let current_value = props
        .value
        .clone()
        .unwrap_or(props.field.default_value.clone().unwrap_or(JsonValue::Null));

    let field_type = props.field.field_type.as_str();
    let placeholder = props.field.description.as_deref().unwrap_or("");
    let str_value = current_value.as_str().unwrap_or("").to_string();
    let int_value = current_value
        .as_i64()
        .map(|n| n.to_string())
        .unwrap_or_default();
    let float_value = current_value
        .as_f64()
        .map(|n| format!("{:.1}", n))
        .unwrap_or_default();
    let bool_value = current_value.as_bool().unwrap_or(false);
    let keywords_str = current_value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-400 mb-1",
                "{props.field.label}"
                if props.field.required {
                    span { class: "text-red-400 ml-1", "*" }
                }
            }

            // Render input based on field type
            if field_type == "string" {
                input {
                    r#type: "text",
                    class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white placeholder-gray-500",
                    placeholder: "{placeholder}",
                    value: "{str_value}",
                    onchange: move |e: Event<FormData>| {
                        props.on_change.call(JsonValue::String(e.value().clone()));
                    }
                }
            } else if field_type == "integer" {
                input {
                    r#type: "number",
                    class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white",
                    value: "{int_value}",
                    onchange: move |e: Event<FormData>| {
                        if let Ok(n) = e.value().parse::<i64>() {
                            props.on_change.call(JsonValue::Number(n.into()));
                        }
                    }
                }
            } else if field_type == "float" || field_type == "sentiment" {
                input {
                    r#type: "number",
                    step: "0.1",
                    class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white",
                    value: "{float_value}",
                    onchange: move |e: Event<FormData>| {
                        if let Ok(n) = e.value().parse::<f64>() {
                            props.on_change.call(serde_json::json!(n));
                        }
                    }
                }
            } else if field_type == "boolean" {
                label { class: "flex items-center gap-2 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded bg-gray-700 border-gray-600 text-blue-500",
                        checked: bool_value,
                        onchange: move |e: Event<FormData>| {
                            props.on_change.call(JsonValue::Bool(e.value() == "true"));
                        }
                    }
                    span { class: "text-sm text-gray-300", "Enabled" }
                }
            } else if field_type == "characterRef" {
                CharacterPicker {
                    world_id: props.world_id.clone(),
                    value: str_value.clone(),
                    on_change: move |id: String| {
                        props.on_change.call(JsonValue::String(id));
                    }
                }
            } else if field_type == "keywords" {
                input {
                    r#type: "text",
                    class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white placeholder-gray-500",
                    placeholder: "Enter keywords, separated by commas",
                    value: "{keywords_str}",
                    onchange: move |e: Event<FormData>| {
                        let kws: Vec<JsonValue> = e.value()
                            .split(',')
                            .map(|s| JsonValue::String(s.trim().to_string()))
                            .filter(|v| !v.as_str().unwrap_or("").is_empty())
                            .collect();
                        props.on_change.call(JsonValue::Array(kws));
                    }
                }
            } else if field_type == "timeOfDay" {
                select {
                    class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white",
                    value: "{str_value}",
                    onchange: move |e: Event<FormData>| {
                        props.on_change.call(JsonValue::String(e.value().clone()));
                    },
                    option { value: "", "Select time..." }
                    option { value: "morning", "Morning (5am-12pm)" }
                    option { value: "afternoon", "Afternoon (12pm-6pm)" }
                    option { value: "evening", "Evening (6pm-10pm)" }
                    option { value: "night", "Night (10pm-5am)" }
                }
            } else {
                // Default: locationRef, regionRef, challengeRef, eventRef, itemRef, or unknown
                input {
                    r#type: "text",
                    class: "w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white placeholder-gray-500",
                    placeholder: "Enter ID or name",
                    value: "{str_value}",
                    onchange: move |e: Event<FormData>| {
                        props.on_change.call(JsonValue::String(e.value().clone()));
                    }
                }
            }

            if let Some(desc) = &props.field.description {
                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
            }
        }
    }
}
