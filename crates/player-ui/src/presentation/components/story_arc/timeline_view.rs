//! Timeline View - Display past story events

use dioxus::prelude::*;

use wrldbldr_player_app::application::dto::{StoryEventData, StoryEventTypeData};
use crate::presentation::components::story_arc::add_dm_marker::AddDmMarkerModal;
use crate::presentation::components::story_arc::timeline_event_card::TimelineEventCard;
use crate::presentation::components::story_arc::timeline_filters::{CharacterOption, LocationOption, TimelineFilters};
use crate::presentation::services::use_story_event_service;
use crate::presentation::state::use_game_state;

/// Filter options for the timeline
#[derive(Debug, Clone, Default)]
pub struct TimelineFilterState {
    pub event_type: Option<String>,
    pub character_id: Option<String>,
    pub location_id: Option<String>,
    pub search_text: String,
    pub show_hidden: bool,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// Simple view-model/helper that encapsulates filtering logic for the
/// timeline so the component can stay mostly declarative and ready for
/// future event-bus-backed projections.
pub struct TimelineViewModel<'a> {
    events: &'a [StoryEventData],
    filters: &'a TimelineFilterState,
}

impl<'a> TimelineViewModel<'a> {
    pub fn new(events: &'a [StoryEventData], filters: &'a TimelineFilterState) -> Self {
        Self { events, filters }
    }

