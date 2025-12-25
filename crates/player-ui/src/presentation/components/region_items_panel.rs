//! Region Items Panel - Player UI for viewing and picking up items in the current region
//!
//! P1.1 Phase 6C: Items visible in the player's current region.

use dioxus::prelude::*;

use wrldbldr_protocol::RegionItemData;

/// Props for the RegionItemsPanel component
#[derive(Props, Clone, PartialEq)]
pub struct RegionItemsPanelProps {
    /// Items visible in the current region
    pub items: Vec<RegionItemData>,
    /// Handler for closing the panel
    pub on_close: EventHandler<()>,
    /// Handler for picking up an item (passes item_id)
    pub on_pickup: EventHandler<String>,
}

/// Region Items Panel - modal overlay showing items in the current region
#[component]
pub fn RegionItemsPanel(props: RegionItemsPanelProps) -> Element {
    rsx! {
        // Overlay background
        div {
            class: "region-items-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-4",
            onclick: move |_| props.on_close.call(()),

            // Panel container
            div {
                class: "region-items-panel bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl w-full max-w-lg max-h-[85vh] overflow-hidden flex flex-col shadow-2xl border border-amber-500/20",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-4 border-b border-white/10 flex justify-between items-center",

                    div {
                        h2 {
                            class: "text-xl font-bold text-white m-0",
                            "Items Nearby"
                        }
                        {
                            let item_count = props.items.len();
                            let plural = if item_count == 1 { "" } else { "s" };
                            rsx! {
                                p {
                                    class: "text-gray-400 text-sm m-0 mt-1",
                                    "{item_count} item{plural}"
                                }
                            }
                        }
                    }

                    button {
                        class: "w-8 h-8 flex items-center justify-center bg-white/5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition-colors",
                        onclick: move |_| props.on_close.call(()),
                        "x"
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    if props.items.is_empty() {
                        div {
                            class: "flex flex-col items-center justify-center py-12 text-center",
                            span {
                                class: "text-4xl mb-4 opacity-50",
                                "~"
                            }
                            p {
                                class: "text-gray-400 m-0",
                                "No items in this area."
                            }
                        }
                    } else {
                        div {
                            class: "grid gap-2",

                            for item in props.items.iter() {
                                RegionItemCard {
                                    key: "{item.id}",
                                    item: item.clone(),
                                    on_pickup: props.on_pickup.clone(),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for RegionItemCard
#[derive(Props, Clone, PartialEq)]
struct RegionItemCardProps {
    item: RegionItemData,
    on_pickup: EventHandler<String>,
}

/// Card displaying a single region item with pickup button
#[component]
fn RegionItemCard(props: RegionItemCardProps) -> Element {
    let item_id = props.item.id.clone();

    // Determine icon based on item type
    let icon = match props.item.item_type.as_deref() {
        Some("Weapon") => "+",
        Some("Consumable") => "o",
        Some("Key") => "#",
        Some("Quest") => "!",
        Some("Armor") => "^",
        _ => ".",
    };

    rsx! {
        div {
            class: "region-item bg-black/30 rounded-lg border border-white/10 p-3 flex items-start gap-3 hover:bg-white/5 transition-colors",

            // Item icon
            span {
                class: "text-lg w-6 text-center text-amber-400",
                "{icon}"
            }

            // Item details
            div {
                class: "flex-1 min-w-0",
                
                // Name and type
                div {
                    class: "flex items-center gap-2 flex-wrap",
                    span {
                        class: "text-white font-medium",
                        "{props.item.name}"
                    }
                    if let Some(ref item_type) = props.item.item_type {
                        span {
                            class: "text-xs text-gray-500 bg-white/5 px-1.5 py-0.5 rounded",
                            "{item_type}"
                        }
                    }
                }

                // Description
                if let Some(ref desc) = props.item.description {
                    p {
                        class: "text-gray-400 text-sm m-0 mt-1 leading-relaxed",
                        "{desc}"
                    }
                }
            }

            // Pickup button
            button {
                class: "px-3 py-1.5 bg-green-500/20 hover:bg-green-500/30 text-green-400 rounded text-sm transition-colors whitespace-nowrap",
                onclick: move |_| props.on_pickup.call(item_id.clone()),
                "Pick Up"
            }
        }
    }
}
