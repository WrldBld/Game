//! Workflow Slot List Component
//!
//! Displays all available workflow slots organized by category,
//! showing which are configured and allowing selection/configuration.

use dioxus::prelude::*;

use crate::presentation::services::use_workflow_service;
use crate::application::services::{WorkflowSlotStatus, WorkflowSlotCategory};

/// Props for the WorkflowSlotList component
#[derive(Props, Clone, PartialEq)]
pub struct WorkflowSlotListProps {
    /// Currently selected slot ID
    pub selected_slot: Option<String>,
    /// Callback when a slot is selected for viewing
    pub on_select: EventHandler<String>,
    /// Callback when configure button is clicked
    pub on_configure: EventHandler<String>,
}

/// List of all workflow slots with their configuration status
#[component]
pub fn WorkflowSlotList(props: WorkflowSlotListProps) -> Element {
    let workflow_service = use_workflow_service();

    // Track loading state
    let mut is_loading = use_signal(|| true);
    // Track error state
    let mut error: Signal<Option<String>> = use_signal(|| None);
    // Store the workflow slot categories (pre-grouped by backend)
    let mut categories: Signal<Vec<WorkflowSlotCategory>> = use_signal(Vec::new);

    // Fetch workflow slots on mount
    use_effect(move || {
        let svc = workflow_service.clone();
        spawn(async move {
            match svc.list_workflows().await {
                Ok(response) => {
                    categories.set(response.categories);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    is_loading.set(false);
                }
            }
        });
    });

    rsx! {
        div {
            class: "workflow-slot-list flex-1 flex flex-col bg-dark-surface rounded-lg overflow-hidden",

            // Header
            div {
                class: "p-4 border-b border-gray-700",

                h3 {
                    class: "text-white text-base m-0 mb-1",
                    "Workflow Slots"
                }
                p {
                    class: "text-gray-500 text-xs m-0",
                    "Configure ComfyUI workflows for asset generation"
                }
            }

            // Content
            div {
                class: "flex-1 overflow-y-auto p-2",

                if *is_loading.read() {
                    div {
                        class: "flex items-center justify-center py-8 text-gray-500",
                        "Loading workflows..."
                    }
                } else if let Some(err) = error.read().as_ref() {
                    div {
                        class: "p-4 bg-red-500 bg-opacity-10 rounded-lg text-red-500 text-sm",

                        p { class: "m-0 mb-2 font-semibold", "Failed to load workflow slots" }
                        p { class: "m-0", "{err}" }

                        // Help text
                        div {
                            class: "mt-4 pt-4 border-t border-red-500 border-opacity-30 text-xs text-gray-400",
                            p { class: "m-0 mb-1", "Troubleshooting:" }
                            ul { class: "m-0 pl-5",
                                li { "Is the Engine server running?" }
                                li { "Check that the Engine API is accessible" }
                            }
                        }
                    }
                } else if categories.read().is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center py-8 text-gray-500 text-center",

                        div { class: "text-3xl mb-2", "⚠️" }
                        p { class: "m-0 mb-2 text-gray-400", "No workflow slots available" }
                        p { class: "m-0 text-xs",
                            "The Engine returned an empty list."
                        }
                    }
                } else {
                    // Render categories directly from backend response - no filtering needed
                    for category in categories.read().iter() {
                        CategorySection {
                            title: category.name.clone(),
                            slots: category.slots.clone(),
                            selected_slot: props.selected_slot.clone(),
                            on_select: props.on_select.clone(),
                            on_configure: props.on_configure.clone(),
                        }
                    }
                }
            }
        }
    }
}

/// Category section with slots
#[derive(Props, Clone, PartialEq)]
struct CategorySectionProps {
    title: String,
    slots: Vec<WorkflowSlotStatus>,
    selected_slot: Option<String>,
    on_select: EventHandler<String>,
    on_configure: EventHandler<String>,
}

#[component]
fn CategorySection(props: CategorySectionProps) -> Element {
    if props.slots.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "category-section mb-4",

            h4 {
                class: "text-gray-400 text-xs uppercase tracking-wide m-0 mb-2 ml-2",
                "{props.title}"
            }

            div {
                class: "flex flex-col gap-1",

                for slot in props.slots.iter() {
                    SlotCard {
                        slot: slot.clone(),
                        is_selected: props.selected_slot.as_ref() == Some(&slot.slot),
                        on_select: props.on_select.clone(),
                        on_configure: props.on_configure.clone(),
                    }
                }
            }
        }
    }
}

/// Individual slot card
#[derive(Props, Clone, PartialEq)]
struct SlotCardProps {
    slot: WorkflowSlotStatus,
    is_selected: bool,
    on_select: EventHandler<String>,
    on_configure: EventHandler<String>,
}

#[component]
fn SlotCard(props: SlotCardProps) -> Element {
    let card_class = if props.is_selected {
        "flex items-center justify-between p-3 bg-blue-500 bg-opacity-20 border border-blue-500 rounded-lg cursor-pointer transition-all"
    } else {
        "flex items-center justify-between p-3 bg-black bg-opacity-20 border border-transparent rounded-lg cursor-pointer transition-all"
    };

    let slot_id = props.slot.slot.clone();
    let slot_id_for_configure = props.slot.slot.clone();

    rsx! {
        div {
            class: "{card_class}",
            onclick: move |_| props.on_select.call(slot_id.clone()),

            // Slot info
            div {
                class: "flex-1 min-w-0",

                div {
                    class: "flex items-center gap-2",

                    // Status indicator
                    div {
                        class: if props.slot.configured {
                            "w-2 h-2 rounded-full bg-green-500"
                        } else {
                            "w-2 h-2 rounded-full bg-gray-500"
                        }
                    }

                    // Name
                    span {
                        class: "text-white text-sm font-medium",
                        "{props.slot.display_name}"
                    }
                }

                // Dimensions
                div {
                    class: "text-gray-500 text-xs mt-1 ml-4",
                    "{props.slot.default_width}×{props.slot.default_height}"
                }

                // Workflow name if configured
                if props.slot.configured {
                    if let Some(ref config) = props.slot.config {
                        div {
                            class: "text-green-500 text-xs mt-1 ml-4 whitespace-nowrap overflow-hidden text-ellipsis",
                            "✓ {config.name}"
                        }
                    }
                }
            }

            // Configure button
            button {
                onclick: move |e| {
                    e.stop_propagation();
                    props.on_configure.call(slot_id_for_configure.clone());
                },
                class: "py-1.5 px-3 bg-gray-700 text-white border-0 rounded-md text-xs cursor-pointer",
                if props.slot.configured { "Edit" } else { "Configure" }
            }
        }
    }
}

