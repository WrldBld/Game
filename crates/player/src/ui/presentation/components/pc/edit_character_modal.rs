//! Edit Character Modal - Edit player character information

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::application::dto::CharacterSheetSchema;
use crate::application::services::{PlayerCharacterData, UpdatePlayerCharacterRequest};
use crate::infrastructure::spawn_task;
use crate::presentation::components::schema_character_sheet::SchemaCharacterSheet;
use crate::presentation::services::{use_player_character_service, use_world_service};

/// Props for EditCharacterModal
#[derive(Props, Clone, PartialEq)]
pub struct EditCharacterModalProps {
    pub pc: PlayerCharacterData,
    pub on_close: EventHandler<()>,
    pub on_saved: EventHandler<PlayerCharacterData>,
}

/// Edit Character Modal component
#[component]
pub fn EditCharacterModal(props: EditCharacterModalProps) -> Element {
    let pc_service = use_player_character_service();
    let world_service = use_world_service();

    // Form state
    let mut name = use_signal(|| props.pc.name.clone());
    let mut description = use_signal(|| props.pc.description.clone().unwrap_or_default());
    let mut sheet_schema: Signal<Option<CharacterSheetSchema>> = use_signal(|| None);
    let sheet_values: Signal<HashMap<String, serde_json::Value>> = use_signal(|| {
        props
            .pc
            .sheet_data
            .as_ref()
            .map(|s| s.values.clone())
            .unwrap_or_default()
    });
    let mut is_saving = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);
    let mut loading = use_signal(|| true);

    // Load sheet schema
    {
        let world_id = props.pc.world_id.clone();
        let world_svc = world_service.clone();
        use_effect(move || {
            let svc = world_svc.clone();
            let world_id_clone = world_id.clone();
            spawn_task(async move {
                if let Ok(schema_json) = svc.get_sheet_template(&world_id_clone).await {
                    if let Ok(schema) = serde_json::from_value::<CharacterSheetSchema>(schema_json) {
                        sheet_schema.set(Some(schema));
                    }
                }
                loading.set(false);
            });
        });
    }

    let save = move |_| {
        let name_val = name.read().clone();
        let _desc_val = description.read().clone();
        let sheet_vals = sheet_values.read().clone();
        let pc_id = props.pc.id.clone();
        let pc_svc = pc_service.clone();
        let on_saved_handler = props.on_saved;
        let on_close_handler = props.on_close;

        if name_val.trim().is_empty() {
            error_message.set(Some("Character name is required".to_string()));
            return;
        }

        is_saving.set(true);
        error_message.set(None);

        spawn_task(async move {
            // Convert sheet values map to JSON value for API
            let sheet_data = if sheet_vals.is_empty() {
                None
            } else {
                Some(serde_json::json!({ "values": sheet_vals }))
            };

            let request = UpdatePlayerCharacterRequest {
                name: Some(name_val),
                sheet_data,
            };

            match pc_svc.update_pc(&pc_id, &request).await {
                Ok(updated_pc) => {
                    on_saved_handler.call(updated_pc);
                    on_close_handler.call(());
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to update character: {}", e)));
                    is_saving.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-[1000]",
            onclick: move |_| {
                props.on_close.call(());
            },
            div {
                class: "bg-dark-surface rounded-lg w-[90%] max-w-[800px] max-h-[90vh] overflow-y-auto flex flex-col",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center p-6 border-b border-gray-700",
                    h2 {
                        class: "m-0 text-white text-xl",
                        "Edit Character"
                    }
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-2 py-1 bg-transparent text-gray-400 border-0 cursor-pointer text-xl",
                        "×"
                    }
                }

                // Error message
                if let Some(err) = error_message.read().as_ref() {
                    div {
                        class: "px-6 py-3 bg-red-500 bg-opacity-10 border-b border-red-500 border-opacity-30 text-red-500 text-sm",
                        "{err}"
                    }
                }

                // Content
                div {
                    class: "p-6 flex flex-col gap-6",

                    // Name
                    div {
                        label {
                            class: "block mb-2 text-gray-400 text-sm font-medium",
                            "Character Name *"
                        }
                        input {
                            r#type: "text",
                            value: "{name.read()}",
                            oninput: move |e| name.set(e.value()),
                            placeholder: "Enter character name",
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-base",
                        }
                    }

                    // Description
                    div {
                        label {
                            class: "block mb-2 text-gray-400 text-sm font-medium",
                            "Description"
                        }
                        textarea {
                            value: "{description.read()}",
                            oninput: move |e| description.set(e.value()),
                            placeholder: "Describe your character...",
                            rows: 4,
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-base resize-y",
                        }
                    }

                    // Character Sheet
                    if !*loading.read() {
                        if let Some(schema) = sheet_schema.read().as_ref() {
                            div {
                                h3 {
                                    class: "m-0 mb-4 text-white text-base",
                                    "Character Sheet"
                                }
                                SchemaCharacterSheet {
                                    schema: schema.clone(),
                                    values: sheet_values,
                                    show_header: false,
                                }
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "px-6 py-4 border-t border-gray-700 flex justify-end gap-3",
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-4 py-2 bg-gray-700 text-white border-0 rounded-lg cursor-pointer",
                        "Cancel"
                    }
                    button {
                        onclick: save,
                        disabled: *is_saving.read(),
                        class: "px-6 py-2 bg-green-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                        if *is_saving.read() {
                            "Saving..."
                        } else {
                            "Save Changes"
                        }
                    }
                }
            }
        }
    }
}
