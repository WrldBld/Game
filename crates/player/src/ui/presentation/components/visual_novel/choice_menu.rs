//! Choice menu component for dialogue choices
//!
//! Displays dialogue choices and handles custom input.
//! Supports expression marker validation from the three-tier emotional model.

use dioxus::prelude::*;

use wrldbldr_domain::{parse_dialogue_markers, validate_markers, ExpressionConfig};
use crate::application::application::dto::DialogueChoice;

/// Props for the ChoiceMenu component
#[derive(Props, Clone, PartialEq)]
pub struct ChoiceMenuProps {
    /// Available dialogue choices
    pub choices: Vec<DialogueChoice>,
    /// Handler for when a choice is selected (receives choice ID)
    pub on_select: EventHandler<String>,
    /// Handler for custom text input
    pub on_custom_input: EventHandler<String>,
}

/// Choice menu component - displays dialogue choices
///
/// Uses `.vn-choice` Tailwind class for choice buttons.
/// Includes a text input field for custom responses when available.
#[component]
pub fn ChoiceMenu(props: ChoiceMenuProps) -> Element {
    let mut custom_text = use_signal(String::new);
    let has_custom = props.choices.iter().any(|c| c.is_custom_input);

    rsx! {
        div {
            class: "choice-menu flex flex-col gap-2 mt-4",

            // Standard choice buttons
            for choice in props.choices.iter().filter(|c| !c.is_custom_input) {
                ChoiceButton {
                    key: "{choice.id}",
                    choice: choice.clone(),
                    on_click: props.on_select,
                }
            }

            // Custom input field (if any choice has is_custom_input)
            if has_custom {
                CustomInputField {
                    value: custom_text,
                    on_submit: move |text: String| {
                        if !text.is_empty() {
                            props.on_custom_input.call(text);
                            custom_text.set(String::new());
                        }
                    }
                }
            }
        }
    }
}

/// Props for the ChoiceButton component
#[derive(Props, Clone, PartialEq)]
pub struct ChoiceButtonProps {
    /// The dialogue choice to display
    pub choice: DialogueChoice,
    /// Click handler
    pub on_click: EventHandler<String>,
}

/// Individual choice button
#[component]
pub fn ChoiceButton(props: ChoiceButtonProps) -> Element {
    let choice_id = props.choice.id.clone();

    rsx! {
        button {
            class: "vn-choice",
            onclick: move |_| props.on_click.call(choice_id.clone()),

            "{props.choice.text}"
        }
    }
}

/// Props for the CustomInputField component
#[derive(Props, Clone, PartialEq)]
pub struct CustomInputFieldProps {
    /// Current input value
    pub value: Signal<String>,
    /// Submit handler
    pub on_submit: EventHandler<String>,
}

/// Custom text input field for free-form responses
///
/// Validates expression markers and shows helpful hints.
#[component]
pub fn CustomInputField(props: CustomInputFieldProps) -> Element {
    let mut value = props.value;
    let mut show_hints = use_signal(|| false);

    // Validate input for expression markers
    let validation = use_memo(move || {
        let text = value.read();
        validate_input_markers(&text)
    });

    rsx! {
        div {
            class: "custom-input-container flex flex-col gap-2 mt-2",

            // Main input row
            div {
                class: "flex gap-2",

                input {
                    class: "input flex-1",
                    r#type: "text",
                    placeholder: "Type your response... (use *expression* for emotions)",
                    value: "{value}",
                    oninput: move |e| value.set(e.value()),
                    onkeypress: move |e: KeyboardEvent| {
                        if e.key() == Key::Enter {
                            let text = value.read().clone();
                            if !text.is_empty() {
                                props.on_submit.call(text);
                            }
                        }
                    },
                    onfocus: move |_| show_hints.set(true),
                    onblur: move |_| {
                        // Hide hints when input loses focus
                        show_hints.set(false);
                    },
                }

                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let text = value.read().clone();
                        if !text.is_empty() {
                            props.on_submit.call(text);
                        }
                    },
                    "Send"
                }
            }

            // Validation warnings
            if !validation.read().warnings.is_empty() {
                div {
                    class: "text-yellow-400 text-xs px-2 py-1 bg-yellow-900/30 rounded",
                    for warning in validation.read().warnings.iter() {
                        div { "{warning}" }
                    }
                }
            }

            // Helpful hints (shown on focus)
            if *show_hints.read() && validation.read().warnings.is_empty() {
                MarkerHints {}
            }

            // Show detected markers preview
            if !validation.read().detected_markers.is_empty() {
                div {
                    class: "flex flex-wrap gap-1 text-xs",
                    for marker in validation.read().detected_markers.iter() {
                        span {
                            class: "px-2 py-0.5 rounded bg-purple-900/50 text-purple-300",
                            "{marker}"
                        }
                    }
                }
            }
        }
    }
}

