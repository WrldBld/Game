//! Action panel component
//!
//! Displays available interactions and system buttons (inventory, character sheet, etc.)

use dioxus::prelude::*;

use wrldbldr_player_app::application::dto::InteractionData;

/// Props for the ActionPanel component
#[derive(Props, Clone, PartialEq)]
pub struct ActionPanelProps {
    /// Available scene interactions
    #[props(default)]
    pub interactions: Vec<InteractionData>,
    /// Handler for interaction selection
    pub on_interaction: EventHandler<InteractionData>,
    /// Handler for inventory button
    #[props(default)]
    pub on_inventory: Option<EventHandler<()>>,
    /// Handler for character sheet button
    #[props(default)]
    pub on_character: Option<EventHandler<()>>,
    /// Handler for map button
    #[props(default)]
    pub on_map: Option<EventHandler<()>>,
    /// Handler for people/known NPCs button
    #[props(default)]
    pub on_people: Option<EventHandler<()>>,
    /// Handler for log button
    #[props(default)]
    pub on_log: Option<EventHandler<()>>,
    /// Handler for skills button
    #[props(default)]
    pub on_skills: Option<EventHandler<()>>,
    /// Handler for region items (loot) button
    #[props(default)]
    pub on_region_items: Option<EventHandler<()>>,
    /// Number of items in the region (for badge display)
    #[props(default = 0)]
    pub region_items_count: usize,
    /// Whether all action buttons should be disabled (e.g., while waiting for response)
    #[props(default = false)]
    pub disabled: bool,
}

/// Action panel - displays system buttons and scene interactions
#[component]
pub fn ActionPanel(props: ActionPanelProps) -> Element {
    let available_interactions: Vec<_> = props
        .interactions
        .iter()
        .filter(|i| i.is_available)
        .collect();

    rsx! {
        div {
            class: "action-panel absolute bottom-4 left-4 flex flex-wrap gap-2 z-20",

            // System buttons
            if let Some(ref handler) = props.on_inventory {
                SystemButton {
                    label: "Inventory",
                    icon: "bag",
                    on_click: *handler,
                    disabled: props.disabled,
                }
            }

            if let Some(ref handler) = props.on_character {
                SystemButton {
                    label: "Character",
                    icon: "person",
                    on_click: *handler,
                    disabled: props.disabled,
                }
            }

            if let Some(ref handler) = props.on_map {
                SystemButton {
                    label: "Map",
                    icon: "map",
                    on_click: *handler,
                    disabled: props.disabled,
                }
            }

            if let Some(ref handler) = props.on_people {
                SystemButton {
                    label: "People",
                    icon: "people",
                    on_click: *handler,
                    disabled: props.disabled,
                }
            }

            if let Some(ref handler) = props.on_log {
                SystemButton {
                    label: "Log",
                    icon: "scroll",
                    on_click: *handler,
                    disabled: props.disabled,
                }
            }

            if let Some(ref handler) = props.on_skills {
                SystemButton {
                    label: "Skills",
                    icon: "skills",
                    on_click: *handler,
                    disabled: props.disabled,
                }
            }

            // Region items (Loot) button - only shown when items exist
            if let Some(ref handler) = props.on_region_items {
                if props.region_items_count > 0 {
                    LootButton {
                        count: props.region_items_count,
                        on_click: *handler,
                        disabled: props.disabled,
                    }
                }
            }

            // Divider between system and scene actions
            if !available_interactions.is_empty() {
                div {
                    class: "action-divider w-px h-8 bg-gray-700 mx-1",
                }
            }

            // Scene-specific interactions
            for interaction in available_interactions {
                InteractionButton {
                    key: "{interaction.id}",
                    interaction: interaction.clone(),
                    on_click: props.on_interaction,
                    disabled: props.disabled,
                }
            }
        }
    }
}

/// Props for SystemButton
#[derive(Props, Clone, PartialEq)]
pub struct SystemButtonProps {
    /// Button label
    pub label: &'static str,
    /// Icon name (for future icon system)
    pub icon: &'static str,
    /// Click handler
    pub on_click: EventHandler<()>,
    /// Whether button is disabled
    #[props(default = false)]
    pub disabled: bool,
}

