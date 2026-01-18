//! Suggestion Button - LLM-powered content suggestions
//!
//! A reusable button component that fetches suggestions from the Engine
//! and displays them in a dropdown for selection.

use dioxus::prelude::*;

use crate::application::services::SuggestionContext;
use crate::infrastructure::spawn_task;
use crate::presentation::services::use_suggestion_service;
use crate::presentation::state::use_generation_state;
use crate::use_platform;

/// Types of suggestions that can be requested
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestionType {
    CharacterName,
    CharacterDescription,
    CharacterWants,
    CharacterFears,
    CharacterBackstory,
    LocationName,
    LocationDescription,
    LocationAtmosphere,
    LocationFeatures,
    LocationSecrets,
    // Actantial Model suggestions
    DeflectionBehavior,
    BehavioralTells,
    WantDescription,
    ActantialReason,
}

impl SuggestionType {
    /// Convert to field type string for API
    pub fn to_field_type(&self) -> &'static str {
        match self {
            SuggestionType::CharacterName => "character_name",
            SuggestionType::CharacterDescription => "character_description",
            SuggestionType::CharacterWants => "character_wants",
            SuggestionType::CharacterFears => "character_fears",
            SuggestionType::CharacterBackstory => "character_backstory",
            SuggestionType::LocationName => "location_name",
            SuggestionType::LocationDescription => "location_description",
            SuggestionType::LocationAtmosphere => "location_atmosphere",
            SuggestionType::LocationFeatures => "location_features",
            SuggestionType::LocationSecrets => "location_secrets",
            // Actantial Model suggestions
            SuggestionType::DeflectionBehavior => "deflection_behavior",
            SuggestionType::BehavioralTells => "behavioral_tells",
            SuggestionType::WantDescription => "want_description",
            SuggestionType::ActantialReason => "actantial_reason",
        }
    }
}

