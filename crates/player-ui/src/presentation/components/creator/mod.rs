//! Creator Mode components - World-building interface
//!
//! Components for the Creator Mode in the DM View, providing
//! entity creation, editing, asset generation, and LLM suggestions.

pub mod asset_gallery;
pub mod character_form;
pub mod comfyui_banner;
pub mod entity_browser;
pub mod generation_queue;
pub mod location_form;
pub mod lore_form;
pub mod motivations_tab;
pub mod sheet_field_input;
pub mod suggestion_button;

use crate::presentation::services::use_generation_service;
use crate::presentation::state::use_generation_state;
use crate::presentation::state::use_session_state;
use crate::use_platform;
use dioxus::prelude::*;

/// Props for CreatorMode
#[derive(Props, Clone, PartialEq)]
pub struct CreatorModeProps {
    /// World ID from the route
    pub world_id: String,
    /// Currently selected tab from route (characters, locations, items, maps)
    #[props(default)]
    pub selected_tab: Option<String>,
}

/// The main Creator Mode container component
#[component]
pub fn CreatorMode(props: CreatorModeProps) -> Element {
    // Parse selected tab from URL, default to Characters
    let selected_entity_type = match props.selected_tab.as_deref() {
        Some("characters") | None => EntityTypeTab::Characters,
        Some("locations") => EntityTypeTab::Locations,
        Some("items") => EntityTypeTab::Items,
        Some("maps") => EntityTypeTab::Maps,
        _ => EntityTypeTab::Characters,
    };

    // Track the currently selected entity ID for editing
    let mut selected_entity_id: Signal<Option<String>> = use_signal(|| None);

    // Entity lists - stored as reactive signals (single source of truth)
    let mut characters: Signal<
        Vec<wrldbldr_player_app::application::services::character_service::CharacterSummary>,
    > = use_signal(Vec::new);
    let mut locations: Signal<
        Vec<wrldbldr_player_app::application::services::location_service::LocationSummary>,
    > = use_signal(Vec::new);

    // Loading and error states
    let mut characters_loading = use_signal(|| true);
    let mut locations_loading = use_signal(|| true);
    let mut characters_error: Signal<Option<String>> = use_signal(|| None);
    let mut locations_error: Signal<Option<String>> = use_signal(|| None);

    // Initial data fetching on mount
    let character_service = crate::presentation::services::use_character_service();
    let location_service = crate::presentation::services::use_location_service();
    let world_id_for_fetch = props.world_id.clone();

    // Fetch characters on mount
    use_effect(move || {
        let world_id = world_id_for_fetch.clone();
        let svc = character_service.clone();
        spawn(async move {
            match svc.list_characters(&world_id).await {
                Ok(fetched) => {
                    characters.set(fetched);
                    characters_loading.set(false);
                }
                Err(e) => {
                    characters_error.set(Some(e.to_string()));
                    characters_loading.set(false);
                }
            }
        });
    });

    // Fetch locations on mount
    let world_id_for_locations = props.world_id.clone();
    use_effect(move || {
        let world_id = world_id_for_locations.clone();
        let svc = location_service.clone();
        spawn(async move {
            match svc.list_locations(&world_id).await {
                Ok(fetched) => {
                    locations.set(fetched);
                    locations_loading.set(false);
                }
                Err(e) => {
                    locations_error.set(Some(e.to_string()));
                    locations_loading.set(false);
                }
            }
        });
    });

    // Hydrate generation queue from Engine on mount
    let platform = use_platform();
    let generation_service = use_generation_service();
    let mut generation_state = use_generation_state();
    let session_state = use_session_state();
    let world_id_for_hydrate = props.world_id.clone();
    use_effect(move || {
        let platform_clone = platform.clone();
        let gen_svc = generation_service.clone();
        let user_id = session_state.user_id().read().clone();
        let world_id = world_id_for_hydrate.clone();
        spawn(async move {
            if let Err(e) = crate::presentation::services::hydrate_generation_queue(
                &gen_svc,
                &mut generation_state,
                user_id.as_deref(),
                &world_id,
            )
            .await
            {
                platform_clone.log_error(&format!(
                    "Failed to hydrate generation queue from Engine: {}",
                    e
                ));
            }
        });
    });

    let session_state = use_session_state();

    rsx! {
        div {
            class: "creator-mode h-full flex flex-col gap-4 p-4",

            // ComfyUI status banner
            if *session_state.comfyui_state().read() != "connected" {
                comfyui_banner::ComfyUIBanner {
                    state: session_state.comfyui_state().read().clone(),
                    message: session_state.comfyui_message().read().clone(),
                    retry_in_seconds: *session_state.comfyui_retry_in_seconds().read(),
                }
            }

            div {
                class: "grid gap-4 flex-1 overflow-hidden",
                style: "grid-template-columns: 280px 1fr;",
                // Left panel - Entity browser and generation queue
            div {
                class: "left-panel flex flex-col gap-4 overflow-hidden",

                // Entity browser (tree view) - now uses router for tab changes
                entity_browser::EntityBrowser {
                    world_id: props.world_id.clone(),
                    selected_type: selected_entity_type,
                    selected_id: selected_entity_id.read().clone(),
                    characters: characters,
                    locations: locations,
                    characters_loading: characters_loading,
                    locations_loading: locations_loading,
                    characters_error: characters_error,
                    locations_error: locations_error,
                    on_select: move |id| selected_entity_id.set(Some(id)),
                }

                // Generation queue panel - navigation handled via entity selection
                generation_queue::GenerationQueuePanel {
                    on_navigate_to_entity: {
                        let mut selected_id = selected_entity_id;
                        let _world_id = props.world_id.clone();
                        move |(_entity_type, entity_id): (String, String)| {
                            // Set the selected entity ID so the form opens
                            selected_id.set(Some(entity_id.clone()));
                            // Note: Navigation to the correct tab is handled by the route
                            // The entity will be selected when the form loads
                        }
                    },
                }
            }

            // Right panel - Editor/Form area
            div {
                class: "editor-panel flex flex-col gap-4 overflow-hidden",

                match (selected_entity_type, selected_entity_id.read().clone()) {
                    (EntityTypeTab::Characters, Some(id)) => rsx! {
                        character_form::CharacterForm {
                            character_id: id,
                            world_id: props.world_id.clone(),
                            characters_signal: characters,
                            on_close: move |_| selected_entity_id.set(None),
                        }
                    },
                    (EntityTypeTab::Characters, None) => rsx! {
                        character_form::CharacterForm {
                            character_id: String::new(),
                            world_id: props.world_id.clone(),
                            characters_signal: characters,
                            on_close: move |_| {},
                        }
                    },
                    (EntityTypeTab::Locations, Some(id)) => rsx! {
                        location_form::LocationForm {
                            location_id: id,
                            world_id: props.world_id.clone(),
                            locations_signal: locations,
                            on_close: move |_| selected_entity_id.set(None),
                        }
                    },
                    (EntityTypeTab::Locations, None) => rsx! {
                        location_form::LocationForm {
                            location_id: String::new(),
                            world_id: props.world_id.clone(),
                            locations_signal: locations,
                            on_close: move |_| {},
                        }
                    },
                    (EntityTypeTab::Items, _) => rsx! {
                        PlaceholderPanel { title: "Item Editor", message: "Item editing coming soon" }
                    },
                    (EntityTypeTab::Maps, _) => rsx! {
                        PlaceholderPanel { title: "Map Editor", message: "Map editing coming soon" }
                    },
                }
            }
            }
        }
    }
}

/// Entity type tabs for the browser
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityTypeTab {
    #[default]
    Characters,
    Locations,
    Items,
    Maps,
}

impl EntityTypeTab {
    pub fn label(&self) -> &'static str {
        match self {
            EntityTypeTab::Characters => "Characters",
            EntityTypeTab::Locations => "Locations",
            EntityTypeTab::Items => "Items",
            EntityTypeTab::Maps => "Maps",
        }
    }
}

/// Placeholder panel for unimplemented features
#[component]
fn PlaceholderPanel(title: &'static str, message: &'static str) -> Element {
    rsx! {
        div {
            class: "placeholder-panel flex-1 flex flex-col items-center justify-center bg-dark-surface rounded-lg text-gray-500",

            h2 { class: "text-gray-400 mb-2", "{title}" }
            p { "{message}" }
        }
    }
}
