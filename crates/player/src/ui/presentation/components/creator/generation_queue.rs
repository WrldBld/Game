//! Generation Queue Panel - Shows active and completed generation batches

use dioxus::prelude::*;

use crate::infrastructure::spawn_task;
use crate::presentation::services::{
    mark_batch_read_and_sync, mark_suggestion_read_and_sync, use_asset_service,
    use_generation_service, use_suggestion_service, visible_batches, visible_suggestions,
};
use crate::presentation::state::{
    use_game_state, use_generation_state, BatchStatus, GenerationBatch, SuggestionStatus,
    SuggestionTask,
};
use crate::use_platform;

/// Filter type for the generation queue
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum QueueFilter {
    #[default]
    All,
    Images,
    Suggestions,
}

/// Sort order for the generation queue
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SortOrder {
    #[default]
    NewestFirst,
    OldestFirst,
    Status,
    Type,
}

/// Props for GenerationQueuePanel
#[derive(Props, Clone, PartialEq)]
pub struct GenerationQueuePanelProps {
    /// Optional callback when user wants to navigate to an entity
    /// Called with (entity_type, entity_id)
    #[props(default)]
    pub on_navigate_to_entity: Option<EventHandler<(String, String)>>,
}

/// Panel showing generation queue status (images and suggestions)
#[component]
pub fn GenerationQueuePanel(props: GenerationQueuePanelProps) -> Element {
    let generation_state = use_generation_state();
    let game_state = use_game_state();
    let _generation_service = use_generation_service();
    let _platform = use_platform();
    let mut selected_suggestion: Signal<Option<SuggestionTask>> = use_signal(|| None);
    let mut show_read: Signal<bool> = use_signal(|| false);
    let mut active_filter: Signal<QueueFilter> = use_signal(|| QueueFilter::All);
    let mut sort_order: Signal<SortOrder> = use_signal(|| SortOrder::NewestFirst);

    let show_read_val = *show_read.read();
    let filter_val = *active_filter.read();
    let all_batches = visible_batches(&generation_state, show_read_val);
    let all_suggestions = visible_suggestions(&generation_state, show_read_val);

    // Compute counts before filtering
    let batch_count = all_batches.len();
    let suggestion_count = all_suggestions.len();
    let total_count = batch_count + suggestion_count;

    // Filter by active filter
    let mut visible_batches = match filter_val {
        QueueFilter::All | QueueFilter::Images => all_batches.clone(),
        QueueFilter::Suggestions => Vec::new(),
    };
    let mut visible_suggestions = match filter_val {
        QueueFilter::All | QueueFilter::Suggestions => all_suggestions.clone(),
        QueueFilter::Images => Vec::new(),
    };

    // Sort items based on sort_order
    let sort_val = *sort_order.read();
    match sort_val {
        SortOrder::NewestFirst => {
            // Already in insertion order (newest last), reverse to show newest first
            visible_batches.reverse();
            visible_suggestions.reverse();
        }
        SortOrder::OldestFirst => {
            // Already in insertion order (oldest first), keep as is
        }
        SortOrder::Status => {
            // Sort by status priority: Queued/Processing > Ready > Failed
            visible_batches.sort_by(|a, b| {
                let a_prio = status_priority(&a.status);
                let b_prio = status_priority(&b.status);
                b_prio.cmp(&a_prio) // Higher priority first
            });
            visible_suggestions.sort_by(|a, b| {
                let a_prio = suggestion_status_priority(&a.status);
                let b_prio = suggestion_status_priority(&b.status);
                b_prio.cmp(&a_prio) // Higher priority first
            });
        }
        SortOrder::Type => {
            // Sort by entity type, then entity_id
            visible_batches.sort_by(|a, b| {
                a.entity_type
                    .cmp(&b.entity_type)
                    .then_with(|| a.entity_id.cmp(&b.entity_id))
            });
            visible_suggestions.sort_by(|a, b| {
                a.field_type
                    .cmp(&b.field_type)
                    .then_with(|| a.entity_id.cmp(&b.entity_id))
            });
        }
    }

    let total_items = visible_batches.len() + visible_suggestions.len();

    // Counts for badge
    let active_batch_count = generation_state.active_count();
    let active_suggestion_count = generation_state.active_suggestion_count();
    let total_active = active_batch_count + active_suggestion_count;

    // Derive world_id from game state if available (for scoping read markers)
    let world_id = game_state.world.read().as_ref().map(|w| w.world.id.clone());

    rsx! {
        div {
            class: "generation-queue bg-dark-surface rounded-lg p-3",

            // Header with filter tabs and toggle for read items
            div {
                class: "mb-2",

                // Title and badge
                div {
                    class: "flex items-center justify-between mb-2",
                    h3 {
                        class: "text-gray-400 text-xs uppercase m-0 flex items-center gap-2",
                        "Generation Queue"
                        if total_active > 0 {
                            span {
                                class: "bg-amber-500 text-white rounded-xl px-1.5 py-0.5 text-[0.625rem] font-bold",
                                "{total_active}"
                            }
                        }
                    }
                    // Clear All Completed button
                    {
                        let completed_count = all_batches.iter()
                            .filter(|b| matches!(b.status, BatchStatus::Ready { .. }))
                            .count();
                        if completed_count > 0 {
                            rsx! {
                                button {
                                    onclick: {
                                        let mut state = use_generation_state();
                                        move |_| {
                                            let batches = state.get_batches();
                                            let to_remove: Vec<_> = batches.iter()
                                                .filter(|b| matches!(b.status, BatchStatus::Ready { .. }))
                                                .map(|b| b.batch_id.clone())
                                                .collect();
                                            for batch_id in to_remove {
                                                state.remove_batch(&batch_id);
                                            }
                                        }
                                    },
                                    class: "px-2 py-1 bg-gray-500 text-white border-none rounded cursor-pointer text-xs",
                                    "Clear All Completed"
                                }
                            }
                        } else {
                            rsx! { }
                        }
                    }
                    label {
                        class: "inline-flex items-center gap-1 text-gray-400 text-xs",
                        input {
                            r#type: "checkbox",
                            checked: *show_read.read(),
                            onchange: move |_| {
                                let current = *show_read.read();
                                show_read.set(!current);
                            },
                        }
                        span { "Show read" }
                    }
                }

                // Filter tabs and sort dropdown
                div {
                    class: "flex justify-between items-center gap-2 mb-2",
                    // Filter tabs
                    div {
                        class: "flex gap-1 border-b border-gray-700 flex-1",
                    FilterTab {
                        label: "All",
                        count: total_count,
                        is_active: filter_val == QueueFilter::All,
                        onclick: move |_| active_filter.set(QueueFilter::All),
                    }
                    FilterTab {
                        label: "Images",
                        count: batch_count,
                        is_active: filter_val == QueueFilter::Images,
                        onclick: move |_| active_filter.set(QueueFilter::Images),
                    }
                    FilterTab {
                        label: "Suggestions",
                        count: suggestion_count,
                        is_active: filter_val == QueueFilter::Suggestions,
                        onclick: move |_| active_filter.set(QueueFilter::Suggestions),
                        }
                    }
                    // Sort dropdown
                    select {
                        value: match *sort_order.read() {
                            SortOrder::NewestFirst => "newest",
                            SortOrder::OldestFirst => "oldest",
                            SortOrder::Status => "status",
                            SortOrder::Type => "type",
                        },
                        onchange: move |evt| {
                            let val = evt.value();
                            sort_order.set(match val.as_str() {
                                "oldest" => SortOrder::OldestFirst,
                                "status" => SortOrder::Status,
                                "type" => SortOrder::Type,
                                _ => SortOrder::NewestFirst,
                            });
                        },
                        class: "px-2 py-1 bg-dark-bg text-gray-400 border border-gray-700 rounded text-xs cursor-pointer",
                        option { value: "newest", "Newest First" }
                        option { value: "oldest", "Oldest First" }
                        option { value: "status", "By Status" }
                        option { value: "type", "By Type" }
                    }
                }
            }

            if total_items == 0 {
                div {
                    class: "text-gray-500 text-sm text-center p-4",
                    "No generations in progress"
                }
            } else {
                div {
                    class: "flex flex-col gap-2",

                    // Show image batches
                    for batch in visible_batches.iter() {
                        QueueItemRow {
                            batch: batch.clone(),
                            show_read: show_read_val,
                            world_id: world_id.clone(),
                            on_navigate_to_entity: props.on_navigate_to_entity,
                        }
                    }

                    // Show suggestion tasks
                    for suggestion in visible_suggestions.iter() {
                        SuggestionQueueRow {
                            suggestion: suggestion.clone(),
                            selected_suggestion,
                            show_read: show_read_val,
                            world_id: world_id.clone(),
                            on_navigate_to_entity: props.on_navigate_to_entity,
                        }
                    }
                }
            }

            // Modal for viewing suggestion details
            if let Some(active) = selected_suggestion.read().as_ref() {
                SuggestionViewModal {
                    suggestion: active.clone(),
                    on_close: {
                        move |_| {
                            selected_suggestion.set(None);
                        }
                    },
                    on_navigate: props.on_navigate_to_entity,
                }
            }
        }
    }
}

