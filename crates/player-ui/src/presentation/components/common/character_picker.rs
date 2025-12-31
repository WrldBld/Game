//! Character Picker Component
//!
//! A searchable dropdown for selecting characters (NPCs or PCs).
//! Used in actantial views to pick targets for helper/opponent/sender/receiver roles.

use dioxus::prelude::*;

use crate::presentation::services::{use_character_service, use_player_character_service};

/// A selectable character entry
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterOption {
    /// Character ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Whether this is a PC (true) or NPC (false)
    pub is_pc: bool,
}

impl CharacterOption {
    /// Get the prefixed ID for selection value (e.g., "npc:abc123" or "pc:def456")
    pub fn prefixed_id(&self) -> String {
        if self.is_pc {
            format!("pc:{}", self.id)
        } else {
            format!("npc:{}", self.id)
        }
    }
}

/// Props for the CharacterPicker component
#[derive(Props, Clone, PartialEq)]
pub struct CharacterPickerProps {
    /// The world ID to fetch NPCs from
    pub world_id: String,
    /// Current selected value (in "type:id" format, e.g., "npc:abc123")
    pub value: String,
    /// Callback when selection changes (passes "type:id" format)
    pub on_change: EventHandler<String>,
    /// Placeholder text when nothing is selected
    #[props(default = "Select a character...")]
    pub placeholder: &'static str,
    /// Optional character ID to exclude from the list (e.g., the character being edited)
    #[props(default)]
    pub exclude_id: Option<String>,
}

/// Character picker with search functionality
#[component]
pub fn CharacterPicker(props: CharacterPickerProps) -> Element {
    let character_service = use_character_service();
    let pc_service = use_player_character_service();

    // State for character options
    let mut characters: Signal<Vec<CharacterOption>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut search_text = use_signal(String::new);
    let mut is_open = use_signal(|| false);

    // Load characters on mount
    {
        let world_id = props.world_id.clone();
        let exclude_id = props.exclude_id.clone();
        let char_service = character_service.clone();
        let pc_svc = pc_service.clone();

        use_effect(move || {
            let world_id = world_id.clone();
            let exclude_id = exclude_id.clone();
            let char_service = char_service.clone();
            let pc_svc = pc_svc.clone();
            spawn(async move {
                is_loading.set(true);
                let mut all_chars: Vec<CharacterOption> = Vec::new();

                // Fetch NPCs
                match char_service.list_characters(&world_id).await {
                    Ok(npcs) => {
                        for npc in npcs {
                            // Skip excluded character
                            if exclude_id.as_ref().map(|e| e == &npc.id).unwrap_or(false) {
                                continue;
                            }
                            all_chars.push(CharacterOption {
                                id: npc.id,
                                name: npc.name,
                                is_pc: false,
                            });
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load NPCs: {:?}", e);
                    }
                }

                // Fetch PCs for this world
                match pc_svc.list_pcs(&world_id).await {
                    Ok(pcs) => {
                        for pc in pcs {
                            // Skip excluded character
                            if exclude_id.as_ref().map(|e| e == &pc.id).unwrap_or(false) {
                                continue;
                            }
                            all_chars.push(CharacterOption {
                                id: pc.id,
                                name: pc.name,
                                is_pc: true,
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load PCs (may not have active world): {:?}", e);
                    }
                }

                // Sort by name
                all_chars.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                characters.set(all_chars);
                is_loading.set(false);
            });
        });
    }

    // Filter characters by search text
    let filtered_chars: Vec<CharacterOption> = {
        let search = search_text.read().to_lowercase();
        if search.is_empty() {
            characters.read().clone()
        } else {
            characters
                .read()
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&search))
                .cloned()
                .collect()
        }
    };

    // Find selected character name for display
    let selected_name: Option<String> = {
        let val = props.value.clone();
        if val.is_empty() {
            None
        } else {
            // Parse "type:id" format
            let id_part = val.split(':').next_back().unwrap_or("");
            characters
                .read()
                .iter()
                .find(|c| c.id == id_part)
                .map(|c| c.name.clone())
        }
    };

    // Handle selection
    let on_select = {
        let on_change = props.on_change;
        move |char: CharacterOption| {
            on_change.call(char.prefixed_id());
            is_open.set(false);
            search_text.set(String::new());
        }
    };

    // Handle clear
    let on_clear = {
        let on_change = props.on_change;
        move |_| {
            on_change.call(String::new());
        }
    };

    rsx! {
        div {
            class: "character-picker relative",

            // Selected value display / trigger button
            div {
                class: "flex items-center gap-1",

                button {
                    r#type: "button",
                    onclick: move |_| {
                        let current = *is_open.read();
                        is_open.set(!current);
                    },
                    class: "flex-1 flex items-center justify-between p-2 bg-dark-bg border border-gray-700 rounded text-left text-sm hover:border-gray-500 transition-colors",

                    if *is_loading.read() {
                        span { class: "text-gray-500", "Loading..." }
                    } else if let Some(name) = &selected_name {
                        span { class: "text-white", "{name}" }
                    } else {
                        span { class: "text-gray-500", "{props.placeholder}" }
                    }

                    // Dropdown arrow
                    span {
                        class: "text-gray-500 ml-2",
                        if *is_open.read() { "▲" } else { "▼" }
                    }
                }

                // Clear button (only show if value is selected)
                if !props.value.is_empty() {
                    button {
                        r#type: "button",
                        onclick: on_clear,
                        class: "p-2 text-gray-500 hover:text-red-400 transition-colors",
                        title: "Clear selection",
                        "✕"
                    }
                }
            }

            // Dropdown panel
            if *is_open.read() {
                div {
                    class: "absolute z-50 mt-1 w-full bg-gray-800 border border-gray-600 rounded shadow-lg max-h-64 overflow-hidden",

                    // Search input
                    div {
                        class: "p-2 border-b border-gray-700",
                        input {
                            r#type: "text",
                            value: "{search_text}",
                            oninput: move |e| search_text.set(e.value()),
                            placeholder: "Search characters...",
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white text-sm focus:border-accent-blue focus:outline-none",
                            autofocus: true,
                        }
                    }

                    // Character list
                    div {
                        class: "overflow-y-auto max-h-48",

                        if filtered_chars.is_empty() {
                            div {
                                class: "p-3 text-gray-500 text-sm text-center",
                                if characters.read().is_empty() {
                                    "No characters available"
                                } else {
                                    "No matches found"
                                }
                            }
                        } else {
                            for char in filtered_chars {
                                {
                                    let char_clone = char.clone();
                                    let mut on_select = on_select;
                                    let is_selected = props.value == char.prefixed_id();
                                    let bg_class = if char.is_pc {
                                        "bg-green-900/30 hover:bg-green-800/50"
                                    } else {
                                        "bg-blue-900/30 hover:bg-blue-800/50"
                                    };
                                    let type_badge = if char.is_pc { "PC" } else { "NPC" };
                                    let badge_class = if char.is_pc {
                                        "bg-green-700 text-green-100"
                                    } else {
                                        "bg-blue-700 text-blue-100"
                                    };

                                    rsx! {
                                        button {
                                            r#type: "button",
                                            key: "{char.prefixed_id()}",
                                            onclick: move |_| on_select(char_clone.clone()),
                                            class: "w-full flex items-center justify-between p-2 text-left text-sm transition-colors {bg_class}",
                                            class: if is_selected { "ring-1 ring-accent-blue" },

                                            span { class: "text-white truncate", "{char.name}" }

                                            span {
                                                class: "ml-2 px-1.5 py-0.5 text-xs rounded {badge_class}",
                                                "{type_badge}"
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
}
