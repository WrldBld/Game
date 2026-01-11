//! Character Panel - Display and manage player character information

use dioxus::prelude::*;

use crate::application::dto::SheetTemplate;
use crate::application::services::PlayerCharacterData;
use crate::infrastructure::spawn_task;
use crate::presentation::services::use_world_service;

/// Props for CharacterPanel
#[derive(Props, Clone, PartialEq)]
pub struct CharacterPanelProps {
    pub pc: PlayerCharacterData,
    pub on_edit: EventHandler<()>,
}

/// Character Panel component - Shows PC information
#[component]
pub fn CharacterPanel(props: CharacterPanelProps) -> Element {
    let world_service = use_world_service();
    let mut sheet_template: Signal<Option<SheetTemplate>> = use_signal(|| None);
    let mut loading = use_signal(|| true);

    // Load sheet template
    {
        let world_id = props.pc.world_id.clone();
        let world_svc = world_service.clone();
        use_effect(move || {
            let svc = world_svc.clone();
            let world_id_clone = world_id.clone();
            spawn_task(async move {
                if let Ok(template_json) = svc.get_sheet_template(&world_id_clone).await {
                    if let Ok(template) = serde_json::from_value::<SheetTemplate>(template_json) {
                        sheet_template.set(Some(template));
                    }
                }
                loading.set(false);
            });
        });
    }

    rsx! {
        div {
            class: "flex flex-col gap-4 p-4 bg-dark-surface rounded-lg",

            // Header
            div {
                class: "flex justify-between items-center",
                h3 {
                    class: "m-0 text-white text-lg",
                    "{props.pc.name}"
                }
                button {
                    onclick: move |_| props.on_edit.call(()),
                    class: "px-4 py-2 bg-blue-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                    "Edit"
                }
            }

            // Description
            if let Some(desc) = props.pc.description.as_ref() {
                p {
                    class: "m-0 text-gray-400 text-sm leading-normal",
                    "{desc}"
                }
            }

            // Location
            div {
                class: "p-3 bg-dark-bg rounded-lg",
                div {
                    class: "text-gray-400 text-xs mb-1 uppercase",
                    "Current Location"
                }
                div {
                    class: "text-white text-sm",
                    "Location ID: {props.pc.current_location_id}"
                }
            }

            // Character Sheet
            if !*loading.read() {
                if let Some(template) = sheet_template.read().as_ref() {
                    if let Some(sheet_data) = props.pc.sheet_data.as_ref() {
                        div {
                            class: "mt-4",
                            h4 {
                                class: "m-0 mb-2 text-white text-base",
                                "Character Sheet"
                            }
                            crate::presentation::components::character_sheet_viewer::CharacterSheetViewer {
                                character_name: props.pc.name.clone(),
                                template: template.clone(),
                                values: sheet_data.values.clone(),
                                on_close: move |_| {},
                            }
                        }
                    }
                }
            }
        }
    }
}
