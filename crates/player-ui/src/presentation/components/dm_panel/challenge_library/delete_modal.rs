//! Delete confirmation modal for challenges

use dioxus::prelude::*;

/// Props for delete confirmation modal
#[derive(Props, Clone, PartialEq)]
pub struct ConfirmDeleteChallengeModalProps {
    pub challenge_name: String,
    pub is_deleting: bool,
    pub on_confirm: EventHandler<()>,
    pub on_cancel: EventHandler<()>,
}

/// Confirmation dialog for challenge deletion
#[component]
pub fn ConfirmDeleteChallengeModal(props: ConfirmDeleteChallengeModalProps) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-[1101]",
            onclick: move |_| props.on_cancel.call(()),

            div {
                class: "bg-dark-surface rounded-xl w-[90%] max-w-md p-6 overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center gap-4 mb-4",

                    div {
                        class: "text-red-600 text-2xl",
                        "⚠"
                    }

                    h2 {
                        class: "text-red-600 text-lg m-0",
                        "Delete Challenge"
                    }
                }

                // Message
                p {
                    class: "text-gray-400 my-4",
                    "Are you sure you want to delete \"{props.challenge_name}\"? This action cannot be undone."
                }

                // Buttons
                div {
                    class: "flex gap-3 justify-end mt-6",

                    button {
                        onclick: move |_| props.on_cancel.call(()),
                        disabled: props.is_deleting,
                        class: "py-2 px-4 bg-gray-700 text-white border-0 rounded-lg cursor-pointer text-sm",
                        "Cancel"
                    }

                    button {
                        onclick: move |_| props.on_confirm.call(()),
                        disabled: props.is_deleting,
                        class: "py-2 px-4 bg-red-600 text-white border-0 rounded-lg cursor-pointer text-sm font-medium",
                        if props.is_deleting { "Deleting..." } else { "Delete Challenge" }
                    }
                }
            }
        }
    }
}
