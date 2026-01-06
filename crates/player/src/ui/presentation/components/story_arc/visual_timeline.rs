//! Visual Timeline - Horizontal zoomable/pannable timeline view of story events

use dioxus::prelude::*;

use crate::presentation::components::story_arc::timeline_filters::{
    CharacterOption, LocationOption, TimelineFilters,
};
use crate::presentation::components::story_arc::timeline_view::{
    get_event_type_icon, TimelineFilterState, TimelineViewModel,
};
use crate::presentation::services::use_story_event_service;
use crate::presentation::state::use_game_state;
use crate::application::application::dto::{StoryEventData, StoryEventTypeData};

/// A cluster of events that are close together on the timeline
#[derive(Debug, Clone)]
struct TimelineCluster {
    /// Events in this cluster
    events: Vec<StoryEventData>,
    /// X position on timeline (percentage of total width)
    x_position: f32,
    /// Whether these events are filtered out (greyed)
    is_filtered_out: bool,
}

/// Calculate timestamp as milliseconds since epoch for positioning
fn timestamp_to_ms(timestamp: &str) -> Option<i64> {
    // Parse ISO 8601 timestamp: "2025-12-25T14:30:00Z" or similar
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.timestamp_millis())
        .ok()
        .or_else(|| {
            // Try parsing without timezone
            chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S")
                .map(|dt| dt.and_utc().timestamp_millis())
                .ok()
        })
}

/// Group events into clusters based on their proximity on the timeline
fn cluster_events(
    events: &[StoryEventData],
    filtered_events: &[StoryEventData],
    zoom: f32,
    container_width: f32,
) -> Vec<TimelineCluster> {
    if events.is_empty() {
        return Vec::new();
    }

    // Get min/max timestamps
    let mut min_ts: Option<i64> = None;
    let mut max_ts: Option<i64> = None;

    for event in events {
        if let Some(ts) = timestamp_to_ms(&event.timestamp) {
            min_ts = Some(min_ts.map_or(ts, |m| m.min(ts)));
            max_ts = Some(max_ts.map_or(ts, |m| m.max(ts)));
        }
    }

    let (min_ts, max_ts) = match (min_ts, max_ts) {
        (Some(min), Some(max)) => (min, max),
        _ => return Vec::new(),
    };

    // Add padding to range
    let range = (max_ts - min_ts).max(1000); // At least 1 second range
    let padded_min = min_ts - (range / 20);
    let padded_range = range + (range / 10);

    // Create a set of filtered event IDs for quick lookup
    let filtered_ids: std::collections::HashSet<&str> =
        filtered_events.iter().map(|e| e.id.as_str()).collect();

    // Calculate x positions for all events
    let mut positioned_events: Vec<(StoryEventData, f32, bool)> = Vec::new();

    for event in events {
        if let Some(ts) = timestamp_to_ms(&event.timestamp) {
            let x = ((ts - padded_min) as f32 / padded_range as f32) * 100.0 * zoom;
            let is_filtered_out = !filtered_ids.contains(event.id.as_str());
            positioned_events.push((event.clone(), x, is_filtered_out));
        }
    }

    // Sort by x position
    positioned_events.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Cluster events that are within 30px of each other (at current zoom)
    let cluster_threshold = 30.0 / container_width.max(1.0) * 100.0;
    let mut clusters: Vec<TimelineCluster> = Vec::new();

    for (event, x, is_filtered_out) in positioned_events {
        // Check if we can add to existing cluster
        let mut added_to_cluster = false;

        for cluster in clusters.iter_mut() {
            // Only cluster same type (filtered vs non-filtered)
            if cluster.is_filtered_out == is_filtered_out {
                let distance = (x - cluster.x_position).abs();
                if distance < cluster_threshold {
                    cluster.events.push(event.clone());
                    // Update cluster position to average
                    let total_x: f32 = cluster
                        .events
                        .iter()
                        .filter_map(|e| timestamp_to_ms(&e.timestamp))
                        .map(|ts| ((ts - padded_min) as f32 / padded_range as f32) * 100.0 * zoom)
                        .sum();
                    cluster.x_position = total_x / cluster.events.len() as f32;
                    added_to_cluster = true;
                    break;
                }
            }
        }

        if !added_to_cluster {
            clusters.push(TimelineCluster {
                events: vec![event],
                x_position: x,
                is_filtered_out,
            });
        }
    }

    clusters
}