/// Helper function to get status priority for sorting (higher = more important)
fn status_priority(status: &BatchStatus) -> u8 {
    match status {
        BatchStatus::Queued { .. } | BatchStatus::Generating { .. } => 3, // Active items first
        BatchStatus::Ready { .. } => 2,
        BatchStatus::Failed { .. } => 1,
    }
}

/// Helper function to get suggestion status priority for sorting
fn suggestion_status_priority(status: &SuggestionStatus) -> u8 {
    match status {
        SuggestionStatus::Queued | SuggestionStatus::Processing => 3, // Active items first
        SuggestionStatus::Ready { .. } => 2,
        SuggestionStatus::Failed { .. } => 1,
    }
}

/// Filter tab component
#[component]
fn FilterTab(
    label: &'static str,
    count: usize,
    is_active: bool,
    onclick: EventHandler<()>,
) -> Element {
    let border_class = if is_active {
        "border-b-2 border-purple-500"
    } else {
        "border-b-2 border-transparent"
    };
    let text_class = if is_active {
        "text-white"
    } else {
        "text-gray-400"
    };

    rsx! {
        button {
            onclick: move |_| onclick.call(()),
            class: "flex-1 px-2 py-1.5 bg-transparent border-none {border_class} {text_class} text-xs cursor-pointer transition-all",
            "{label}"
            if count > 0 {
                span {
                    class: "ml-1 text-gray-500",
                    "({count})"
                }
            }
        }
    }
}

