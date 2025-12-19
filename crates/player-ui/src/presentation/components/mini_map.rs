//! Mini-Map Component - Visual map with clickable regions
//!
//! US-NAV-010: Visual map showing regions with click-to-navigate.

use dioxus::prelude::*;

/// Region data for mini-map display (includes bounds)
#[derive(Clone, Debug, PartialEq)]
pub struct MapRegionData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub backdrop_asset: Option<String>,
    /// Map bounds for positioning on the location map
    pub bounds: Option<MapBounds>,
    /// Whether this is a spawn point
    pub is_spawn_point: bool,
}

/// Bounds for a region on the map
#[derive(Clone, Debug, PartialEq)]
pub struct MapBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Props for the MiniMap component
#[derive(Props, Clone, PartialEq)]
pub struct MiniMapProps {
    /// Location name
    pub location_name: String,
    /// Location map image URL (top-down map)
    pub map_image: Option<String>,
    /// All regions in this location with bounds
    pub regions: Vec<MapRegionData>,
    /// Currently active region ID
    pub current_region_id: Option<String>,
    /// IDs of regions that are navigable (not locked)
    pub navigable_region_ids: Vec<String>,
    /// IDs of locked regions
    pub locked_region_ids: Vec<String>,
    /// Whether data is loading
    #[props(default = false)]
    pub is_loading: bool,
    /// Handler for clicking a region
    pub on_region_click: EventHandler<String>,
    /// Handler for closing the map
    pub on_close: EventHandler<()>,
}

