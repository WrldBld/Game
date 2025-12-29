//! Suggestion Button - LLM-powered content suggestions
//!
//! A reusable button component that fetches suggestions from the Engine
//! and displays them in a dropdown for selection.

use dioxus::prelude::*;

use crate::presentation::services::use_suggestion_service;
use crate::presentation::state::use_generation_state;
use wrldbldr_player_adapters::Platform;
use wrldbldr_player_app::application::services::SuggestionContext;

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
/// Fetches suggestions from the API when clicked and displays them
/// in a dropdown. When a suggestion is selected, it calls the on_select handler.
#[component]
pub fn SuggestionButton(
    suggestion_type: SuggestionType,
    /// World ID for routing - required to receive WebSocket response
    world_id: String,
    context: SuggestionContext,
    on_select: EventHandler<String>,
) -> Element {
    let platform = use_context::<Platform>();
    let suggestion_service = use_suggestion_service();
    let mut generation_state = use_generation_state();
    let mut loading = use_signal(|| false);
    let mut request_id: Signal<Option<String>> = use_signal(|| None);
    let mut suggestions: Signal<Vec<String>> = use_signal(Vec::new);
    let mut show_dropdown = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Watch for suggestion completion from queue
    let field_type = suggestion_type.to_field_type();
    use_effect(move || {
        if let Some(req_id) = request_id.read().as_ref() {
            // Check if this suggestion is ready
            let all_suggestions = generation_state.get_suggestions();
            if let Some(task) = all_suggestions.iter().find(|s| s.request_id == *req_id) {
                match &task.status {
                    crate::presentation::state::SuggestionStatus::Ready {
                        suggestions: results,
                    } => {
                        if !results.is_empty() {
                            suggestions.set(results.clone());
                            show_dropdown.set(true);
                            loading.set(false);
                        }
                    }
                    crate::presentation::state::SuggestionStatus::Failed { error: err } => {
                        error.set(Some(err.clone()));
                        loading.set(false);
                    }
                    _ => {
                        // Still processing
                    }
                }
            }
        }
    });

    // Close dropdown when clicking outside
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

            spawn(async move {
                loading.set(true);
                error.set(None);
                suggestions.set(Vec::new());

                platform.log_info(&format!("Enqueueing suggestion request for {}", field_type));

                // Enqueue the suggestion request
                match service
                    .enqueue_suggestion(&field_type, &world_id, &context)
                    .await
                {
                    Ok(req_id) => {
                        platform.log_info(&format!("Suggestion request queued: {}", req_id));
                        request_id.set(Some(req_id.clone()));

                        // Add to generation state with context for retry
                        generation_state.add_suggestion_task(
                            req_id,
                            field_type,
                            None,                   // entity_id not available here
                            Some(context.clone()),  // Store context for retry
                            Some(world_id.clone()), // Store world_id for retry
                        );
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
                disabled: *loading.read() || request_id.read().is_some(),
                class: "py-2 px-3 bg-purple-500 text-white border-0 rounded cursor-pointer text-xs whitespace-nowrap transition-colors",
                onmouseenter: move |_| {},  // Could add hover state
                if *loading.read() || request_id.read().is_some() {
                    "Queued..."
                } else {
                    "Suggest"
                }
            }

            // Error tooltip
            if let Some(err) = error.read().as_ref() {
                div {
                    class: "absolute top-full left-0 mt-1 p-2 bg-red-500 text-white rounded text-xs whitespace-nowrap z-100",
                    "{err}"
                }
            }

            // Dropdown with suggestions
            if *show_dropdown.read() && !suggestions.read().is_empty() {
                // Backdrop to catch outside clicks
                div {
                    onclick: close_dropdown,
                    class: "fixed inset-0 z-99",
                }

                // Dropdown menu
                div {
                    class: "suggestion-dropdown absolute top-full right-0 mt-1 min-w-48 max-w-md max-h-72 overflow-y-auto bg-gray-800 border border-gray-700 rounded-md z-100 shadow-lg",

                    for (i, suggestion) in suggestions.read().iter().enumerate() {
                        SuggestionItem {
                            key: "{i}",
                            text: suggestion.clone(),
                            on_click: {
                                let suggestion = suggestion.clone();
                                move |_| {
                                    on_select.call(suggestion.clone());
                                    show_dropdown.set(false);
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Individual suggestion item in the dropdown
#[component]
fn SuggestionItem(text: String, on_click: EventHandler<()>) -> Element {
    rsx! {
        div {
            onclick: move |_| on_click.call(()),
            class: "py-3 px-4 text-gray-200 cursor-pointer border-b border-gray-700 transition-colors",
            onmouseenter: move |_evt| {
                // Would be nice to highlight on hover, but we can't easily change style here
                // In real app, would use a class and CSS hover state
            },
            "{text}"
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
