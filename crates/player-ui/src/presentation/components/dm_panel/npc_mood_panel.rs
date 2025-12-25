//! NPC Mood Management Panel (P1.4)
//!
//! Allows the DM to view and manage NPC moods and relationships toward PCs.
//! This component emits events that the parent should handle to persist changes.

use dioxus::prelude::*;

/// Mood options available for selection (matches domain::MoodLevel)
pub const MOOD_OPTIONS: &[&str] = &[
    "Friendly",
    "Neutral",
    "Suspicious",
    "Hostile",
    "Afraid",
    "Grateful",
    "Annoyed",
    "Curious",
    "Melancholic",
];

/// Relationship options (matches domain::RelationshipLevel)
pub const RELATIONSHIP_OPTIONS: &[&str] = &[
    "Ally",
    "Friend",
    "Acquaintance",
    "Stranger",
    "Rival",
    "Enemy",
    "Nemesis",
];

/// Event emitted when DM changes an NPC's mood
#[derive(Clone, PartialEq)]
pub struct MoodChangeEvent {
    pub npc_id: String,
    pub pc_id: String,
    pub mood: String,
    pub reason: Option<String>,
}

/// Event emitted when DM changes an NPC's relationship
#[derive(Clone, PartialEq)]
pub struct RelationshipChangeEvent {
    pub npc_id: String,
    pub pc_id: String,
    pub relationship: String,
}

/// Props for the NpcMoodPanel component
#[derive(Props, Clone, PartialEq)]
pub struct NpcMoodPanelProps {
    /// NPC ID
    pub npc_id: String,
    /// NPC name
    pub npc_name: String,
    /// PC ID (whose perspective we're showing mood for)
    pub pc_id: String,
    /// PC name
    pub pc_name: String,
    /// Current mood (from server or default)
    #[props(default = "Neutral".to_string())]
    pub current_mood: String,
    /// Current relationship (from server or default)
    #[props(default = "Stranger".to_string())]
    pub current_relationship: String,
    /// Handler called when mood is changed
    pub on_mood_change: EventHandler<MoodChangeEvent>,
    /// Handler called when relationship is changed
    pub on_relationship_change: EventHandler<RelationshipChangeEvent>,
}

/// NpcMoodPanel component - Manage NPC mood toward a specific PC
///
/// Displays NPC mood and relationship with controls to update them.
/// Emits events for the parent to handle.
#[component]
pub fn NpcMoodPanel(props: NpcMoodPanelProps) -> Element {
    let mut selected_mood = use_signal(|| props.current_mood.clone());
    let mut selected_relationship = use_signal(|| props.current_relationship.clone());
    let mut reason = use_signal(|| String::new());

    let npc_id = props.npc_id.clone();
    let pc_id = props.pc_id.clone();

    rsx! {
        div {
            class: "npc-mood-panel p-3 bg-dark-bg rounded-lg mb-2 border-l-4 border-amber-500",

            // Header with NPC name and PC context
            div {
                class: "flex justify-between items-center mb-3",
                h4 {
                    class: "text-amber-500 text-sm m-0",
                    "{props.npc_name}"
                }
                span {
                    class: "text-gray-400 text-xs",
                    "toward {props.pc_name}"
                }
            }

            // Mood selector
            div {
                class: "mb-3",
                label {
                    class: "block text-gray-400 text-xs uppercase mb-1",
                    "Mood"
                }
                select {
                    value: "{selected_mood}",
                    onchange: {
                        let npc_id = npc_id.clone();
                        let pc_id = pc_id.clone();
                        move |e: Event<FormData>| {
                            let new_mood = e.value();
                            selected_mood.set(new_mood.clone());

                            let reason_str = reason.read().clone();
                            let reason_opt = if reason_str.is_empty() { None } else { Some(reason_str) };
                            
                            props.on_mood_change.call(MoodChangeEvent {
                                npc_id: npc_id.clone(),
                                pc_id: pc_id.clone(),
                                mood: new_mood,
                                reason: reason_opt,
                            });
                        }
                    },
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm cursor-pointer",

                    for mood in MOOD_OPTIONS.iter() {
                        option {
                            value: "{mood}",
                            selected: *mood == selected_mood.read().as_str(),
                            "{mood}"
                        }
                    }
                }
            }

            // Relationship selector
            div {
                class: "mb-3",
                label {
                    class: "block text-gray-400 text-xs uppercase mb-1",
                    "Relationship"
                }
                select {
                    value: "{selected_relationship}",
                    onchange: {
                        let npc_id = npc_id.clone();
                        let pc_id = pc_id.clone();
                        move |e: Event<FormData>| {
                            let new_rel = e.value();
                            selected_relationship.set(new_rel.clone());
                            
                            props.on_relationship_change.call(RelationshipChangeEvent {
                                npc_id: npc_id.clone(),
                                pc_id: pc_id.clone(),
                                relationship: new_rel,
                            });
                        }
                    },
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm cursor-pointer",

                    for rel in RELATIONSHIP_OPTIONS.iter() {
                        option {
                            value: "{rel}",
                            selected: *rel == selected_relationship.read().as_str(),
                            "{rel}"
                        }
                    }
                }
            }

            // Optional reason input
            div {
                label {
                    class: "block text-gray-400 text-xs uppercase mb-1",
                    "Reason (optional)"
                }
                input {
                    r#type: "text",
                    value: "{reason}",
                    placeholder: "Why is this NPC's mood changing?",
                    oninput: move |e| reason.set(e.value()),
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm box-border",
                }
            }
        }
    }
}

/// Props for the NpcMoodListPanel component
#[derive(Props, Clone, PartialEq)]
pub struct NpcMoodListPanelProps {
    /// PC ID to show moods for
    pub pc_id: String,
    /// PC name
    pub pc_name: String,
    /// NPCs in the current scene
    pub scene_npcs: Vec<SceneNpcInfo>,
    /// Handler called when mood is changed for any NPC
    pub on_mood_change: EventHandler<MoodChangeEvent>,
    /// Handler called when relationship is changed for any NPC
    pub on_relationship_change: EventHandler<RelationshipChangeEvent>,
}

/// NPC info for the mood list
#[derive(Clone, PartialEq)]
pub struct SceneNpcInfo {
    pub id: String,
    pub name: String,
    pub current_mood: Option<String>,
    pub current_relationship: Option<String>,
}

/// NpcMoodListPanel - Shows moods for all NPCs in a scene toward a PC
#[component]
pub fn NpcMoodListPanel(props: NpcMoodListPanelProps) -> Element {
    if props.scene_npcs.is_empty() {
        return rsx! {
            div {
                class: "text-gray-400 text-sm italic p-3",
                "No NPCs in scene"
            }
        };
    }

    rsx! {
        div {
            class: "npc-mood-list",
            for npc in props.scene_npcs.iter() {
                NpcMoodPanel {
                    key: "{npc.id}",
                    npc_id: npc.id.clone(),
                    npc_name: npc.name.clone(),
                    pc_id: props.pc_id.clone(),
                    pc_name: props.pc_name.clone(),
                    current_mood: npc.current_mood.clone().unwrap_or_else(|| "Neutral".to_string()),
                    current_relationship: npc.current_relationship.clone().unwrap_or_else(|| "Stranger".to_string()),
                    on_mood_change: props.on_mood_change.clone(),
                    on_relationship_change: props.on_relationship_change.clone(),
                }
            }
        }
    }
}
