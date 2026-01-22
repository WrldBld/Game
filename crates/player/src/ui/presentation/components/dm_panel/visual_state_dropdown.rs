//! Visual state dropdown component with generation button
//!
//! Shows a dropdown of available visual states with a "Generate New" button.
//! Used in pre-stage modal and staging approval popup.

use dioxus::prelude::*;

use wrldbldr_shared::types::{StateOptionData, VisualStateSourceData};

/// Visual state type
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum VisualStateType {
    #[default]
    Location,
    Region,
}

/// Props for VisualStateDropdown
#[derive(Props, Clone, PartialEq)]
pub struct VisualStateDropdownProps {
    /// State type (location or region)
    pub state_type: VisualStateType,
    /// Available states from catalog
    pub available_states: Vec<StateOptionData>,
    /// Currently selected state ID (None = auto-resolved)
    pub selected_id: Option<String>,
    /// ID of auto-resolved state (if any)
    pub resolved_id: Option<String>,
    /// Match reason for resolved state
    pub resolved_reason: Option<String>,
    /// Handler when selection changes
    pub on_select: EventHandler<Option<String>>,
    /// Handler to open generation modal
    pub on_generate: EventHandler<()>,
}

/// Visual state dropdown with Generate New button
#[component]
pub fn VisualStateDropdown(props: VisualStateDropdownProps) -> Element {
    let label_text = match props.state_type {
        VisualStateType::Location => "Location State",
        VisualStateType::Region => "Region State",
    };

    // Determine the current selection
    let current_value = props.selected_id.as_ref().or_else(|| {
        // If no manual selection, show resolved ID if available
        if props.resolved_id.is_some() {
            props.resolved_id.as_ref()
        } else {
            None
        }
    });

    rsx! {
        div { class: "flex flex-col gap-2",
            label {
                class: "text-gray-400 text-sm mb-1",
                "{label_text}"
            }

            div { class: "flex gap-2",
                // State dropdown
                select {
                    class: "flex-1 p-3 bg-dark-bg border border-gray-700 rounded-lg text-white",
                    value: current_value.as_deref().unwrap_or(""),
                    onchange: move |e| {
                        let value = e.value();
                        if value == "default" {
                            props.on_select.call(None);
                        } else if !value.is_empty() {
                            props.on_select.call(Some(value));
                        }
                    },

                    // Default (auto-resolved) option
                    if props.resolved_id.is_some() {
                        option {
                            value: props.resolved_id.as_deref().unwrap_or("default"),
                            class: if props.selected_id.is_none() {
                                "text-amber-400 font-medium"
                            } else {
                                ""
                            },
                            "Default (auto-resolved)"
                        }
                    }

                    // Available catalog states
                    for state in props.available_states.iter() {
                        option {
                            value: &state.id,
                            class: if props.selected_id.as_ref().map(|id| id == &state.id).unwrap_or(false) {
                                "text-amber-400 font-medium"
                            } else {
                                ""
                            },
                            "{state.name}"
                        }
                    }
                }

                // Generate New button
                button {
                    onclick: move |_| props.on_generate.call(()),
                    class: "px-3 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors whitespace-nowrap",
                    "Generate New ➤"
                }
            }

            // Match reason for resolved state
            if props.selected_id.is_none() {
                if let Some(ref reason) = props.resolved_reason {
                    div {
                        class: "text-xs text-green-400 mt-1",
                        "✓ {reason}"
                    }
                }
            }
        }
    }
}
