//! Dynamic log entry component for conversation logs

use dioxus::prelude::*;

/// Dynamic log entry that accepts String values
#[derive(Props, Clone, PartialEq)]
pub struct DynamicLogEntryProps {
    pub speaker: String,
    pub text: String,
    pub is_system: bool,
}

#[component]
pub fn DynamicLogEntry(props: DynamicLogEntryProps) -> Element {
    rsx! {
        div {
            class: if props.is_system { "p-2 rounded bg-blue-500 bg-opacity-10 text-blue-400 text-sm" }
                   else { "p-2 rounded text-white" },
            if !props.is_system {
                span { class: "text-blue-500 font-bold", "{props.speaker}: " }
            }
            span { "{props.text}" }
        }
    }
}
