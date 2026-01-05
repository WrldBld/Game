//! Expression Configuration Editor
//!
//! UI component for editing a character's expression config:
//! - Default mood dropdown (Tier 2)
//! - Available expressions list (Tier 3)
//! - Available actions list (for stage directions)
//!
//! Part of the three-tier emotional model implementation.

use dioxus::prelude::*;

use wrldbldr_domain::{ExpressionConfig, MoodState};

/// Props for the ExpressionConfigEditor component
#[derive(Props, Clone, PartialEq)]
pub struct ExpressionConfigEditorProps {
    /// Current expression config
    pub config: ExpressionConfig,
    /// Current default mood for the character
    pub default_mood: MoodState,
    /// Callback when config changes
    pub on_config_change: EventHandler<ExpressionConfig>,
    /// Callback when default mood changes
    pub on_mood_change: EventHandler<MoodState>,
}

/// Expression configuration editor component
///
/// Allows DMs to configure a character's available expressions and actions.
#[component]
pub fn ExpressionConfigEditor(props: ExpressionConfigEditorProps) -> Element {
    // Local state for new expression/action input
    let mut new_expression = use_signal(String::new);
    let mut new_action = use_signal(String::new);
    let mut show_standard_expressions = use_signal(|| false);
    let mut show_standard_actions = use_signal(|| false);

    // Clone config for modifications
    let config = props.config.clone();

    rsx! {
        div {
            class: "expression-config-editor flex flex-col gap-4",

            // Default Mood dropdown
            div {
                class: "mood-section",

                label {
                    class: "block text-sm text-gray-400 mb-2",
                    "Default Mood"
                    span {
                        class: "ml-2 text-gray-600 text-xs",
                        "(affects starting expression and dialogue tone)"
                    }
                }

                select {
                    class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                    value: "{props.default_mood.display_name()}",
                    onchange: move |e| {
                        if let Ok(mood) = e.value().parse::<MoodState>() {
                            props.on_mood_change.call(mood);
                        }
                    },

                    for mood in MoodState::all() {
                        option {
                            value: "{mood.display_name().to_lowercase()}",
                            selected: *mood == props.default_mood,
                            "{mood.emoji()} {mood.display_name()}"
                        }
                    }
                }

                // Mood description
                div {
                    class: "text-xs text-gray-500 mt-1",
                    "{props.default_mood.description()}"
                }
            }

            // Expressions section
            div {
                class: "expressions-section",

                div {
                    class: "flex justify-between items-center mb-2",

                    label {
                        class: "text-sm text-gray-400",
                        "Available Expressions"
                        span {
                            class: "ml-2 text-gray-600 text-xs",
                            "({config.expressions.len()})"
                        }
                    }

                    button {
                        class: "text-xs text-purple-400 hover:text-purple-300",
                        onclick: move |_| {
                            let current = *show_standard_expressions.read();
                            show_standard_expressions.set(!current);
                        },
                        if *show_standard_expressions.read() { "Hide suggestions" } else { "Show suggestions" }
                    }
                }

                // Standard expressions quick-add
                if *show_standard_expressions.read() {
                    div {
                        class: "flex flex-wrap gap-1 mb-2 p-2 bg-purple-900/20 rounded border border-purple-800/30",

                        for expr in ExpressionConfig::standard_expressions() {
                            {
                                let has_expr = config.expressions.iter().any(|e| e.eq_ignore_ascii_case(&expr));
                                let expr_clone = expr.clone();
                                let config_clone = config.clone();
                                rsx! {
                                    button {
                                        key: "{expr}",
                                        class: format!(
                                            "px-2 py-0.5 text-xs rounded {}",
                                            if has_expr {
                                                "bg-purple-600/50 text-purple-300 cursor-not-allowed"
                                            } else {
                                                "bg-purple-700 text-white hover:bg-purple-600 cursor-pointer"
                                            }
                                        ),
                                        disabled: has_expr,
                                        onclick: move |_| {
                                            if !has_expr {
                                                let mut new_config = config_clone.clone();
                                                new_config.add_expression(expr_clone.clone());
                                                props.on_config_change.call(new_config);
                                            }
                                        },
                                        if has_expr { "✓ " } else { "+ " }
                                        "{expr}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Current expressions tags
                div {
                    class: "flex flex-wrap gap-1 mb-2",

                    for (idx, expr) in config.expressions.iter().enumerate() {
                        {
                            let expr_clone = expr.clone();
                            let config_clone = config.clone();
                            let is_default = expr.eq_ignore_ascii_case(&config.default_expression);
                            rsx! {
                                span {
                                    key: "{idx}-{expr}",
                                    class: format!(
                                        "inline-flex items-center gap-1 px-2 py-1 rounded text-sm {}",
                                        if is_default {
                                            "bg-purple-600 text-white"
                                        } else {
                                            "bg-gray-700 text-gray-300"
                                        }
                                    ),

                                    "{expr}"

                                    if is_default {
                                        span {
                                            class: "text-xs text-purple-200",
                                            "(default)"
                                        }
                                    }

                                    // Remove button
                                    button {
                                        class: "ml-1 text-gray-400 hover:text-red-400",
                                        onclick: move |_| {
                                            let mut new_config = config_clone.clone();
                                            new_config.expressions.retain(|e| e != &expr_clone);
                                            // If removed the default, reset to first available or "neutral"
                                            if expr_clone.eq_ignore_ascii_case(&new_config.default_expression) {
                                                new_config.default_expression = new_config.expressions
                                                    .first()
                                                    .cloned()
                                                    .unwrap_or_else(|| "neutral".to_string());
                                            }
                                            props.on_config_change.call(new_config);
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }

                // Add new expression
                div {
                    class: "flex gap-2",

                    input {
                        class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                        r#type: "text",
                        placeholder: "Add expression...",
                        value: "{new_expression}",
                        oninput: move |e| new_expression.set(e.value()),
                        onkeypress: {
                            let config_for_add = config.clone();
                            move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    let expr = new_expression.read().trim().to_string();
                                    if !expr.is_empty() && !config_for_add.has_expression(&expr) {
                                        let mut new_config = config_for_add.clone();
                                        new_config.add_expression(expr);
                                        props.on_config_change.call(new_config);
                                        new_expression.set(String::new());
                                    }
                                }
                            }
                        },
                    }

                    button {
                        class: "px-3 py-2 bg-purple-600 text-white rounded text-sm hover:bg-purple-500",
                        onclick: {
                            let config_for_btn = config.clone();
                            move |_| {
                                let expr = new_expression.read().trim().to_string();
                                if !expr.is_empty() && !config_for_btn.has_expression(&expr) {
                                    let mut new_config = config_for_btn.clone();
                                    new_config.add_expression(expr);
                                    props.on_config_change.call(new_config);
                                    new_expression.set(String::new());
                                }
                            }
                        },
                        "Add"
                    }
                }

                // Default expression selector
                if !config.expressions.is_empty() {
                    div {
                        class: "mt-2",

                        label {
                            class: "text-xs text-gray-500",
                            "Default expression: "
                        }

                        select {
                            class: "ml-2 px-2 py-1 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                            value: "{config.default_expression}",
                            onchange: {
                                let config_for_select = config.clone();
                                move |e| {
                                    let mut new_config = config_for_select.clone();
                                    new_config.default_expression = e.value();
                                    props.on_config_change.call(new_config);
                                }
                            },

                            for expr in config.expressions.iter() {
                                option {
                                    value: "{expr}",
                                    selected: expr == &config.default_expression,
                                    "{expr}"
                                }
                            }
                        }
                    }
                }
            }

            // Actions section
            div {
                class: "actions-section",

                div {
                    class: "flex justify-between items-center mb-2",

                    label {
                        class: "text-sm text-gray-400",
                        "Available Actions"
                        span {
                            class: "ml-2 text-gray-600 text-xs",
                            "({config.actions.len()})"
                        }
                    }

                    button {
                        class: "text-xs text-amber-400 hover:text-amber-300",
                        onclick: move |_| {
                            let current = *show_standard_actions.read();
                            show_standard_actions.set(!current);
                        },
                        if *show_standard_actions.read() { "Hide suggestions" } else { "Show suggestions" }
                    }
                }

                // Standard actions quick-add
                if *show_standard_actions.read() {
                    div {
                        class: "flex flex-wrap gap-1 mb-2 p-2 bg-amber-900/20 rounded border border-amber-800/30",

                        for action in ExpressionConfig::standard_actions() {
                            {
                                let has_action = config.actions.iter().any(|a| a.eq_ignore_ascii_case(&action));
                                let action_clone = action.clone();
                                let config_clone = config.clone();
                                rsx! {
                                    button {
                                        key: "{action}",
                                        class: format!(
                                            "px-2 py-0.5 text-xs rounded {}",
                                            if has_action {
                                                "bg-amber-600/50 text-amber-300 cursor-not-allowed"
                                            } else {
                                                "bg-amber-700 text-white hover:bg-amber-600 cursor-pointer"
                                            }
                                        ),
                                        disabled: has_action,
                                        onclick: move |_| {
                                            if !has_action {
                                                let mut new_config = config_clone.clone();
                                                new_config.add_action(action_clone.clone());
                                                props.on_config_change.call(new_config);
                                            }
                                        },
                                        if has_action { "✓ " } else { "+ " }
                                        "{action}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Current actions tags
                div {
                    class: "flex flex-wrap gap-1 mb-2",

                    for (idx, action) in config.actions.iter().enumerate() {
                        {
                            let action_clone = action.clone();
                            let config_clone = config.clone();
                            rsx! {
                                span {
                                    key: "{idx}-{action}",
                                    class: "inline-flex items-center gap-1 px-2 py-1 bg-amber-900/50 text-amber-200 rounded text-sm",

                                    "[{action}]"

                                    // Remove button
                                    button {
                                        class: "ml-1 text-amber-400 hover:text-red-400",
                                        onclick: move |_| {
                                            let mut new_config = config_clone.clone();
                                            new_config.actions.retain(|a| a != &action_clone);
                                            props.on_config_change.call(new_config);
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }

                // Add new action
                div {
                    class: "flex gap-2",

                    input {
                        class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                        r#type: "text",
                        placeholder: "Add action (e.g., sighs, waves)...",
                        value: "{new_action}",
                        oninput: move |e| new_action.set(e.value()),
                        onkeypress: {
                            let config_for_add = config.clone();
                            move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    let action = new_action.read().trim().to_string();
                                    if !action.is_empty() && !config_for_add.has_action(&action) {
                                        let mut new_config = config_for_add.clone();
                                        new_config.add_action(action);
                                        props.on_config_change.call(new_config);
                                        new_action.set(String::new());
                                    }
                                }
                            }
                        },
                    }

                    button {
                        class: "px-3 py-2 bg-amber-600 text-white rounded text-sm hover:bg-amber-500",
                        onclick: {
                            let config_for_btn = config.clone();
                            move |_| {
                                let action = new_action.read().trim().to_string();
                                if !action.is_empty() && !config_for_btn.has_action(&action) {
                                    let mut new_config = config_for_btn.clone();
                                    new_config.add_action(action);
                                    props.on_config_change.call(new_config);
                                    new_action.set(String::new());
                                }
                            }
                        },
                        "Add"
                    }
                }
            }

            // Preview section
            div {
                class: "preview-section mt-4 p-3 bg-gray-800/50 rounded border border-gray-700",

                h4 {
                    class: "text-sm text-gray-400 mb-2",
                    "LLM Context Preview"
                }

                pre {
                    class: "text-xs text-gray-300 whitespace-pre-wrap",
                    "{config.format_for_llm()}"
                }
            }
        }
    }
}

/// Compact version of the editor for inline use
#[component]
pub fn ExpressionConfigSummary(config: ExpressionConfig, default_mood: MoodState) -> Element {
    rsx! {
        div {
            class: "expression-config-summary flex flex-wrap gap-2 text-xs",

            // Mood badge
            span {
                class: "px-2 py-1 bg-amber-900/50 text-amber-200 rounded",
                "{default_mood.emoji()} {default_mood.display_name()}"
            }

            // Expression count
            span {
                class: "px-2 py-1 bg-purple-900/50 text-purple-200 rounded",
                "{config.expressions.len()} expressions"
            }

            // Action count
            if !config.actions.is_empty() {
                span {
                    class: "px-2 py-1 bg-gray-700 text-gray-300 rounded",
                    "{config.actions.len()} actions"
                }
            }
        }
    }
}
