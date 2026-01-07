//! Add DM Marker Modal - Create DM notes/markers on the timeline

use dioxus::prelude::*;

use crate::presentation::services::use_story_event_service;
use crate::application::services::CreateDmMarkerRequest;

#[derive(Props, Clone, PartialEq)]
pub struct AddDmMarkerModalProps {
    pub world_id: String,
    pub on_close: EventHandler<()>,
    pub on_created: EventHandler<()>,
}

#[component]
pub fn AddDmMarkerModal(props: AddDmMarkerModalProps) -> Element {
    // Get story event service
    let story_event_service = use_story_event_service();
    let mut title = use_signal(String::new);
    let mut note = use_signal(String::new);
    let mut importance = use_signal(|| "normal".to_string());
    let mut marker_type = use_signal(|| "note".to_string());
    let mut tags_input = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let importance_options = [
        ("minor", "Minor", "Low priority note"),
        ("normal", "Normal", "Standard note"),
        ("major", "Major", "Important plot point"),
        ("critical", "Critical", "Critical story moment"),
    ];

    let marker_type_options = [
        ("note", "Note", "📝"),
        ("plot_point", "Plot Point", "⭐"),
        ("foreshadowing", "Foreshadowing", "🔮"),
        ("session_break", "Session Break", "⏸️"),
        ("chapter_break", "Chapter Break", "📖"),
        ("recap", "Recap", "📋"),
    ];

    let can_save = !title.read().trim().is_empty();

    rsx! {
        div {
            class: "modal-overlay fixed inset-0 bg-black bg-opacity-80 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl p-6 max-w-[500px] w-[90%]",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center mb-6",

                    h2 { class: "text-white m-0 text-xl", "📝 Add DM Marker" }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-none text-gray-400 text-2xl cursor-pointer",
                        "×"
                    }
                }

                // Form
                div {
                    class: "flex flex-col gap-4",

                    // Title
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1.5",
                            "Title *"
                        }
                        input {
                            r#type: "text",
                            placeholder: "Enter marker title...",
                            value: "{title}",
                            oninput: move |e| title.set(e.value()),
                            class: "w-full px-2.5 py-2.5 bg-dark-bg border border-gray-700 rounded-md text-white text-[0.9375rem] box-border",
                        }
                    }

                    // Note
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1.5",
                            "Note"
                        }
                        textarea {
                            placeholder: "Add details, context, or reminders...",
                            value: "{note}",
                            oninput: move |e| note.set(e.value()),
                            class: "w-full min-h-[100px] px-2.5 py-2.5 bg-dark-bg border border-gray-700 rounded-md text-white text-[0.9375rem] resize-y box-border",
                        }
                    }

                    // Marker Type
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1.5",
                            "Marker Type"
                        }
                        div {
                            class: "flex flex-wrap gap-2",

                            for (value, label, icon) in marker_type_options.iter() {
                                {
                                    let is_selected = *marker_type.read() == *value;
                                    let value = *value;
                                    let button_classes = if is_selected {
                                        "bg-blue-500 text-white border-blue-500"
                                    } else {
                                        "bg-dark-bg text-gray-400 border-gray-700"
                                    };
                                    rsx! {
                                        button {
                                            onclick: move |_| marker_type.set(value.to_string()),
                                            class: "px-3 py-2 rounded-md cursor-pointer flex items-center gap-1.5 text-[0.8125rem] border {button_classes}",
                                            span { "{icon}" }
                                            span { "{label}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Importance
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1.5",
                            "Importance"
                        }
                        div {
                            class: "flex gap-2",

                            for (value, label, description) in importance_options.iter() {
                                {
                                    let is_selected = *importance.read() == *value;
                                    let value = *value;
                                    let color_bg = get_importance_color(value);
                                    rsx! {
                                        button {
                                            onclick: move |_| importance.set(value.to_string()),
                                            title: "{description}",
                                            class: if is_selected {
                                                "flex-1 px-2 py-2 rounded-md cursor-pointer text-[0.8125rem] text-white border"
                                            } else {
                                                "flex-1 px-2 py-2 rounded-md cursor-pointer text-[0.8125rem] bg-dark-bg text-gray-400 border border-gray-700"
                                            },
                                            style: if is_selected {
                                                format!("background-color: {}; border-color: {}", color_bg, color_bg)
                                            } else {
                                                String::new()
                                            },
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Tags
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1.5",
                            "Tags (comma separated)"
                        }
                        input {
                            r#type: "text",
                            placeholder: "e.g., act1, villain, mystery",
                            value: "{tags_input}",
                            oninput: move |e| tags_input.set(e.value()),
                            class: "w-full px-2 py-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm box-border",
                        }
                    }

                    // Error display
                    if let Some(err) = error.read().as_ref() {
                        div {
                            class: "bg-red-500 bg-opacity-10 border border-red-500 rounded-md p-3 text-red-500 text-sm",
                            "{err}"
                        }
                    }

                    // Buttons
                    div {
                        class: "flex justify-end gap-3 mt-2",

                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "px-5 py-2.5 bg-gray-700 text-white border-none rounded-md cursor-pointer",
                            "Cancel"
                        }

                        {
                            let save_disabled = !can_save || *is_saving.read();
                            let save_bg = if can_save { "bg-purple-500" } else { "bg-gray-600 opacity-50" };
                            let save_cursor = if can_save && !*is_saving.read() { "cursor-pointer" } else { "cursor-not-allowed" };
                            let save_text = if *is_saving.read() { "Saving..." } else { "Create Marker" };
                            rsx! {
                                button {
                                    onclick: {
                                        let world_id = props.world_id.clone();
                                        let service = story_event_service.clone();
                                        move |_| {
                                            if !can_save { return; }

                                            let title_val = title.read().trim().to_string();
                                            let note_val = note.read().trim().to_string();
                                            let importance_val = importance.read().clone();
                                            let marker_type_val = marker_type.read().clone();
                                            let tags: Vec<String> = tags_input.read()
                                                .split(',')
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();

                                            let world_id = world_id.clone();
                                            let service = service.clone();
                                            spawn(async move {
                                                is_saving.set(true);
                                                error.set(None);

                                                let request = CreateDmMarkerRequest {
                                                    title: title_val,
                                                    note: note_val,
                                                    importance: importance_val,
                                                    marker_type: marker_type_val,
                                                    tags,
                                                };

                                                match service.create_dm_marker(&world_id, &request).await {
                                                    Ok(_) => {
                                                        props.on_created.call(());
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(format!("Failed to create marker: {}", e)));
                                                    }
                                                }

                                                is_saving.set(false);
                                            });
                                        }
                                    },
                                    disabled: save_disabled,
                                    class: "px-5 py-2.5 text-white border-none rounded-md {save_bg} {save_cursor}",
                                    "{save_text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn get_importance_color(importance: &str) -> &'static str {
    match importance {
        "critical" => "#ef4444",
        "major" => "#f59e0b",
        "normal" => "#3b82f6",
        "minor" => "#6b7280",
        _ => "#6b7280",
    }
}
