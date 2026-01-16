//! Character Sheet Viewer - Read-only display of character stats for players

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::application::dto::CharacterSheetSchema;
use crate::presentation::components::schema_character_sheet::SchemaCharacterSheet;
use wrldbldr_protocol::character_sheet::SheetValue;

/// Props for the character sheet viewer
#[derive(Props, Clone, PartialEq)]
pub struct CharacterSheetViewerProps {
    /// The character's name
    pub character_name: String,
    /// The sheet schema
    pub schema: CharacterSheetSchema,
    /// The character's values
    pub values: HashMap<String, SheetValue>,
    /// Handler for closing the viewer
    pub on_close: EventHandler<()>,
}

/// Character Sheet Viewer - modal overlay showing character stats
#[component]
pub fn CharacterSheetViewer(props: CharacterSheetViewerProps) -> Element {
    // Create a signal from the values for the SchemaCharacterSheet
    let values_signal = use_signal(|| props.values.clone());

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
                            "{props.schema.system_name}"
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

                    SchemaCharacterSheet {
                        schema: props.schema.clone(),
                        values: values_signal,
                        show_header: false,
                        read_only: true,
                    }
                }
            }
        }
    }
}