/// Suggestion button component with dropdown
///
/// Fetches suggestions from the API when clicked. The button subscribes to
/// suggestions by field_type - when suggestions arrive for this field type,
/// they appear in a dropdown next to the input. No request ID tracking needed.
#[component]
pub fn SuggestionButton(
    suggestion_type: SuggestionType,
    /// World ID for routing - required to receive WebSocket response
    world_id: String,
    context: SuggestionContext,
    on_select: EventHandler<String>,
) -> Element {
    let platform = use_platform();
    let suggestion_service = use_suggestion_service();
    let mut generation_state = use_generation_state();
    let mut loading = use_signal(|| false);
    let mut show_dropdown = use_signal(|| false);
    let mut current_suggestions: Signal<Vec<String>> = use_signal(Vec::new);
    let mut current_request_id: Signal<Option<String>> = use_signal(|| None);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let field_type = suggestion_type.to_field_type();

    // Watch for suggestions by field_type (not request_id)
    // This allows the button to "subscribe" to suggestions for its field type
    use_effect(move || {
        let all_suggestions = generation_state.get_suggestions();

        // Find the most recent Ready suggestion for this field_type
        if let Some(task) = all_suggestions.iter().find(|s| s.field_type == field_type) {
            match &task.status {
                crate::presentation::state::SuggestionStatus::Ready { suggestions } => {
                    if !suggestions.is_empty() && !*show_dropdown.read() {
                        // New suggestions arrived - show dropdown
                        current_suggestions.set(suggestions.clone());
                        current_request_id.set(Some(task.request_id.clone()));
                        show_dropdown.set(true);
                        loading.set(false);
                    }
                }
                crate::presentation::state::SuggestionStatus::Failed { error: err } => {
                    error.set(Some(err.clone()));
                    loading.set(false);
                    show_dropdown.set(false);
                }
                crate::presentation::state::SuggestionStatus::Queued
                | crate::presentation::state::SuggestionStatus::Processing => {
                    // Still loading
                    loading.set(true);
                }
            }

            // Also handle selection from modal (if user clicks in GenerationQueuePanel)
            if let Some(selected_text) = &task.selected_suggestion {
                on_select.call(selected_text.clone());
                show_dropdown.set(false);
                loading.set(false);
            }
        } else {
            // No suggestion for this field_type - reset state
            if *loading.read() || *show_dropdown.read() {
                loading.set(false);
                show_dropdown.set(false);
            }
        }
    });

    let close_dropdown = move |_| {
        show_dropdown.set(false);
    };

    let fetch_suggestions = {
        let svc = suggestion_service.clone();
        let plat = platform.clone();
        let field_type_str = field_type.to_string();
        let world_id = world_id.clone();
        move |_| {
            let context = context.clone();
            let field_type = field_type_str.clone();
            let service = svc.clone();
            let platform = plat.clone();
            let world_id = world_id.clone();

            spawn_task(async move {
                loading.set(true);
                error.set(None);
                show_dropdown.set(false);
                current_suggestions.set(Vec::new());

                platform.log_info(&format!("Enqueueing suggestion request for {}", field_type));

                match service
                    .enqueue_suggestion(&field_type, &world_id, &context)
                    .await
                {
                    Ok(req_id) => {
                        platform.log_info(&format!("Suggestion request queued: {}", req_id));

                        generation_state.add_suggestion_task(
                            req_id,
                            field_type,
                            None,
                            Some(context.clone()),
                            Some(world_id.clone()),
                        );
                        // Loading state will be managed by use_effect watching for status changes
                    }
                    Err(e) => {
                        platform.log_error(&format!("Failed to enqueue suggestion: {}", e));
                        error.set(Some(e.to_string()));
                        loading.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "suggestion-button-container relative inline-block",

            // The button
            button {
                onclick: fetch_suggestions,
                disabled: *loading.read(),
                class: "py-2 px-3 bg-purple-500 text-white border-0 rounded cursor-pointer text-xs whitespace-nowrap transition-colors hover:bg-purple-600 disabled:bg-purple-400 disabled:cursor-wait",
                if *loading.read() {
                    "Generating..."
                } else {
                    "Suggest"
                }
            }

            // Error tooltip
            if let Some(err) = error.read().as_ref() {
                div {
                    class: "absolute top-full left-0 mt-1 p-2 bg-red-500 text-white rounded text-xs whitespace-nowrap z-100 cursor-pointer",
                    onclick: move |_| error.set(None),
                    title: "Click to dismiss",
                    "{err}"
                }
            }

            // Dropdown with suggestions (appears next to the input)
            if *show_dropdown.read() && !current_suggestions.read().is_empty() {
                // Backdrop to catch outside clicks
                div {
                    onclick: close_dropdown,
                    class: "fixed inset-0 z-99",
                }

                // Dropdown menu
                div {
                    onclick: move |evt| evt.stop_propagation(),
                    class: "suggestion-dropdown absolute top-full right-0 mt-1 min-w-48 max-w-md max-h-72 overflow-y-auto bg-gray-800 border border-gray-700 rounded-md z-100 shadow-lg",

                    for (idx, suggestion) in current_suggestions.read().iter().enumerate() {
                        {
                            let suggestion_text = suggestion.clone();
                            let req_id = current_request_id.read().clone();
                            rsx! {
                                div {
                                    key: "{idx}",
                                    onclick: move |evt| {
                                        evt.stop_propagation();
                                        // Apply to form field
                                        on_select.call(suggestion_text.clone());
                                        // Close dropdown and reset state
                                        show_dropdown.set(false);
                                        current_suggestions.set(Vec::new());
                                        loading.set(false);
                                        // Remove from generation state
                                        if let Some(rid) = req_id.as_ref() {
                                            generation_state.remove_suggestion(rid);
                                        }
                                    },
                                    class: "py-3 px-4 text-gray-200 cursor-pointer border-b border-gray-700 transition-colors hover:bg-purple-700",
                                    "{suggestion}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Compact suggestion button for inline use (smaller, icon-style)
#[component]
pub fn SuggestIcon(
    suggestion_type: SuggestionType,
    /// World ID for routing - required to receive WebSocket response
    world_id: String,
    context: SuggestionContext,
    on_select: EventHandler<String>,
) -> Element {
    // Wrapper that uses the full SuggestionButton but with compact styling
    rsx! {
        SuggestionButton {
            suggestion_type,
            world_id,
            context,
            on_select,
        }
    }
}
