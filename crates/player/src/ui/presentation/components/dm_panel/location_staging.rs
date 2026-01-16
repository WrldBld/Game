//! Location Staging Component
//!
//! DM tool for pre-staging regions before players arrive.
//! Shows all regions in a location with their current staging status.

use dioxus::prelude::*;

use crate::application::dto::ApprovedNpcInfo;
use crate::application::services::CharacterSummary;
use crate::infrastructure::spawn_task;
use crate::infrastructure::websocket::ClientMessageBuilder;
use crate::presentation::services::{use_character_service, use_command_bus, use_location_service};
use crate::presentation::state::game_state::RegionStagingStatus;
use crate::presentation::state::use_game_state;

/// Region staging status
#[derive(Clone, PartialEq)]
pub enum StagingStatus {
    /// No staging set - will prompt when player enters
    None,
    /// Staging is active with expiry time
    Active {
        expires_in_hours: f32,
        npc_names: Vec<String>,
    },
    /// Staging expired - can be refreshed
    Expired {
        hours_ago: f32,
        previous_npc_names: Vec<String>,
    },
}

/// Region data with staging info
#[derive(Clone, PartialEq)]
pub struct RegionStagingInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: StagingStatus,
}

/// Props for LocationStagingPanel
#[derive(Props, Clone, PartialEq)]
pub struct LocationStagingPanelProps {
    /// Location ID to show staging for
    pub location_id: String,
    /// Location name for display
    pub location_name: String,
    /// World ID for loading NPCs
    pub world_id: String,
}