/// Individual queue item row for image batches
#[component]
fn QueueItemRow(
    batch: GenerationBatch,
    #[props(default = false)] show_read: bool,
    world_id: Option<String>,
    #[props(default)] on_navigate_to_entity: Option<EventHandler<(String, String)>>,
) -> Element {
    let generation_service = use_generation_service();
    let platform = use_platform();
    let mut expanded_error: Signal<bool> = use_signal(|| false);
    let mut expanded_details: Signal<bool> = use_signal(|| false);
    let mut action_error: Signal<Option<String>> = use_signal(|| None);
    let batch_id = batch.batch_id.clone();
    let (status_icon, status_color, _status_text) = match &batch.status {
        BatchStatus::Queued { position } => ("🖼️", "#9ca3af", format!("#{} in queue", position)),
        BatchStatus::Generating { progress } => ("⚙️", "#f59e0b", format!("{}%", progress)),
        BatchStatus::Ready { asset_count } => ("✅", "#22c55e", format!("{} ready", asset_count)),
        BatchStatus::Failed { error: _ } => ("❌", "#ef4444", "Failed".into()),
    };

    let display_name = format!("{} ({})", batch.entity_id, batch.entity_type);

    // Dim read items when history is shown
    let opacity_class = if batch.is_read && show_read {
        "opacity-60"
    } else {
        ""
    };

    rsx! {
        div {
            class: "flex flex-col",

            div {
                class: "queue-item flex items-center gap-2 p-2 bg-dark-bg rounded {opacity_class}",

                span { style: format!("color: {};", status_color), "{status_icon}" }

                div { class: "flex-1 min-w-0",
                    div { class: "text-white text-sm overflow-hidden text-ellipsis whitespace-nowrap",
                        "{display_name}"
                    }
                    div { class: "text-gray-500 text-xs",
                        "{batch.asset_type}"
                    }
                }

                div {
                    class: "flex items-center gap-1",
                    match &batch.status {
                        BatchStatus::Queued { .. } => rsx! {
                            button {
                                onclick: {
                                    let batch_id = batch.batch_id.clone();
                                    let asset_service = use_asset_service();
                                    let state = use_generation_state();
                                    move |_| {
                                        let bid = batch_id.clone();
                                        let svc = asset_service.clone();
                                        let mut gen_state = state;
                                        spawn_task(async move {
                                            match svc.cancel_batch(&bid).await {
                                                Ok(_) => {
                                                    tracing::info!("Cancelled batch: {}", bid);
                                                    gen_state.remove_batch(&bid);
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to cancel batch {}: {}", bid, e);
                                                    action_error.set(Some(format!("Failed to cancel: {}", e)));
                                                }
                                            }
                                        });
                                    }
                                },
                                class: "px-2 py-1 bg-red-500 text-white border-none rounded cursor-pointer text-xs",
                                "Cancel"
                            }
                            button {
                                onclick: move |_| {
                                    let current = *expanded_details.read();
                                    expanded_details.set(!current);
                                },
                                class: "px-2 py-1 bg-gray-700 text-white border-none rounded cursor-pointer text-xs",
                                if *expanded_details.read() { "Hide Details" } else { "Details" }
                            }
                        },
                        BatchStatus::Generating { progress } => rsx! {
                            div {
                                class: "w-[50px] h-1 bg-gray-700 rounded-sm overflow-hidden",
                                div {
                                    style: format!("width: {}%; height: 100%; background: #f59e0b;", progress),
                                }
                            }
                            button {
                                onclick: {
                                    let batch_id = batch.batch_id.clone();
                                    let asset_service = use_asset_service();
                                    let state = use_generation_state();
                                    move |_| {
                                        let bid = batch_id.clone();
                                        let svc = asset_service.clone();
                                        let mut gen_state = state;
                                        spawn_task(async move {
                                            match svc.cancel_batch(&bid).await {
                                                Ok(_) => {
                                                    tracing::info!("Cancelled batch: {}", bid);
                                                    gen_state.remove_batch(&bid);
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to cancel batch {}: {}", bid, e);
                                                    action_error.set(Some(format!("Failed to cancel: {}", e)));
                                                }
                                            }
                                        });
                                    }
                                },
                                class: "px-2 py-1 bg-red-500 text-white border-none rounded cursor-pointer text-xs",
                                "Cancel"
                            }
                            button {
                                onclick: move |_| {
                                    let current = *expanded_details.read();
                                    expanded_details.set(!current);
                                },
                                class: "px-2 py-1 bg-gray-700 text-white border-none rounded cursor-pointer text-xs",
                                if *expanded_details.read() { "Hide Details" } else { "Details" }
                            }
                        },
                        BatchStatus::Ready { .. } => rsx! {
                            button {
                                onclick: {
                                    let batch_id = batch.batch_id.clone();
                                    let entity_type = batch.entity_type.clone();
                                    let entity_id = batch.entity_id.clone();
                                    let state = use_generation_state();
                                    let world_id_clone = world_id.clone();
                                    let nav_handler = on_navigate_to_entity;
                                    let gen_svc = generation_service.clone();
                                    let plat_clone = platform.clone();
                                    move |_| {
                                        let bid = batch_id.clone();
                                        let wid = world_id_clone.clone();
                                        let mut gen_state = state;
                                        let nav = nav_handler;
                                        let svc = gen_svc.clone();
                                        let plat = plat_clone.clone();
                                    spawn_task(async move {
                                            if let Err(e) = mark_batch_read_and_sync(&svc, &mut gen_state, &bid, wid.as_deref(), plat.as_ref()).await {
                                            tracing::error!("Failed to mark batch read and sync: {}", e);
                                        }
                                    });
                                        // Navigate to entity if handler provided
                                        if let Some(handler) = nav {
                                            handler.call((entity_type.clone(), entity_id.clone()));
                                        }
                                    }
                                },
                                class: "px-2 py-1 bg-green-500 text-white border-none rounded cursor-pointer text-xs",
                                "Select"
                            }
                            button {
                                onclick: {
                                    let batch_id = batch_id.clone();
                                    move |_| {
                                        let mut state = use_generation_state();
                                        state.remove_batch(&batch_id);
                                    }
                                },
                                class: "px-2 py-1 bg-gray-500 text-white border-none rounded cursor-pointer text-xs",
                                "Clear"
                            }
                            button {
                                onclick: move |_| {
                                    let current = *expanded_details.read();
                                    expanded_details.set(!current);
                                },
                                class: "px-2 py-1 bg-gray-700 text-white border-none rounded cursor-pointer text-xs",
                                if *expanded_details.read() { "Hide Details" } else { "Details" }
                            }
                        },
                        BatchStatus::Failed { error: _ } => rsx! {
                            button {
                                onclick: move |_| {
                                    let current = *expanded_error.read();
                                    expanded_error.set(!current);
                                },
                                class: "px-2 py-1 bg-red-500 text-white border-none rounded cursor-pointer text-xs",
                                if *expanded_error.read() { "Hide Error" } else { "Show Error" }
                            }
                            button {
                                onclick: {
                                    let batch_id = batch.batch_id.clone();
                                    let asset_service = use_asset_service();
                                    let state = use_generation_state();
                                    move |_| {
                                        let bid = batch_id.clone();
                                        let svc = asset_service.clone();
                                        let mut gen_state = state;
                                        spawn_task(async move {
                                            match svc.retry_batch(&bid).await {
                                                Ok(new_batch_id) => {
                                                    tracing::info!("Retried batch {} -> {}", bid, new_batch_id);
                                                    // Remove old failed batch
                                                    gen_state.remove_batch(&bid);
                                                    // New batch will be added via WebSocket event
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to retry batch {}: {}", bid, e);
                                                    action_error.set(Some(format!("Failed to retry: {}", e)));
                                                }
                                            }
                                        });
                                    }
                                },
                                class: "px-2 py-1 bg-amber-500 text-white border-none rounded cursor-pointer text-xs",
                                "Retry"
                            }
                            button {
                                onclick: {
                                    let batch_id_copy = batch_id.clone();
                                    move |_| {
                                        let mut state = use_generation_state();
                                        state.remove_batch(&batch_id_copy);
                                    }
                                },
                                class: "px-2 py-1 bg-gray-500 text-white border-none rounded cursor-pointer text-xs",
                                "Clear"
                            }
                            button {
                                onclick: move |_| {
                                    let current = *expanded_details.read();
                                    expanded_details.set(!current);
                                },
                                class: "px-2 py-1 bg-gray-700 text-white border-none rounded cursor-pointer text-xs",
                                if *expanded_details.read() { "Hide Details" } else { "Details" }
                            }
                        },
                    }
                }
            }


            // Action error feedback
            if let Some(ref err) = *action_error.read() {
                div {
                    class: "mt-2 px-3 py-2 bg-red-500/20 border border-red-500/50 rounded text-red-400 text-xs cursor-pointer",
                    onclick: move |_| action_error.set(None),
                    "{err}"
                }
            }

            // Expanded error details for failed batches
            if let BatchStatus::Failed { error } = &batch.status {
                if *expanded_error.read() {
                    div {
                        class: "mt-2 p-3 bg-gray-800 rounded-md border-l-4 border-red-500 shadow-md",
                        div {
                            class: "flex items-center gap-2 mb-2",
                            span { class: "text-red-500 text-base", "⚠️" }
                            div {
                                class: "text-red-500 text-xs font-bold",
                                "Error Details"
                            }
                        }
                        div {
                            class: "text-gray-200 text-xs whitespace-pre-wrap break-words leading-relaxed font-mono",
                            "{error}"
                        }
                    }
                }
            }

            // Expanded batch details
            if *expanded_details.read() {
                div {
                    class: "mt-2 p-3 bg-gray-800 rounded-md border-l-4 border-purple-500",
                    div {
                        class: "text-gray-400 text-xs mb-2",
                        "Entity: {batch.entity_type} - {batch.entity_id}"
                    }
                    div {
                        class: "text-gray-400 text-xs mb-2",
                        "Asset Type: {batch.asset_type}"
                    }
                    div {
                        class: "text-gray-400 text-xs",
                        "Batch ID: {batch.batch_id}"
                    }
                }
            }
        }
    }
}

