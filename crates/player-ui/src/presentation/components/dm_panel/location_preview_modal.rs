//! Location Preview Modal - Shows detailed location information

use dioxus::prelude::*;

use wrldbldr_player_app::application::services::location_service::{
    ConnectionData, LocationFormData, RegionData,
};

use crate::presentation::services::use_location_service;

#[derive(Props, Clone, PartialEq)]
pub struct LocationPreviewModalProps {
    /// The ID of the location to preview
    pub location_id: String,
    /// World ID for API context
    pub world_id: String,
    /// Called when the modal should close
    pub on_close: EventHandler<()>,
}

/// Modal displaying detailed location information
#[component]
pub fn LocationPreviewModal(props: LocationPreviewModalProps) -> Element {
    let location_service = use_location_service();

    let mut location: Signal<Option<LocationFormData>> = use_signal(|| None);
    let mut regions: Signal<Vec<RegionData>> = use_signal(Vec::new);
    let mut connections: Signal<Vec<ConnectionData>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Load location data
    {
        let loc_id = props.location_id.clone();
        let world_id = props.world_id.clone();
        let svc = location_service.clone();

        use_effect(move || {
            let loc_id = loc_id.clone();
            let world_id = world_id.clone();
            let svc = svc.clone();

            spawn(async move {
                loading.set(true);
                error.set(None);

                // Fetch location data first
                match svc.get_location(&loc_id).await {
                    Ok(loc_data) => location.set(Some(loc_data)),
                    Err(e) => {
                        error.set(Some(format!("Failed to load location: {}", e)));
                        loading.set(false);
                        return;
                    }
                }

                // Then fetch regions and connections
                if let Ok(r) = svc.get_regions(&loc_id).await {
                    regions.set(r);
                }

                if let Ok(c) = svc.get_connections(&loc_id).await {
                    connections.set(c);
                }

                loading.set(false);
            });
        });
    }

    rsx! {
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-xl max-w-[700px] w-[90%] max-h-[85vh] overflow-hidden flex flex-col",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between p-4 border-b border-gray-700",

                    h2 {
                        class: "m-0 text-white text-xl",
                        "Location Preview"
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-none text-gray-400 text-2xl cursor-pointer hover:text-white",
                        "×"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    if *loading.read() {
                        div {
                            class: "flex items-center justify-center p-12 text-gray-400",
                            "Loading location..."
                        }
                    } else if let Some(err) = error.read().as_ref() {
                        div {
                            class: "bg-red-500/10 border border-red-500 rounded-lg p-4 text-red-400",
                            "{err}"
                        }
                    } else if let Some(loc) = location.read().as_ref() {
                        LocationContent {
                            location: loc.clone(),
                            regions: regions.read().clone(),
                            connections: connections.read().clone(),
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct LocationContentProps {
    location: LocationFormData,
    regions: Vec<RegionData>,
    connections: Vec<ConnectionData>,
}

#[component]
fn LocationContent(props: LocationContentProps) -> Element {
    let loc = &props.location;

    rsx! {
        div {
            class: "flex flex-col gap-6",

            // Backdrop image (if available)
            if let Some(ref backdrop) = loc.backdrop_asset {
                div {
                    class: "w-full h-[200px] rounded-lg overflow-hidden bg-gray-800",
                    img {
                        src: "{backdrop}",
                        class: "w-full h-full object-cover",
                        alt: "Location backdrop",
                    }
                }
            }

            // Name and type
            div {
                class: "flex items-center gap-3",

                span { class: "text-2xl", "" }

                div {
                    h3 {
                        class: "m-0 text-white text-xl",
                        "{loc.name}"
                    }

                    if let Some(ref loc_type) = loc.location_type {
                        span {
                            class: "text-gray-400 text-sm",
                            "{loc_type}"
                        }
                    }
                }
            }

            // Description
            if let Some(ref desc) = loc.description {
                div {
                    class: "bg-dark-bg rounded-lg p-4",
                    p {
                        class: "m-0 text-gray-300 leading-relaxed",
                        "{desc}"
                    }
                }
            }

            // Atmosphere
            if let Some(ref atmosphere) = loc.atmosphere {
                div {
                    class: "flex flex-col gap-2",
                    h4 { class: "m-0 text-gray-400 text-sm uppercase", "Atmosphere" }
                    p { class: "m-0 text-gray-300 italic", "{atmosphere}" }
                }
            }

            // Notable Features
            if let Some(ref features) = loc.notable_features {
                div {
                    class: "flex flex-col gap-2",
                    h4 { class: "m-0 text-gray-400 text-sm uppercase", "Notable Features" }
                    p { class: "m-0 text-gray-300", "{features}" }
                }
            }

            // Regions section
            if !props.regions.is_empty() {
                div {
                    class: "flex flex-col gap-3",

                    h4 {
                        class: "m-0 text-gray-400 text-sm uppercase flex items-center gap-2",
                        span { "" }
                        "Regions ({props.regions.len()})"
                    }

                    div {
                        class: "flex flex-col gap-2",

                        for region in props.regions.iter() {
                            RegionRow {
                                key: "{region.id}",
                                region: region.clone(),
                            }
                        }
                    }
                }
            }

            // Connections section
            if !props.connections.is_empty() {
                div {
                    class: "flex flex-col gap-3",

                    h4 {
                        class: "m-0 text-gray-400 text-sm uppercase flex items-center gap-2",
                        span { "" }
                        "Connections ({props.connections.len()})"
                    }

                    div {
                        class: "flex flex-col gap-2",

                        for connection in props.connections.iter() {
                            ConnectionRow {
                                key: "{connection.to_location_id}",
                                connection: connection.clone(),
                            }
                        }
                    }
                }
            }

            // Hidden secrets (DM only info)
            if let Some(ref secrets) = loc.hidden_secrets {
                div {
                    class: "bg-purple-900/20 border border-purple-500/30 rounded-lg p-4",

                    h4 {
                        class: "m-0 mb-2 text-purple-400 text-sm uppercase flex items-center gap-2",
                        span { "" }
                        "Hidden Secrets (DM Only)"
                    }

                    p {
                        class: "m-0 text-purple-200 text-sm",
                        "{secrets}"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct RegionRowProps {
    region: RegionData,
}

#[component]
fn RegionRow(props: RegionRowProps) -> Element {
    let region = &props.region;

    rsx! {
        div {
            class: "bg-dark-bg rounded-lg p-3 flex items-start gap-3",

            span { class: "text-gray-500 mt-0.5", "" }

            div {
                class: "flex-1",

                div {
                    class: "flex items-center gap-2",

                    span {
                        class: "text-white font-medium",
                        "{region.name}"
                    }

                    if region.is_spawn_point {
                        span {
                            class: "text-xs bg-green-500/20 text-green-400 px-2 py-0.5 rounded",
                            "Spawn Point"
                        }
                    }
                }

                if !region.description.is_empty() {
                    p {
                        class: "m-0 mt-1 text-gray-400 text-sm",
                        "{region.description}"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ConnectionRowProps {
    connection: ConnectionData,
}

#[component]
fn ConnectionRow(props: ConnectionRowProps) -> Element {
    let conn = &props.connection;

    rsx! {
        div {
            class: "bg-dark-bg rounded-lg p-3 flex items-center gap-3",

            span {
                class: "text-blue-400",
                if conn.bidirectional { "" } else { "" }
            }

            div {
                class: "flex-1",

                span {
                    class: "text-white",
                    // Note: We only have the ID, not the name. In a real implementation,
                    // we'd either include the name in ConnectionData or fetch it separately.
                    "Location: {conn.to_location_id}"
                }

                if !conn.description.is_empty() {
                    span {
                        class: "text-gray-400 text-sm ml-2",
                        "- {conn.description}"
                    }
                }
            }

            if let Some(ref conn_type) = conn.connection_type {
                span {
                    class: "text-xs bg-gray-700 text-gray-300 px-2 py-0.5 rounded",
                    "{conn_type}"
                }
            }
        }
    }
}
