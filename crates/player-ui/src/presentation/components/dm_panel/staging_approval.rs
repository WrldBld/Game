//! Staging Approval Popup Component
//!
//! Shows DM approval UI for NPC presence in a region when a player enters.
//! Displays rule-based and LLM-suggested NPCs with checkboxes for customization.

use dioxus::prelude::*;
use crate::presentation::state::game_state::{
    StagingApprovalData, PreviousStagingData,
};

/// NPC selection state for approval
#[derive(Clone, PartialEq)]
pub struct NpcSelection {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
    pub source: String, // "rule" or "llm"
}

/// Data emitted when staging is approved
#[derive(Clone, PartialEq)]
pub struct StagingApprovalResult {
    pub request_id: String,
    /// NPCs that were selected as present
    pub approved_npcs: Vec<(String, bool, bool)>, // (character_id, is_present, is_hidden_from_players)
    pub ttl_hours: i32,
    /// How this was finalized: "rule", "llm", "custom", "previous"
    pub source: String,
}

/// Data emitted when regeneration is requested
#[derive(Clone, PartialEq)]
pub struct StagingRegenerateRequest {
    pub request_id: String,
    pub guidance: String,
}

/// Props for the StagingApprovalPopup component
#[derive(Props, Clone, PartialEq)]
pub struct StagingApprovalPopupProps {
    /// The staging approval data from game state
    pub data: StagingApprovalData,
    /// Handler when DM approves staging
    pub on_approve: EventHandler<StagingApprovalResult>,
    /// Handler when DM requests LLM regeneration
    pub on_regenerate: EventHandler<StagingRegenerateRequest>,
    /// Handler when popup is dismissed
    pub on_close: EventHandler<()>,
}

