//! Choice menu component for dialogue choices
//!
//! Displays dialogue choices and handles custom input.

use dioxus::prelude::*;

use crate::application::dto::DialogueChoice;

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
    let mut custom_text = use_signal(|| String::new());
    let has_custom = props.choices.iter().any(|c| c.is_custom_input);

    rsx! {
        div {
            class: "choice-menu flex flex-col gap-2 mt-4",

            // Standard choice buttons
            for choice in props.choices.iter().filter(|c| !c.is_custom_input) {
                ChoiceButton {
                    key: "{choice.id}",
                    choice: choice.clone(),
                    on_click: props.on_select.clone(),
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
#[component]
pub fn CustomInputField(props: CustomInputFieldProps) -> Element {
    let mut value = props.value;

    rsx! {
        div {
            class: "custom-input-container flex gap-2 mt-2",

            input {
                class: "input flex-1",
                r#type: "text",
                placeholder: "Type your response...",
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