/// LocationStagingPanel - Shows all regions with staging status
#[component]
pub fn LocationStagingPanel(props: LocationStagingPanelProps) -> Element {
    let location_service = use_location_service();
    let character_service = use_character_service();
    let game_state = use_game_state();

    let mut regions: Signal<Vec<RegionStagingInfo>> = use_signal(Vec::new);
    let mut characters: Signal<Vec<CharacterSummary>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Pre-staging modal state
    let mut show_prestage_modal = use_signal(|| false);
    let mut selected_region: Signal<Option<RegionStagingInfo>> = use_signal(|| None);

    // Load regions on mount
    {
        let location_id = props.location_id.clone();
        let world_id = props.world_id.clone();
        let loc_svc = location_service.clone();
        let char_svc = character_service.clone();

        use_effect(move || {
            let lid = location_id.clone();
            let wid = world_id.clone();
            let loc_service = loc_svc.clone();
            let char_service = char_svc.clone();

            loading.set(true);
            spawn_task(async move {
                // Load regions
                match loc_service.get_regions(&lid).await {
                    Ok(region_list) => {
                        // Convert to RegionStagingInfo - status will be read from game_state below
                        let region_infos: Vec<RegionStagingInfo> = region_list
                            .into_iter()
                            .map(|r| RegionStagingInfo {
                                id: r.id,
                                name: r.name,
                                description: Some(r.description),
                                status: StagingStatus::None, // Will be updated reactively
                            })
                            .collect();
                        regions.set(region_infos);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load regions: {}", e)));
                    }
                }

                // Load NPCs for the world
                match char_service.list_characters(&wid).await {
                    Ok(char_list) => {
                        // Use all characters - we'll show them all for selection
                        // In a full implementation, we'd filter by NPC vs PC
                        characters.set(char_list);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load characters: {}", e);
                    }
                }

                loading.set(false);
            });
        });
    }

    // Apply staging status from game_state reactively
    let regions_with_status: Vec<RegionStagingInfo> = regions
        .read()
        .iter()
        .map(|r| {
            let status = match game_state.get_region_staging_status(&r.id) {
                RegionStagingStatus::None => StagingStatus::None,
                RegionStagingStatus::Pending => StagingStatus::None, // Show as None while pending
                RegionStagingStatus::Active { npc_names, .. } => StagingStatus::Active {
                    expires_in_hours: 24.0, // Default, could be enhanced with actual TTL later
                    npc_names,
                },
            };
            RegionStagingInfo {
                id: r.id.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                status,
            }
        })
        .collect();

    rsx! {
        div {
            class: "location-staging-panel h-full flex flex-col",

            // Header
            div {
                class: "p-4 border-b border-white/10",

                h2 {
                    class: "text-xl font-bold text-white m-0 mb-2",
                    "Staging: {props.location_name}"
                }
                p {
                    class: "text-gray-400 text-sm m-0",
                    "Pre-stage regions before players arrive"
                }
            }

            // Error display
            if let Some(e) = error.read().as_ref() {
                div {
                    class: "m-4 p-3 bg-red-500/10 border border-red-500/50 rounded-lg text-red-400 text-sm",
                    "{e}"
                }
            }

            // Content
            div {
                class: "flex-1 overflow-y-auto p-4",

                if *loading.read() {
                    div {
                        class: "flex items-center justify-center h-32 text-gray-400",
                        "Loading regions..."
                    }
                } else if regions.read().is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center h-32 text-gray-500",
                        p { class: "mb-2", "No regions in this location" }
                        p { class: "text-sm", "Add regions in the Location Editor" }
                    }
                } else {
                    {
                        rsx! {
                            div {
                                class: "space-y-4",
                                {regions_with_status.into_iter().map(|region| {
                                    let region_for_prestage = region.clone();
                                    let region_for_clear = region.clone();
                                    rsx! {
                                        RegionStagingCard {
                                            key: "{region.id}",
                                            region: region.clone(),
                                            on_prestage: move |_| {
                                                selected_region.set(Some(region_for_prestage.clone()));
                                                show_prestage_modal.set(true);
                                            },
                                            on_clear: {
                                                let command_bus = use_command_bus();
                                                let region_id_for_clear = region_for_clear.id.clone();
                                                move |_| {
                                                    tracing::info!("Clearing staging for region {}", region_id_for_clear);
                                                    // Clear staging by pre-staging with empty NPC list
                                                    let msg = ClientMessageBuilder::pre_stage_region(&region_id_for_clear, vec![], 1);
                                                    if let Err(e) = command_bus.send(msg) {
                                                        tracing::error!("Failed to clear staging: {}", e);
                                                    }
                                                }
                                            },
                                        }
                                    }
                                })}
                            }
                        }
                    }
                }
            }

            // Pre-stage Modal
            if *show_prestage_modal.read() {
                if let Some(region) = selected_region.read().as_ref() {
                    PreStageModal {
                        region: region.clone(),
                        npcs: characters.read().clone(),
                        on_approve: {
                            let command_bus = use_command_bus();
                            let region_id = region.id.clone();
                            move |data: PreStageApprovalData| {
                                tracing::info!("Pre-staging region {} with {} NPCs, TTL {} hours",
                                    region_id, data.approved.len(), data.ttl_hours);

                                // Send pre-stage request to engine via CommandBus
                                let npcs: Vec<wrldbldr_shared::ApprovedNpcInfo> = data.approved
                                    .into_iter()
                                    .filter(|(_, is_present, _)| *is_present)
                                    .map(|(character_id, is_present, is_hidden_from_players)| {
                                        let local = ApprovedNpcInfo {
                                            character_id,
                                            is_present,
                                            reasoning: None,
                                            is_hidden_from_players,
                                            mood: None, // Use character's default_mood
                                        };
                                        local.into()
                                    })
                                    .collect();

                                let msg = ClientMessageBuilder::pre_stage_region(&region_id, npcs, data.ttl_hours);
                                if let Err(e) = command_bus.send(msg) {
                                    tracing::error!("Failed to pre-stage region: {}", e);
                                }

                                show_prestage_modal.set(false);
                                selected_region.set(None);
                            }
                        },
                        on_close: move |_| {
                            show_prestage_modal.set(false);
                            selected_region.set(None);
                        },
                    }
                }
            }
        }
    }
}

/// Card showing a region's staging status
#[derive(Props, Clone, PartialEq)]
struct RegionStagingCardProps {
    region: RegionStagingInfo,
    on_prestage: EventHandler<()>,
    on_clear: EventHandler<()>,
}

