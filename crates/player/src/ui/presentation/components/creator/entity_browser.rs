//! Entity Browser - Tree view of world entities

use dioxus::prelude::*;

use super::EntityTypeTab;
use crate::routes::Route;
use crate::application::application::services::character_service::CharacterSummary;
use crate::application::application::services::location_service::LocationSummary;

/// Props for the EntityBrowser component
#[component]
pub fn EntityBrowser(
    world_id: String,
    selected_type: EntityTypeTab,
    selected_id: Option<String>,
    characters: Signal<Vec<CharacterSummary>>,
    locations: Signal<Vec<LocationSummary>>,
    characters_loading: Signal<bool>,
    locations_loading: Signal<bool>,
    characters_error: Signal<Option<String>>,
    locations_error: Signal<Option<String>>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "entity-browser flex-1 flex flex-col bg-dark-surface rounded-lg overflow-hidden",

            // Tab buttons for entity types - uses router Links
            div {
                class: "browser-tabs flex border-b border-gray-700",

                EntityTypeTabLink {
                    world_id: world_id.clone(),
                    tab: EntityTypeTab::Characters,
                    active: selected_type == EntityTypeTab::Characters,
                }
                EntityTypeTabLink {
                    world_id: world_id.clone(),
                    tab: EntityTypeTab::Locations,
                    active: selected_type == EntityTypeTab::Locations,
                }
                EntityTypeTabLink {
                    world_id: world_id.clone(),
                    tab: EntityTypeTab::Items,
                    active: selected_type == EntityTypeTab::Items,
                }
                EntityTypeTabLink {
                    world_id: world_id.clone(),
                    tab: EntityTypeTab::Maps,
                    active: selected_type == EntityTypeTab::Maps,
                }
            }

            // Search/filter bar
            div {
                class: "browser-search p-2",

                input {
                    r#type: "text",
                    placeholder: "Search...",
                    class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                }
            }

            // Entity list
            div {
                class: "browser-list flex-1 overflow-y-auto p-2",

                match selected_type {
                    EntityTypeTab::Characters => rsx! {
                        CharacterList {
                            characters: characters,
                            selected_id: selected_id.clone(),
                            loading: characters_loading,
                            error: characters_error,
                            on_select: move |id| on_select.call(id),
                        }
                    },
                    EntityTypeTab::Locations => rsx! {
                        LocationList {
                            locations: locations,
                            selected_id: selected_id.clone(),
                            loading: locations_loading,
                            error: locations_error,
                            on_select: move |id| on_select.call(id),
                        }
                    },
                    EntityTypeTab::Items => rsx! {
                        div { class: "text-gray-500 text-center p-4",
                            "No items yet"
                        }
                    },
                    EntityTypeTab::Maps => rsx! {
                        div { class: "text-gray-500 text-center p-4",
                            "No maps yet"
                        }
                    },
                }
            }

            // New entity button
            div {
                class: "browser-actions p-2 border-t border-gray-700",

                button {
                    class: "w-full p-2 bg-blue-500 text-white border-0 rounded cursor-pointer font-medium",
                    onclick: move |_| on_select.call(String::new()),
                    "+ New {selected_type.label()}"
                }
            }
        }
    }
}

/// Tab link that uses router navigation
#[component]
fn EntityTypeTabLink(world_id: String, tab: EntityTypeTab, active: bool) -> Element {
    let bg_class = if active {
        "bg-blue-500"
    } else {
        "bg-transparent"
    };
    let short_label = match tab {
        EntityTypeTab::Characters => "Char",
        EntityTypeTab::Locations => "Loc",
        EntityTypeTab::Items => "Item",
        EntityTypeTab::Maps => "Map",
    };
    let subtab = match tab {
        EntityTypeTab::Characters => "characters",
        EntityTypeTab::Locations => "locations",
        EntityTypeTab::Items => "items",
        EntityTypeTab::Maps => "maps",
    };

    rsx! {
        Link {
            to: Route::DMCreatorSubTabRoute {
                world_id,
                subtab: subtab.to_string(),
            },
            class: format!("flex-1 py-2 px-1 {} text-white border-0 cursor-pointer text-xs no-underline text-center", bg_class),
            "{short_label}"
        }
    }
}

/// Character list - renders from reactive signal
#[component]
fn CharacterList(
    characters: Signal<Vec<CharacterSummary>>,
    selected_id: Option<String>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        if *loading.read() {
            div {
                class: "flex items-center justify-center p-8 text-gray-500",
                "Loading characters..."
            }
        } else if let Some(err) = error.read().as_ref() {
            div {
                class: "p-4 bg-red-500 bg-opacity-10 rounded-lg text-red-500 text-sm",
                "Error: {err}"
            }
        } else {
            div {
                class: "flex flex-col gap-1",

                for character in characters.read().iter() {
                    EntityListItem {
                        id: character.id.clone(),
                        name: character.name.clone(),
                        subtitle: character.archetype.clone().unwrap_or_else(|| "Unknown".to_string()),
                        selected: selected_id.as_deref() == Some(&character.id),
                        on_click: {
                            let char_id = character.id.clone();
                            move |_| on_select.call(char_id.clone())
                        },
                    }
                }

                if characters.read().is_empty() {
                    div {
                        class: "text-gray-500 text-center p-4 text-sm",
                        "No characters yet"
                    }
                }
            }
        }
    }
}

/// Location list - renders from reactive signal
#[component]
fn LocationList(
    locations: Signal<Vec<LocationSummary>>,
    selected_id: Option<String>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        if *loading.read() {
            div {
                class: "flex items-center justify-center p-8 text-gray-500",
                "Loading locations..."
            }
        } else if let Some(err) = error.read().as_ref() {
            div {
                class: "p-4 bg-red-500 bg-opacity-10 rounded-lg text-red-500 text-sm",
                "Error: {err}"
            }
        } else {
            div {
                class: "flex flex-col gap-1",

                for location in locations.read().iter() {
                    EntityListItem {
                        id: location.id.clone(),
                        name: location.name.clone(),
                        subtitle: location.location_type.clone().unwrap_or_else(|| "Unknown".to_string()),
                        selected: selected_id.as_deref() == Some(&location.id),
                        on_click: {
                            let loc_id = location.id.clone();
                            move |_| on_select.call(loc_id.clone())
                        },
                    }
                }

                if locations.read().is_empty() {
                    div {
                        class: "text-gray-500 text-center p-4 text-sm",
                        "No locations yet"
                    }
                }
            }
        }
    }
}

/// Reusable entity list item
#[component]
fn EntityListItem(
    id: String,
    name: String,
    subtitle: String,
    selected: bool,
    on_click: EventHandler<()>,
) -> Element {
    let bg_class = if selected {
        "bg-blue-500 bg-opacity-20"
    } else {
        "bg-transparent"
    };
    let border_class = if selected {
        "border border-blue-500"
    } else {
        "border border-transparent"
    };

    rsx! {
        div {
            onclick: move |_| on_click.call(()),
            class: format!("p-2 {} {} rounded cursor-pointer", bg_class, border_class),

            div { class: "text-white text-sm", "{name}" }
            div { class: "text-gray-500 text-xs", "{subtitle}" }
        }
    }
}
