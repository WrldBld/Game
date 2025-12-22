//! Narrative Event Library - Browse and manage future narrative events

use dioxus::prelude::*;

use wrldbldr_player_app::application::dto::{CreateNarrativeEventRequest, NarrativeEventData};
use crate::presentation::components::story_arc::narrative_event_card::NarrativeEventCard;
use crate::presentation::services::use_narrative_event_service;

#[derive(Props, Clone, PartialEq)]
pub struct NarrativeEventLibraryProps {
    pub world_id: String,
}

#[component]
pub fn NarrativeEventLibrary(props: NarrativeEventLibraryProps) -> Element {
    let mut events: Signal<Vec<NarrativeEventData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut search_text = use_signal(|| String::new());
    let mut filter_status = use_signal(|| "all".to_string());
    let mut show_favorites_only = use_signal(|| false);
    let mut selected_event: Signal<Option<NarrativeEventData>> = use_signal(|| None);
    let mut show_create_form = use_signal(|| false);

    // Get narrative event service
    let narrative_event_service = use_narrative_event_service();
    let narrative_event_service_for_effect = narrative_event_service.clone();

    // Load events
    let world_id = props.world_id.clone();
    use_effect(move || {
        let world_id = world_id.clone();
        let service = narrative_event_service_for_effect.clone();
        spawn(async move {
            is_loading.set(true);
            error.set(None);

            match service.list_narrative_events(&world_id).await {
                Ok(loaded) => events.set(loaded),
                Err(e) => error.set(Some(format!("Failed to load narrative events: {}", e))),
            }
            is_loading.set(false);
        });
    });

    // Filter events
    let filtered_events = {
        let search = search_text.read().to_lowercase();
        let status = filter_status.read().clone();
        let favorites_only = *show_favorites_only.read();
        let all_events = events.read().clone();

        all_events.into_iter().filter(|event| {
            // Filter by favorites
            if favorites_only && !event.is_favorite {
                return false;
            }

            // Filter by status
            match status.as_str() {
                "active" => {
                    if !event.is_active {
                        return false;
                    }
                }
                "triggered" => {
                    if !event.is_triggered {
                        return false;
                    }
                }
                "pending" => {
                    if event.is_triggered || !event.is_active {
                        return false;
                    }
                }
                _ => {}
            }

            // Filter by search
            if !search.is_empty() {
                let matches_name = event.name.to_lowercase().contains(&search);
                let matches_desc = event.description.to_lowercase().contains(&search);
                let matches_tags = event.tags.iter().any(|t| t.to_lowercase().contains(&search));
                if !matches_name && !matches_desc && !matches_tags {
                    return false;
                }
            }

            true
        }).collect::<Vec<_>>()
    };

    rsx! {
        div {
            class: "narrative-event-library h-full flex flex-col gap-4 p-4",

            // Header
            div {
                class: "flex justify-between items-center",

                h2 { class: "text-white m-0 text-xl", "Narrative Events" }

                button {
                    onclick: move |_| show_create_form.set(true),
                    class: "px-4 py-2 bg-purple-500 text-white border-none rounded-lg cursor-pointer flex items-center gap-2",
                    span { "+" }
                    span { "New Event" }
                }
            }

            // Filters
            div {
                class: "bg-dark-surface rounded-lg p-3 flex gap-3 items-center flex-wrap",

                // Search
                input {
                    r#type: "text",
                    placeholder: "Search events...",
                    value: "{search_text}",
                    oninput: move |e| search_text.set(e.value()),
                    class: "flex-1 min-w-[200px] px-3 py-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm",
                }

                // Status filter
                select {
                    value: "{filter_status}",
                    onchange: move |e| filter_status.set(e.value()),
                    class: "px-3 py-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm",

                    option { value: "all", "All Events" }
                    option { value: "active", "Active Only" }
                    option { value: "pending", "Pending" }
                    option { value: "triggered", "Triggered" }
                }

                // Favorites toggle
                {
                    let is_favorites = *show_favorites_only.read();
                    rsx! {
                        label {
                            class: "flex items-center gap-1.5 text-gray-400 text-sm cursor-pointer",

                            input {
                                r#type: "checkbox",
                                checked: is_favorites,
                                onchange: move |_| show_favorites_only.set(!is_favorites),
                            }
                            "⭐ Favorites"
                        }
                    }
                }
            }

            // Stats bar
            div {
                class: "flex gap-4 text-gray-500 text-sm",

                span { "{events.read().len()} total" }
                span { "{events.read().iter().filter(|e| e.is_active).count()} active" }
                span { "{events.read().iter().filter(|e| e.is_triggered).count()} triggered" }
                span { "{events.read().iter().filter(|e| e.is_favorite).count()} favorites" }
            }

            // Event list
            div {
                class: "flex-1 overflow-y-auto",

                if *is_loading.read() {
                    div {
                        class: "flex justify-center items-center p-12 text-gray-400",
                        "Loading narrative events..."
                    }
                } else if let Some(err) = error.read().as_ref() {
                    div {
                        class: "bg-red-500 bg-opacity-10 border border-red-500 rounded-lg p-4 text-red-500",
                        "Error: {err}"
                    }
                } else if filtered_events.is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center p-12 text-gray-500",

                        div { class: "text-5xl mb-4", "⭐" }

                        if events.read().is_empty() {
                            p { "No narrative events yet" }
                            p { class: "text-sm", "Create events to set up story hooks and branching narratives" }
                        } else {
                            p { "No events match your filters" }
                        }
                    }
                } else {
                    div {
                        class: "grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4",

                        for event in filtered_events.iter() {
                            NarrativeEventCard {
                                key: "{event.id}",
                                event: event.clone(),
                                on_click: {
                                    let event = event.clone();
                                    move |_| selected_event.set(Some(event.clone()))
                                },
                                on_toggle_favorite: {
                                    let event_id = event.id.clone();
                                    let world_id = props.world_id.clone();
                                    let service = narrative_event_service.clone();
                                    move |_| {
                                        let event_id = event_id.clone();
                                        let world_id = world_id.clone();
                                        let service = service.clone();
                                        spawn(async move {
                                            if let Err(e) = service.toggle_favorite(&event_id).await {
                                                tracing::error!("Failed to toggle favorite: {}", e);
                                            }
                                            if let Ok(reloaded) = service.list_narrative_events(&world_id).await {
                                                events.set(reloaded);
                                            }
                                        });
                                    }
                                },
                                on_toggle_active: {
                                    let event_id = event.id.clone();
                                    let is_active = event.is_active;
                                    let world_id = props.world_id.clone();
                                    let service = narrative_event_service.clone();
                                    move |_| {
                                        let event_id = event_id.clone();
                                        let world_id = world_id.clone();
                                        let service = service.clone();
                                        spawn(async move {
                                            if let Err(e) = service.set_active(&event_id, !is_active).await {
                                                tracing::error!("Failed to toggle active: {}", e);
                                            }
                                            if let Ok(reloaded) = service.list_narrative_events(&world_id).await {
                                                events.set(reloaded);
                                            }
                                        });
                                    }
                                },
                            }
                        }
                    }
                }
            }

            // Create form modal
            if *show_create_form.read() {
                NarrativeEventFormModal {
                    world_id: props.world_id.clone(),
                    on_save: {
                        let mut events = events.clone();
                        move |new_event: NarrativeEventData| {
                            events.write().push(new_event);
                            show_create_form.set(false);
                        }
                    },
                    on_close: move |_| show_create_form.set(false),
                }
            }
        }
    }
}