/// System button (inventory, character, etc.)
#[component]
pub fn SystemButton(props: SystemButtonProps) -> Element {
    let icon_char = match props.icon {
        "bag" => "🎒",
        "person" => "📋",
        "map" => "🗺️",
        "people" => "👥",
        "scroll" => "📜",
        "skills" => "⚔️",
        _ => "⚙️",
    };

    // CRITICAL: Extract conditional classes BEFORE rsx! - no inline if in class strings
    let opacity_class = if props.disabled {
        "opacity-50"
    } else {
        "opacity-100"
    };
    let cursor_class = if props.disabled {
        "cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    rsx! {
        button {
            class: "btn btn-secondary flex items-center gap-2 px-3 py-2 {opacity_class} {cursor_class}",
            disabled: props.disabled,
            onclick: move |_| {
                if !props.disabled {
                    props.on_click.call(())
                }
            },

            span { "{icon_char}" }
            span { "{props.label}" }
        }
    }
}

/// Props for InteractionButton
#[derive(Props, Clone, PartialEq)]
pub struct InteractionButtonProps {
    /// The interaction to display
    pub interaction: InteractionData,
    /// Click handler
    pub on_click: EventHandler<InteractionData>,
    /// Whether button is disabled
    #[props(default = false)]
    pub disabled: bool,
}

/// Scene interaction button
#[component]
pub fn InteractionButton(props: InteractionButtonProps) -> Element {
    let icon = get_interaction_icon(&props.interaction.interaction_type);
    let interaction = props.interaction.clone();

    // CRITICAL: Extract conditional classes BEFORE rsx! - no inline if in class strings
    let opacity_class = if props.disabled {
        "opacity-50"
    } else {
        "opacity-100"
    };
    let cursor_class = if props.disabled {
        "cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    rsx! {
        button {
            class: "btn btn-secondary flex items-center gap-2 px-3 py-2 {opacity_class} {cursor_class}",
            disabled: props.disabled,
            onclick: move |_| {
                if !props.disabled {
                    props.on_click.call(interaction.clone())
                }
            },

            span { "{icon}" }
            span { "{props.interaction.name}" }

            // Show target if available
            if let Some(ref target) = props.interaction.target_name {
                span {
                    class: "text-gray-400 text-xs",
                    "({target})"
                }
            }
        }
    }
}

/// Get an emoji icon for the interaction type
fn get_interaction_icon(interaction_type: &str) -> &'static str {
    match interaction_type.to_lowercase().as_str() {
        "talk" | "dialogue" | "speak" => "💬",
        "examine" | "look" | "inspect" => "🔍",
        "use" | "interact" => "✋",
        "travel" | "go" | "move" => "🚪",
        "take" | "pickup" | "grab" => "📥",
        "give" | "offer" => "📤",
        "attack" | "fight" | "combat" => "⚔️",
        "buy" | "purchase" | "shop" => "💰",
        "sell" => "🏷️",
        "rest" | "sleep" => "😴",
        "read" => "📖",
        "open" => "📦",
        "close" => "📪",
        "lock" | "unlock" => "🔐",
        _ => "✨",
    }
}

/// Props for LootButton
#[derive(Props, Clone, PartialEq)]
pub struct LootButtonProps {
    /// Number of items available
    pub count: usize,
    /// Click handler
    pub on_click: EventHandler<()>,
    /// Whether button is disabled
    #[props(default = false)]
    pub disabled: bool,
}

/// Loot button with item count badge
#[component]
pub fn LootButton(props: LootButtonProps) -> Element {
    let opacity_class = if props.disabled {
        "opacity-50"
    } else {
        "opacity-100"
    };
    let cursor_class = if props.disabled {
        "cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    rsx! {
        button {
            class: "btn btn-secondary flex items-center gap-2 px-3 py-2 relative {opacity_class} {cursor_class} bg-amber-500/20 hover:bg-amber-500/30 border-amber-500/50",
            disabled: props.disabled,
            onclick: move |_| {
                if !props.disabled {
                    props.on_click.call(())
                }
            },

            span { "📦" }
            span { "Loot" }

            // Item count badge
            span {
                class: "absolute -top-1 -right-1 bg-amber-500 text-black text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center",
                "{props.count}"
            }
        }
    }
}
