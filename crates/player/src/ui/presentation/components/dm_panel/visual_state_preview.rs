//! Visual state preview component
//!
//! Shows inline thumbnail, description, and asset information for a selected visual state.
//! Used in pre-stage modal and staging approval popup.

use dioxus::prelude::*;

use wrldbldr_shared::types::ResolvedVisualStateData;

/// Props for VisualStatePreview
#[derive(Props, Clone, PartialEq)]
pub struct VisualStatePreviewProps {
    /// Complete resolved visual state (location + region)
    pub resolved_state: Option<ResolvedVisualStateData>,
    /// Handler to open details modal
    pub on_details: EventHandler<()>,
}

/// Visual state preview card
#[component]
pub fn VisualStatePreview(props: VisualStatePreviewProps) -> Element {
    let (location_state, region_state) = match props.resolved_state.as_ref() {
        Some(state) => (state.location_state.as_ref(), state.region_state.as_ref()),
        None => (None, None),
    };

    let has_state = location_state.is_some() || region_state.is_some();

    rsx! {
        div { class: "bg-black/30 rounded-lg p-4",
            if !has_state {
                div {
                    class: "text-gray-500 text-center py-8",
                    "No visual state selected"
                }
            } else {
                div { class: "space-y-3",
                    // Thumbnail preview
                    div { class: "relative bg-black/50 rounded-lg overflow-hidden aspect-video flex items-center justify-center",
                        if let (Some(loc_state), Some(reg_state)) = (location_state, region_state) {
                            // Combined state preview - show region state layered over location
                            div { class: "relative w-full h-full",
                                // Location backdrop
                                if let Some(ref backdrop) = loc_state.backdrop_override {
                                    img {
                                        src: "{backdrop}",
                                        class: "w-full h-full object-cover opacity-50",
                                        alt: "Location backdrop"
                                    }
                                }
                                // Region state indicator
                                if reg_state.backdrop_override.is_some() {
                                    div {
                                        class: "absolute inset-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent flex items-end p-4",
                                        div {
                                            class: "text-white text-sm",
                                            "🖼️ Combined Preview"
                                        }
                                    }
                                }
                            }
                        } else if let Some(ref loc_state) = location_state {
                            // Location state only
                            if let Some(ref backdrop) = loc_state.backdrop_override {
                                img {
                                    src: "{backdrop}",
                                    class: "w-full h-full object-cover",
                                    alt: "{loc_state.name}"
                                }
                            } else {
                                div {
                                    class: "w-full h-full flex items-center justify-center text-gray-500",
                                    "No backdrop image"
                                }
                            }
                        } else if let Some(ref reg_state) = region_state {
                            // Region state only
                            if let Some(ref backdrop) = reg_state.backdrop_override {
                                img {
                                    src: "{backdrop}",
                                    class: "w-full h-full object-cover",
                                    alt: "{reg_state.name}"
                                }
                            } else {
                                div {
                                    class: "w-full h-full flex items-center justify-center text-gray-500",
                                    "No backdrop image"
                                }
                            }
                        }
                    }

                    // Description section
                    div { class: "space-y-2",
                        // Location state info
                        if let Some(ref state) = location_state {
                            div {
                                class: "p-2 bg-blue-500/10 border-l-2 border-blue-500",
                                div { class: "text-blue-400 font-medium text-xs", "Location:" }
                                div { class: "text-white text-sm", "{state.name}" }
                                if let Some(ref atmosphere) = state.atmosphere_override {
                                    div { class: "text-gray-400 text-xs mt-1 italic", "{atmosphere}" }
                                }
                            }
                        }

                        // Region state info
                        if let Some(ref state) = region_state {
                            div {
                                class: "p-2 bg-purple-500/10 border-l-2 border-purple-500",
                                div { class: "text-purple-400 font-medium text-xs", "Region:" }
                                div { class: "text-white text-sm", "{state.name}" }
                                if let Some(ref atmosphere) = state.atmosphere_override {
                                    div { class: "text-gray-400 text-xs mt-1 italic", "{atmosphere}" }
                                }
                            }
                        }
                    }

                    // Asset paths
                    if let (Some(loc_state), Some(reg_state)) = (location_state, region_state) {
                        div { class: "space-y-1 text-xs text-gray-400",
                            div { class: "flex justify-between",
                                span { "Location Backdrop:" }
                                span {
                                    class: "text-gray-300 font-mono truncate ml-2",
                                    "{loc_state.backdrop_override.as_deref().unwrap_or(\"(none)\")}"
                                }
                            }
                            div { class: "flex justify-between",
                                span { "Region Backdrop:" }
                                span {
                                    class: "text-gray-300 font-mono truncate ml-2",
                                    "{reg_state.backdrop_override.as_deref().unwrap_or(\"(none)\")}"
                                }
                            }
                            if let Some(ref sound) = loc_state.ambient_sound.as_ref().or(reg_state.ambient_sound.as_ref()) {
                                div { class: "flex justify-between",
                                    span { "Ambient Sound:" }
                                    span {
                                        class: "text-gray-300 font-mono truncate ml-2",
                                        "{sound}"
                                    }
                                }
                            }
                        }
                    } else if let Some(ref state) = location_state.or(region_state) {
                        div { class: "space-y-1 text-xs text-gray-400",
                            if let Some(ref backdrop) = state.backdrop_override {
                                div { class: "flex justify-between",
                                    span { "Backdrop:" }
                                    span {
                                        class: "text-gray-300 font-mono truncate ml-2",
                                        "{backdrop}"
                                    }
                                }
                            }
                            if let Some(ref atmosphere) = state.atmosphere_override {
                                div { class: "flex justify-between",
                                    span { "Atmosphere:" }
                                    span {
                                        class: "text-gray-300 font-mono truncate ml-2",
                                        "{atmosphere}"
                                    }
                                }
                            }
                            if let Some(ref sound) = state.ambient_sound {
                                div { class: "flex justify-between",
                                    span { "Ambient Sound:" }
                                    span {
                                        class: "text-gray-300 font-mono truncate ml-2",
                                        "{sound}"
                                    }
                                }
                            }
                        }
                    }

                    // Details button
                    button {
                        onclick: move |_| props.on_details.call(()),
                        class: "w-full px-3 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded text-sm transition-colors",
                        "🔍 Details"
                    }
                }
            }
        }
    }
}