/// Mini-Map modal showing location with clickable regions
#[component]
pub fn MiniMap(props: MiniMapProps) -> Element {
    // Calculate the map dimensions based on region bounds
    let (map_width, map_height) = calculate_map_dimensions(&props.regions);

    rsx! {
        // Overlay background
        div {
            class: "mini-map-overlay fixed inset-0 bg-black/90 z-[1000] flex items-center justify-center p-4",
            onclick: move |_| props.on_close.call(()),

            // Map container
            div {
                class: "mini-map-container bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col shadow-2xl border border-blue-500/20",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-4 border-b border-white/10 flex justify-between items-center",

                    div {
                        h2 {
                            class: "text-xl font-bold text-white m-0",
                            "{props.location_name}"
                        }
                        p {
                            class: "text-gray-400 text-sm m-0 mt-1",
                            "Click a region to navigate"
                        }
                    }

                    button {
                        class: "w-8 h-8 flex items-center justify-center bg-white/5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition-colors",
                        onclick: move |_| props.on_close.call(()),
                        "x"
                    }
                }

                // Legend
                div {
                    class: "px-4 py-2 bg-black/20 border-b border-white/5 flex gap-4 text-xs text-gray-400",

                    span {
                        class: "flex items-center gap-1",
                        span { class: "w-3 h-3 bg-blue-500 rounded-sm inline-block" }
                        "Current"
                    }
                    span {
                        class: "flex items-center gap-1",
                        span { class: "w-3 h-3 bg-green-500/50 rounded-sm inline-block" }
                        "Available"
                    }
                    span {
                        class: "flex items-center gap-1",
                        span { class: "w-3 h-3 bg-gray-600 rounded-sm inline-block" }
                        "Locked"
                    }
                }

                // Map area
                div {
                    class: "flex-1 overflow-auto p-4 flex items-center justify-center",

                    if props.is_loading {
                        div {
                            class: "text-gray-400",
                            "Loading map..."
                        }
                    } else if props.regions.is_empty() {
                        div {
                            class: "text-gray-400 text-center",
                            p { "No region data available." }
                            p { class: "text-sm text-gray-500 mt-2", "Use the navigation panel instead." }
                        }
                    } else if let Some(ref map_url) = props.map_image {
                        // Map with image background
                        div {
                            class: "relative",
                            style: "width: {map_width}px; height: {map_height}px; max-width: 100%;",

                            // Background map image
                            img {
                                src: "{map_url}",
                                alt: "Location map",
                                class: "absolute inset-0 w-full h-full object-contain opacity-30",
                            }

                            // Region overlays
                            for region in props.regions.iter() {
                                if let Some(ref bounds) = region.bounds {
                                    {
                                        let is_current = props.current_region_id.as_ref() == Some(&region.id);
                                        let is_navigable = props.navigable_region_ids.contains(&region.id);
                                        let is_locked = props.locked_region_ids.contains(&region.id);
                                        let region_id = region.id.clone();

                                        let bg_color = if is_current {
                                            "bg-blue-500/60"
                                        } else if is_locked {
                                            "bg-gray-600/40"
                                        } else if is_navigable {
                                            "bg-green-500/40 hover:bg-green-500/60 cursor-pointer"
                                        } else {
                                            "bg-gray-500/20"
                                        };

                                        let border_color = if is_current {
                                            "border-blue-400"
                                        } else if is_locked {
                                            "border-gray-500"
                                        } else if is_navigable {
                                            "border-green-400"
                                        } else {
                                            "border-gray-600"
                                        };

                                        rsx! {
                                            div {
                                                key: "{region.id}",
                                                class: "absolute rounded-lg border-2 {bg_color} {border_color} transition-colors flex items-center justify-center",
                                                style: "left: {bounds.x}px; top: {bounds.y}px; width: {bounds.width}px; height: {bounds.height}px;",
                                                onclick: {
                                                    let on_click = props.on_region_click.clone();
                                                    let rid = region_id.clone();
                                                    let can_click = is_navigable && !is_current;
                                                    move |e| {
                                                        e.stop_propagation();
                                                        if can_click {
                                                            on_click.call(rid.clone());
                                                        }
                                                    }
                                                },

                                                div {
                                                    class: "text-center p-1",
                                                    
                                                    span {
                                                        class: if is_current { "text-white font-bold text-sm" } else { "text-gray-200 text-sm" },
                                                        "{region.name}"
                                                    }

                                                    if is_locked {
                                                        span {
                                                            class: "block text-xs text-gray-400",
                                                            "[Locked]"
                                                        }
                                                    }

                                                    if is_current {
                                                        span {
                                                            class: "block text-xs text-blue-300",
                                                            "(You are here)"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Grid layout when no map image
                        MapGridView {
                            regions: props.regions.clone(),
                            current_region_id: props.current_region_id.clone(),
                            navigable_region_ids: props.navigable_region_ids.clone(),
                            locked_region_ids: props.locked_region_ids.clone(),
                            on_region_click: props.on_region_click.clone(),
                        }
                    }
                }
            }
        }
    }
}

/// Calculate map dimensions from region bounds
fn calculate_map_dimensions(regions: &[MapRegionData]) -> (u32, u32) {
    let mut max_x = 400u32;
    let mut max_y = 300u32;

    for region in regions {
        if let Some(ref bounds) = region.bounds {
            let right = bounds.x + bounds.width;
            let bottom = bounds.y + bounds.height;
            if right > max_x {
                max_x = right;
            }
            if bottom > max_y {
                max_y = bottom;
            }
        }
    }

    (max_x + 20, max_y + 20) // Add padding
}

/// Props for MapGridView
#[derive(Props, Clone, PartialEq)]
struct MapGridViewProps {
    regions: Vec<MapRegionData>,
    current_region_id: Option<String>,
    navigable_region_ids: Vec<String>,
    locked_region_ids: Vec<String>,
    on_region_click: EventHandler<String>,
}

/// Fallback grid view when no map image or bounds available
#[component]
fn MapGridView(props: MapGridViewProps) -> Element {
    rsx! {
        div {
            class: "grid grid-cols-2 md:grid-cols-3 gap-3 w-full max-w-2xl",

            for region in props.regions.iter() {
                {
                    let is_current = props.current_region_id.as_ref() == Some(&region.id);
                    let is_navigable = props.navigable_region_ids.contains(&region.id);
                    let is_locked = props.locked_region_ids.contains(&region.id);
                    let region_id = region.id.clone();

                    let card_class = if is_current {
                        "bg-blue-500/30 border-blue-400"
                    } else if is_locked {
                        "bg-gray-700/30 border-gray-600 opacity-60"
                    } else if is_navigable {
                        "bg-green-500/20 border-green-500/50 hover:bg-green-500/30 cursor-pointer"
                    } else {
                        "bg-gray-700/20 border-gray-600/50"
                    };

                    rsx! {
                        button {
                            key: "{region.id}",
                            class: "p-4 rounded-lg border {card_class} text-left transition-colors disabled:cursor-not-allowed",
                            disabled: is_locked || is_current,
                            onclick: {
                                let on_click = props.on_region_click.clone();
                                let rid = region_id.clone();
                                let can_click = is_navigable && !is_current;
                                move |_| {
                                    if can_click {
                                        on_click.call(rid.clone());
                                    }
                                }
                            },

                            div {
                                class: "flex items-center gap-2 mb-1",

                                span {
                                    class: if is_current { "text-blue-400 font-bold" } else { "text-white font-medium" },
                                    "{region.name}"
                                }

                                if is_current {
                                    span {
                                        class: "text-xs bg-blue-500/30 text-blue-300 px-1.5 py-0.5 rounded",
                                        "Here"
                                    }
                                }

                                if is_locked {
                                    span {
                                        class: "text-xs text-gray-500",
                                        "[Locked]"
                                    }
                                }
                            }

                            if !region.description.is_empty() {
                                p {
                                    class: "text-xs text-gray-400 m-0 line-clamp-2",
                                    "{region.description}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
