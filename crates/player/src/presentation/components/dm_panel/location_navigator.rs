//! Location Navigator - DM tool to preview any location

use dioxus::prelude::*;

use crate::presentation::services::use_location_service;

/// Props for LocationNavigator
#[derive(Props, Clone, PartialEq)]
pub struct LocationNavigatorProps {
    pub world_id: String,
    pub on_preview: EventHandler<String>,
}

/// Location Navigator component
#[component]
pub fn LocationNavigator(props: LocationNavigatorProps) -> Element {
    let location_service = use_location_service();
    let mut locations: Signal<Vec<crate::application::services::location_service::LocationSummary>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Load locations on mount
    {
        let world_id = props.world_id.clone();
        let loc_svc = location_service.clone();
        use_effect(move || {
            let wid = world_id.clone();
            let svc = loc_svc.clone();
            loading.set(true);
            spawn(async move {
                match svc.list_locations(&wid).await {
                    Ok(loc_list) => {
                        locations.set(loc_list);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load locations: {}", e)));
                        loading.set(false);
                    }
                }
            });
        });
    }

    let locs = locations.read().clone();
    let err = error.read().clone();

    rsx! {
        div {
            class: "flex flex-col gap-4 p-4 bg-dark-surface rounded-lg",

            h3 {
                class: "m-0 text-white text-lg",
                "Location Navigator"
            }

            if let Some(e) = err.as_ref() {
                div {
                    class: "p-3 bg-red-500 bg-opacity-10 border border-red-500 rounded-lg text-red-500 text-sm",
                    "{e}"
                }
            }

            if *loading.read() {
                div {
                    class: "p-8 text-center text-gray-400",
                    "Loading locations..."
                }
            } else if locs.is_empty() {
                div {
                    class: "p-8 text-center text-gray-400",
                    "No locations in this world"
                }
            } else {
                div {
                    class: "flex flex-col gap-3 max-h-[400px] overflow-y-auto",
                    {locs.into_iter().map(|location| {
                        let loc_id = location.id.clone();
                        rsx! {
                            LocationCard {
                                location,
                                on_preview: move |_| props.on_preview.call(loc_id.clone()),
                            }
                        }
                    })}
                }
            }
        }
    }
}

/// Location Card component
#[derive(Props, Clone, PartialEq)]
struct LocationCardProps {
    location: crate::application::services::LocationSummary,
    on_preview: EventHandler<()>,
}

#[component]
fn LocationCard(props: LocationCardProps) -> Element {
    rsx! {
        div {
            class: "p-4 bg-dark-bg rounded-lg border border-gray-700 flex justify-between items-center",

            div {
                h4 {
                    class: "m-0 mb-1 text-white text-base",
                    "{props.location.name}"
                }
                if let Some(loc_type) = props.location.location_type.as_ref() {
                    div {
                        class: "text-gray-400 text-xs",
                        "{loc_type}"
                    }
                }
            }
            button {
                onclick: move |_| props.on_preview.call(()),
                class: "py-2 px-4 bg-blue-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                "Preview"
            }
        }
    }
}