#[component]
fn RegionStagingCard(props: RegionStagingCardProps) -> Element {
    let (status_icon, status_text, status_class) = match &props.region.status {
        StagingStatus::None => ("⚠️", "No staging set".to_string(), "text-yellow-400"),
        StagingStatus::Active {
            expires_in_hours,
            npc_names,
        } => (
            "✓",
            format!(
                "Active (expires in {:.1}h) - {} NPCs",
                expires_in_hours,
                npc_names.len()
            ),
            "text-green-400",
        ),
        StagingStatus::Expired {
            hours_ago,
            previous_npc_names,
        } => (
            "⏸️",
            format!(
                "Expired {:.1}h ago - {} NPCs",
                hours_ago,
                previous_npc_names.len()
            ),
            "text-gray-400",
        ),
    };

    rsx! {
        div {
            class: "bg-dark-surface border border-white/10 rounded-lg p-4",

            // Header
            div {
                class: "flex justify-between items-start mb-3",

                div {
                    h3 {
                        class: "text-white font-medium m-0",
                        "{props.region.name}"
                    }
                    if let Some(desc) = &props.region.description {
                        p {
                            class: "text-gray-500 text-sm m-0 mt-1",
                            "{desc}"
                        }
                    }
                }

                // Status badge
                div {
                    class: "px-2 py-1 bg-black/30 rounded text-sm {status_class}",
                    "{status_icon} {status_text}"
                }
            }

            // NPCs in current staging
            match &props.region.status {
                StagingStatus::Active { npc_names, .. } if !npc_names.is_empty() => rsx! {
                    div {
                        class: "mb-3 flex flex-wrap gap-1",
                        for name in npc_names.iter() {
                            span {
                                class: "px-2 py-0.5 bg-green-500/20 text-green-300 rounded text-xs",
                                "{name}"
                            }
                        }
                    }
                },
                StagingStatus::Expired { previous_npc_names, .. } if !previous_npc_names.is_empty() => rsx! {
                    div {
                        class: "mb-3",
                        p { class: "text-gray-500 text-xs mb-1", "Previous:" }
                        div {
                            class: "flex flex-wrap gap-1",
                            for name in previous_npc_names.iter() {
                                span {
                                    class: "px-2 py-0.5 bg-gray-500/20 text-gray-400 rounded text-xs",
                                    "{name}"
                                }
                            }
                        }
                    }
                },
                _ => rsx! {}
            }

            // Action buttons
            div {
                class: "flex gap-2",

                match &props.region.status {
                    StagingStatus::None => rsx! {
                        button {
                            onclick: move |_| props.on_prestage.call(()),
                            class: "px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-400 transition-colors text-sm font-medium",
                            "Pre-Stage Now"
                        }
                    },
                    StagingStatus::Active { .. } => rsx! {
                        button {
                            onclick: move |_| props.on_prestage.call(()),
                            class: "px-4 py-2 bg-blue-500/20 text-blue-300 border border-blue-500/50 rounded-lg hover:bg-blue-500/30 transition-colors text-sm",
                            "View/Edit"
                        }
                        button {
                            onclick: move |_| props.on_clear.call(()),
                            class: "px-4 py-2 bg-red-500/20 text-red-300 border border-red-500/50 rounded-lg hover:bg-red-500/30 transition-colors text-sm",
                            "Clear"
                        }
                    },
                    StagingStatus::Expired { .. } => rsx! {
                        button {
                            onclick: move |_| props.on_prestage.call(()),
                            class: "px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-400 transition-colors text-sm font-medium",
                            "Refresh Staging"
                        }
                    },
                }
            }
        }
    }
}

/// Data for pre-stage approval
#[derive(Clone, PartialEq)]
pub struct PreStageApprovalData {
    pub approved: Vec<(String, bool, bool)>,
    pub ttl_hours: i32,
}

/// Modal for pre-staging a region
#[derive(Props, Clone, PartialEq)]
struct PreStageModalProps {
    region: RegionStagingInfo,
    npcs: Vec<CharacterSummary>,
    on_approve: EventHandler<PreStageApprovalData>,
    on_close: EventHandler<()>,
}