    /// Return events that are visible under the current filters.
    pub fn filtered_events(&self) -> Vec<StoryEventData> {
        let filter_state = self.filters;

        self.events
            .iter()
            .cloned()
            .filter(|event| {
                // Hide hidden events unless show_hidden is true
                if event.is_hidden && !filter_state.show_hidden {
                    return false;
                }

                // Filter by event type
                if let Some(ref type_filter) = filter_state.event_type {
                    let event_type_name = get_event_type_name(&event.event_type);
                    if &event_type_name != type_filter {
                        return false;
                    }
                }

                // Filter by character
                if let Some(ref char_id) = filter_state.character_id {
                    if !event.involved_characters.contains(char_id) {
                        return false;
                    }
                }

                // Filter by location
                if let Some(ref loc_id) = filter_state.location_id {
                    if event.location_id.as_ref() != Some(loc_id) {
                        return false;
                    }
                }

                // Filter by search text
                if !filter_state.search_text.is_empty() {
                    let search = filter_state.search_text.to_lowercase();
                    let matches_summary = event.summary.to_lowercase().contains(&search);
                    let matches_tags =
                        event.tags.iter().any(|t| t.to_lowercase().contains(&search));
                    if !matches_summary && !matches_tags {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct TimelineViewProps {
    pub world_id: String,
}

#[component]
pub fn TimelineView(props: TimelineViewProps) -> Element {
    let game_state = use_game_state();

    let mut events: Signal<Vec<StoryEventData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut filters = use_signal(TimelineFilterState::default);
    let mut show_add_marker = use_signal(|| false);
    let mut selected_event: Signal<Option<StoryEventData>> = use_signal(|| None);

    // Get story event service
    let story_event_service = use_story_event_service();
    let story_event_service_for_effect = story_event_service.clone();

    // Load events when component mounts or world changes
    let world_id = props.world_id.clone();
    use_effect(move || {
        let world_id = world_id.clone();
        let service = story_event_service_for_effect.clone();
        spawn(async move {
            is_loading.set(true);
            error.set(None);

            match service.list_story_events(&world_id).await {
                Ok(loaded_events) => {
                    events.set(loaded_events);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load events: {}", e)));
                }
            }
            is_loading.set(false);
        });
    });

    // Filter events based on current filter state via view-model helper
    let filtered_events = {
        let filter_state = filters.read().clone();
        let all_events = events.read().clone();
        let vm = TimelineViewModel::new(&all_events, &filter_state);
        vm.filtered_events()
    };

    rsx! {
        div {
            class: "timeline-view h-full flex flex-col gap-4 p-4",

            // Header with title and add marker button
            div {
                class: "flex justify-between items-center",

                h2 { class: "text-white m-0 text-xl", "Timeline" }

                button {
                    onclick: move |_| show_add_marker.set(true),
                    class: "px-4 py-2 bg-purple-500 text-white border-none rounded-lg cursor-pointer flex items-center gap-2",
                    span { "+" }
                    span { "Add DM Marker" }
                }
            }

            // Filters
            {
                // Extract character and location options from game state
                let (characters, locations) = {
                    let world = game_state.world.read();
                    if let Some(ref snapshot) = *world {
                        let chars = snapshot.characters.iter()
                            .map(|c| CharacterOption { id: c.id.clone(), name: c.name.clone() })
                            .collect::<Vec<_>>();
                        let locs = snapshot.locations.iter()
                            .map(|l| LocationOption { id: l.id.clone(), name: l.name.clone() })
                            .collect::<Vec<_>>();
                        (chars, locs)
                    } else {
                        (Vec::new(), Vec::new())
                    }
                };

                rsx! {
                    TimelineFilters {
                        filters: filters.clone(),
                        on_filter_change: move |new_filters: TimelineFilterState| filters.set(new_filters),
                        characters: characters,
                        locations: locations,
                    }
                }
            }

            // Event list
            div {
                class: "flex-1 overflow-y-auto flex flex-col gap-3",

                if *is_loading.read() {
                    div {
                        class: "flex justify-center items-center p-12 text-gray-400",
                        "Loading timeline..."
                    }
                } else if let Some(err) = error.read().as_ref() {
                    div {
                        class: "bg-red-500 bg-opacity-10 border border-red-500 rounded-lg p-4 text-red-500",
                        "Error loading timeline: {err}"
                    }
                } else if filtered_events.is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center p-12 text-gray-500",

                        div { class: "text-5xl mb-4", "📜" }

                        if events.read().is_empty() {
                            p { "No events recorded yet" }
                            p { class: "text-sm", "Events will appear here as gameplay progresses" }
                        } else {
                            p { "No events match your filters" }
                            button {
                                onclick: move |_| filters.set(TimelineFilterState::default()),
                                class: "mt-2 px-4 py-2 bg-gray-700 text-white border-none rounded cursor-pointer",
                                "Clear Filters"
                            }
                        }
                    }
                } else {
                    // Event count
                    {
                        let count = filtered_events.len();
                        let suffix = if count == 1 { "" } else { "s" };
                        rsx! {
                            div {
                                class: "text-gray-500 text-sm mb-2",
                                "{count} event{suffix}"
                            }
                        }
                    }

                    for event in filtered_events.iter() {
                        TimelineEventCard {
                            key: "{event.id}",
                            event: event.clone(),
                            on_click: {
                                let event = event.clone();
                                move |_| selected_event.set(Some(event.clone()))
                            },
                            on_toggle_visibility: {
                                let event_id = event.id.clone();
                                let world_id = props.world_id.clone();
                                let service = story_event_service.clone();
                                move |_| {
                                    let event_id = event_id.clone();
                                    let world_id = world_id.clone();
                                    let service = service.clone();
                                    spawn(async move {
                                        if let Err(e) = service.toggle_event_visibility(&event_id).await {
                                            tracing::error!("Failed to toggle visibility: {}", e);
                                        }
                                        // Reload events
                                if let Ok(reloaded) = service.list_story_events(&world_id).await {
                                            events.set(reloaded);
                                        }
                                    });
                                }
                            },
                        }
                    }
                }
            }

            // Add DM Marker modal
            if *show_add_marker.read() {
                AddDmMarkerModal {
                    world_id: props.world_id.clone(),
                    on_close: move |_| show_add_marker.set(false),
                    on_created: {
                        let world_id = props.world_id.clone();
                        let service = story_event_service.clone();
                        move |_| {
                            show_add_marker.set(false);
                            // Reload events
                            let world_id = world_id.clone();
                            let service = service.clone();
                            spawn(async move {
                                if let Ok(reloaded) = service.list_story_events(&world_id).await {
                                    events.set(reloaded);
                                }
                            });
                        }
                    },
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

/// Get a human-readable name for an event type
fn get_event_type_name(event_type: &StoryEventTypeData) -> String {
    match event_type {
        StoryEventTypeData::LocationChange { .. } => "Location Change".to_string(),
        StoryEventTypeData::DialogueExchange { .. } => "Dialogue".to_string(),
        StoryEventTypeData::CombatEvent { .. } => "Combat".to_string(),
        StoryEventTypeData::ChallengeAttempted { .. } => "Challenge".to_string(),
        StoryEventTypeData::ItemAcquired { .. } => "Item Acquired".to_string(),
        StoryEventTypeData::RelationshipChanged { .. } => "Relationship".to_string(),
        StoryEventTypeData::SceneTransition { .. } => "Scene Transition".to_string(),
        StoryEventTypeData::InformationRevealed { .. } => "Information".to_string(),
        StoryEventTypeData::DmMarker { .. } => "DM Marker".to_string(),
        StoryEventTypeData::NarrativeEventTriggered { .. } => "Narrative Event".to_string(),
        StoryEventTypeData::SessionStarted { .. } => "Session Start".to_string(),
        StoryEventTypeData::SessionEnded { .. } => "Session End".to_string(),
        StoryEventTypeData::Custom { .. } => "Custom".to_string(),
    }
}

/// Event detail modal
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
    let type_name = get_event_type_name(&event.event_type);
    let icon = get_event_type_icon(&event.event_type);

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
                        span { class: "text-2xl", "{icon}" }
                        div {
                            h3 { class: "text-white m-0 text-lg", "{type_name}" }
                            p { class: "text-gray-500 m-0 text-xs", "{event.timestamp}" }
                        }
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-none text-gray-400 text-2xl cursor-pointer",
                        "×"
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

                    // Show event type specific details
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
                        StoryEventTypeData::SceneTransition { from_scene_name, to_scene_name, trigger_reason, .. } => rsx! {
                            if let Some(from) = from_scene_name {
                                DetailRow { label: "From", value: from.clone() }
                            }
                            DetailRow { label: "To", value: to_scene_name.clone() }
                            DetailRow { label: "Reason", value: trigger_reason.clone() }
                        },
                        StoryEventTypeData::RelationshipChanged { reason, previous_sentiment, new_sentiment, .. } => rsx! {
                            DetailRow { label: "Reason", value: reason.clone() }
                            if let Some(prev) = previous_sentiment {
                                DetailRow { label: "Previous", value: format!("{:.1}", prev) }
                            }
                            DetailRow { label: "New", value: format!("{:.1}", new_sentiment) }
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

                    // Visibility status
                    if event.is_hidden {
                        div {
                            class: "text-amber-500 text-sm mt-2",
                            "👁 Hidden from timeline"
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
            span { class: "text-gray-500 min-w-[80px]", "{props.label}:" }
            span { class: "text-white", "{props.value}" }
        }
    }
}

/// Get an icon for an event type
pub fn get_event_type_icon(event_type: &StoryEventTypeData) -> &'static str {
    match event_type {
        StoryEventTypeData::LocationChange { .. } => "🚶",
        StoryEventTypeData::DialogueExchange { .. } => "💬",
        StoryEventTypeData::CombatEvent { .. } => "⚔️",
        StoryEventTypeData::ChallengeAttempted { .. } => "🎲",
        StoryEventTypeData::ItemAcquired { .. } => "📦",
        StoryEventTypeData::RelationshipChanged { .. } => "❤️",
        StoryEventTypeData::SceneTransition { .. } => "🎬",
        StoryEventTypeData::InformationRevealed { .. } => "💡",
        StoryEventTypeData::DmMarker { .. } => "📝",
        StoryEventTypeData::NarrativeEventTriggered { .. } => "⭐",
        StoryEventTypeData::SessionStarted { .. } => "▶️",
        StoryEventTypeData::SessionEnded { .. } => "⏹️",
        StoryEventTypeData::Custom { .. } => "📌",
    }
}

