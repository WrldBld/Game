//! Motivations Tab - Actantial model management for NPCs
//!
//! This tab provides the interface for managing NPC wants, goals, and
//! actantial relationships (helpers, opponents, senders, receivers).
//!
//! Features:
//! - View and edit character wants with visibility levels
//! - Set want targets (characters, items, goals)
//! - Manage actantial views (who helps/opposes which wants)
//! - Browse and create world goals
//! - View aggregated social stance (allies/enemies)

use dioxus::prelude::*;

use crate::application::dto::{
    ActantialActorData, ActantialRoleData, ActorTypeData, GoalData, NpcActantialContextData,
    SocialRelationData, WantTargetTypeData, WantVisibilityData,
};
// Note: WantData is re-exported from dto/player_events for PlayerEvent types
// For service request/response types, use messages::WantData
use wrldbldr_shared::messages::WantData;
use crate::application::services::{
    AddActantialViewRequest, CreateGoalRequest, CreateWantRequest, RemoveActantialViewRequest,
    SetWantTargetRequest, SuggestionContext, UpdateWantRequest,
};
use crate::infrastructure::spawn_task;
use crate::presentation::components::common::CharacterPicker;
use crate::presentation::components::creator::suggestion_button::{
    SuggestionButton, SuggestionType,
};
use crate::presentation::services::{use_actantial_service, use_character_service};
use crate::presentation::state::use_game_state;

/// Props for the motivations tab
#[derive(Props, Clone, PartialEq)]
pub struct MotivationsTabProps {
    /// The character ID being edited
    pub character_id: String,
    /// The world ID for goal access
    pub world_id: String,
    /// Character name for display
    pub character_name: String,
}

