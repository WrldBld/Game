//! Inventory Panel - Player UI for viewing and managing inventory
//!
//! US-CHAR-009: Player inventory with equipped items and actions.

use dioxus::prelude::*;

use crate::application::dto::InventoryItemData;

/// Props for the InventoryPanel component
#[derive(Props, Clone, PartialEq)]
pub struct InventoryPanelProps {
    /// Character name for display
    pub character_name: String,
    /// Inventory items
    pub items: Vec<InventoryItemData>,
    /// Whether data is still loading
    #[props(default = false)]
    pub is_loading: bool,
    /// Handler for closing the panel
    pub on_close: EventHandler<()>,
    /// Handler for using an item
    #[props(default)]
    pub on_use_item: Option<EventHandler<String>>,
    /// Handler for equipping/unequipping an item
    #[props(default)]
    pub on_toggle_equip: Option<EventHandler<String>>,
    /// Handler for dropping an item
    #[props(default)]
    pub on_drop_item: Option<EventHandler<String>>,
}

/// Inventory Panel - modal overlay showing character inventory
#[component]
pub fn InventoryPanel(props: InventoryPanelProps) -> Element {
    // Group items by type
    let equipped_items: Vec<_> = props.items.iter().filter(|i| i.equipped).collect();
    let weapon_items: Vec<_> = props
        .items
        .iter()
        .filter(|i| !i.equipped && i.is_weapon())
        .collect();
    let consumable_items: Vec<_> = props
        .items
        .iter()
        .filter(|i| !i.equipped && i.is_consumable())
        .collect();
    let key_items: Vec<_> = props
        .items
        .iter()
        .filter(|i| !i.equipped && (i.is_key() || i.is_quest()))
        .collect();
    let other_items: Vec<_> = props
        .items
        .iter()
        .filter(|i| {
            !i.equipped && !i.is_weapon() && !i.is_consumable() && !i.is_key() && !i.is_quest()
        })
        .collect();

    rsx! {
        // Overlay background
        div {
            class: "inventory-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-4",
            onclick: move |_| props.on_close.call(()),

            // Panel container
            div {
                class: "inventory-panel bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl w-full max-w-2xl max-h-[85vh] overflow-hidden flex flex-col shadow-2xl border border-amber-500/20",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-4 border-b border-white/10 flex justify-between items-center",

                    div {
                        h2 {
                            class: "text-xl font-bold text-white m-0",
                            "Inventory"
                        }
                        p {
                            class: "text-gray-400 text-sm m-0 mt-1",
                            "{props.character_name}"
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

                    if props.is_loading {
                        div {
                            class: "flex items-center justify-center py-12",
                            span {
                                class: "text-gray-400",
                                "Loading inventory..."
                            }
                        }
                    } else if props.items.is_empty() {
                        div {
                            class: "flex flex-col items-center justify-center py-12 text-center",
                            span {
                                class: "text-4xl mb-4",
                                "~"
                            }
                            p {
                                class: "text-gray-400 m-0",
                                "Your inventory is empty."
                            }
                        }
                    } else {
                        div {
                            class: "space-y-6",

                            // Equipped items section
                            if !equipped_items.is_empty() {
                                InventorySection {
                                    title: "Equipped",
                                    icon: "*",
                                    items: equipped_items.into_iter().cloned().collect(),
                                    on_use: props.on_use_item,
                                    on_toggle_equip: props.on_toggle_equip,
                                    on_drop: props.on_drop_item,
                                }
                            }

                            // Weapons section
                            if !weapon_items.is_empty() {
                                InventorySection {
                                    title: "Weapons",
                                    icon: "+",
                                    items: weapon_items.into_iter().cloned().collect(),
                                    on_use: props.on_use_item,
                                    on_toggle_equip: props.on_toggle_equip,
                                    on_drop: props.on_drop_item,
                                }
                            }

                            // Consumables section
                            if !consumable_items.is_empty() {
                                InventorySection {
                                    title: "Consumables",
                                    icon: "o",
                                    items: consumable_items.into_iter().cloned().collect(),
                                    on_use: props.on_use_item,
                                    on_toggle_equip: props.on_toggle_equip,
                                    on_drop: props.on_drop_item,
                                }
                            }

                            // Key & Quest items section
                            if !key_items.is_empty() {
                                InventorySection {
                                    title: "Key Items",
                                    icon: "#",
                                    items: key_items.into_iter().cloned().collect(),
                                    on_use: props.on_use_item,
                                    on_toggle_equip: props.on_toggle_equip,
                                    on_drop: props.on_drop_item,
                                }
                            }

                            // Other items section
                            if !other_items.is_empty() {
                                InventorySection {
                                    title: "Items",
                                    icon: ".",
                                    items: other_items.into_iter().cloned().collect(),
                                    on_use: props.on_use_item,
                                    on_toggle_equip: props.on_toggle_equip,
                                    on_drop: props.on_drop_item,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for InventorySection
#[derive(Props, Clone, PartialEq)]
struct InventorySectionProps {
    title: &'static str,
    icon: &'static str,
    items: Vec<InventoryItemData>,
    on_use: Option<EventHandler<String>>,
    on_toggle_equip: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<String>>,
}

/// A section of the inventory (e.g., Weapons, Consumables)
#[component]
fn InventorySection(props: InventorySectionProps) -> Element {
    rsx! {
        div {
            class: "inventory-section",

            // Section header
            h3 {
                class: "text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2",
                span { "{props.icon}" }
                "{props.title}"
                span {
                    class: "text-xs text-gray-500",
                    "({props.items.len()})"
                }
            }

            // Items grid
            div {
                class: "grid gap-2",

                for item in props.items.iter() {
                    InventoryItemCard {
                        key: "{item.item.id}",
                        item: item.clone(),
                        on_use: props.on_use,
                        on_toggle_equip: props.on_toggle_equip,
                        on_drop: props.on_drop,
                    }
                }
            }
        }
    }
}

/// Props for InventoryItemCard
#[derive(Props, Clone, PartialEq)]
struct InventoryItemCardProps {
    item: InventoryItemData,
    on_use: Option<EventHandler<String>>,
    on_toggle_equip: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<String>>,
}

/// Card displaying a single inventory item
#[component]
fn InventoryItemCard(props: InventoryItemCardProps) -> Element {
    let mut expanded = use_signal(|| false);

    let border_class = if props.item.equipped {
        "border-amber-500/50"
    } else {
        "border-white/10"
    };

    let item_id = props.item.item.id.clone();

    rsx! {
        div {
            class: "inventory-item bg-black/30 rounded-lg border {border_class} overflow-hidden",

            // Item header (always visible)
            button {
                class: "w-full p-3 flex items-center gap-3 text-left bg-transparent border-none cursor-pointer hover:bg-white/5 transition-colors",
                onclick: move |_| {
                    let current = *expanded.read();
                    expanded.set(!current);
                },

                // Item icon/type indicator
                span {
                    class: "text-lg w-6 text-center",
                    match props.item.item.item_type.as_deref() {
                        Some("Weapon") => "+",
                        Some("Consumable") => "o",
                        Some("Key") => "#",
                        Some("Quest") => "!",
                        _ => ".",
                    }
                }

                // Item name and quantity
                div {
                    class: "flex-1",
                    div {
                        class: "flex items-center gap-2",
                        span {
                            class: if props.item.equipped { "text-amber-400 font-medium" } else { "text-white font-medium" },
                            "{props.item.item.name}"
                        }
                        if props.item.equipped {
                            span {
                                class: "text-xs text-amber-400/70 bg-amber-500/20 px-1.5 py-0.5 rounded",
                                "Equipped"
                            }
                        }
                    }
                    if props.item.quantity > 1 {
                        span {
                            class: "text-xs text-gray-500",
                            "x{props.item.quantity}"
                        }
                    }
                }

                // Expand indicator
                span {
                    class: "text-gray-500 text-sm",
                    if *expanded.read() { "v" } else { ">" }
                }
            }

            // Expanded details
            if *expanded.read() {
                div {
                    class: "px-3 pb-3 border-t border-white/5 pt-3",

                    // Description
                    if let Some(ref desc) = props.item.item.description {
                        p {
                            class: "text-gray-400 text-sm m-0 mb-3 leading-relaxed",
                            "{desc}"
                        }
                    }

                    // Item metadata
                    div {
                        class: "flex flex-wrap gap-2 text-xs text-gray-500 mb-3",

                        span {
                            "{props.item.type_display()}"
                        }

                        if let Some(ref method) = props.item.acquisition_method {
                            span {
                                " | {method}"
                            }
                        }
                    }

                    // Action buttons
                    div {
                        class: "flex gap-2",

                        // Use button (for consumables)
                        if props.item.is_consumable() {
                            if let Some(ref handler) = props.on_use {
                                {
                                    let handler = *handler;
                                    let id = item_id.clone();
                                    rsx! {
                                        button {
                                            class: "px-3 py-1.5 bg-green-500/20 hover:bg-green-500/30 text-green-400 rounded text-sm transition-colors",
                                            onclick: move |_| handler.call(id.clone()),
                                            "Use"
                                        }
                                    }
                                }
                            }
                        }

                        // Equip/Unequip button (for weapons, etc.)
                        if props.item.is_weapon() || props.item.item.item_type.as_deref() == Some("Armor") {
                            if let Some(ref handler) = props.on_toggle_equip {
                                {
                                    let handler = *handler;
                                    let id = item_id.clone();
                                    let is_equipped = props.item.equipped;
                                    rsx! {
                                        button {
                                            class: if is_equipped {
                                                "px-3 py-1.5 bg-amber-500/20 hover:bg-amber-500/30 text-amber-400 rounded text-sm transition-colors"
                                            } else {
                                                "px-3 py-1.5 bg-blue-500/20 hover:bg-blue-500/30 text-blue-400 rounded text-sm transition-colors"
                                            },
                                            onclick: move |_| handler.call(id.clone()),
                                            if is_equipped { "Unequip" } else { "Equip" }
                                        }
                                    }
                                }
                            }
                        }

                        // Drop button (not for key/quest items)
                        if !props.item.is_key() && !props.item.is_quest() {
                            if let Some(ref handler) = props.on_drop {
                                {
                                    let handler = *handler;
                                    let id = item_id.clone();
                                    rsx! {
                                        button {
                                            class: "px-3 py-1.5 bg-red-500/10 hover:bg-red-500/20 text-red-400/70 rounded text-sm transition-colors",
                                            onclick: move |_| handler.call(id.clone()),
                                            "Drop"
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