/// Get color for event type
fn get_event_color(event_type: &StoryEventTypeData) -> &'static str {
    match event_type {
        StoryEventTypeData::LocationChange { .. } => "#22c55e", // green
        StoryEventTypeData::DialogueExchange { .. } => "#3b82f6", // blue
        StoryEventTypeData::CombatEvent { .. } => "#ef4444",    // red
        StoryEventTypeData::ChallengeAttempted { .. } => "#f59e0b", // amber
        StoryEventTypeData::ItemAcquired { .. } => "#a855f7",   // purple
        StoryEventTypeData::RelationshipChanged { .. } => "#ec4899", // pink
        StoryEventTypeData::SceneTransition { .. } => "#06b6d4", // cyan
        StoryEventTypeData::InformationRevealed { .. } => "#eab308", // yellow
        StoryEventTypeData::DmMarker { .. } => "#8b5cf6",       // violet
        StoryEventTypeData::NarrativeEventTriggered { .. } => "#f97316", // orange
        StoryEventTypeData::SessionStarted { .. } => "#10b981", // emerald
        StoryEventTypeData::SessionEnded { .. } => "#6b7280",   // gray
        StoryEventTypeData::Custom { .. } => "#64748b",         // slate
    }
}

/// Format date for display
fn format_date(timestamp: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%b %d").to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Format time for tooltip
fn format_datetime(timestamp: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%b %d, %H:%M").to_string()
    } else {
        timestamp.to_string()
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct VisualTimelineProps {
    pub world_id: String,
}

#[component]
pub fn VisualTimeline(props: VisualTimelineProps) -> Element {
    let game_state = use_game_state();
    let story_event_service = use_story_event_service();

    // State
    let mut events: Signal<Vec<StoryEventData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut filters = use_signal(TimelineFilterState::default);
    let mut show_filters = use_signal(|| false);

    // Zoom and pan state
    let mut zoom_level = use_signal(|| 1.0_f32);
    let mut scroll_offset = use_signal(|| 0.0_f32);

    // Interaction state
    let mut hovered_event: Signal<Option<StoryEventData>> = use_signal(|| None);
    let mut selected_event: Signal<Option<StoryEventData>> = use_signal(|| None);
    let mut expanded_cluster_idx: Signal<Option<usize>> = use_signal(|| None);

    // Container width (approximate for clustering)
    let container_width = 800.0_f32;

    // Load events
    let world_id = props.world_id.clone();
    let service = story_event_service.clone();
    use_effect(move || {
        let world_id = world_id.clone();
        let service = service.clone();
        spawn(async move {
            is_loading.set(true);
            error.set(None);
            match service.list_story_events(&world_id).await {
                Ok(loaded) => events.set(loaded),
                Err(e) => error.set(Some(format!("Failed to load: {}", e))),
            }
            is_loading.set(false);
        });
    });

    // Filter events
    let all_events = events.read().clone();
    let filter_state = filters.read().clone();
    let vm = TimelineViewModel::new(&all_events, &filter_state);
    let filtered_events = vm.filtered_events();

    // Cluster events
    let zoom = *zoom_level.read();
    let clusters = cluster_events(&all_events, &filtered_events, zoom, container_width);

    // Extract options for filters
    let (characters, locations) = {
        let world = game_state.world.read();
        if let Some(ref snapshot) = *world {
            let chars = snapshot
                .characters
                .iter()
                .map(|c| CharacterOption {
                    id: c.id.clone(),
                    name: c.name.clone(),
                })
                .collect::<Vec<_>>();
            let locs = snapshot
                .locations
                .iter()
                .map(|l| LocationOption {
                    id: l.id.clone(),
                    name: l.name.clone(),
                })
                .collect::<Vec<_>>();
            (chars, locs)
        } else {
            (Vec::new(), Vec::new())
        }
    };

    // Calculate date markers from all events
    let date_markers: Vec<(String, f32)> = {
        let mut markers = std::collections::BTreeMap::new();

        if !all_events.is_empty() {
            let mut min_ts: Option<i64> = None;
            let mut max_ts: Option<i64> = None;

            for event in &all_events {
                if let Some(ts) = timestamp_to_ms(&event.timestamp) {
                    min_ts = Some(min_ts.map_or(ts, |m| m.min(ts)));
                    max_ts = Some(max_ts.map_or(ts, |m| m.max(ts)));

                    let date = format_date(&event.timestamp);
                    markers.entry(date).or_insert(ts);
                }
            }

            if let (Some(min), Some(max)) = (min_ts, max_ts) {
                let range = (max - min).max(1000);
                let padded_min = min - (range / 20);
                let padded_range = range + (range / 10);

                markers
                    .into_iter()
                    .map(|(date, ts)| {
                        let x = ((ts - padded_min) as f32 / padded_range as f32) * 100.0 * zoom;
                        (date, x)
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    rsx! {
        div {
            class: "visual-timeline h-full flex flex-col bg-dark-bg",

            // Header with controls
            div {
                class: "flex justify-between items-center p-4 border-b border-gray-700",

                div {
                    class: "flex items-center gap-4",
                    h2 { class: "text-white m-0 text-xl", "Visual Timeline" }

                    // Filter toggle
                    {
                        let is_showing = *show_filters.read();
                        rsx! {
                            button {
                                onclick: move |_| show_filters.set(!is_showing),
                                class: if is_showing {
                                    "px-3 py-1.5 bg-blue-500 text-white border-none rounded cursor-pointer text-sm"
                                } else {
                                    "px-3 py-1.5 bg-gray-700 text-gray-300 border-none rounded cursor-pointer text-sm"
                                },
                                "Filters"
                            }
                        }
                    }
                }

                // Zoom controls
                div {
                    class: "flex items-center gap-2",

                    button {
                        onclick: move |_| {
                            let new_zoom = (*zoom_level.read() - 0.25).max(0.25);
                            zoom_level.set(new_zoom);
                        },
                        disabled: *zoom_level.read() <= 0.25,
                        class: "w-8 h-8 bg-gray-700 text-white border-none rounded cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                        "-"
                    }

                    span {
                        class: "text-gray-400 text-sm min-w-[60px] text-center",
                        "{(*zoom_level.read() * 100.0) as i32}%"
                    }

                    button {
                        onclick: move |_| {
                            let new_zoom = (*zoom_level.read() + 0.25).min(4.0);
                            zoom_level.set(new_zoom);
                        },
                        disabled: *zoom_level.read() >= 4.0,
                        class: "w-8 h-8 bg-gray-700 text-white border-none rounded cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                        "+"
                    }

                    button {
                        onclick: move |_| zoom_level.set(1.0),
                        class: "px-2 py-1 bg-gray-700 text-gray-300 border-none rounded cursor-pointer text-xs ml-2",
                        "Reset"
                    }
                }
            }

            // Filters panel (collapsible)
            if *show_filters.read() {
                div {
                    class: "p-4 border-b border-gray-700 bg-dark-surface",
                    TimelineFilters {
                        filters: filters,
                        on_filter_change: move |new_filters: TimelineFilterState| filters.set(new_filters),
                        characters: characters.clone(),
                        locations: locations.clone(),
                    }
                }
            }

            // Timeline area
            div {
                class: "flex-1 relative overflow-hidden",

                if *is_loading.read() {
                    div {
                        class: "flex justify-center items-center h-full text-gray-400",
                        "Loading timeline..."
                    }
                } else if let Some(err) = error.read().as_ref() {
                    div {
                        class: "flex justify-center items-center h-full text-red-500",
                        "Error: {err}"
                    }
                } else if all_events.is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center h-full text-gray-500",
                        div { class: "text-5xl mb-4", "📊" }
                        p { "No events recorded yet" }
                        p { class: "text-sm", "Events will appear here as gameplay progresses" }
                    }
                } else {
                    // Pan buttons
                    div {
                        class: "absolute left-2 top-1/2 -translate-y-1/2 z-10",
                        button {
                            onclick: move |_| {
                                let new_offset = (*scroll_offset.read() - 100.0).max(0.0);
                                scroll_offset.set(new_offset);
                            },
                            class: "w-10 h-10 bg-gray-800 text-white border border-gray-600 rounded-full cursor-pointer hover:bg-gray-700",
                            "◀"
                        }
                    }

                    div {
                        class: "absolute right-2 top-1/2 -translate-y-1/2 z-10",
                        button {
                            onclick: move |_| {
                                let max_scroll = (zoom * 100.0 - 100.0).max(0.0);
                                let new_offset = (*scroll_offset.read() + 100.0).min(max_scroll);
                                scroll_offset.set(new_offset);
                            },
                            class: "w-10 h-10 bg-gray-800 text-white border border-gray-600 rounded-full cursor-pointer hover:bg-gray-700",
                            "▶"
                        }
                    }

                    // Scrollable timeline container
                    div {
                        class: "h-full overflow-x-auto px-14",
                        style: "scroll-behavior: smooth;",

                        div {
                            class: "relative h-full min-h-[300px]",
                            style: "width: {zoom * 100.0}%; min-width: 100%; transform: translateX(-{*scroll_offset.read()}px);",

                            // Date markers at top
                            div {
                                class: "absolute top-4 left-0 right-0 h-6",

                                for (date, x) in date_markers.iter() {
                                    div {
                                        key: "{date}",
                                        class: "absolute text-gray-500 text-xs whitespace-nowrap",
                                        style: "left: {x}%;",
                                        "{date}"
                                    }
                                }
                            }

                            // Timeline axis
                            div {
                                class: "absolute top-16 left-0 right-0 h-0.5 bg-gray-600"
                            }

                            // Event nodes
                            div {
                                class: "absolute top-20 left-0 right-0 bottom-20",

                                for (idx, cluster) in clusters.iter().enumerate() {
                                    {
                                        let is_expanded = expanded_cluster_idx.read().is_some_and(|i| i == idx);
                                        let show_expand = cluster.events.len() > 3 && !is_expanded;
                                        let visible_events = if is_expanded || cluster.events.len() <= 3 {
                                            cluster.events.clone()
                                        } else {
                                            cluster.events.iter().take(3).cloned().collect()
                                        };
                                        let remaining = cluster.events.len().saturating_sub(3);

                                        rsx! {
                                            div {
                                                key: "cluster-{idx}",
                                                class: "absolute flex flex-col items-center gap-1",
                                                style: "left: {cluster.x_position}%; transform: translateX(-50%);",

                                                // Vertical connector line
                                                div {
                                                    class: "w-0.5 h-4 bg-gray-600"
                                                }

                                                // Event nodes
                                                for (_event_idx, event) in visible_events.iter().enumerate() {
                                                    {
                                                        let color = if cluster.is_filtered_out {
                                                            "#4b5563" // gray for filtered out
                                                        } else {
                                                            get_event_color(&event.event_type)
                                                        };
                                                        let icon = get_event_type_icon(&event.event_type);
                                                        let event_for_hover = event.clone();
                                                        let event_for_click = event.clone();

                                                        rsx! {
                                                            div {
                                                                key: "{event.id}",
                                                                class: if cluster.is_filtered_out {
                                                                    "w-10 h-10 rounded-full flex items-center justify-center cursor-pointer opacity-40 border-2 border-gray-600 transition-all hover:scale-110"
                                                                } else {
                                                                    "w-10 h-10 rounded-full flex items-center justify-center cursor-pointer border-2 transition-all hover:scale-110"
                                                                },
                                                                style: "background-color: {color}; border-color: {color};",
                                                                onmouseenter: move |_| hovered_event.set(Some(event_for_hover.clone())),
                                                                onmouseleave: move |_| hovered_event.set(None),
                                                                onclick: move |_| selected_event.set(Some(event_for_click.clone())),
                                                                title: "{event.summary}",
                                                                span { class: "text-lg", "{icon}" }
                                                            }
                                                        }
                                                    }
                                                }

                                                // "+N more" button for clusters
                                                if show_expand {
                                                    button {
                                                        onclick: move |_| expanded_cluster_idx.set(Some(idx)),
                                                        class: "px-2 py-1 bg-gray-700 text-gray-300 text-xs rounded border-none cursor-pointer hover:bg-gray-600",
                                                        "+{remaining} more"
                                                    }
                                                }

                                                // Collapse button for expanded clusters
                                                if is_expanded && cluster.events.len() > 3 {
                                                    button {
                                                        onclick: move |_| expanded_cluster_idx.set(None),
                                                        class: "px-2 py-1 bg-gray-600 text-gray-300 text-xs rounded border-none cursor-pointer hover:bg-gray-500",
                                                        "Show less"
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
            }

            // Hover tooltip / info bar
            div {
                class: "h-16 border-t border-gray-700 px-4 py-2 bg-dark-surface flex items-center",

                if let Some(event) = hovered_event.read().as_ref() {
                    {
                        let icon = get_event_type_icon(&event.event_type);
                        let color = get_event_color(&event.event_type);
                        rsx! {
                            div {
                                class: "flex items-center gap-3",

                                div {
                                    class: "w-8 h-8 rounded-full flex items-center justify-center",
                                    style: "background-color: {color};",
                                    span { "{icon}" }
                                }

                                div {
                                    p {
                                        class: "text-white m-0 text-sm line-clamp-1",
                                        "{event.summary}"
                                    }
                                    p {
                                        class: "text-gray-500 m-0 text-xs",
                                        "{format_datetime(&event.timestamp)}"
                                        if !event.involved_characters.is_empty() {
                                            " • {event.involved_characters.len()} character(s)"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div {
                        class: "text-gray-500 text-sm",
                        "Hover over an event to see details • Click to view full info"
                    }
                }
            }

            // Event detail modal
            if let Some(event) = selected_event.read().as_ref() {
                EventDetailModal {
                    event: event.clone(),
                    on_close: move |_| selected_event.set(None),
                }
            }
        }
    }
}

/// Event detail modal (simplified version, reuses structure from timeline_view)
#[derive(Props, Clone)]
struct EventDetailModalProps {
    event: StoryEventData,
    on_close: EventHandler<()>,
}

impl PartialEq for EventDetailModalProps {
    fn eq(&self, other: &Self) -> bool {
        self.event.id == other.event.id
    }
}

#[component]
fn EventDetailModal(props: EventDetailModalProps) -> Element {
    let event = &props.event;
    let icon = get_event_type_icon(&event.event_type);
    let color = get_event_color(&event.event_type);

    // Get event type name
    let type_name = match &event.event_type {
        StoryEventTypeData::LocationChange { .. } => "Location Change",
        StoryEventTypeData::DialogueExchange { .. } => "Dialogue",
        StoryEventTypeData::CombatEvent { .. } => "Combat",
        StoryEventTypeData::ChallengeAttempted { .. } => "Challenge",
        StoryEventTypeData::ItemAcquired { .. } => "Item Acquired",
        StoryEventTypeData::RelationshipChanged { .. } => "Relationship",
        StoryEventTypeData::SceneTransition { .. } => "Scene Transition",
        StoryEventTypeData::InformationRevealed { .. } => "Information",
        StoryEventTypeData::DmMarker { .. } => "DM Marker",
        StoryEventTypeData::NarrativeEventTriggered { .. } => "Narrative Event",
        StoryEventTypeData::SessionStarted { .. } => "Session Start",
        StoryEventTypeData::SessionEnded { .. } => "Session End",
        StoryEventTypeData::Custom { .. } => "Custom",
    };

    rsx! {
        div {
            class: "modal-overlay fixed inset-0 bg-black bg-opacity-80 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl p-6 max-w-[600px] w-[90%] max-h-[80vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-start mb-4",

                    div {
                        class: "flex items-center gap-3",
                        div {
                            class: "w-10 h-10 rounded-full flex items-center justify-center",
                            style: "background-color: {color};",
                            span { class: "text-xl", "{icon}" }
                        }
                        div {
                            h3 { class: "text-white m-0 text-lg", "{type_name}" }
                            p { class: "text-gray-500 m-0 text-xs", "{format_datetime(&event.timestamp)}" }
                        }
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-none text-gray-400 text-2xl cursor-pointer hover:text-white",
                        "x"
                    }
                }

                // Summary
                div {
                    class: "bg-dark-bg rounded-lg p-4 mb-4",
                    p { class: "text-white m-0", "{event.summary}" }
                }

                // Event-specific details
                div {
                    class: "flex flex-col gap-3",

                    match &event.event_type {
                        StoryEventTypeData::DialogueExchange { npc_name, player_dialogue, npc_response, topics_discussed, .. } => rsx! {
                            DetailRow { label: "NPC", value: npc_name.clone() }
                            DetailRow { label: "Player said", value: player_dialogue.clone() }
                            DetailRow { label: "NPC responded", value: npc_response.clone() }
                            if !topics_discussed.is_empty() {
                                DetailRow { label: "Topics", value: topics_discussed.join(", ") }
                            }
                        },
                        StoryEventTypeData::ChallengeAttempted { challenge_name, skill_used, roll_result, outcome, .. } => rsx! {
                            DetailRow { label: "Challenge", value: challenge_name.clone() }
                            if let Some(skill) = skill_used {
                                DetailRow { label: "Skill", value: skill.clone() }
                            }
                            if let Some(roll) = roll_result {
                                DetailRow { label: "Roll", value: roll.to_string() }
                            }
                            DetailRow { label: "Outcome", value: outcome.clone() }
                        },
                        StoryEventTypeData::DmMarker { title, note, importance, marker_type } => rsx! {
                            DetailRow { label: "Title", value: title.clone() }
                            DetailRow { label: "Note", value: note.clone() }
                            DetailRow { label: "Importance", value: importance.clone() }
                            DetailRow { label: "Type", value: marker_type.clone() }
                        },
                        StoryEventTypeData::LocationChange { to_location, from_location, .. } => rsx! {
                            if let Some(ref from) = from_location {
                                DetailRow { label: "From", value: from.clone() }
                            }
                            DetailRow { label: "To", value: to_location.clone() }
                        },
                        StoryEventTypeData::ItemAcquired { item_name, quantity, source, .. } => rsx! {
                            DetailRow { label: "Item", value: item_name.clone() }
                            DetailRow { label: "Quantity", value: quantity.to_string() }
                            DetailRow { label: "Source", value: source.clone() }
                        },
                        _ => rsx! {}
                    }

                    // Tags
                    if !event.tags.is_empty() {
                        div {
                            class: "flex flex-wrap gap-1 mt-2",
                            for tag in event.tags.iter() {
                                span {
                                    class: "bg-gray-700 text-gray-400 px-2 py-1 rounded text-xs",
                                    "#{tag}"
                                }
                            }
                        }
                    }

                    // Game time if available
                    if let Some(game_time) = &event.game_time {
                        div {
                            class: "text-gray-500 text-sm mt-2",
                            "Game time: {game_time}"
                        }
                    }

                    // Visibility status
                    if event.is_hidden {
                        div {
                            class: "text-amber-500 text-sm mt-2",
                            "Hidden from timeline"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DetailRowProps {
    label: String,
    value: String,
}

#[component]
fn DetailRow(props: DetailRowProps) -> Element {
    rsx! {
        div {
            class: "flex gap-2",
            span { class: "text-gray-500 min-w-[100px]", "{props.label}:" }
            span { class: "text-white", "{props.value}" }
        }
    }
}
