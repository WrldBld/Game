//! Challenge Library - Browse and manage challenges for a world
//!
//! Displays challenges organized by type with options to:
//! - View all challenges with filtering
//! - Quick access to favorites
//! - Create, edit, and delete challenges
//! - Toggle active/favorite status

mod challenge_editor;
mod challenge_list;
mod delete_modal;

pub use challenge_editor::ChallengeFormModal;
pub use challenge_list::ChallengeTypeSection;
pub use delete_modal::ConfirmDeleteChallengeModal;

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::presentation::services::use_challenge_service;
use wrldbldr_player_app::application::dto::{ChallengeData, ChallengeType, SkillData};

/// Props for ChallengeLibrary
#[derive(Props, Clone, PartialEq)]
pub struct ChallengeLibraryProps {
    /// The world ID to manage challenges for
    pub world_id: String,
    /// Skills for this world (for skill name lookup)
    pub skills: Vec<SkillData>,
    /// Called when the panel should close
    pub on_close: EventHandler<()>,
    /// Called when a challenge should be triggered
    pub on_trigger_challenge: Option<EventHandler<ChallengeData>>,
}

/// Challenge Library component
#[component]
pub fn ChallengeLibrary(props: ChallengeLibraryProps) -> Element {
    let mut challenges: Signal<Vec<ChallengeData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut filter_type: Signal<Option<ChallengeType>> = use_signal(|| None);
    let mut search_query = use_signal(String::new);
    let mut show_only_favorites = use_signal(|| false);
    let mut show_only_active = use_signal(|| false);
    let mut show_create_form = use_signal(|| false);
    let mut editing_challenge: Signal<Option<ChallengeData>> = use_signal(|| None);
    let mut show_delete_confirmation: Signal<Option<String>> = use_signal(|| None);
    let mut is_deleting = use_signal(|| false);

    // Build skill lookup map
    let skills_map: HashMap<String, String> = props
        .skills
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    let world_id = props.world_id.clone();
    let world_id_for_effect = world_id.clone();

    // Get challenge service
    let challenge_service = use_challenge_service();
    let challenge_service_for_effect = challenge_service.clone();

    // Load challenges on mount
    use_effect(move || {
        let world_id = world_id_for_effect.clone();
        let service = challenge_service_for_effect.clone();
        spawn(async move {
            match service.list_challenges(&world_id).await {
                Ok(list) => {
                    challenges.set(list);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load challenges: {}", e)));
                    is_loading.set(false);
                }
            }
        });
    });

    // Filter challenges based on current filters
    let filtered_challenges: Vec<ChallengeData> = {
        let all_challenges = challenges.read();
        let search = search_query.read().to_lowercase();
        let type_filter = *filter_type.read();
        let favorites_only = *show_only_favorites.read();
        let active_only = *show_only_active.read();

        all_challenges
            .iter()
            .filter(|c| {
                // Type filter
                if let Some(t) = type_filter {
                    if c.challenge_type != t {
                        return false;
                    }
                }
                // Favorites filter
                if favorites_only && !c.is_favorite {
                    return false;
                }
                // Active filter
                if active_only && !c.active {
                    return false;
                }
                // Search filter
                if !search.is_empty() {
                    let name_match = c.name.to_lowercase().contains(&search);
                    let desc_match = c.description.to_lowercase().contains(&search);
                    let tag_match = c.tags.iter().any(|t| t.to_lowercase().contains(&search));
                    if !name_match && !desc_match && !tag_match {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    };

    // Group challenges by type for display
    let challenges_by_type: HashMap<ChallengeType, Vec<ChallengeData>> = {
        let mut grouped: HashMap<ChallengeType, Vec<ChallengeData>> = HashMap::new();
        for challenge in filtered_challenges.iter() {
            grouped
                .entry(challenge.challenge_type)
                .or_default()
                .push(challenge.clone());
        }
        // Sort by order within each type
        for challenges_vec in grouped.values_mut() {
            challenges_vec.sort_by(|a, b| {
                // Favorites first, then by order
                b.is_favorite
                    .cmp(&a.is_favorite)
                    .then(a.order.cmp(&b.order))
            });
        }
        grouped
    };

    let handle_toggle_favorite = {
        let service = challenge_service.clone();
        move |challenge_id: String| {
            let id = challenge_id.clone();
            let service = service.clone();
            spawn(async move {
                // Save original state for rollback
                let mut challenges_write = challenges.write();
                let original_state = challenges_write
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.is_favorite);

                if let Some(c) = challenges_write.iter_mut().find(|c| c.id == id) {
                    c.is_favorite = !c.is_favorite;
                }
                drop(challenges_write);

                // Call API via service
                match service.toggle_favorite(&id).await {
                    Ok(is_favorite) => {
                        // Update with confirmed state from server
                        let mut challenges_write = challenges.write();
                        if let Some(c) = challenges_write.iter_mut().find(|c| c.id == id) {
                            c.is_favorite = is_favorite;
                        }
                    }
                    Err(_) => {
                        // Rollback on error
                        let mut challenges_write = challenges.write();
                        if let Some(c) = challenges_write.iter_mut().find(|c| c.id == id) {
                            if let Some(original) = original_state {
                                c.is_favorite = original;
                            }
                        }
                    }
                }
            });
        }
    };

    let handle_toggle_active = {
        let service = challenge_service.clone();
        move |challenge_id: String| {
            let id = challenge_id.clone();
            let service = service.clone();
            spawn(async move {
                // Save original state for rollback
                let mut challenges_write = challenges.write();
                let original_active = challenges_write
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.active);

                if let Some(c) = challenges_write.iter_mut().find(|c| c.id == id) {
                    c.active = !c.active;
                }
                drop(challenges_write);

                let new_active = match original_active {
                    Some(was_active) => !was_active,
                    None => true,
                };

                // Call API via service
                match service.set_active(&id, new_active).await {
                    Ok(()) => {
                        // State already updated optimistically, confirmed by server
                    }
                    Err(_) => {
                        // Rollback on error
                        let mut challenges_write = challenges.write();
                        if let Some(c) = challenges_write.iter_mut().find(|c| c.id == id) {
                            if let Some(original) = original_active {
                                c.active = original;
                            }
                        }
                    }
                }
            });
        }
    };

    let handle_delete = move |challenge_id: String| {
        show_delete_confirmation.set(Some(challenge_id));
    };

    let do_delete = {
        let service = challenge_service.clone();
        move |_| {
            if let Some(challenge_id) = show_delete_confirmation.read().clone() {
                let id = challenge_id.clone();
                let service = service.clone();
                spawn(async move {
                    is_deleting.set(true);
                    if service.delete_challenge(&id).await.is_ok() {
                        challenges.write().retain(|c| c.id != id);
                        show_delete_confirmation.set(None);
                    } else {
                        is_deleting.set(false);
                    }
                });
            }
        }
    };

    let cancel_delete = move |_| {
        show_delete_confirmation.set(None);
        is_deleting.set(false);
    };

    let type_value = match *filter_type.read() {
        Some(t) => format!("{:?}", t),
        None => String::new(),
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/85 flex items-center justify-center z-[1000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-xl w-[95%] max-w-[1000px] max-h-[90vh] overflow-hidden flex flex-col",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center px-6 py-4 border-b border-gray-700 bg-black/20",

                    h2 { class: "text-white m-0 text-xl", "Challenge Library" }

                    div { class: "flex gap-3 items-center",
                        button {
                            onclick: move |_| show_create_form.set(true),
                            class: "px-4 py-2 bg-emerald-500 text-white border-0 rounded-lg cursor-pointer text-sm",
                            "+ New Challenge"
                        }

                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "p-2 bg-transparent border-0 text-gray-400 cursor-pointer text-2xl",
                            "×"
                        }
                    }
                }

                // Filters bar
                div {
                    class: "px-6 py-3 border-b border-gray-700 flex flex-wrap gap-3 items-center",

                    // Search input
                    input {
                        r#type: "text",
                        placeholder: "Search challenges...",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                        class: "px-3 py-2 bg-dark-bg border border-gray-700 rounded text-white flex-1 min-w-[200px]",
                    }

                    // Type filter dropdown
                    select {
                        value: "{type_value}",
                        onchange: move |e| {
                            let val = e.value();
                            filter_type.set(if val.is_empty() {
                                None
                            } else {
                                match val.as_str() {
                                    "SkillCheck" => Some(ChallengeType::SkillCheck),
                                    "AbilityCheck" => Some(ChallengeType::AbilityCheck),
                                    "SavingThrow" => Some(ChallengeType::SavingThrow),
                                    "OpposedCheck" => Some(ChallengeType::OpposedCheck),
                                    "ComplexChallenge" => Some(ChallengeType::ComplexChallenge),
                                    _ => None,
                                }
                            });
                        },
                        class: "p-2 bg-dark-bg border border-gray-700 rounded text-white",
                        option { value: "", "All Types" }
                        for challenge_type in ChallengeType::all() {
                            option { value: "{challenge_type:?}", "{challenge_type.display_name()}" }
                        }
                    }

                    // Toggle buttons
                    label { class: "flex items-center gap-1.5 text-gray-400 text-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: *show_only_favorites.read(),
                            onchange: move |e| show_only_favorites.set(e.checked()),
                        }
                        "Favorites"
                    }

                    label { class: "flex items-center gap-1.5 text-gray-400 text-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: *show_only_active.read(),
                            onchange: move |e| show_only_active.set(e.checked()),
                        }
                        "Active Only"
                    }
                }

                // Error message
                if let Some(err) = error.read().as_ref() {
                    div {
                        class: "px-6 py-3 bg-red-500/10 text-red-500 text-sm",
                        "{err}"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4 px-6",

                    if *is_loading.read() {
                        div {
                            class: "flex items-center justify-center p-12 text-gray-400",
                            "Loading challenges..."
                        }
                    } else if filtered_challenges.is_empty() {
                        div {
                            class: "flex flex-col items-center justify-center p-12 text-gray-500 text-center",
                            div { class: "text-4xl mb-2", "🎲" }
                            p { class: "m-0", "No challenges found" }
                            if !search_query.read().is_empty() || filter_type.read().is_some() {
                                p { class: "m-0 mt-2 text-sm", "Try adjusting your filters" }
                            } else {
                                button {
                                    onclick: move |_| show_create_form.set(true),
                                    class: "mt-4 px-4 py-2 bg-blue-500 text-white border-0 rounded-lg cursor-pointer",
                                    "Create Your First Challenge"
                                }
                            }
                        }
                    } else {
                        div { class: "flex flex-col gap-6",
                            // Render by type
                            for challenge_type in ChallengeType::all() {
                                if let Some(type_challenges) = challenges_by_type.get(&challenge_type) {
                                    if !type_challenges.is_empty() {
                                        ChallengeTypeSection {
                                            key: "{challenge_type:?}",
                                            challenge_type: challenge_type,
                                            challenges: type_challenges.clone(),
                                            skills_map: skills_map.clone(),
                                            on_toggle_favorite: handle_toggle_favorite.clone(),
                                            on_toggle_active: handle_toggle_active.clone(),
                                            on_edit: {
                                                let mut editing = editing_challenge;
                                                move |c: ChallengeData| editing.set(Some(c))
                                            },
                                            on_delete: handle_delete,
                                            on_trigger: props.on_trigger_challenge,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Create form modal (overlay on top)
            if *show_create_form.read() {
                ChallengeFormModal {
                    world_id: world_id.clone(),
                    challenge: None,
                    skills: props.skills.clone(),
                    on_save: {
                        let mut challenges = challenges;
                        move |challenge: ChallengeData| {
                            challenges.write().push(challenge);
                            show_create_form.set(false);
                        }
                    },
                    on_close: move |_| show_create_form.set(false),
                }
            }

            // Edit form modal
            if let Some(challenge) = editing_challenge.read().clone() {
                ChallengeFormModal {
                    world_id: world_id.clone(),
                    challenge: Some(challenge.clone()),
                    skills: props.skills.clone(),
                    on_save: {
                        let mut challenges = challenges;
                        let challenge_id = challenge.id.clone();
                        move |updated: ChallengeData| {
                            let mut write = challenges.write();
                            if let Some(c) = write.iter_mut().find(|c| c.id == challenge_id) {
                                *c = updated;
                            }
                            editing_challenge.set(None);
                        }
                    },
                    on_close: move |_| editing_challenge.set(None),
                }
            }

            // Delete confirmation modal
            if let Some(challenge_id) = show_delete_confirmation.read().clone() {
                if let Some(challenge) = challenges.read().iter().find(|c| c.id == challenge_id).cloned() {
                    ConfirmDeleteChallengeModal {
                        challenge_name: challenge.name.clone(),
                        is_deleting: *is_deleting.read(),
                        on_confirm: do_delete,
                        on_cancel: cancel_delete,
                    }
                }
            }
        }
    }
}