/// Queue row for suggestion tasks (text generation)
#[component]
fn SuggestionQueueRow(
    suggestion: SuggestionTask,
    selected_suggestion: Signal<Option<SuggestionTask>>,
    #[props(default = false)] show_read: bool,
    world_id: Option<String>,
    #[props(default)] on_navigate_to_entity: Option<EventHandler<(String, String)>>,
) -> Element {
    let generation_service = use_generation_service();
    let suggestion_service = use_suggestion_service();
    let mut generation_state = use_generation_state();
    let platform = use_platform();
    let mut expanded_error: Signal<bool> = use_signal(|| false);
    let mut action_error: Signal<Option<String>> = use_signal(|| None);
    let (status_icon, status_color, status_text) = match &suggestion.status {
        SuggestionStatus::Queued => ("💭", "#9ca3af", "Queued".to_string()),
        SuggestionStatus::Processing => ("⚙️", "#f59e0b", "Processing".to_string()),
        SuggestionStatus::Ready {
            suggestions: results,
        } => ("✅", "#22c55e", format!("{} ready", results.len())),
        SuggestionStatus::Failed { error: _ } => ("❌", "#ef4444", "Failed".to_string()),
    };

    let display_name = format!("{} suggestion", suggestion.field_type.replace("_", " "));
    let suggestion_clone = suggestion.clone();
    let request_id_for_view = suggestion.request_id.clone();
    let request_id_for_clear = suggestion.request_id.clone();
    let request_id_for_failed_clear = suggestion.request_id.clone();

    let opacity_class = if suggestion.is_read && show_read {
        "opacity-60"
    } else {
        ""
    };

    rsx! {
        div {
            class: "queue-item flex items-center gap-2 p-2 bg-dark-bg rounded {opacity_class}",

            span { style: format!("color: {};", status_color), "{status_icon}" }

            div { class: "flex-1 min-w-0",
                div { class: "text-white text-sm overflow-hidden text-ellipsis whitespace-nowrap",
                    "{display_name}"
                }
                if let Some(entity_id) = &suggestion.entity_id {
                    div { class: "text-gray-500 text-xs",
                        "{entity_id}"
                    }
                }
            }

            div {
                class: "flex items-center gap-1",
                match &suggestion.status {
                    SuggestionStatus::Ready { .. } => rsx! {
                        button {
                            onclick: {
                                let req_id = request_id_for_view.clone();
                                let world_id_clone = world_id.clone();
                                let gen_svc = generation_service.clone();
                                let plat_clone = platform.clone();
                                move |_| {
                                    // Show modal only - don't auto-navigate to form
                                    // Navigation can be done from the modal if user wants to apply
                                    selected_suggestion.set(Some(suggestion_clone.clone()));
                                    let req_id_clone = req_id.clone();
                                    let wid = world_id_clone.clone();
                                    let mut gen_state = generation_state;
                                    let svc = gen_svc.clone();
                                    let plat = plat_clone.clone();
                                    spawn_task(async move {
                                        if let Err(e) = mark_suggestion_read_and_sync(&svc, &mut gen_state, &req_id_clone, wid.as_deref(), plat.as_ref()).await {
                                            tracing::error!("Failed to mark suggestion read and sync: {}", e);
                                        }
                                    });
                                }
                            },
                            class: "px-2 py-1 bg-green-500 text-white border-none rounded cursor-pointer text-xs",
                            "View"
                        }
                        button {
                            onclick: {
                                let req_id = request_id_for_clear.clone();
                                let gen_svc = generation_service.clone();
                                move |_| {
                                    let request_id = req_id.clone();
                                    let svc = gen_svc.clone();
                                    tracing::info!(request_id = %request_id, "Clear button clicked");

                                    // IMPORTANT: Spawn the async dismiss task BEFORE modifying state
                                    // to ensure it's scheduled before any re-render occurs
                                    tracing::info!(request_id = %request_id, "Spawning dismiss_suggestion task via spawn_task");
                                    let req_for_spawn = request_id.clone();
                                    spawn_task(async move {
                                        tracing::info!("ASYNC BLOCK STARTED - dismiss task");
                                        tracing::info!(request_id = %req_for_spawn, "Calling dismiss_suggestion");
                                        match svc.dismiss_suggestion(&req_for_spawn).await {
                                            Ok(()) => {
                                                tracing::info!(request_id = %req_for_spawn, "Successfully dismissed suggestion from server");
                                            }
                                            Err(e) => {
                                                tracing::error!(request_id = %req_for_spawn, error = %e, "Failed to dismiss suggestion from server");
                                            }
                                        }
                                    });

                                    // Remove from local state after spawning for responsiveness
                                    tracing::info!(request_id = %req_id, "Removing from local state");
                                    generation_state.remove_suggestion(&req_id);
                                }
                            },
                            class: "px-2 py-1 bg-gray-500 text-white border-none rounded cursor-pointer text-xs",
                            "Clear"
                        }
                    },
                    SuggestionStatus::Queued | SuggestionStatus::Processing => rsx! {
                        span { style: format!("color: {}; font-size: 0.75rem;", status_color), "{status_text}" }
                        button {
                            onclick: {
                                let request_id = suggestion.request_id.clone();
                                let svc = suggestion_service.clone();
                                move |_| {
                                    let req_id = request_id.clone();
                                    let svc = svc.clone();
                                    spawn_task(async move {
                                        match svc.cancel_suggestion(&req_id).await {
                                            Ok(_) => {
                                                tracing::info!("Cancelled suggestion: {}", req_id);
                                                // The WebSocket event will update the status to Failed
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to cancel suggestion {}: {}", req_id, e);
                                                action_error.set(Some(format!("Failed to cancel: {}", e)));
                                            }
                                        }
                                    });
                                }
                            },
                            class: "px-1.5 py-0.5 bg-red-500 text-white border-none rounded cursor-pointer text-[0.625rem]",
                            "Cancel"
                        }
                    },
                    SuggestionStatus::Failed { error: _ } => rsx! {
                        button {
                            onclick: move |_| {
                                let current = *expanded_error.read();
                                expanded_error.set(!current);
                            },
                            class: "px-2 py-1 bg-red-500 text-white border-none rounded cursor-pointer text-xs",
                            if *expanded_error.read() { "Hide Error" } else { "Show Error" }
                        }
                        button {
                            onclick: {
                                let request_id = suggestion.request_id.clone();
                                let field_type = suggestion.field_type.clone();
                                let context = suggestion.context.clone();
                                let world_id_for_retry = suggestion.world_id.clone();
                                let svc = suggestion_service.clone();
                                move |_| {
                                    if let (Some(ctx), Some(wid)) = (context.clone(), world_id_for_retry.clone()) {
                                        let req_id = request_id.clone();
                                        let field = field_type.clone();
                                        let svc = svc.clone();
                                        let mut gen_state = generation_state;
                                        spawn_task(async move {
                                            match svc.enqueue_suggestion(&field, &wid, &ctx).await {
                                                Ok(new_request_id) => {
                                                    tracing::info!("Retried suggestion {} -> {}", req_id, new_request_id);
                                                    // Remove old failed suggestion
                                                    gen_state.remove_suggestion(&req_id);
                                                    // Add new one with context
                                                    gen_state.add_suggestion_task(
                                                        new_request_id,
                                                        field,
                                                        None,
                                                        Some(ctx),
                                                        Some(wid),
                                                    );
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to retry suggestion {}: {}", req_id, e);
                                                    action_error.set(Some(format!("Failed to retry: {}", e)));
                                                }
                                            }
                                        });
                                    } else {
                                        action_error.set(Some("Cannot retry: missing context".to_string()));
                                    }
                                }
                            },
                            class: "px-2 py-1 bg-amber-500 text-white border-none rounded cursor-pointer text-xs",
                            "Retry"
                        }
                        button {
                            onclick: {
                                let req_id = request_id_for_failed_clear.clone();
                                let gen_svc = generation_service.clone();
                                move |_| {
                                    let request_id = req_id.clone();
                                    let svc = gen_svc.clone();
                                    tracing::info!(request_id = %request_id, "Clear (failed) button clicked");

                                    // IMPORTANT: Spawn the async dismiss task BEFORE modifying state
                                    spawn_task(async move {
                                        tracing::info!(request_id = %request_id, "Calling dismiss_suggestion for failed item");
                                        match svc.dismiss_suggestion(&request_id).await {
                                            Ok(()) => {
                                                tracing::info!(request_id = %request_id, "Successfully dismissed failed suggestion from server");
                                            }
                                            Err(e) => {
                                                tracing::error!(request_id = %request_id, error = %e, "Failed to dismiss suggestion from server");
                                            }
                                        }
                                    });

                                    // Remove from local state after spawning
                                    generation_state.remove_suggestion(&req_id);
                                }
                            },
                            class: "px-2 py-1 bg-gray-500 text-white border-none rounded cursor-pointer text-xs",
                            "Clear"
                        }
                    },
                }
            }

            // Action error feedback
            if let Some(ref err) = *action_error.read() {
                div {
                    class: "mt-2 px-3 py-2 bg-red-500/20 border border-red-500/50 rounded text-red-400 text-xs cursor-pointer",
                    onclick: move |_| action_error.set(None),
                    "{err}"
                }
            }

            // Expanded error details for failed suggestions
            if let SuggestionStatus::Failed { error } = &suggestion.status {
                if *expanded_error.read() {
                    div {
                        class: "mt-2 p-3 bg-gray-800 rounded-md border-l-4 border-red-500 shadow-md",
                        div {
                            class: "flex items-center gap-2 mb-2",
                            span { class: "text-red-500 text-base", "⚠️" }
                            div {
                                class: "text-red-500 text-xs font-bold",
                                "Error Details"
                            }
                        }
                        div {
                            class: "text-gray-200 text-xs whitespace-pre-wrap break-words leading-relaxed font-mono",
                            "{error}"
                        }
                    }
                }
            }
        }
    }
}

