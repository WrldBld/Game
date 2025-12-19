//! Directorial notes component
//!
//! Provides an editable textarea for DM's live notes and scene observations.

use dioxus::prelude::*;

/// Props for the DirectorialNotes component
#[derive(Props, Clone, PartialEq)]
pub struct DirectorialNotesProps {
    /// Current notes text
    pub notes: String,
    /// Handler called when notes are updated
    pub on_change: EventHandler<String>,
    /// Optional placeholder text
    #[props(default = "Scene notes, observations, plot hooks...")]
    pub placeholder: &'static str,
}

/// DirectorialNotes component - Editable textarea for DM notes
///
/// Allows DMs to write and edit live notes during gameplay.
/// Useful for tracking scene observations, player reactions, and plot developments.
#[component]
pub fn DirectorialNotes(props: DirectorialNotesProps) -> Element {
    rsx! {
        div {
            class: "directorial-notes flex flex-col h-full",

            // Header
            div {
                class: "flex items-center gap-2 mb-2",

                h3 {
                    class: "text-gray-400 text-sm uppercase m-0",
                    "Directorial Notes"
                }

                // Word count indicator
                span {
                    class: "text-gray-500 text-xs ml-auto",
                    "{props.notes.len()} characters"
                }
            }

            // Textarea
            textarea {
                class: "notes-textarea flex-1 w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white font-mono text-sm leading-normal resize-none box-border transition-colors",
                value: "{props.notes}",
                placeholder: props.placeholder,
                oninput: move |e| props.on_change.call(e.value()),
            }

            // Footer info
            div {
                class: "text-gray-500 text-xs mt-2",
                "Last modified: just now"
            }
        }
    }
}
