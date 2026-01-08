//! NPC Disposition Management Panel (P1.4)
//!
//! Allows the DM to view and manage NPC dispositions and relationships toward PCs.
//! This component emits events that the parent should handle to persist changes.

use dioxus::prelude::*;

/// Disposition options available for selection (matches domain::DispositionLevel)
pub const DISPOSITION_OPTIONS: &[&str] = &[
    "Hostile",
    "Suspicious",
    "Dismissive",
    "Neutral",
    "Respectful",
    "Friendly",
    "Grateful",
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

/// Event emitted when DM changes an NPC's disposition
#[derive(Clone, PartialEq)]
pub struct DispositionChangeEvent {
    pub npc_id: String,
    pub pc_id: String,
    pub disposition: String,
    pub reason: Option<String>,
}

/// Event emitted when DM changes an NPC's relationship
#[derive(Clone, PartialEq)]
pub struct RelationshipChangeEvent {
    pub npc_id: String,
    pub pc_id: String,
    pub relationship: String,
}

/// Props for the NpcDispositionPanel component
#[derive(Props, Clone, PartialEq)]
pub struct NpcDispositionPanelProps {
    /// NPC ID
    pub npc_id: String,
    /// NPC name
    pub npc_name: String,
    /// PC ID (whose perspective we're showing disposition for)
    pub pc_id: String,
    /// PC name
    pub pc_name: String,
    /// Current disposition (from server or default)
    #[props(default = "Neutral".to_string())]
    pub current_disposition: String,
    /// Current relationship (from server or default)
    #[props(default = "Stranger".to_string())]
    pub current_relationship: String,
    /// Handler called when disposition is changed
    pub on_disposition_change: EventHandler<DispositionChangeEvent>,
    /// Handler called when relationship is changed
    pub on_relationship_change: EventHandler<RelationshipChangeEvent>,
}

/// NpcDispositionPanel component - Manage NPC disposition toward a specific PC
///
/// Displays NPC disposition and relationship with controls to update them.
/// Emits events for the parent to handle.
#[component]
pub fn NpcDispositionPanel(props: NpcDispositionPanelProps) -> Element {
    let mut selected_disposition = use_signal(|| props.current_disposition.clone());
    let mut selected_relationship = use_signal(|| props.current_relationship.clone());
    let mut reason = use_signal(String::new);

    let npc_id = props.npc_id.clone();
    let pc_id = props.pc_id.clone();

    rsx! {
        div {
            class: "npc-disposition-panel p-3 bg-dark-bg rounded-lg mb-2 border-l-4 border-amber-500",

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

            // Disposition selector
            div {
                class: "mb-3",
                label {
                    class: "block text-gray-400 text-xs uppercase mb-1",
                    "Disposition"
                }
                select {
                    value: "{selected_disposition}",
                    onchange: {
                        let npc_id = npc_id.clone();
                        let pc_id = pc_id.clone();
                        move |e: Event<FormData>| {
                            let new_disposition = e.value();
                            selected_disposition.set(new_disposition.clone());

                            let reason_str = reason.read().clone();
                            let reason_opt = if reason_str.is_empty() { None } else { Some(reason_str) };

                            props.on_disposition_change.call(DispositionChangeEvent {
                                npc_id: npc_id.clone(),
                                pc_id: pc_id.clone(),
                                disposition: new_disposition,
                                reason: reason_opt,
                            });
                        }
                    },
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm cursor-pointer",

                    for disposition in DISPOSITION_OPTIONS.iter() {
                        option {
                            value: "{disposition}",
                            selected: *disposition == selected_disposition.read().as_str(),
                            "{disposition}"
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
                    placeholder: "Why is this NPC's disposition changing?",
                    oninput: move |e| reason.set(e.value()),
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm box-border",
                }
            }
        }
    }
}

/// Props for the NpcDispositionListPanel component
#[derive(Props, Clone, PartialEq)]
pub struct NpcDispositionListPanelProps {
    /// PC ID to show dispositions for
    pub pc_id: String,
    /// PC name
    pub pc_name: String,
    /// NPCs in the current scene
    pub scene_npcs: Vec<SceneNpcInfo>,
    /// Handler called when disposition is changed for any NPC
    pub on_disposition_change: EventHandler<DispositionChangeEvent>,
    /// Handler called when relationship is changed for any NPC
    pub on_relationship_change: EventHandler<RelationshipChangeEvent>,
}

/// NPC info for the disposition list
#[derive(Clone, PartialEq)]
pub struct SceneNpcInfo {
    pub id: String,
    pub name: String,
    pub current_disposition: Option<String>,
    pub current_relationship: Option<String>,
}

/// NpcDispositionListPanel - Shows dispositions for all NPCs in a scene toward a PC
#[component]
pub fn NpcDispositionListPanel(props: NpcDispositionListPanelProps) -> Element {
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
            class: "npc-disposition-list",
            for npc in props.scene_npcs.iter() {
                NpcDispositionPanel {
                    key: "{npc.id}",
                    npc_id: npc.id.clone(),
                    npc_name: npc.name.clone(),
                    pc_id: props.pc_id.clone(),
                    pc_name: props.pc_name.clone(),
                    current_disposition: npc.current_disposition.clone().unwrap_or_else(|| "Neutral".to_string()),
                    current_relationship: npc.current_relationship.clone().unwrap_or_else(|| "Stranger".to_string()),
                    on_disposition_change: props.on_disposition_change,
                    on_relationship_change: props.on_relationship_change,
                }
            }
        }
    }
}
