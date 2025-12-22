//! Timeline Filters - Filter controls for the timeline view

use dioxus::prelude::*;

use crate::presentation::components::story_arc::timeline_view::TimelineFilterState;

/// Simple character option for dropdown
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterOption {
    pub id: String,
    pub name: String,
}

/// Simple location option for dropdown
#[derive(Debug, Clone, PartialEq)]
pub struct LocationOption {
    pub id: String,
    pub name: String,
}

#[derive(Props, Clone, PartialEq)]
pub struct TimelineFiltersProps {
    pub filters: Signal<TimelineFilterState>,
    pub on_filter_change: EventHandler<TimelineFilterState>,
    #[props(default)]
    pub characters: Vec<CharacterOption>,
    #[props(default)]
    pub locations: Vec<LocationOption>,
}

#[component]
pub fn TimelineFilters(props: TimelineFiltersProps) -> Element {
    let mut expanded = use_signal(|| false);
    let current_filters = props.filters.read().clone();

    // Event type options
    let event_types = vec![
        ("", "All Events"),
        ("Location Change", "Location Change"),
        ("Dialogue", "Dialogue"),
        ("Combat", "Combat"),
        ("Challenge", "Challenge"),
        ("Item Acquired", "Item Acquired"),
        ("Relationship", "Relationship"),
        ("Scene Transition", "Scene Transition"),
        ("Information", "Information"),
        ("DM Marker", "DM Marker"),
        ("Narrative Event", "Narrative Event"),
        ("Session Start", "Session Start"),
        ("Session End", "Session End"),
    ];

    rsx! {
        div {
            class: "timeline-filters bg-dark-surface rounded-lg p-3",

            // Compact filter bar
            div {
                class: "flex gap-3 items-center flex-wrap",

                // Search input
                div {
                    class: "flex-1 min-w-[200px]",
                    input {
                        r#type: "text",
                        placeholder: "Search events...",
                        value: "{current_filters.search_text}",
                        oninput: {
                            let mut filters = current_filters.clone();
                            move |e: Event<FormData>| {
                                filters.search_text = e.value().clone();
                                props.on_filter_change.call(filters.clone());
                            }
                        },
                        class: "w-full px-3 py-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm",
                    }
                }

                // Event type dropdown
                {
                    let selected_type = current_filters.event_type.clone().unwrap_or_default();
                    rsx! {
                        select {
                            value: "{selected_type}",
                            onchange: {
                                let mut filters = current_filters.clone();
                                move |e: Event<FormData>| {
                                    let val = e.value();
                                    filters.event_type = if val.is_empty() { None } else { Some(val) };
                                    props.on_filter_change.call(filters.clone());
                                }
                            },
                            class: "px-3 py-2 bg-dark-bg border border-gray-700 rounded-md text-white text-sm",

                            for (value, label) in event_types.iter() {
                                option { value: "{value}", "{label}" }
                            }
                        }
                    }
                }

                // Show hidden toggle
                label {
                    class: "flex items-center gap-1.5 text-gray-400 text-sm cursor-pointer whitespace-nowrap",

                    input {
                        r#type: "checkbox",
                        checked: current_filters.show_hidden,
                        onchange: {
                            let mut filters = current_filters.clone();
                            move |_| {
                                filters.show_hidden = !filters.show_hidden;
                                props.on_filter_change.call(filters.clone());
                            }
                        },
                        class: "cursor-pointer",
                    }
                    "Show hidden"
                }

                // Expand/collapse advanced filters
                {
                    let is_expanded = *expanded.read();
                    rsx! {
                        button {
                            onclick: move |_| expanded.set(!is_expanded),
                            class: "bg-transparent border-none text-blue-400 cursor-pointer text-sm flex items-center gap-1",
                            if is_expanded { "Less ▲" } else { "More ▼" }
                        }
                    }
                }

                // Clear filters button (only show if filters active)
                if has_active_filters(&current_filters) {
                    button {
                        onclick: {
                            move |_| props.on_filter_change.call(TimelineFilterState::default())
                        },
                        class: "px-3 py-1.5 bg-gray-700 text-white border-none rounded-md cursor-pointer text-xs",
                        "Clear"
                    }
                }
            }

            // Advanced filters (expandable)
            if *expanded.read() {
                div {
                    class: "mt-3 pt-3 border-t border-gray-700 grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3",

                    // Date from
                    {
                        let date_from = current_filters.date_from.clone().unwrap_or_default();
                        rsx! {
                            div {
                                label {
                                    class: "block text-gray-400 text-xs mb-1",
                                    "From Date"
                                }
                                input {
                                    r#type: "date",
                                    value: "{date_from}",
                                    onchange: {
                                        let mut filters = current_filters.clone();
                                        move |e: Event<FormData>| {
                                            let val = e.value();
                                            filters.date_from = if val.is_empty() { None } else { Some(val) };
                                            props.on_filter_change.call(filters.clone());
                                        }
                                    },
                                    class: "w-full p-1.5 bg-dark-bg border border-gray-700 rounded text-white text-[0.8125rem]",
                                }
                            }
                        }
                    }

                    // Date to
                    {
                        let date_to = current_filters.date_to.clone().unwrap_or_default();
                        rsx! {
                            div {
                                label {
                                    class: "block text-gray-400 text-xs mb-1",
                                    "To Date"
                                }
                                input {
                                    r#type: "date",
                                    value: "{date_to}",
                                    onchange: {
                                        let mut filters = current_filters.clone();
                                        move |e: Event<FormData>| {
                                            let val = e.value();
                                            filters.date_to = if val.is_empty() { None } else { Some(val) };
                                            props.on_filter_change.call(filters.clone());
                                        }
                                    },
                                    class: "w-full p-1.5 bg-dark-bg border border-gray-700 rounded text-white text-[0.8125rem]",
                                }
                            }
                        }
                    }

                    // Character filter
                    {
                        let char_id = current_filters.character_id.clone().unwrap_or_default();
                        let characters = props.characters.clone();
                        rsx! {
                            div {
                                label {
                                    class: "block text-gray-400 text-xs mb-1",
                                    "Character"
                                }
                                select {
                                    value: "{char_id}",
                                    onchange: {
                                        let mut filters = current_filters.clone();
                                        move |e: Event<FormData>| {
                                            let val = e.value();
                                            filters.character_id = if val.is_empty() { None } else { Some(val) };
                                            props.on_filter_change.call(filters.clone());
                                        }
                                    },
                                    class: "w-full p-1.5 bg-dark-bg border border-gray-700 rounded text-white text-[0.8125rem]",

                                    option { value: "", "All Characters" }
                                    for character in characters.iter() {
                                        option { value: "{character.id}", "{character.name}" }
                                    }
                                }
                            }
                        }
                    }

                    // Location filter
                    {
                        let loc_id = current_filters.location_id.clone().unwrap_or_default();
                        let locations = props.locations.clone();
                        rsx! {
                            div {
                                label {
                                    class: "block text-gray-400 text-xs mb-1",
                                    "Location"
                                }
                                select {
                                    value: "{loc_id}",
                                    onchange: {
                                        let mut filters = current_filters.clone();
                                        move |e: Event<FormData>| {
                                            let val = e.value();
                                            filters.location_id = if val.is_empty() { None } else { Some(val) };
                                            props.on_filter_change.call(filters.clone());
                                        }
                                    },
                                    class: "w-full p-1.5 bg-dark-bg border border-gray-700 rounded text-white text-[0.8125rem]",

                                    option { value: "", "All Locations" }
                                    for location in locations.iter() {
                                        option { value: "{location.id}", "{location.name}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Check if any non-default filters are active
fn has_active_filters(filters: &TimelineFilterState) -> bool {
    !filters.search_text.is_empty()
        || filters.event_type.is_some()
        || filters.character_id.is_some()
        || filters.location_id.is_some()
        || filters.show_hidden
        || filters.date_from.is_some()
        || filters.date_to.is_some()
}
