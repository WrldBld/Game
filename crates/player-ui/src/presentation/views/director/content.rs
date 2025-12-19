//! Director mode content - Directing gameplay

use dioxus::prelude::*;

use wrldbldr_player_app::application::dto::{ChallengeData, SkillData};
use wrldbldr_player_ports::outbound::{ApprovalDecision, ApprovedNpcInfo, Platform};
use wrldbldr_player_app::application::services::SessionCommandService;
use crate::presentation::components::dm_panel::challenge_library::ChallengeLibrary;
use crate::presentation::components::dm_panel::decision_queue::DecisionQueuePanel;
use crate::presentation::components::dm_panel::staging_approval::{
    StagingApprovalPopup, StagingApprovalResult, StagingRegenerateRequest,
};
use crate::presentation::components::dm_panel::trigger_challenge_modal::TriggerChallengeModal;
use crate::presentation::components::dm_panel::log_entry::DynamicLogEntry;
use crate::presentation::services::{use_challenge_service, use_skill_service};
use crate::presentation::state::{use_game_state, use_session_state, use_generation_state, PendingApproval};

/// The original Director mode content (directing gameplay)
#[component]
pub fn DirectorModeContent() -> Element {
    let session_state = use_session_state();
    let game_state = use_game_state();
    let skill_service = use_skill_service();
    let challenge_service = use_challenge_service();
    let generation_state = use_generation_state();
    let mut show_queue_panel = use_signal(|| false);

    // Local state for directorial inputs
    let mut scene_notes = use_signal(|| String::new());
    let mut current_tone = use_signal(|| "Serious".to_string());
    let mut show_challenge_library = use_signal(|| false);
    let mut show_trigger_challenge = use_signal(|| false);
    let mut show_pc_management = use_signal(|| false);
    let mut show_location_navigator = use_signal(|| false);
    let mut show_character_perspective = use_signal(|| false);
    let mut skills: Signal<Vec<SkillData>> = use_signal(Vec::new);
    let mut challenges: Signal<Vec<ChallengeData>> = use_signal(Vec::new);

    // Load skills and challenges when world is available
    let world_id_for_skills = game_state.world.read().as_ref().map(|w| w.world.id.clone());
    let world_id_for_challenges = game_state.world.read().as_ref().map(|w| w.world.id.clone());
    use_effect(move || {
        if let Some(world_id) = world_id_for_skills.clone() {
            let svc = skill_service.clone();
            spawn(async move {
                if let Ok(skill_list) = svc.list_skills(&world_id).await {
                    // Convert service types to DTO types via JSON
                    if let Ok(json) = serde_json::to_value(&skill_list) {
                        if let Ok(dto_skills) = serde_json::from_value::<Vec<SkillData>>(json) {
                            skills.set(dto_skills);
                        }
                    }
                }
            });
        }
    });
    use_effect(move || {
        if let Some(world_id) = world_id_for_challenges.clone() {
            let svc = challenge_service.clone();
            spawn(async move {
                if let Ok(challenge_list) = svc.list_challenges(&world_id).await {
                    // Convert service types to DTO types via JSON
                    if let Ok(json) = serde_json::to_value(&challenge_list) {
                        if let Ok(dto_challenges) = serde_json::from_value::<Vec<ChallengeData>>(json) {
                            challenges.set(dto_challenges);
                        }
                    }
                }
            });
        }
    });

    // Get pending approvals from state
    let pending_approvals = session_state.pending_approvals().read().clone();
    let conversation_log = session_state.conversation_log().read().clone();

    // Get scene characters from game state
    let scene_characters = game_state.scene_characters.read().clone();

    rsx! {
        div {
            class: "h-full grid grid-cols-[1fr_350px] gap-4 p-4",

            // Left panel - Scene preview and conversation
            div {
                class: "main-panel flex flex-col gap-4",

                // Scene preview (smaller version of what players see)
                div {
                    class: "scene-preview h-[200px] bg-gradient-to-b from-dark-surface to-dark-purple-end rounded-lg relative overflow-hidden",

                    // Show actual characters in scene
                    div {
                        class: "absolute bottom-[20%] left-1/2 -translate-x-1/2 flex gap-8",
                        for character in scene_characters.iter() {
                            div {
                                key: "{character.id}",
                                class: "flex flex-col items-center",
                                div {
                                    class: "w-20 h-[120px] bg-blue-500/20 rounded flex items-center justify-center",
                                    if character.sprite_asset.is_some() {
                                        // Would show actual sprite here
                                        span { class: "text-blue-400 text-4xl", "🧑" }
                                    } else {
                                        span { class: "text-blue-400 text-4xl", "🧑" }
                                    }
                                }
                                span { class: "text-gray-400 text-xs mt-1", "{character.name}" }
                            }
                        }
                        if scene_characters.is_empty() {
                            div { class: "text-gray-500 italic", "No characters in scene" }
                        }
                    }
                }

                // Conversation log
                div {
                    class: "conversation-log flex-1 bg-dark-surface rounded-lg p-4 overflow-y-auto",

                    h3 { class: "text-gray-400 mb-4 text-sm uppercase", "Conversation Log" }

                    div {
                        class: "flex flex-col gap-3",

                        if conversation_log.is_empty() {
                            div { class: "text-gray-500 italic text-center p-8",
                                "Waiting for session activity..."
                            }
                        }

                        for (idx, entry) in conversation_log.iter().enumerate() {
                            DynamicLogEntry {
                                key: "{idx}",
                                speaker: entry.speaker.clone(),
                                text: entry.text.clone(),
                                is_system: entry.is_system,
                            }
                        }
                    }
                }

                // Approval popup(s)
                for approval in pending_approvals.iter() {
                    ApprovalPopup {
                        key: "{approval.request_id}",
                        approval: approval.clone(),
                    }
                }

                if pending_approvals.is_empty() && !conversation_log.is_empty() {
                    div {
                        class: "bg-gray-800 border border-gray-700 rounded-lg p-4 text-center text-gray-400",
                        "No pending approvals"
                    }
                }
            }

            // Right panel - Directorial controls
            div {
                class: "control-panel flex flex-col gap-4 overflow-y-auto",

                // Connection status
                div {
                    class: "panel-section bg-dark-surface rounded-lg p-4",

                    h3 { class: "text-gray-400 mb-3 text-sm uppercase", "Session Info" }

                    div { class: "text-white text-sm",
                        if let Some(session_id) = session_state.session_id().read().as_ref() {
                            p { class: "my-1", "Session: {session_id}" }
                        } else {
                            p { class: "my-1 text-amber-500", "Not connected to session" }
                        }
                    }
                }

                // Decision queue (pending approvals + recent decisions)
                div {
                    class: "panel-section bg-dark-surface rounded-lg p-4",

                    DecisionQueuePanel {}
                }

                // Scene notes
                div {
                    class: "panel-section bg-dark-surface rounded-lg p-4",

                    h3 { class: "text-gray-400 mb-3 text-sm uppercase", "Scene Notes" }
                    textarea {
                        value: "{scene_notes}",
                        oninput: move |e| scene_notes.set(e.value()),
                        placeholder: "Add notes for the current scene...",
                        class: "w-full h-[100px] p-3 bg-dark-bg border border-gray-700 rounded-lg text-white resize-y box-border",
                    }
                }

                // Tone selection
                div {
                    class: "panel-section bg-dark-surface rounded-lg p-4",

                    h3 { class: "text-gray-400 mb-3 text-sm uppercase", "Tone" }
                    select {
                        value: "{current_tone}",
                        onchange: move |e| current_tone.set(e.value()),
                        class: "w-full p-2 bg-dark-bg border border-gray-700 rounded-lg text-white",
                        option { value: "Serious", "Serious" }
                        option { value: "Lighthearted", "Lighthearted" }
                        option { value: "Tense", "Tense" }
                        option { value: "Mysterious", "Mysterious" }
                        option { value: "Comedic", "Comedic" }
                    }
                }

                // Scene NPCs (from real data)
                div {
                    class: "panel-section bg-dark-surface rounded-lg p-4",

                    h3 { class: "text-gray-400 mb-3 text-sm uppercase", "Scene Characters" }

                    div { class: "flex flex-col gap-2",
                        if scene_characters.is_empty() {
                            div { class: "text-gray-500 italic", "No characters loaded" }
                        }
                        for character in scene_characters.iter() {
                            div {
                                key: "{character.id}",
                                class: "flex items-center gap-2 p-2 bg-dark-bg rounded",
                                span { class: "text-blue-400", "🧑" }
                                span { class: "text-white", "{character.name}" }
                                if character.is_speaking {
                                    span { class: "text-green-400 text-xs ml-auto", "(speaking)" }
                                }
                            }
                        }
                    }
                }

                // Quick actions
                div {
                    class: "panel-section bg-dark-surface rounded-lg p-4",

                    h3 { class: "text-gray-400 mb-3 text-sm uppercase", "Quick Actions" }

                    div { class: "flex flex-col gap-2",
                        button {
                            onclick: move |_| show_challenge_library.set(true),
                            class: "p-2 bg-amber-500 text-white border-none rounded-lg cursor-pointer",
                            "Manage Challenges"
                        }
                        button {
                            onclick: move |_| show_trigger_challenge.set(true),
                            class: "p-2 bg-pink-500 text-white border-none rounded-lg cursor-pointer",
                            "⚔️ Trigger Challenge"
                        }
                        button { class: "p-2 bg-blue-500 text-white border-none rounded-lg cursor-pointer", "View Social Graph" }
                        button { class: "p-2 bg-purple-500 text-white border-none rounded-lg cursor-pointer", "View Timeline" }
                        button { class: "p-2 bg-red-500 text-white border-none rounded-lg cursor-pointer", "Start Combat" }
                    }
                }
            }

            // Challenge Library Modal
            if *show_challenge_library.read() {
                {
                    let world_id = game_state.world.read().as_ref().map(|w| w.world.id.clone());
                    if let Some(world_id) = world_id {
                        rsx! {
                            ChallengeLibrary {
                                world_id: world_id,
                                skills: skills.read().clone(),
                                on_close: move |_| show_challenge_library.set(false),
                                on_trigger_challenge: None,
                            }
                        }
                    } else {
                        rsx! {
                            div {
                                class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
                                onclick: move |_| show_challenge_library.set(false),
                                div {
                                    class: "bg-dark-surface p-8 rounded-lg text-center",
                                    onclick: move |e| e.stop_propagation(),
                                    p { class: "text-red-500", "No world loaded. Start a session first." }
                                    button {
                                        onclick: move |_| show_challenge_library.set(false),
                                        class: "mt-4 px-4 py-2 bg-gray-700 text-white border-none rounded cursor-pointer",
                                        "Close"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // PC Management Modal
            if *show_pc_management.read() {
                if let Some(session_id) = session_state.session_id().read().as_ref() {
                    div {
                        class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
                        onclick: move |_| show_pc_management.set(false),
                        div {
                            class: "bg-dark-surface rounded-lg w-[90%] max-w-[800px] max-h-[90vh] overflow-y-auto p-6",
                            onclick: move |e| e.stop_propagation(),
                            div {
                                class: "flex justify-between items-center mb-4",
                                h2 {
                                    class: "m-0 text-white text-xl",
                                    "Player Character Management"
                                }
                                button {
                                    onclick: move |_| show_pc_management.set(false),
                                    class: "px-2 py-1 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                                    "×"
                                }
                            }
                            crate::presentation::components::dm_panel::pc_management::PCManagementPanel {
                                session_id: session_id.clone(),
                                on_view_as_character: move |character_id| {
                                    // TODO (Phase 23 Player Perspective): Implement view-as-character mode
                                    tracing::info!("View as character: {}", character_id);
                                    show_pc_management.set(false);
                                },
                            }
                        }
                    }
                }
            }

            // Director Queue Panel
            if *show_queue_panel.read() {
                crate::presentation::components::dm_panel::director_queue_panel::DirectorQueuePanel {
                    on_close: move |_| show_queue_panel.set(false),
                }
            }

            // Location Navigator Modal
            if *show_location_navigator.read() {
                if let Some(world_id) = game_state.world.read().as_ref().map(|w| w.world.id.clone()) {
                    div {
                        class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
                        onclick: move |_| show_location_navigator.set(false),
                        div {
                            class: "bg-dark-surface rounded-lg w-[90%] max-w-[800px] max-h-[90vh] overflow-y-auto p-6",
                            onclick: move |e| e.stop_propagation(),
                            div {
                                class: "flex justify-between items-center mb-4",
                                h2 {
                                    class: "m-0 text-white text-xl",
                                    "Location Navigator"
                                }
                                button {
                                    onclick: move |_| show_location_navigator.set(false),
                                    class: "px-2 py-1 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                                    "×"
                                }
                            }
                            crate::presentation::components::dm_panel::location_navigator::LocationNavigator {
                                world_id: world_id.clone(),
                                on_preview: move |location_id| {
                                    // TODO (Phase 23 Location Preview): Open location details panel/modal
                                    tracing::info!("Preview location: {}", location_id);
                                    show_location_navigator.set(false);
                                },
                            }
                        }
                    }
                }
            }

            // Character Perspective Viewer Modal
            if *show_character_perspective.read() {
                if let (Some(session_id), Some(world_id)) = (
                    session_state.session_id().read().as_ref().map(|s| s.clone()),
                    game_state.world.read().as_ref().map(|w| w.world.id.clone())
                ) {
                    div {
                        class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
                        onclick: move |_| show_character_perspective.set(false),
                        div {
                            class: "bg-dark-surface rounded-lg w-[90%] max-w-[800px] max-h-[90vh] overflow-y-auto p-6",
                            onclick: move |e| e.stop_propagation(),
                            div {
                                class: "flex justify-between items-center mb-4",
                                h2 {
                                    class: "m-0 text-white text-xl",
                                    "Character Perspective Viewer"
                                }
                                button {
                                    onclick: move |_| show_character_perspective.set(false),
                                    class: "px-2 py-1 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                                    "×"
                                }
                            }
                            crate::presentation::components::dm_panel::character_perspective::CharacterPerspectiveViewer {
                                session_id: session_id.clone(),
                                world_id: world_id.clone(),
                                on_view_as: move |character_id| {
                                    // TODO (Phase 23 Player Perspective): Implement view-as-character mode
                                    tracing::info!("View as character: {}", character_id);
                                    show_character_perspective.set(false);
                                },
                            }
                        }
                    }
                }
            }

            // Trigger Challenge Modal
            if *show_trigger_challenge.read() {
                {
                    let active_challenges: Vec<ChallengeData> = challenges.read().iter()
                        .filter(|c| c.active)
                        .cloned()
                        .collect();
                    let chars = scene_characters.clone();

                    if active_challenges.is_empty() {
                        rsx! {
                            div {
                                class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
                                onclick: move |_| show_trigger_challenge.set(false),
                                div {
                                    class: "bg-dark-surface p-8 rounded-lg text-center max-w-[400px]",
                                    onclick: move |e| e.stop_propagation(),
                                    h3 { class: "text-amber-500 mb-4", "⚔️ No Active Challenges" }
                                    p { class: "text-gray-400 mb-4", "Create and activate challenges in the Challenge Library first." }
                                    button {
                                        onclick: move |_| {
                                            show_trigger_challenge.set(false);
                                            show_challenge_library.set(true);
                                        },
                                        class: "px-4 py-2 bg-amber-500 text-white border-none rounded cursor-pointer mr-2",
                                        "Open Challenge Library"
                                    }
                                    button {
                                        onclick: move |_| show_trigger_challenge.set(false),
                                        class: "px-4 py-2 bg-gray-700 text-white border-none rounded cursor-pointer",
                                        "Close"
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            TriggerChallengeModal {
                                challenges: active_challenges,
                                scene_characters: chars,
                                on_trigger: move |(challenge_id, character_id): (String, String)| {
                                    tracing::info!("Triggering challenge {} for character {}", challenge_id, character_id);
                                    if let Some(client) = session_state.engine_client().read().as_ref() {
                                        let svc = SessionCommandService::new(std::sync::Arc::clone(client));
                                        if let Err(e) = svc.trigger_challenge(&challenge_id, &character_id) {
                                            tracing::error!("Failed to trigger challenge: {}", e);
                                        }
                                    } else {
                                        tracing::warn!("No engine client available to trigger challenge");
                                    }
                                    show_trigger_challenge.set(false);
                                },
                                on_close: move |_| show_trigger_challenge.set(false),
                            }
                        }
                    }
                }
            }

            // Staging Approval Popup - shown when a PC is trying to enter a region
            if let Some(staging_data) = game_state.pending_staging_approval.read().as_ref() {
                {
                    // Clone session_state for the closures
                    let session_state_for_approve = use_session_state();
                    let session_state_for_regenerate = use_session_state();
                    let mut game_state_for_approve = game_state.clone();
                    let mut game_state_for_close = game_state.clone();
                    
                    rsx! {
                        StagingApprovalPopup {
                            data: staging_data.clone(),
                            on_approve: move |result: StagingApprovalResult| {
                                tracing::info!("Staging approved for request {}: {} NPCs, ttl={} hours",
                                    result.request_id, result.approved_npcs.len(), result.ttl_hours);
                                
                                // Send approval to engine
                                if let Some(client) = session_state_for_approve.engine_client().read().as_ref() {
                                    let approved_npcs: Vec<ApprovedNpcInfo> = result.approved_npcs
                                        .into_iter()
                                        .map(|(character_id, is_present)| ApprovedNpcInfo {
                                            character_id,
                                            is_present,
                                            reasoning: None,
                                        })
                                        .collect();
                                    
                                    if let Err(e) = client.send_staging_approval(
                                        &result.request_id,
                                        approved_npcs,
                                        result.ttl_hours,
                                        &result.source,
                                    ) {
                                        tracing::error!("Failed to send staging approval: {}", e);
                                    }
                                }
                                
                                // Clear the pending approval
                                game_state_for_approve.clear_pending_staging_approval();
                            },
                            on_regenerate: move |request: StagingRegenerateRequest| {
                                tracing::info!("Regenerating staging for request {}: {}",
                                    request.request_id, request.guidance);
                                
                                // Send regenerate request to engine
                                if let Some(client) = session_state_for_regenerate.engine_client().read().as_ref() {
                                    if let Err(e) = client.request_staging_regenerate(
                                        &request.request_id,
                                        &request.guidance,
                                    ) {
                                        tracing::error!("Failed to send staging regenerate request: {}", e);
                                    }
                                }
                            },
                            on_close: move |_| {
                                // Clear without sending approval (dismiss)
                                game_state_for_close.clear_pending_staging_approval();
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Approval popup for DM to approve/reject LLM responses
#[derive(Props, Clone, PartialEq)]
struct ApprovalPopupProps {
    approval: PendingApproval,
}

#[component]
fn ApprovalPopup(props: ApprovalPopupProps) -> Element {
    let session_state = use_session_state();
    let platform = use_context::<Platform>();
    let mut modified_dialogue = use_signal(|| props.approval.proposed_dialogue.clone());
    let mut show_reasoning = use_signal(|| false);
    let mut rejection_feedback = use_signal(|| String::new());
    let mut show_reject_input = use_signal(|| false);

    // Track which tools are approved
    let mut approved_tools = use_signal(|| {
        props.approval.proposed_tools.iter().map(|t| (t.id.clone(), true)).collect::<std::collections::HashMap<_, _>>()
    });

    let request_id = props.approval.request_id.clone();
    let npc_name = props.approval.npc_name.clone();

    rsx! {
        div {
            class: "approval-popup bg-gray-800 border-2 border-amber-500 rounded-xl p-5 mb-4",

            h4 { class: "text-amber-500 mb-4 flex justify-between items-center",
                span { "Approval Required" }
                span { class: "text-xs text-gray-400 font-normal", "{props.approval.request_id}" }
            }

            div { class: "mb-4",
                p { class: "text-gray-400 text-sm mb-1", "{npc_name} will say:" }
                textarea {
                    value: "{modified_dialogue}",
                    oninput: move |e| modified_dialogue.set(e.value()),
                    class: "w-full min-h-[80px] p-3 bg-dark-bg border border-gray-700 rounded-lg text-white resize-y box-border italic",
                }
            }

            // Show/hide reasoning
            {
                let current_showing = *show_reasoning.read();
                rsx! {
                    button {
                        onclick: move |_| show_reasoning.set(!current_showing),
                        class: "bg-none border-none text-blue-400 cursor-pointer text-sm mb-2",
                        if current_showing { "Hide reasoning ▲" } else { "Show reasoning ▼" }
                    }
                }
            }

            if *show_reasoning.read() {
                div { class: "mb-4 p-3 bg-black/30 rounded-lg",
                    p { class: "text-gray-400 text-xs m-0", "{props.approval.internal_reasoning}" }
                }
            }

            // Proposed tools
            if !props.approval.proposed_tools.is_empty() {
                div { class: "mb-4",
                    p { class: "text-gray-400 text-sm mb-2", "Proposed Actions:" }
                    div { class: "flex flex-col gap-2",
                        for tool in props.approval.proposed_tools.iter() {
                            {
                                let tool_id = tool.id.clone();
                                let tool_id_for_change = tool.id.clone();
                                let is_approved = *approved_tools.read().get(&tool_id).unwrap_or(&true);
                                rsx! {
                                    div {
                                        key: "{tool_id}",
                                        class: "flex items-center gap-2 p-2 bg-black/20 rounded",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_approved,
                                            onchange: move |_| {
                                                let mut tools = approved_tools.write();
                                                if let Some(val) = tools.get_mut(&tool_id_for_change) {
                                                    *val = !*val;
                                                }
                                            },
                                        }
                                        div {
                                            span { class: "text-white text-sm", "{tool.name}" }
                                            span { class: "text-gray-400 text-xs ml-2", "- {tool.description}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Narrative event suggestion section
            if let Some(suggestion) = &props.approval.narrative_event_suggestion {
                div {
                    class: "mb-4 p-4 bg-purple-500/10 border border-purple-500 rounded-lg",

                    h4 {
                        class: "text-purple-500 m-0 mb-3 text-sm flex gap-2 items-center",
                        "📖 Narrative Event Suggested"
                    }

                    div {
                        class: "mb-2",
                        span {
                            class: "text-white font-bold text-sm",
                            "{suggestion.event_name}"
                        }
                    }

                    p {
                        class: "text-gray-400 text-xs m-0 mb-2",
                        "Confidence: {suggestion.confidence}"
                    }

                    p {
                        class: "text-gray-400 text-xs italic m-0 mb-3 leading-snug",
                        "\"{suggestion.reasoning}\""
                    }

                    if let Some(outcome) = &suggestion.suggested_outcome {
                        p {
                            class: "text-purple-300 text-xs m-0 mb-3 leading-snug",
                            "Suggested Outcome: {outcome}"
                        }
                    }

                    p {
                        class: "text-gray-400 text-[0.65rem] m-0",
                        "Note: Narrative event triggers are handled separately via the NarrativeEventSuggestionDecision message"
                    }
                }
            }

            // Rejection feedback input
            if *show_reject_input.read() {
                div { class: "mb-4",
                    p { class: "text-gray-400 text-sm mb-1", "Feedback for LLM:" }
                    textarea {
                        value: "{rejection_feedback}",
                        oninput: move |e| rejection_feedback.set(e.value()),
                        placeholder: "Tell the LLM what to change...",
                        class: "w-full min-h-[60px] p-2 bg-dark-bg border border-red-500 rounded-lg text-white resize-y box-border",
                    }
                    div { class: "flex gap-2 mt-2",
                        {
                            let feedback = rejection_feedback.read().clone();
                            let request_id = request_id.clone();
                            let mut session_state = session_state.clone();
                            let platform_reject = platform.clone();
                            rsx! {
                                button {
                                    onclick: move |_| {
                                        session_state.record_approval_decision(
                                            request_id.clone(),
                                            &ApprovalDecision::Reject {
                                                feedback: feedback.clone(),
                                            },
                                            &platform_reject,
                                        );
                                    },
                                    class: "flex-1 p-2 bg-red-500 text-white border-none rounded-lg cursor-pointer",
                                    "Send Rejection"
                                }
                            }
                        }
                        button {
                            onclick: move |_| show_reject_input.set(false),
                            class: "p-2 bg-gray-700 text-white border-none rounded-lg cursor-pointer",
                            "Cancel"
                        }
                    }
                }
            }

            // Action buttons
            if !*show_reject_input.read() {
                {
                    let request_id_accept = request_id.clone();
                    let mut session_state_accept = session_state.clone();
                    let request_id_modify = request_id.clone();
                    let session_state_modify = session_state.clone();
                    let platform_accept = platform.clone();
                    let platform_modify = platform.clone();
                    let dialogue = modified_dialogue.read().clone();
                    let original = props.approval.proposed_dialogue.clone();
                    let approved = approved_tools.read().clone();
                    let tools = props.approval.proposed_tools.clone();

                    rsx! {
                        div { class: "flex gap-2",
                            button {
                                onclick: move |_| {
                                    session_state_accept.record_approval_decision(
                                        request_id_accept.clone(),
                                        &ApprovalDecision::Accept,
                                        &platform_accept,
                                    );
                                },
                                class: "flex-1 p-3 bg-green-500 text-white border-none rounded-lg cursor-pointer font-semibold",
                                "Accept"
                            }
                            button {
                                onclick: {
                                    let dialogue = dialogue.clone();
                                    let original = original.clone();
                                    let approved = approved.clone();
                                    let tools = tools.clone();
                                    let request_id = request_id_modify.clone();
                                    let mut session_state = session_state_modify.clone();
                                    let platform = platform_modify.clone();
                                    move |_| {
                                        // Only send modification if something changed
                                        if dialogue != original || approved.values().any(|&v| !v) {
                                            let approved_list: Vec<String> = tools.iter()
                                                .filter(|t| *approved.get(&t.id).unwrap_or(&true))
                                                .map(|t| t.id.clone())
                                                .collect();
                                            let rejected_list: Vec<String> = tools.iter()
                                                .filter(|t| !*approved.get(&t.id).unwrap_or(&true))
                                                .map(|t| t.id.clone())
                                                .collect();
                                            session_state.record_approval_decision(
                                                request_id.clone(),
                                                &ApprovalDecision::AcceptWithModification {
                                                    modified_dialogue: dialogue.clone(),
                                                    approved_tools: approved_list,
                                                    rejected_tools: rejected_list,
                                                },
                                                &platform,
                                            );
                                        } else {
                                            session_state.record_approval_decision(
                                                request_id.clone(),
                                                &ApprovalDecision::Accept,
                                                &platform,
                                            );
                                        }
                                    }
                                },
                                class: "flex-1 p-3 bg-blue-500 text-white border-none rounded-lg cursor-pointer font-semibold",
                                "Accept Modified"
                            }
                            button {
                                onclick: move |_| show_reject_input.set(true),
                                class: "flex-1 p-3 bg-red-500 text-white border-none rounded-lg cursor-pointer font-semibold",
                                "Reject"
                            }
                        }
                    }
                }
            }
        }
    }
}