#[component]
fn PreStageModal(props: PreStageModalProps) -> Element {
    // Initialize NPC selections - all NPCs available, none selected by default
    let mut selections: Signal<Vec<(String, String, bool, bool)>> = use_signal(|| {
        props
            .npcs
            .iter()
            .map(|npc| (npc.id.clone(), npc.name.clone(), false, false))
            .collect()
    });
    let mut ttl_hours = use_signal(|| 4i32);

    let handle_approve = {
        let on_approve = props.on_approve;
        move |_| {
            let approved: Vec<(String, bool, bool)> = selections
                .read()
                .iter()
                .map(|(id, _, is_present, is_hidden_from_players)| {
                    (id.clone(), *is_present, *is_hidden_from_players)
                })
                .collect();
            on_approve.call(PreStageApprovalData {
                approved,
                ttl_hours: *ttl_hours.read(),
            });
        }
    };

    rsx! {
        // Overlay
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000] p-4",
            onclick: move |_| props.on_close.call(()),

            // Modal
            div {
                class: "bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl max-w-lg w-full max-h-[80vh] overflow-hidden border border-amber-500/30 flex flex-col",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-6 border-b border-white/10",

                    div {
                        class: "flex justify-between items-start",

                        div {
                            h2 {
                                class: "text-xl font-bold text-amber-400 m-0 mb-2",
                                "Pre-Stage: {props.region.name}"
                            }
                            p {
                                class: "text-gray-400 text-sm m-0",
                                "Set up NPCs before players arrive"
                            }
                        }

                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "p-2 text-gray-400 hover:text-white transition-colors",
                            "X"
                        }
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-6",

                    // NPC Selection
                    div {
                        class: "mb-6",

                        h3 {
                            class: "text-white font-medium mb-3",
                            "Select NPCs to be Present"
                        }

                        if props.npcs.is_empty() {
                            p {
                                class: "text-gray-500 italic text-center py-4",
                                "No NPCs available in this world"
                            }
                        } else {
                            div {
                                class: "space-y-2 max-h-[300px] overflow-y-auto",
                                for (idx, (id, name, is_present, is_hidden_from_players)) in selections.read().iter().enumerate() {
                                    {
                                        let npc_id = id.clone();
                                        let npc_name = name.clone();
                                        let checked_present = *is_present;
                                        let checked_hidden = *is_hidden_from_players;
                                        rsx! {
                                            div {
                                                key: "{npc_id}",
                                                class: "flex items-center gap-3 p-3 bg-black/20 rounded-lg hover:bg-black/30 transition-colors",

                                                label {
                                                    class: "flex items-center gap-3 cursor-pointer",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: checked_present,
                                                        onchange: move |_| {
                                                            let mut current = selections.read().clone();
                                                            if idx < current.len() {
                                                                current[idx].2 = !current[idx].2;
                                                                if !current[idx].2 {
                                                                    current[idx].3 = false;
                                                                }
                                                            }
                                                            selections.set(current);
                                                        },
                                                        class: "w-5 h-5 rounded border-gray-600 bg-dark-bg text-amber-500",
                                                    }

                                                    span {
                                                        class: "text-white",
                                                        "{npc_name}"
                                                    }
                                                }

                                                label {
                                                    class: "ml-auto flex items-center gap-2 text-xs text-gray-400 select-none",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: checked_hidden,
                                                        disabled: !checked_present,
                                                        onchange: move |_| {
                                                            let mut current = selections.read().clone();
                                                            if idx < current.len()
                                                                && current[idx].2 {
                                                                    current[idx].3 = !current[idx].3;
                                                                }
                                                            selections.set(current);
                                                        },
                                                        class: "w-4 h-4 rounded border-gray-600 bg-dark-bg text-purple-500 disabled:opacity-50",
                                                    }
                                                    "Hidden"
                                                }
                                            }
                                        }
                                    }
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

                // Footer
                div {
                    class: "p-6 border-t border-white/10 flex justify-end gap-3",

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-500 transition-colors",
                        "Cancel"
                    }

                    button {
                        onclick: handle_approve,
                        class: "px-6 py-2 bg-gradient-to-br from-amber-500 to-amber-600 text-white font-semibold rounded-lg hover:from-amber-400 hover:to-amber-500 transition-all",
                        "Pre-Stage Region"
                    }
                }
            }
        }
    }
}