/// Modal form for creating a new narrative event
#[derive(Props, Clone, PartialEq)]
struct NarrativeEventFormModalProps {
    world_id: String,
    on_save: EventHandler<NarrativeEventData>,
    on_close: EventHandler<()>,
}

#[component]
fn NarrativeEventFormModal(props: NarrativeEventFormModalProps) -> Element {
    let narrative_event_service = use_narrative_event_service();

    let mut name = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut scene_direction = use_signal(|| String::new());
    let mut is_saving = use_signal(|| false);
    let mut save_error: Signal<Option<String>> = use_signal(|| None);

    let save_event = {
        let world_id = props.world_id.clone();
        let service = narrative_event_service.clone();
        let on_save = props.on_save.clone();
        move |_| {
            let world_id = world_id.clone();
            let service = service.clone();
            let on_save = on_save.clone();
            let name_val = name.read().clone();
            let desc_val = description.read().clone();
            let direction_val = scene_direction.read().clone();

            if name_val.trim().is_empty() {
                save_error.set(Some("Name is required".to_string()));
                return;
            }

            is_saving.set(true);
            save_error.set(None);

            spawn(async move {
                let request = CreateNarrativeEventRequest {
                    name: name_val,
                    description: desc_val,
                    scene_direction: direction_val,
                    ..Default::default()
                };

                match service.create_narrative_event(&world_id, request).await {
                    Ok(new_event) => {
                        on_save.call(new_event);
                    }
                    Err(e) => {
                        save_error.set(Some(format!("Failed to create event: {}", e)));
                        is_saving.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-80 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-xl max-w-[500px] w-[90%] max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center px-6 py-4 border-b border-gray-700",
                    h3 { class: "text-white m-0", "New Narrative Event" }
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-none text-gray-400 text-2xl cursor-pointer",
                        "×"
                    }
                }

                // Form
                div {
                    class: "p-6 flex flex-col gap-4",

                    // Name field
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1",
                            "Event Name *"
                        }
                        input {
                            r#type: "text",
                            placeholder: "Enter event name...",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                            class: "w-full px-3 py-3 bg-dark-bg border border-gray-700 rounded-lg text-white box-border",
                        }
                    }

                    // Description field
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1",
                            "Description"
                        }
                        textarea {
                            placeholder: "What happens when this event triggers?",
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                            class: "w-full min-h-[80px] px-3 py-3 bg-dark-bg border border-gray-700 rounded-lg text-white resize-y box-border",
                        }
                    }

                    // Scene direction field
                    div {
                        label {
                            class: "block text-gray-400 text-sm mb-1",
                            "Scene Direction"
                        }
                        textarea {
                            placeholder: "How should the DM/AI present this event?",
                            value: "{scene_direction}",
                            oninput: move |e| scene_direction.set(e.value()),
                            class: "w-full min-h-[60px] px-3 py-3 bg-dark-bg border border-gray-700 rounded-lg text-white resize-y box-border",
                        }
                    }

                    // Error message
                    if let Some(err) = save_error.read().as_ref() {
                        div {
                            class: "bg-red-500 bg-opacity-10 border border-red-500 rounded-lg p-3 text-red-500 text-sm",
                            "{err}"
                        }
                    }
                }

                // Footer
                div {
                    class: "flex justify-end gap-3 px-6 py-4 border-t border-gray-700",

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-4 py-2 bg-gray-700 text-white border-none rounded-lg cursor-pointer",
                        "Cancel"
                    }

                    button {
                        onclick: save_event,
                        disabled: *is_saving.read(),
                        class: "px-4 py-2 bg-purple-500 text-white border-none rounded-lg cursor-pointer",
                        if *is_saving.read() { "Creating..." } else { "Create Event" }
                    }
                }
            }
        }
    }
}

