//! Form field wrapper component with label and required indicator.

use dioxus::prelude::*;

#[component]
pub fn FormField(label: &'static str, required: bool, children: Element) -> Element {
    rsx! {
        div {
            class: "form-field flex flex-col gap-1",
            label {
                class: "text-gray-400 text-sm",
                "{label}"
                if required {
                    span { class: "text-red-500 ml-1", "*" }
                }
            }
            {children}
        }
    }
}