/// Main motivations tab component
#[component]
pub fn MotivationsTab(props: MotivationsTabProps) -> Element {
    // Get the actantial service
    let actantial_service = use_actantial_service();

    // Get game state for WebSocket-triggered refreshes
    let game_state = use_game_state();

    // State for actantial context
    let mut context: Signal<Option<NpcActantialContextData>> = use_signal(|| None);
    let mut goals: Signal<Vec<GoalData>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    // Modal state
    let mut show_want_modal = use_signal(|| false);
    let mut show_goal_modal = use_signal(|| false);
    let mut editing_want_id: Signal<Option<String>> = use_signal(|| None);

    // Refresh trigger - increment to trigger a refresh
    let mut refresh_counter = use_signal(|| 0u32);

    // Load actantial context and goals on mount and when refresh_counter changes
    {
        let char_id = props.character_id.clone();
        let world_id = props.world_id.clone();
        let service = actantial_service.clone();

        use_effect(move || {
            let char_id = char_id.clone();
            let world_id = world_id.clone();
            let service = service.clone();
            let _refresh = *refresh_counter.read(); // Subscribe to local refresh counter
            let _global_refresh = *game_state.actantial_refresh_counter.read(); // Subscribe to WebSocket refresh

            spawn_task(async move {
                is_loading.set(true);
                error_message.set(None);

                // Fetch actantial context
                match service.get_actantial_context(&char_id).await {
                    Ok(ctx) => {
                        context.set(Some(ctx));
                    }
                    Err(e) => {
                        tracing::error!("Failed to load actantial context: {:?}", e);
                        error_message.set(Some(format!("Failed to load motivations: {}", e)));
                    }
                }

                // Fetch world goals
                match service.list_goals(&world_id).await {
                    Ok(goal_list) => {
                        // Convert GoalResponse to GoalData
                        let goal_data: Vec<GoalData> = goal_list
                            .into_iter()
                            .map(|g| GoalData {
                                id: g.id,
                                name: g.name,
                                description: g.description,
                                usage_count: 0, // Will be populated from context if needed
                            })
                            .collect();
                        goals.set(goal_data);
                    }
                    Err(e) => {
                        tracing::error!("Failed to load goals: {:?}", e);
                        // Don't overwrite context error if there is one
                        if error_message.read().is_none() {
                            error_message.set(Some(format!("Failed to load goals: {}", e)));
                        }
                    }
                }

                is_loading.set(false);
            });
        });
    }

    // Delete want handler
    let delete_want = {
        let service = actantial_service.clone();
        move |want_id: String| {
            let service = service.clone();
            spawn_task(async move {
                match service.delete_want(&want_id).await {
                    Ok(_) => {
                        tracing::info!("Want deleted: {}", want_id);
                        let current = *refresh_counter.read();
                        refresh_counter.set(current + 1);
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete want: {:?}", e);
                        error_message.set(Some(format!("Failed to delete want: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "motivations-tab flex flex-col gap-4 p-4",

            // Error display
            if let Some(msg) = error_message.read().as_ref() {
                div {
                    class: "px-4 py-3 bg-red-500/10 border border-red-500/30 rounded text-red-500 text-sm",
                    "{msg}"
                }
            }

            // Loading state
            if *is_loading.read() {
                div {
                    class: "flex items-center justify-center p-8 text-gray-500",
                    "Loading motivations..."
                }
            } else {
                // Wants section
                WantsSection {
                    wants: context.read().as_ref().map(|c| c.wants.clone()).unwrap_or_default(),
                    character_id: props.character_id.clone(),
                    world_id: props.world_id.clone(),
                    available_goals: goals.read().clone(),
                    on_add_want: move |_| {
                        editing_want_id.set(None);
                        show_want_modal.set(true);
                    },
                    on_edit_want: move |want_id| {
                        editing_want_id.set(Some(want_id));
                        show_want_modal.set(true);
                    },
                    on_delete_want: {
                        let delete_fn = delete_want.clone();
                        move |want_id: String| {
                            delete_fn(want_id);
                        }
                    },
                    on_refresh: {
                        move |_| {
                            let current = *refresh_counter.read();
                            refresh_counter.set(current + 1);
                        }
                    },
                }

                // Goals library section
                GoalsSection {
                    goals: goals.read().clone(),
                    world_id: props.world_id.clone(),
                    on_add_goal: move |_| {
                        show_goal_modal.set(true);
                    },
                    on_refresh: {
                        move |_| {
                            let current = *refresh_counter.read();
                            refresh_counter.set(current + 1);
                        }
                    },
                }

                // Social stance section (aggregated allies/enemies)
                if let Some(ctx) = context.read().as_ref() {
                    SocialStanceSection {
                        allies: ctx.social_views.allies.clone(),
                        enemies: ctx.social_views.enemies.clone(),
                    }
                }
            }

            // Want editor modal
            if *show_want_modal.read() {
                WantEditorModal {
                    character_id: props.character_id.clone(),
                    character_name: props.character_name.clone(),
                    world_id: props.world_id.clone(),
                    want_id: editing_want_id.read().clone(),
                    existing_wants: context.read().as_ref().map(|c| c.wants.clone()).unwrap_or_default(),
                    available_goals: goals.read().clone(),
                    on_close: move |_| {
                        show_want_modal.set(false);
                        editing_want_id.set(None);
                    },
                    on_save: {
                        move |_| {
                            show_want_modal.set(false);
                            editing_want_id.set(None);
                            let current = *refresh_counter.read();
                            refresh_counter.set(current + 1);
                        }
                    },
                }
            }

            // Goal editor modal
            if *show_goal_modal.read() {
                GoalEditorModal {
                    world_id: props.world_id.clone(),
                    on_close: move |_| show_goal_modal.set(false),
                    on_save: {
                        move |_| {
                            show_goal_modal.set(false);
                            let current = *refresh_counter.read();
                            refresh_counter.set(current + 1);
                        }
                    },
                }
            }
        }
    }
}

// === Wants Section ===

#[derive(Props, Clone, PartialEq)]
struct WantsSectionProps {
    wants: Vec<WantData>,
    character_id: String,
    world_id: String,
    available_goals: Vec<GoalData>,
    on_add_want: EventHandler<()>,
    on_edit_want: EventHandler<String>,
    on_delete_want: EventHandler<String>,
    on_refresh: EventHandler<()>,
}

#[component]
fn WantsSection(props: WantsSectionProps) -> Element {
    rsx! {
        div {
            class: "wants-section",

            // Header
            div {
                class: "flex justify-between items-center mb-3",
                h3 { class: "text-white text-lg font-semibold m-0", "Wants" }
                button {
                    onclick: move |_| props.on_add_want.call(()),
                    class: "px-3 py-1 bg-accent-blue text-white rounded text-sm hover:bg-blue-600 transition-colors",
                    "+ Add Want"
                }
            }

            // Wants list
            if props.wants.is_empty() {
                div {
                    class: "text-gray-500 text-sm py-4 text-center border border-dashed border-gray-700 rounded",
                    "No wants defined. Add a want to define what this character desires."
                }
            } else {
                div {
                    class: "flex flex-col gap-2",
                    for want in props.wants.iter() {
                        WantCard {
                            key: "{want.id}",
                            want: want.clone(),
                            character_id: props.character_id.clone(),
                            world_id: props.world_id.clone(),
                            available_goals: props.available_goals.clone(),
                            on_edit: move |id| props.on_edit_want.call(id),
                            on_delete: move |id| props.on_delete_want.call(id),
                            on_refresh: move |_| props.on_refresh.call(()),
                        }
                    }
                }
            }
        }
    }
}

// === Want Card ===

#[derive(Props, Clone, PartialEq)]
struct WantCardProps {
    want: WantData,
    character_id: String,
    world_id: String,
    available_goals: Vec<GoalData>,
    on_edit: EventHandler<String>,
    on_delete: EventHandler<String>,
    on_refresh: EventHandler<()>,
}

#[component]
fn WantCard(props: WantCardProps) -> Element {
    let _actantial_service = use_actantial_service();
    let _character_service = use_character_service();
    let mut expanded = use_signal(|| false);
    let _show_add_view = use_signal(|| false);
    let want = &props.want;

    let visibility_badge = match want.visibility {
        WantVisibilityData::Known => ("Known", "bg-green-600"),
        WantVisibilityData::Suspected => ("Suspected", "bg-yellow-600"),
        WantVisibilityData::Hidden | WantVisibilityData::Unknown => ("Hidden", "bg-red-600"),
    };

    let intensity_percent = (want.intensity * 100.0) as i32;

    rsx! {
        div {
            class: "want-card bg-dark-bg border border-gray-700 rounded-lg p-3",

            // Header row
            div {
                class: "flex justify-between items-start gap-2",

                // Priority and description
                div {
                    class: "flex-1",
                    div {
                        class: "flex items-center gap-2 mb-1",
                        span {
                            class: "text-yellow-400 text-sm",
                            "★ Priority {want.priority}"
                        }
                        span {
                            class: "text-xs px-2 py-0.5 rounded {visibility_badge.1}",
                            "{visibility_badge.0}"
                        }
                    }
                    p {
                        class: "text-white m-0 text-sm",
                        "{want.description}"
                    }
                }

                // Actions
                div {
                    class: "flex gap-1",
                    button {
                        onclick: {
                            let id = want.id.clone();
                            move |_| props.on_edit.call(id.clone())
                        },
                        class: "px-2 py-1 bg-gray-700 text-gray-300 rounded text-xs hover:bg-gray-600",
                        "Edit"
                    }
                    button {
                        onclick: {
                            let id = want.id.clone();
                            move |_| props.on_delete.call(id.clone())
                        },
                        class: "px-2 py-1 bg-gray-700 text-red-400 rounded text-xs hover:bg-red-900",
                        "×"
                    }
                }
            }

            // Target and intensity row
            div {
                class: "flex items-center gap-4 mt-2 text-sm text-gray-400",
                if let Some(ref target) = want.target {
                    span { "Target: {target.name}" }
                }
                span { "Intensity: {intensity_percent}%" }
            }

            // Expand/collapse for actantial roles and secret behavior
            {
                let is_expanded = *expanded.read();
                rsx! {
                    button {
                        onclick: move |_| expanded.set(!is_expanded),
                        class: "mt-2 text-gray-500 text-xs hover:text-gray-300",
                        if is_expanded { "▼ Hide details" } else { "▶ Show actantial roles & secret behavior" }
                    }
                }
            }

            // Expanded content
            if *expanded.read() {
                div {
                    class: "mt-3 pt-3 border-t border-gray-700",

                    // Actantial roles with add/remove capabilities
                    ActantialViewsEditor {
                        want: want.clone(),
                        character_id: props.character_id.clone(),
                        world_id: props.world_id.clone(),
                        available_goals: props.available_goals.clone(),
                        on_refresh: move |_| props.on_refresh.call(()),
                    }

                    // Secret behavior (only shown for Hidden/Suspected)
                    if matches!(want.visibility, WantVisibilityData::Hidden | WantVisibilityData::Suspected) {
                        div {
                            h4 { class: "text-gray-400 text-xs uppercase tracking-wide mb-2", "Secret Behavior" }
                            div {
                                class: "grid grid-cols-2 gap-2 text-sm",
                                div {
                                    span { class: "text-gray-500", "Deflection: " }
                                    span { class: "text-gray-300", "{want.deflection_behavior.clone().unwrap_or_else(|| \"Not set\".to_string())}" }
                                }
                                div {
                                    span { class: "text-gray-500", "Tells: " }
                                    {
                                        let tells_text = if want.tells.is_empty() { "Not set".to_string() } else { want.tells.join(", ") };
                                        rsx! { span { class: "text-gray-300", "{tells_text}" } }
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

// === Actantial Views Editor ===

#[derive(Props, Clone, PartialEq)]
struct ActantialViewsEditorProps {
    want: WantData,
    character_id: String,
    world_id: String,
    available_goals: Vec<GoalData>,
    on_refresh: EventHandler<()>,
}

#[component]
fn ActantialViewsEditor(props: ActantialViewsEditorProps) -> Element {
    let actantial_service = use_actantial_service();
    let mut show_add_form = use_signal(|| false);
    let mut selected_role = use_signal(|| "helper".to_string());
    let mut selected_target = use_signal(String::new);
    let mut reason = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut action_error: Signal<Option<String>> = use_signal(|| None);

    let want = &props.want;
    let has_actors = !want.helpers.is_empty()
        || !want.opponents.is_empty()
        || want.sender.is_some()
        || want.receiver.is_some();

    // Add view handler
    let add_view = {
        let service = actantial_service.clone();
        let character_id = props.character_id.clone();
        let want_id = props.want.id.clone();
        let on_refresh = props.on_refresh;

        move |_| {
            let service = service.clone();
            let character_id = character_id.clone();
            let want_id = want_id.clone();
            let on_refresh = on_refresh;

            let role_str = selected_role.read().clone();
            let target_str = selected_target.read().clone();
            let reason_str = reason.read().clone();

            if target_str.is_empty() {
                return;
            }

            // Parse target: "npc:{id}" or "pc:{id}"
            let (actor_id, actor_type) = if let Some(id) = target_str.strip_prefix("npc:") {
                (id.to_string(), ActorTypeData::Npc)
            } else if let Some(id) = target_str.strip_prefix("pc:") {
                (id.to_string(), ActorTypeData::Pc)
            } else {
                // Default to NPC
                (target_str.clone(), ActorTypeData::Npc)
            };

            let role = match role_str.as_str() {
                "helper" => ActantialRoleData::Helper,
                "opponent" => ActantialRoleData::Opponent,
                "sender" => ActantialRoleData::Sender,
                "receiver" => ActantialRoleData::Receiver,
                _ => ActantialRoleData::Helper,
            };

            is_saving.set(true);

            spawn_task(async move {
                let req = AddActantialViewRequest {
                    want_id: want_id.clone(),
                    actor_id,
                    actor_type,
                    role,
                    reason: if reason_str.is_empty() {
                        None
                    } else {
                        Some(reason_str)
                    },
                };

                match service.add_actantial_view(&character_id, &req).await {
                    Ok(_) => {
                        is_saving.set(false);
                        show_add_form.set(false);
                        selected_target.set(String::new());
                        reason.set(String::new());
                        on_refresh.call(());
                    }
                    Err(e) => {
                        is_saving.set(false);
                        tracing::error!("Failed to add actantial view: {:?}", e);
                        action_error.set(Some(format!("Failed to add role: {}", e)));
                    }
                }
            });
        }
    };

    // Remove view handler - takes actor and role
    let remove_view = {
        let service = actantial_service.clone();
        let character_id = props.character_id.clone();
        let want_id = props.want.id.clone();
        let on_refresh = props.on_refresh;

        move |actor: ActantialActorData, role: ActantialRoleData| {
            let service = service.clone();
            let character_id = character_id.clone();
            let want_id = want_id.clone();
            let on_refresh = on_refresh;
            let actor_type = actor.actor_type;

            spawn_task(async move {
                let req = RemoveActantialViewRequest {
                    want_id: want_id.clone(),
                    actor_id: actor.id.clone(),
                    actor_type,
                    role,
                };

                match service.remove_actantial_view(&character_id, &req).await {
                    Ok(_) => {
                        on_refresh.call(());
                    }
                    Err(e) => {
                        tracing::error!("Failed to remove actantial view: {:?}", e);
                        action_error.set(Some(format!("Failed to remove role: {}", e)));
                    }
                }
            });
        }
    };

    // Check if target is empty (for button disabled state)
    let is_target_empty = selected_target.read().is_empty();

    rsx! {
        div {
            class: "mb-3",

            // Action error feedback
            if let Some(ref err) = *action_error.read() {
                div {
                    class: "mb-2 px-3 py-2 bg-red-500/20 border border-red-500/50 rounded text-red-400 text-xs cursor-pointer",
                    onclick: move |_| action_error.set(None),
                    "{err}"
                }
            }

            // Header with add button
            div {
                class: "flex justify-between items-center mb-2",
                h4 { class: "text-gray-400 text-xs uppercase tracking-wide", "Actantial Roles" }
                button {
                    onclick: move |_| {
                        let current = *show_add_form.read();
                        show_add_form.set(!current);
                    },
                    class: "text-xs px-2 py-1 bg-gray-700 text-gray-300 rounded hover:bg-gray-600",
                    if *show_add_form.read() { "Cancel" } else { "+ Add Role" }
                }
            }

            // Add form (shown when toggled)
            if *show_add_form.read() {
                div {
                    class: "mb-3 p-3 bg-gray-800 rounded border border-gray-600",

                    // Role selector
                    div {
                        class: "mb-2",
                        label { class: "text-gray-400 text-xs block mb-1", "Role" }
                        select {
                            value: "{selected_role}",
                            onchange: move |e| selected_role.set(e.value()),
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                            option { value: "helper", "Helper - Aids the subject" }
                            option { value: "opponent", "Opponent - Blocks the subject" }
                            option { value: "sender", "Sender - Gave this desire" }
                            option { value: "receiver", "Receiver - Benefits from success" }
                        }
                    }

                    // Character picker
                    div {
                        class: "mb-2",
                        label { class: "text-gray-400 text-xs block mb-1", "Character" }
                        CharacterPicker {
                            world_id: props.world_id.clone(),
                            value: selected_target.read().clone(),
                            on_change: move |val| selected_target.set(val),
                            placeholder: "Select a character...",
                            exclude_id: Some(props.character_id.clone()),
                        }
                    }

                    // Reason (optional)
                    div {
                        class: "mb-2",
                        label { class: "text-gray-400 text-xs block mb-1", "Reason (optional)" }
                        input {
                            r#type: "text",
                            value: "{reason}",
                            oninput: move |e| reason.set(e.value()),
                            placeholder: "Why does this character have this role?",
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                        }
                    }

                    div {
                        class: "flex items-center gap-2",
                        button {
                            onclick: add_view,
                            disabled: *is_saving.read() || is_target_empty,
                            class: "px-3 py-1 bg-accent-blue text-white rounded text-sm hover:bg-blue-600 disabled:opacity-50",
                            if *is_saving.read() { "Adding..." } else { "Add" }
                        }
                    }
                }
            }

            // Existing views
            if !has_actors {
                p { class: "text-gray-500 text-sm italic", "No actantial views defined" }
            } else {
                div {
                    class: "flex flex-wrap gap-2",
                    for actor in want.helpers.iter() {
                        ActantialActorBadgeRemovable {
                            actor: actor.clone(),
                            role: "Helper",
                            on_remove: {
                                let actor = actor.clone();
                                let remove_fn = remove_view.clone();
                                move |_| remove_fn(actor.clone(), ActantialRoleData::Helper)
                            },
                        }
                    }
                    for actor in want.opponents.iter() {
                        ActantialActorBadgeRemovable {
                            actor: actor.clone(),
                            role: "Opponent",
                            on_remove: {
                                let actor = actor.clone();
                                let remove_fn = remove_view.clone();
                                move |_| remove_fn(actor.clone(), ActantialRoleData::Opponent)
                            },
                        }
                    }
                    if let Some(ref actor) = want.sender {
                        ActantialActorBadgeRemovable {
                            actor: actor.clone(),
                            role: "Sender",
                            on_remove: {
                                let actor = actor.clone();
                                let remove_fn = remove_view.clone();
                                move |_| remove_fn(actor.clone(), ActantialRoleData::Sender)
                            },
                        }
                    }
                    if let Some(ref actor) = want.receiver {
                        ActantialActorBadgeRemovable {
                            actor: actor.clone(),
                            role: "Receiver",
                            on_remove: {
                                let actor = actor.clone();
                                let remove_fn = remove_view.clone();
                                move |_| remove_fn(actor.clone(), ActantialRoleData::Receiver)
                            },
                        }
                    }
                }
            }
        }
    }
}

// === Actantial Actor Badge (Removable) ===

#[derive(Props, Clone, PartialEq)]
struct ActantialActorBadgeRemovableProps {
    actor: ActantialActorData,
    role: &'static str,
    on_remove: EventHandler<()>,
}

#[component]
fn ActantialActorBadgeRemovable(props: ActantialActorBadgeRemovableProps) -> Element {
    let role_color = match props.role {
        "Helper" => "bg-green-800",
        "Opponent" => "bg-red-800",
        "Sender" => "bg-blue-800",
        "Receiver" => "bg-purple-800",
        _ => "bg-gray-800",
    };

    rsx! {
        div {
            class: "flex items-center gap-1 px-2 py-1 rounded text-xs {role_color} group",
            span { class: "text-gray-300", "{props.actor.name}" }
            span { class: "text-gray-500", "as" }
            span { class: "text-white font-medium", "{props.role}" }
            button {
                onclick: move |_| props.on_remove.call(()),
                class: "ml-1 text-gray-400 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity",
                "×"
            }
        }
    }
}

// === Actantial Actor Badge ===

#[derive(Props, Clone, PartialEq)]
struct ActantialActorBadgeProps {
    actor: ActantialActorData,
    role: &'static str,
}

#[component]
fn ActantialActorBadge(props: ActantialActorBadgeProps) -> Element {
    let role_color = match props.role {
        "Helper" => "bg-green-800",
        "Opponent" => "bg-red-800",
        "Sender" => "bg-blue-800",
        "Receiver" => "bg-purple-800",
        _ => "bg-gray-800",
    };

    rsx! {
        div {
            class: "flex items-center gap-1 px-2 py-1 rounded text-xs {role_color}",
            span { class: "text-gray-300", "{props.actor.name}" }
            span { class: "text-gray-500", "as" }
            span { class: "text-white font-medium", "{props.role}" }
        }
    }
}

// === Goals Section ===

#[derive(Props, Clone, PartialEq)]
struct GoalsSectionProps {
    goals: Vec<GoalData>,
    world_id: String,
    on_add_goal: EventHandler<()>,
    on_refresh: EventHandler<()>,
}

#[component]
fn GoalsSection(props: GoalsSectionProps) -> Element {
    let actantial_service = use_actantial_service();
    let mut adding_common = use_signal(|| false);

    // Add common goals handler
    let add_common_goals = {
        let service = actantial_service.clone();
        let world_id = props.world_id.clone();
        let on_refresh = props.on_refresh;
        move |_| {
            let service = service.clone();
            let world_id = world_id.clone();
            let on_refresh = on_refresh;
            adding_common.set(true);

            spawn_task(async move {
                // Common goals from domain layer
                let common_goals = vec![
                    ("Power", "Political or personal dominance over others"),
                    ("Wealth", "Accumulation of material resources and riches"),
                    (
                        "Knowledge",
                        "Understanding of secrets, lore, or hidden truths",
                    ),
                    ("Revenge", "Retribution against those who caused harm"),
                    ("Justice", "Righting wrongs and upholding fairness"),
                    ("Love", "Deep connection and affection with another"),
                    ("Freedom", "Liberation from constraints or oppression"),
                    ("Honor", "Maintaining reputation and personal integrity"),
                    ("Survival", "Self-preservation in the face of threats"),
                    ("Peace", "End to conflict and establishment of harmony"),
                    (
                        "Recognition",
                        "Fame, acknowledgment, or validation from others",
                    ),
                    ("Redemption", "Atonement for past sins or mistakes"),
                ];

                let mut added = 0;
                for (name, description) in common_goals {
                    let req = CreateGoalRequest {
                        name: name.to_string(),
                        description: Some(description.to_string()),
                    };
                    match service.create_goal(&world_id, &req).await {
                        Ok(_) => added += 1,
                        Err(e) => {
                            // Goal might already exist, that's ok
                            tracing::debug!("Could not add goal {}: {:?}", name, e);
                        }
                    }
                }

                tracing::info!("Added {} common goals", added);
                adding_common.set(false);
                on_refresh.call(()); // Trigger refresh in parent
            });
        }
    };

    rsx! {
        div {
            class: "goals-section mt-6",

            // Header
            div {
                class: "flex justify-between items-center mb-3",
                h3 { class: "text-white text-lg font-semibold m-0", "World Goals" }
                div {
                    class: "flex gap-2",
                    button {
                        onclick: add_common_goals,
                        disabled: *adding_common.read(),
                        class: "px-3 py-1 bg-gray-700 text-gray-300 rounded text-sm hover:bg-gray-600 transition-colors disabled:opacity-50",
                        if *adding_common.read() { "Adding..." } else { "+ Common Goals" }
                    }
                    button {
                        onclick: move |_| props.on_add_goal.call(()),
                        class: "px-3 py-1 bg-accent-blue text-white rounded text-sm hover:bg-blue-600 transition-colors",
                        "+ New Goal"
                    }
                }
            }

            // Goals list
            if props.goals.is_empty() {
                div {
                    class: "text-gray-500 text-sm py-4 text-center border border-dashed border-gray-700 rounded",
                    "No world goals defined. Goals can be used as want targets."
                }
            } else {
                div {
                    class: "flex flex-wrap gap-2",
                    for goal in props.goals.iter() {
                        div {
                            key: "{goal.id}",
                            class: "px-3 py-2 bg-dark-bg border border-gray-700 rounded",
                            span { class: "text-white", "{goal.name}" }
                            if goal.usage_count > 0 {
                                span { class: "text-gray-500 text-xs ml-1", "({goal.usage_count})" }
                            }
                            if let Some(ref desc) = goal.description {
                                span { class: "text-gray-500 text-sm ml-2", "- {desc}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// === Social Stance Section ===

#[derive(Props, Clone, PartialEq)]
struct SocialStanceSectionProps {
    allies: Vec<SocialRelationData>,
    enemies: Vec<SocialRelationData>,
}

#[component]
fn SocialStanceSection(props: SocialStanceSectionProps) -> Element {
    rsx! {
        div {
            class: "social-stance-section mt-6",

            h3 { class: "text-white text-lg font-semibold mb-3", "Social Stance (Aggregated)" }

            div {
                class: "grid grid-cols-2 gap-4",

                // Allies
                div {
                    class: "bg-green-900/20 border border-green-800 rounded p-3",
                    h4 { class: "text-green-400 text-sm font-medium mb-2", "Allies" }
                    if props.allies.is_empty() {
                        p { class: "text-gray-500 text-sm", "None" }
                    } else {
                        ul {
                            class: "list-none m-0 p-0",
                            for ally in props.allies.iter() {
                                li {
                                    class: "text-gray-300 text-sm",
                                    "{ally.name}"
                                }
                            }
                        }
                    }
                }

                // Enemies
                div {
                    class: "bg-red-900/20 border border-red-800 rounded p-3",
                    h4 { class: "text-red-400 text-sm font-medium mb-2", "Enemies" }
                    if props.enemies.is_empty() {
                        p { class: "text-gray-500 text-sm", "None" }
                    } else {
                        ul {
                            class: "list-none m-0 p-0",
                            for enemy in props.enemies.iter() {
                                li {
                                    class: "text-gray-300 text-sm",
                                    "{enemy.name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// === Want Editor Modal ===

#[derive(Props, Clone, PartialEq)]
struct WantEditorModalProps {
    character_id: String,
    character_name: String,
    world_id: String,
    want_id: Option<String>,
    existing_wants: Vec<WantData>,
    available_goals: Vec<GoalData>,
    on_close: EventHandler<()>,
    on_save: EventHandler<()>,
}

#[component]
fn WantEditorModal(props: WantEditorModalProps) -> Element {
    let actantial_service = use_actantial_service();
    let is_new = props.want_id.is_none();

    // Find existing want if editing
    let existing_want = props
        .want_id
        .as_ref()
        .and_then(|id| props.existing_wants.iter().find(|w| &w.id == id).cloned());

    // Form state - initialized from existing want if editing
    let mut description = use_signal(|| {
        existing_want
            .as_ref()
            .map(|w| w.description.clone())
            .unwrap_or_default()
    });
    let mut intensity = use_signal(|| existing_want.as_ref().map(|w| w.intensity).unwrap_or(0.5));
    let mut priority = use_signal(|| existing_want.as_ref().map(|w| w.priority).unwrap_or(1));
    let mut visibility = use_signal(|| {
        existing_want
            .as_ref()
            .map(|w| w.visibility)
            .unwrap_or(WantVisibilityData::Known)
    });
    let mut deflection = use_signal(|| {
        existing_want
            .as_ref()
            .and_then(|w| w.deflection_behavior.clone())
            .unwrap_or_default()
    });
    let mut tells = use_signal(|| {
        existing_want
            .as_ref()
            .map(|w| w.tells.join(", "))
            .unwrap_or_default()
    });
    let mut is_saving = use_signal(|| false);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);

    // Target state - "none", "goal:{id}", "character:{id}", "item:{id}"
    let mut target_selection = use_signal(|| {
        existing_want
            .as_ref()
            .and_then(|w| w.target.as_ref())
            .map(|t| {
                let type_str = match t.target_type {
                    WantTargetTypeData::Goal => "goal",
                    WantTargetTypeData::Character | WantTargetTypeData::Unknown => "character",
                    WantTargetTypeData::Item => "item",
                };
                format!("{}:{}", type_str, t.id)
            })
            .unwrap_or_else(|| "none".to_string())
    });

    // Save handler
    let save_want = {
        let service = actantial_service.clone();
        let character_id = props.character_id.clone();
        let want_id = props.want_id.clone();
        let on_save = props.on_save;

        move |_| {
            let service = service.clone();
            let character_id = character_id.clone();
            let want_id = want_id.clone();
            let on_save = on_save;

            let desc = description.read().clone();
            let int = *intensity.read();
            let pri = *priority.read();
            let vis = *visibility.read();
            let defl = deflection.read().clone();
            let tls = tells.read().clone();
            let target_sel = target_selection.read().clone();

            // Parse target selection
            let (target_id, target_type) = if target_sel == "none" || target_sel.is_empty() {
                (None, None)
            } else {
                let parts: Vec<&str> = target_sel.splitn(2, ':').collect();
                if parts.len() == 2 {
                    (Some(parts[1].to_string()), Some(parts[0].to_string()))
                } else {
                    (None, None)
                }
            };

            is_saving.set(true);
            error_msg.set(None);

            spawn_task(async move {
                let result = if let Some(want_id) = want_id {
                    // Update existing want
                    let req = UpdateWantRequest {
                        description: Some(desc),
                        intensity: Some(int),
                        priority: Some(pri),
                        visibility: Some(vis),
                        deflection_behavior: if defl.is_empty() { None } else { Some(defl) },
                        tells: if tls.is_empty() { None } else { Some(tls) },
                    };
                    let update_result = service.update_want(&want_id, &req).await;

                    // Also update target if changed
                    if update_result.is_ok() {
                        if let (Some(tid), Some(ttype)) = (&target_id, &target_type) {
                            // Convert string to WantTargetTypeData
                            let target_type_data = match ttype.as_str() {
                                "Character" => WantTargetTypeData::Character,
                                "Item" => WantTargetTypeData::Item,
                                "Goal" => WantTargetTypeData::Goal,
                                _ => WantTargetTypeData::Character, // Default
                            };
                            let _ = service
                                .set_want_target(
                                    &want_id,
                                    &SetWantTargetRequest {
                                        target_id: tid.clone(),
                                        target_type: target_type_data,
                                    },
                                )
                                .await;
                        } else {
                            // Remove target if set to none
                            let _ = service.remove_want_target(&want_id).await;
                        }
                    }
                    update_result.map(|_| ())
                } else {
                    // Create new want
                    let req = CreateWantRequest {
                        description: desc,
                        intensity: int,
                        priority: pri,
                        visibility: vis,
                        target_id,
                        target_type,
                        deflection_behavior: if defl.is_empty() { None } else { Some(defl) },
                        tells: if tls.is_empty() { None } else { Some(tls) },
                    };
                    service.create_want(&character_id, &req).await.map(|_| ())
                };

                is_saving.set(false);

                match result {
                    Ok(_) => {
                        on_save.call(());
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to save: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-lg p-6 w-full max-w-lg max-h-[80vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center mb-4",
                    h2 {
                        class: "text-white text-xl m-0",
                        if is_new { "Add Want" } else { "Edit Want" }
                    }
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "text-gray-400 hover:text-white text-2xl",
                        "×"
                    }
                }

                // Error display
                if let Some(msg) = error_msg.read().as_ref() {
                    div {
                        class: "mb-4 px-3 py-2 bg-red-500/10 border border-red-500/30 rounded text-red-500 text-sm",
                        "{msg}"
                    }
                }

                // Form fields
                div {
                    class: "flex flex-col gap-4",

                    // Description with suggestion button
                    div {
                        div {
                            class: "flex justify-between items-center mb-1",
                            label { class: "text-gray-300 text-sm", "Description *" }
                            SuggestionButton {
                                suggestion_type: SuggestionType::WantDescription,
                                world_id: props.world_id.clone(),
                                context: SuggestionContext {
                                    entity_name: Some(props.character_name.clone()),
                                    world_setting: None,
                                    hints: None,
                                    additional_context: None,
                                    entity_type: Some("character".to_string()),
                                    world_id: None,
                                },
                                on_select: move |suggestion| {
                                    description.set(suggestion);
                                },
                            }
                        }
                        textarea {
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                            placeholder: "What does this character want?",
                            class: "w-full min-h-[80px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y",
                        }
                    }

                    // Intensity slider
                    div {
                        label { class: "text-gray-300 text-sm block mb-1", "Intensity: {(*intensity.read() * 100.0) as i32}%" }
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{(*intensity.read() * 100.0) as i32}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<i32>() {
                                    intensity.set(v as f32 / 100.0);
                                }
                            },
                            class: "w-full",
                        }
                    }

                    // Priority
                    div {
                        label { class: "text-gray-300 text-sm block mb-1", "Priority" }
                        input {
                            r#type: "number",
                            min: "1",
                            max: "10",
                            value: "{priority}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    priority.set(v.clamp(1, 10));
                                }
                            },
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                        }
                    }

                    // Visibility
                    div {
                        label { class: "text-gray-300 text-sm block mb-1", "Visibility" }
                        select {
                            value: match *visibility.read() {
                                WantVisibilityData::Known => "known",
                                WantVisibilityData::Suspected => "suspected",
                                WantVisibilityData::Hidden | WantVisibilityData::Unknown => "hidden",
                            },
                            onchange: move |e| {
                                visibility.set(match e.value().as_str() {
                                    "suspected" => WantVisibilityData::Suspected,
                                    "hidden" => WantVisibilityData::Hidden,
                                    _ => WantVisibilityData::Known,
                                });
                            },
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            option { value: "known", "Known - Players know about this want" }
                            option { value: "suspected", "Suspected - Players have hints" }
                            option { value: "hidden", "Hidden - Players don't know" }
                        }
                    }

                    // Target selection (what this want is directed at)
                    div {
                        label { class: "text-gray-300 text-sm block mb-1", "Target (Optional)" }
                        select {
                            value: "{target_selection}",
                            onchange: move |e| {
                                target_selection.set(e.value());
                            },
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            option { value: "none", "No specific target" }
                            // Goals as targets
                            if !props.available_goals.is_empty() {
                                optgroup {
                                    label: "Goals",
                                    for goal in props.available_goals.iter() {
                                        option {
                                            key: "{goal.id}",
                                            value: "goal:{goal.id}",
                                            "{goal.name}"
                                        }
                                    }
                                }
                            }
                        }
                        p {
                            class: "text-gray-500 text-xs mt-1",
                            "What this character wants to achieve or obtain"
                        }
                    }

                    // Secret behavior (only for hidden/suspected)
                    if matches!(*visibility.read(), WantVisibilityData::Hidden | WantVisibilityData::Suspected) {
                        div {
                            class: "border-t border-gray-700 pt-4 mt-2",
                            h4 { class: "text-gray-400 text-sm mb-3", "Secret Behavior" }

                            // Deflection behavior with suggestion
                            div {
                                class: "mb-3",
                                div {
                                    class: "flex justify-between items-center mb-1",
                                    label { class: "text-gray-300 text-sm", "Deflection Behavior" }
                                    SuggestionButton {
                                        suggestion_type: SuggestionType::DeflectionBehavior,
                                        world_id: props.world_id.clone(),
                                        context: SuggestionContext {
                                            entity_name: Some(props.character_name.clone()),
                                            world_setting: None,
                                            hints: Some(description.read().clone()), // The want being hidden
                                            additional_context: None,
                                            entity_type: Some("character".to_string()),
                                            world_id: None,
                                        },
                                        on_select: move |suggestion| {
                                            deflection.set(suggestion);
                                        },
                                    }
                                }
                                input {
                                    r#type: "text",
                                    value: "{deflection}",
                                    oninput: move |e| deflection.set(e.value()),
                                    placeholder: "How does the NPC hide this want?",
                                    class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                }
                            }

                            // Behavioral tells with suggestion
                            div {
                                div {
                                    class: "flex justify-between items-center mb-1",
                                    label { class: "text-gray-300 text-sm", "Behavioral Tells (comma-separated)" }
                                    SuggestionButton {
                                        suggestion_type: SuggestionType::BehavioralTells,
                                        world_id: props.world_id.clone(),
                                        context: SuggestionContext {
                                            entity_name: Some(props.character_name.clone()),
                                            world_setting: None,
                                            hints: Some(description.read().clone()), // The want being hidden
                                            additional_context: None,
                                            entity_type: Some("character".to_string()),
                                            world_id: None,
                                        },
                                        on_select: move |suggestion| {
                                            // Append to existing tells
                                            let current = tells.read().clone();
                                            if current.is_empty() {
                                                tells.set(suggestion);
                                            } else {
                                                tells.set(format!("{}, {}", current, suggestion));
                                            }
                                        },
                                    }
                                }
                                input {
                                    r#type: "text",
                                    value: "{tells}",
                                    oninput: move |e| tells.set(e.value()),
                                    placeholder: "Subtle signs that reveal the hidden want",
                                    class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                }
                            }
                        }
                    }
                }

                // Actions
                div {
                    class: "flex justify-end gap-2 mt-6",
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-4 py-2 bg-gray-700 text-white rounded hover:bg-gray-600",
                        "Cancel"
                    }
                    button {
                        onclick: save_want,
                        disabled: *is_saving.read() || description.read().is_empty(),
                        class: "px-4 py-2 bg-accent-blue text-white rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
                        if *is_saving.read() { "Saving..." } else { "Save" }
                    }
                }
            }
        }
    }
}

// === Goal Editor Modal ===

#[derive(Props, Clone, PartialEq)]
struct GoalEditorModalProps {
    world_id: String,
    on_close: EventHandler<()>,
    on_save: EventHandler<()>,
}

#[component]
fn GoalEditorModal(props: GoalEditorModalProps) -> Element {
    let actantial_service = use_actantial_service();
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);

    // Save handler
    let save_goal = {
        let service = actantial_service.clone();
        let world_id = props.world_id.clone();
        let on_save = props.on_save;

        move |_| {
            let service = service.clone();
            let world_id = world_id.clone();
            let on_save = on_save;

            let goal_name = name.read().clone();
            let goal_desc = description.read().clone();

            is_saving.set(true);
            error_msg.set(None);

            spawn_task(async move {
                let req = CreateGoalRequest {
                    name: goal_name,
                    description: if goal_desc.is_empty() {
                        None
                    } else {
                        Some(goal_desc)
                    },
                };

                match service.create_goal(&world_id, &req).await {
                    Ok(_) => {
                        is_saving.set(false);
                        on_save.call(());
                    }
                    Err(e) => {
                        is_saving.set(false);
                        error_msg.set(Some(format!("Failed to create goal: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-lg p-6 w-full max-w-md",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center mb-4",
                    h2 { class: "text-white text-xl m-0", "New Goal" }
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "text-gray-400 hover:text-white text-2xl",
                        "×"
                    }
                }

                // Error display
                if let Some(msg) = error_msg.read().as_ref() {
                    div {
                        class: "mb-4 px-3 py-2 bg-red-500/10 border border-red-500/30 rounded text-red-500 text-sm",
                        "{msg}"
                    }
                }

                // Form fields
                div {
                    class: "flex flex-col gap-4",

                    div {
                        label { class: "text-gray-300 text-sm block mb-1", "Name *" }
                        input {
                            r#type: "text",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                            placeholder: "e.g., Redemption, Power, Peace",
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                        }
                    }

                    div {
                        label { class: "text-gray-300 text-sm block mb-1", "Description" }
                        textarea {
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                            placeholder: "Optional description of what this goal represents",
                            class: "w-full min-h-[60px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y",
                        }
                    }
                }

                // Actions
                div {
                    class: "flex justify-end gap-2 mt-6",
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-4 py-2 bg-gray-700 text-white rounded hover:bg-gray-600",
                        "Cancel"
                    }
                    button {
                        onclick: save_goal,
                        disabled: *is_saving.read() || name.read().is_empty(),
                        class: "px-4 py-2 bg-accent-blue text-white rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
                        if *is_saving.read() { "Saving..." } else { "Create Goal" }
                    }
                }
            }
        }
    }
}