/// StagingApprovalPopup - DM UI for approving NPC presence
#[component]
pub fn StagingApprovalPopup(props: StagingApprovalPopupProps) -> Element {
    // Initialize NPC selections from both rule and LLM sources
    let initial_selections: Vec<NpcSelection> = {
        let mut selections = Vec::new();
        
        // Add rule-based NPCs
        for npc in &props.data.rule_based_npcs {
            selections.push(NpcSelection {
                character_id: npc.character_id.clone(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: npc.is_present,
                is_hidden_from_players: npc.is_hidden_from_players,
                reasoning: npc.reasoning.clone(),
                source: "rule".to_string(),
            });
        }
        
        // Add LLM-based NPCs (that aren't already in rule-based)
        for npc in &props.data.llm_based_npcs {
            if !selections.iter().any(|s| s.character_id == npc.character_id) {
                selections.push(NpcSelection {
                    character_id: npc.character_id.clone(),
                    name: npc.name.clone(),
                    sprite_asset: npc.sprite_asset.clone(),
                    portrait_asset: npc.portrait_asset.clone(),
                    is_present: npc.is_present,
                    is_hidden_from_players: npc.is_hidden_from_players,
                    reasoning: npc.reasoning.clone(),
                    source: "llm".to_string(),
                });
            }
        }
        
        selections
    };

    let mut selections = use_signal(|| initial_selections);
    let mut ttl_hours = use_signal(|| props.data.default_ttl_hours);
    let mut show_regenerate = use_signal(|| false);
    let mut regenerate_guidance = use_signal(String::new);
    let mut source_type = use_signal(|| "custom".to_string());

    // Update selections when data changes (e.g., after regeneration)
    use_effect({
        let llm_npcs = props.data.llm_based_npcs.clone();
        move || {
            let mut current = selections.read().clone();
            
            // Update LLM-based NPCs
            for npc in &llm_npcs {
                if let Some(sel) = current.iter_mut().find(|s| s.character_id == npc.character_id && s.source == "llm") {
                    sel.is_present = npc.is_present;
                    sel.is_hidden_from_players = npc.is_hidden_from_players;
                    sel.reasoning = npc.reasoning.clone();
                }
            }
            
            selections.set(current);
        }
    });

    let handle_approve = {
        let request_id = props.data.request_id.clone();
        let on_approve = props.on_approve.clone();
        move |_| {
            let approved: Vec<(String, bool, bool)> = selections
                .read()
                .iter()
                .map(|s| (s.character_id.clone(), s.is_present, s.is_hidden_from_players))
                .collect();
            
            on_approve.call(StagingApprovalResult {
                request_id: request_id.clone(),
                approved_npcs: approved,
                ttl_hours: *ttl_hours.read(),
                source: source_type.read().clone(),
            });
        }
    };

    let handle_use_previous = {
        let request_id = props.data.request_id.clone();
        let on_approve = props.on_approve.clone();
        let previous = props.data.previous_staging.clone();
        move |_| {
            if let Some(ref prev) = previous {
                let approved: Vec<(String, bool, bool)> = prev.npcs
                    .iter()
                    .map(|n| (n.character_id.clone(), n.is_present, n.is_hidden_from_players))
                    .collect();
                
                on_approve.call(StagingApprovalResult {
                    request_id: request_id.clone(),
                    approved_npcs: approved,
                    ttl_hours: *ttl_hours.read(),
                    source: "previous".to_string(),
                });
            }
        }
    };

    let handle_regenerate_submit = {
        let request_id = props.data.request_id.clone();
        let on_regenerate = props.on_regenerate.clone();
        move |_| {
            let guidance = regenerate_guidance.read().clone();
            if !guidance.is_empty() {
                on_regenerate.call(StagingRegenerateRequest {
                    request_id: request_id.clone(),
                    guidance,
                });
                show_regenerate.set(false);
                regenerate_guidance.set(String::new());
            }
        }
    };

    rsx! {
        // Overlay
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000] p-4",
            onclick: move |_| props.on_close.call(()),

            // Modal content
            div {
                class: "bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl max-w-3xl w-full max-h-[90vh] overflow-hidden border border-amber-500/30 flex flex-col",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-6 border-b border-white/10",

                    div {
                        class: "flex justify-between items-start",

                        div {
                            h2 {
                                class: "text-2xl font-bold text-amber-400 m-0 mb-2",
                                "Stage Scene: {props.data.region_name}"
                            }
                            p {
                                class: "text-gray-400 text-sm m-0",
                                "{props.data.location_name} - {crate::presentation::game_time_format::display_date(props.data.game_time)}"
                            }
                        }

                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "p-2 text-gray-400 hover:text-white transition-colors",
                            "X"
                        }
                    }

                    // Waiting PCs
                    if !props.data.waiting_pcs.is_empty() {
                        div {
                            class: "mt-4 p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg",

                            p {
                                class: "text-blue-300 text-sm font-medium m-0 mb-2",
                                "Waiting Players"
                            }
                            div {
                                class: "flex flex-wrap gap-2",
                                for pc in props.data.waiting_pcs.iter() {
                                    span {
                                        key: "{pc.pc_id}",
                                        class: "px-2 py-1 bg-blue-500/20 text-blue-200 rounded text-sm",
                                        "{pc.pc_name}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Scrollable content
                div {
                    class: "flex-1 overflow-y-auto p-6",

                    // Previous staging section
                    if let Some(ref previous) = props.data.previous_staging {
                        PreviousStagingSection {
                            previous: previous.clone(),
                            on_use: handle_use_previous.clone(),
                        }
                    }

                    // NPC Selection
                    div {
                        class: "mb-6",

                        h3 {
                            class: "text-lg font-semibold text-white mb-4",
                            "NPCs in Scene"
                        }

                        if selections.read().is_empty() {
                            p {
                                class: "text-gray-500 italic text-center py-4",
                                "No NPCs suggested for this region"
                            }
                        } else {
                            div {
                                class: "space-y-2",
                                for (idx, npc) in selections.read().iter().enumerate() {
                                    NpcSelectionRow {
                                        key: "{npc.character_id}",
                                        npc: npc.clone(),
                                        on_toggle_present: move |_| {
                                            let mut current = selections.read().clone();
                                            if idx < current.len() {
                                                current[idx].is_present = !current[idx].is_present;
                                                if !current[idx].is_present {
                                                    current[idx].is_hidden_from_players = false;
                                                }
                                                source_type.set("custom".to_string());
                                            }
                                            selections.set(current);
                                        },
                                        on_toggle_hidden: move |_| {
                                            let mut current = selections.read().clone();
                                            if idx < current.len() {
                                                if current[idx].is_present {
                                                    current[idx].is_hidden_from_players = !current[idx].is_hidden_from_players;
                                                    source_type.set("custom".to_string());
                                                }
                                            }
                                            selections.set(current);
                                        },
                                    }
                                }
                            }
                        }
                    }

                    // Regenerate section
                    if *show_regenerate.read() {
                        div {
                            class: "mb-6 p-4 bg-purple-500/10 border border-purple-500/30 rounded-lg",

                            h4 {
                                class: "text-purple-300 font-medium mb-3",
                                "Regenerate LLM Suggestions"
                            }

                            textarea {
                                class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white text-sm min-h-[80px] resize-y mb-3",
                                placeholder: "Provide guidance for regeneration (e.g., 'Include more antagonistic NPCs' or 'Make it feel more crowded')",
                                value: "{regenerate_guidance}",
                                oninput: move |e| regenerate_guidance.set(e.value()),
                            }

                            div {
                                class: "flex gap-2 justify-end",

                                button {
                                    onclick: move |_| show_regenerate.set(false),
                                    class: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-500 transition-colors",
                                    "Cancel"
                                }

                                button {
                                    onclick: handle_regenerate_submit,
                                    disabled: regenerate_guidance.read().is_empty(),
                                    class: "px-4 py-2 bg-purple-500 text-white rounded-lg hover:bg-purple-400 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                                    "Regenerate"
                                }
                            }
                        }
                    }

                    // TTL Selection
                    div {
                        class: "mb-4",

                        label {
                            class: "block text-gray-400 text-sm mb-2",
                            "Staging Duration (hours)"
                        }
                        select {
                            class: "w-full p-3 bg-dark-bg border border-gray-700 rounded-lg text-white",
                            value: "{ttl_hours}",
                            onchange: move |e| {
                                if let Ok(hours) = e.value().parse::<i32>() {
                                    ttl_hours.set(hours);
                                }
                            },

                            option { value: "1", "1 hour" }
                            option { value: "2", "2 hours" }
                            option { value: "4", "4 hours" }
                            option { value: "8", "8 hours" }
                            option { value: "12", "12 hours" }
                            option { value: "24", "24 hours (1 day)" }
                            option { value: "48", "48 hours (2 days)" }
                            option { value: "168", "1 week" }
                        }
                    }
                }

                // Footer with action buttons
                div {
                    class: "p-6 border-t border-white/10 flex justify-between items-center",

                    // Left side: Regenerate button
                    if !props.data.llm_based_npcs.is_empty() || props.data.rule_based_npcs.is_empty() {
                        {
                            let is_showing = *show_regenerate.read();
                            rsx! {
                                button {
                                    onclick: move |_| show_regenerate.set(!is_showing),
                                    class: "px-4 py-2 bg-purple-500/20 text-purple-300 border border-purple-500/50 rounded-lg hover:bg-purple-500/30 transition-colors",
                                    if is_showing { "Hide Regenerate" } else { "Regenerate Suggestions" }
                                }
                            }
                        }
                    } else {
                        div {} // Placeholder for flex spacing
                    }

                    // Right side: Approve button
                    button {
                        onclick: handle_approve,
                        class: "px-6 py-3 bg-gradient-to-br from-green-500 to-green-600 text-white font-semibold rounded-lg hover:from-green-400 hover:to-green-500 transition-all",
                        "Approve & Continue"
                    }
                }
            }
        }
    }
}

/// Section showing previous staging for quick reuse
#[component]
fn PreviousStagingSection(
    previous: PreviousStagingData,
    on_use: EventHandler<()>,
) -> Element {
    let present_count = previous.npcs.iter().filter(|n| n.is_present).count();
    
    rsx! {
        div {
            class: "mb-6 p-4 bg-amber-500/10 border border-amber-500/30 rounded-lg",

            div {
                class: "flex justify-between items-start mb-3",

                div {
                    h4 {
                        class: "text-amber-300 font-medium m-0",
                        "Previous Staging Available"
                    }
                    p {
                        class: "text-gray-500 text-xs m-0 mt-1",
                        "Approved {previous.approved_at} - {present_count} NPCs present"
                    }
                }

                button {
                    onclick: move |_| on_use.call(()),
                    class: "px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-400 transition-colors text-sm font-medium",
                    "Use Previous"
                }
            }

            // Show NPCs that were present
            if present_count > 0 {
                div {
                    class: "flex flex-wrap gap-2",
                    for npc in previous.npcs.iter().filter(|n| n.is_present) {
                        span {
                            key: "{npc.character_id}",
                            class: "px-2 py-1 bg-amber-500/20 text-amber-200 rounded text-xs",
                            "{npc.name}"
                        }
                    }
                }
            }
        }
    }
}

/// Individual NPC selection row with toggle
#[component]
fn NpcSelectionRow(
    npc: NpcSelection,
    on_toggle_present: EventHandler<()>,
    on_toggle_hidden: EventHandler<()>,
) -> Element {
    let source_badge = match npc.source.as_str() {
        "rule" => ("bg-blue-500/20 text-blue-300", "Rule"),
        "llm" => ("bg-purple-500/20 text-purple-300", "LLM"),
        _ => ("bg-gray-500/20 text-gray-300", "Custom"),
    };

    rsx! {
        label {
            class: "flex items-center gap-4 p-3 bg-black/20 rounded-lg cursor-pointer hover:bg-black/30 transition-colors",

            // Present checkbox
            input {
                r#type: "checkbox",
                checked: npc.is_present,
                onchange: move |_| on_toggle_present.call(()),
                class: "w-5 h-5 rounded border-gray-600 bg-dark-bg text-amber-500 focus:ring-amber-500",
            }

            // Hidden checkbox
            label {
                class: "flex items-center gap-2 text-xs text-gray-400 select-none",
                input {
                    r#type: "checkbox",
                    checked: npc.is_hidden_from_players,
                    disabled: !npc.is_present,
                    onchange: move |_| on_toggle_hidden.call(()),
                    class: "w-4 h-4 rounded border-gray-600 bg-dark-bg text-purple-500 focus:ring-purple-500 disabled:opacity-50",
                }
                "Hidden"
            }

            // Portrait/Avatar
            div {
                class: "w-10 h-10 rounded-full bg-gray-700 flex-shrink-0 overflow-hidden",
                if let Some(ref portrait) = npc.portrait_asset {
                    img {
                        src: "{portrait}",
                        alt: "{npc.name}",
                        class: "w-full h-full object-cover",
                    }
                } else {
                    div {
                        class: "w-full h-full flex items-center justify-center text-gray-500 text-lg",
                        "{npc.name.chars().next().unwrap_or('?')}"
                    }
                }
            }

            // Info
            div {
                class: "flex-1 min-w-0",

                div {
                    class: "flex items-center gap-2",

                    span {
                        class: "text-white font-medium truncate",
                        "{npc.name}"
                    }

                    span {
                        class: "px-2 py-0.5 rounded text-xs {source_badge.0}",
                        "{source_badge.1}"
                    }
                }

                if !npc.reasoning.is_empty() {
                    p {
                        class: "text-gray-500 text-xs m-0 mt-1 truncate",
                        "{npc.reasoning}"
                    }
                }
            }

            // Status indicator
            div {
                class: if npc.is_present { "text-green-400 text-sm" } else { "text-gray-600 text-sm" },
                if npc.is_present { "Present" } else { "Away" }
            }
        }
    }
}