/// Validation result for player input
#[derive(Clone, Default, PartialEq)]
struct InputValidation {
    /// Warning messages for the player
    warnings: Vec<String>,
    /// Detected expression/action markers (for preview)
    detected_markers: Vec<String>,
}

/// Validate expression markers in player input
fn validate_input_markers(text: &str) -> InputValidation {
    let markers = parse_dialogue_markers(text);

    if markers.is_empty() {
        return InputValidation::default();
    }

    // Use standard config for validation
    let config = ExpressionConfig::default();
    let expressions: Vec<String> = config.expressions.iter().cloned().collect();
    let actions: Vec<String> = config.actions.iter().cloned().collect();

    // Get warnings from domain validation
    let domain_warnings = validate_markers(&markers, &expressions, &actions);

    // Build detected markers preview
    let detected_markers: Vec<String> = markers
        .iter()
        .map(|m| match (&m.action, &m.expression) {
            (Some(action), Some(expr)) => format!("{}|{}", action, expr),
            (Some(action), None) => format!("[{}]", action),
            (None, Some(expr)) => expr.clone(),
            (None, None) => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    // Check for common mistakes
    let mut warnings = domain_warnings;

    // Check for unclosed markers
    let open_count = text.matches('*').count();
    if open_count % 2 != 0 {
        warnings.push("Unclosed marker - use *expression* format".to_string());
    }

    // Check for reversed pipe format (expression|action instead of action|expression)
    for marker in &markers {
        if let (Some(action), Some(_expr)) = (&marker.action, &marker.expression) {
            // If the "action" looks like an expression name, suggest fix
            if expressions.iter().any(|e| e.eq_ignore_ascii_case(action)) {
                warnings.push(format!(
                    "Tip: Use *action|expression* format (e.g., *sighs|sad*)"
                ));
                break;
            }
        }
    }

    InputValidation {
        warnings,
        detected_markers,
    }
}

/// Helpful hints component for expression markers
#[component]
fn MarkerHints() -> Element {
    rsx! {
        div {
            class: "text-gray-500 text-xs px-2 py-1 bg-gray-800/50 rounded",

            div { class: "font-medium mb-1", "Expression markers:" }
            div { class: "flex flex-wrap gap-2",
                span { class: "text-purple-400", "*happy*" }
                span { class: "text-purple-400", "*sad*" }
                span { class: "text-purple-400", "*angry*" }
                span { class: "text-purple-400", "*surprised*" }
            }
            div { class: "mt-1 text-gray-600",
                "Combine with actions: "
                span { class: "text-purple-400", "*sighs|sad*" }
            }
        }
    }
}

/// Continue prompt shown when no choices are available
#[derive(Props, Clone, PartialEq)]
pub struct ContinuePromptProps {
    /// Click handler to advance dialogue
    pub on_continue: EventHandler<()>,
}

#[component]
pub fn ContinuePrompt(props: ContinuePromptProps) -> Element {
    rsx! {
        button {
            class: "continue-prompt text-gray-400 text-sm bg-transparent border-none cursor-pointer py-2 px-0 text-left animate-pulse",
            onclick: move |_| props.on_continue.call(()),

            "Click to continue..."
        }
    }
}