/// Modal displaying full suggestion options for a selected task
#[component]
fn SuggestionViewModal(
    suggestion: SuggestionTask,
    on_close: EventHandler<()>,
    #[props(default)] on_navigate: Option<EventHandler<(String, String)>>,
) -> Element {
    let mut generation_state = use_generation_state();

    // Extract suggestions if ready
    let suggestions_list = match &suggestion.status {
        SuggestionStatus::Ready { suggestions } => suggestions.clone(),
        _ => Vec::new(),
    };

    let field_type = suggestion.field_type.clone();
    let request_id = suggestion.request_id.clone();
    let title = format!("Suggestions for {}", field_type.replace("_", " "));

    // Determine entity type from field type for navigation
    let entity_type = if field_type.starts_with("character_")
        || field_type.starts_with("deflection_")
        || field_type.starts_with("behavioral_")
        || field_type.starts_with("want_")
        || field_type.starts_with("actantial_")
    {
        "characters"
    } else if field_type.starts_with("location_") {
        "locations"
    } else {
        "characters" // Default fallback
    };

    let navigate_data = suggestion
        .entity_id
        .clone()
        .zip(on_navigate)
        .map(|(id, handler)| (id, entity_type.to_string(), handler));

    rsx! {
        // Backdrop
        div {
            onclick: move |_| on_close.call(()),
            class: "fixed inset-0 bg-black/50 flex items-center justify-center z-[200]",

            // Modal content
            div {
                onclick: move |evt| evt.stop_propagation(),
                class: "bg-gray-900 rounded-lg p-4 px-5 max-w-[480px] w-full max-h-[70vh] overflow-y-auto shadow-2xl",

                h3 {
                    class: "text-white text-[0.95rem] mb-2",
                    "{title}"
                }

                // Show field type context
                div {
                    class: "text-gray-400 text-xs mb-3",
                    "Field: {field_type}"
                    if let Some(entity_id) = &suggestion.entity_id {
                        " • Entity: {entity_id}"
                    }
                }

                if suggestions_list.is_empty() {
                    div {
                        class: "text-gray-400 text-[0.85rem]",
                        "No suggestion options available (still processing or failed)."
                    }
                } else {
                    div {
                        class: "text-gray-400 text-xs mb-2",
                        "Click a suggestion to apply it to the form field."
                    }
                    div {
                        class: "flex flex-col gap-2",
                        for (idx, text) in suggestions_list.iter().enumerate() {
                            {
                                let text_clone = text.clone();
                                let field_type_clone = field_type.clone();
                                let request_id_clone = request_id.clone();
                                rsx! {
                                    div {
                                        key: "{idx}",
                                        onclick: move |_| {
                                            // Set the selected suggestion - this triggers SuggestionButton's use_effect
                                            generation_state.select_suggestion(&field_type_clone, text_clone.clone());
                                            // Remove from queue since it's been used
                                            generation_state.remove_suggestion(&request_id_clone);
                                            // Close the modal
                                            on_close.call(());
                                        },
                                        class: "px-3 py-2 bg-gray-800 hover:bg-purple-700 rounded-md text-gray-200 text-sm cursor-pointer transition-colors",
                                        title: "Click to apply this suggestion",
                                        "{text}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "flex justify-end gap-2 mt-3",
                    if let Some((entity_id, entity_type, handler)) = navigate_data.clone() {
                        button {
                            onclick: move |_| {
                                handler.call((entity_type.clone(), entity_id.clone()));
                                on_close.call(());
                            },
                            class: "px-3 py-1 bg-blue-600 text-white border-none rounded-md text-[0.8rem] cursor-pointer",
                            "Go to Form"
                        }
                    }
                    button {
                        onclick: move |_| on_close.call(()),
                        class: "px-3 py-1 bg-gray-600 text-white border-none rounded-md text-[0.8rem] cursor-pointer",
                        "Close"
                    }
                }
            }
        }
    }
}
