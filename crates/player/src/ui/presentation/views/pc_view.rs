//! Player Character View - Visual novel style gameplay
//!
//! Main view for players, displaying the visual novel interface
//! with backdrop, character sprites, dialogue, and choices.

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::application::dto::CharacterSheetSchema;
use crate::application::dto::InventoryItemData;
use crate::application::dto::{DiceInput, InteractionData, PlayerAction};
use crate::infrastructure::messaging::CommandBus;
use crate::infrastructure::spawn_task;
use crate::infrastructure::websocket::ClientMessageBuilder;
use crate::presentation::components::action_panel::ActionPanel;
use crate::presentation::components::character_sheet_viewer::CharacterSheetViewer;
use crate::presentation::components::common::TimeAdvanceToast;
use crate::presentation::components::event_overlays::{
    ApproachEventOverlay, EndConversationConfirmation, LocationEventBanner,
};
use crate::presentation::components::inventory_panel::InventoryPanel;
use crate::presentation::components::known_npcs_panel::{KnownNpcsPanel, NpcObservationData};
use crate::presentation::components::mini_map::{MapBounds, MapRegionData, MiniMap};
use crate::presentation::components::navigation_panel::NavigationPanel;
use crate::presentation::components::region_items_panel::RegionItemsPanel;
use crate::presentation::components::tactical::{
    ChallengeRollModal, PlayerSkillData, SkillsDisplay,
};
use crate::presentation::components::visual_novel::{
    Backdrop, CharacterLayer, DialogueBox, EmptyDialogueBox,
};
use crate::presentation::services::{
    use_character_service, use_command_bus, use_location_service, use_observation_service,
    use_skill_service, use_world_service,
};
use crate::presentation::state::{
    use_dialogue_state, use_game_state, use_session_state, use_typewriter_effect,
    RollSubmissionStatus,
};
use wrldbldr_shared::character_sheet::SheetValue;

/// Animation timing for backdrop transitions.
/// Must match the transition duration defined in tailwind.config.js.
const BACKDROP_TRANSITION_MS: u64 = 500;

/// Player Character View - visual novel gameplay interface
///
/// Connection handling and back navigation are provided by WorldSessionLayout wrapper.
#[component]
pub fn PCView() -> Element {
    // Get global state from context
    let game_state = use_game_state();
    let mut dialogue_state = use_dialogue_state();
    let session_state = use_session_state();
    let navigator = use_navigator();

    // Get command bus for sending messages
    let command_bus = use_command_bus();

    // Get services
    let world_service = use_world_service();
    let character_service = use_character_service();
    let observation_service = use_observation_service();
    let location_service = use_location_service();
    let skill_service = use_skill_service();

    // Character sheet viewer state
    let mut show_character_sheet = use_signal(|| false);
    let mut character_sheet_schema: Signal<Option<CharacterSheetSchema>> = use_signal(|| None);
    let mut character_sheet_values: Signal<HashMap<String, SheetValue>> = use_signal(HashMap::new);
    let mut player_character_name = use_signal(|| "Your Character".to_string());
    let mut selected_character_id: Signal<Option<String>> = use_signal(|| None);
    let mut is_loading_sheet = use_signal(|| false);

    // Navigation panel state
    let mut show_navigation_panel = use_signal(|| false);

    // Inventory panel state
    let mut show_inventory_panel = use_signal(|| false);
    let mut inventory_items: Signal<Vec<InventoryItemData>> = use_signal(Vec::new);
    let mut is_loading_inventory = use_signal(|| false);

    // Known NPCs panel state
    let mut show_known_npcs_panel = use_signal(|| false);
    let mut known_npcs: Signal<Vec<NpcObservationData>> = use_signal(Vec::new);
    let mut is_loading_npcs = use_signal(|| false);

    // Mini-map state
    let mut show_mini_map = use_signal(|| false);

    // Backdrop transition effect - auto-clear after animation completes
    let game_state_for_effect = game_state.clone();
    let transitioning = *game_state_for_effect.backdrop_transitioning.read();
    let platform = crate::use_platform();
    use_effect(move || {
        if transitioning {
            let mut gs = game_state_for_effect.clone();
            let sleep_future = platform.sleep_ms(BACKDROP_TRANSITION_MS);
            spawn_task(async move {
                // Wait for animation to complete
                sleep_future.await;
                gs.clear_backdrop_transition();
            });
        }
    });
    let mut map_regions: Signal<Vec<MapRegionData>> = use_signal(Vec::new);
    let mut is_loading_map = use_signal(|| false);

    // Skills panel state
    let mut show_skills_panel = use_signal(|| false);
    let mut player_skills: Signal<Vec<PlayerSkillData>> = use_signal(Vec::new);
    let mut is_loading_skills = use_signal(|| false);

    // Region items panel state (items visible in current region)
    let mut show_region_items_panel = use_signal(|| false);

    // End conversation confirmation modal state
    let show_end_conversation_modal = use_signal(|| false);

    // Error feedback state for user actions
    let mut action_error: Signal<Option<String>> = use_signal(|| None);

    // Run typewriter effect
    use_typewriter_effect(&mut dialogue_state);

    // Auto-refresh observations when refresh counter changes (if panel is open)
    // Track of refresh counter - this will trigger re-render when it changes
    let observations_refresh = *game_state.observations_refresh_counter.read();
    let is_panel_open = *show_known_npcs_panel.read();
    let pc_id = game_state.selected_pc_id.read().clone();
    let obs_svc = observation_service.clone();

    use_effect(move || {
        // Use counter to establish reactive dependency (even if we just log it)
        let _ = observations_refresh;

        // Only refresh if panel is currently open and we have a PC
        if is_panel_open {
            if let Some(pid) = pc_id.clone() {
                let obs_svc = obs_svc.clone();
                spawn_task(async move {
                    match obs_svc.list_observations(&pid).await {
                        Ok(observations) => {
                            let npc_data: Vec<NpcObservationData> = observations
                                .into_iter()
                                .map(|o| NpcObservationData {
                                    npc_id: o.npc_id,
                                    npc_name: o.npc_name,
                                    npc_portrait: o.npc_portrait,
                                    location_name: o.location_name,
                                    region_name: o.region_name,
                                    game_time: o.game_time,
                                    observation_type: o.observation_type,
                                    observation_type_icon: o.observation_type_icon,
                                    notes: o.notes,
                                })
                                .collect();
                            known_npcs.set(npc_data);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to refresh observations: {}", e);
                        }
                    }
                });
            }
        }
    });

    // Read scene characters from game state (reactive)
    let scene_characters = game_state.scene_characters.read().clone();

    // Get current dialogue state
    let speaker_name = dialogue_state.speaker_name.read().clone();
    let displayed_text = dialogue_state.displayed_text.read().clone();
    let is_typing = *dialogue_state.is_typing.read();
    let choices = dialogue_state.choices.read().clone();
    let has_dialogue = dialogue_state.has_dialogue();
    let is_llm_processing = *dialogue_state.is_llm_processing.read();

    // Get interactions from game state
    let interactions = game_state.interactions.read().clone();

    // Get active challenge if any
    let active_challenge = session_state.active_challenge().read().clone();

    // Get roll status for result popup (Phase D)
    let roll_status = session_state.roll_status().read().clone();

    // Check if connected
    let is_connected = session_state.connection_status().read().is_connected();

    // Get navigation data from game state
    let current_region = game_state.current_region.read().clone();
    let navigation = game_state.navigation.read().clone();
    let selected_pc_id = game_state.selected_pc_id.read().clone();

    // Get event data from game state
    let approach_event = game_state.approach_event.read().clone();
    let location_event = game_state.location_event.read().clone();

    rsx! {
        div {
            class: "pc-view h-full flex flex-col relative",

            // Location and status indicator (top right)
            div {
                class: "absolute top-4 right-4 z-[100] flex flex-col gap-2 items-end",

                // Location/Region name - prefer region data if available
                if let Some(ref region) = current_region {
                    div {
                        class: "px-4 py-2 bg-black/70 text-white rounded-lg text-sm font-medium",
                        "📍 {region.name}"
                    }
                    div {
                        class: "px-3 py-1 bg-black/50 text-gray-300 rounded-lg text-xs",
                        "{region.location_name}"
                    }
                } else if let Some(scene) = game_state.current_scene.read().as_ref() {
                    div {
                        class: "px-4 py-2 bg-black/70 text-white rounded-lg text-sm font-medium",
                        "📍 {scene.location_name}"
                    }
                }

                // Connection status
                if !is_connected {
                    div {
                        class: "px-4 py-2 bg-red-500/80 text-white rounded-lg text-xs",
                        "Disconnected"
                    }
                }

                // Action error feedback (click to dismiss)
                if let Some(ref err) = *action_error.read() {
                    div {
                        class: "px-4 py-2 bg-red-600/90 text-white rounded-lg text-xs max-w-[250px] cursor-pointer",
                        onclick: move |_| action_error.set(None),
                        "{err}"
                    }
                }
            }

            // Visual novel stage
            Backdrop {
                image_url: game_state.backdrop_url(),
                transitioning: *game_state.backdrop_transitioning.read(),

                // Character layer with real scene characters
                CharacterLayer {
                    characters: scene_characters,
                    on_character_click: {
                        let command_bus = command_bus.clone();
                        move |character_id: String| {
                            tracing::info!("Clicked character: {}", character_id);
                            // Send a talk action when clicking a character
                            if let Err(e) = send_player_action(
                                &command_bus,
                                PlayerAction::talk(&character_id, None),
                            ) {
                                action_error.set(Some(e));
                            }
                        }
                    }
                }
            }

            // Dialogue box (fixed at bottom)
            div {
                class: "dialogue-container absolute bottom-0 left-0 right-0 z-10",

                if has_dialogue {
                    DialogueBox {
                        speaker_name: speaker_name.clone(),
                        dialogue_text: displayed_text,
                        is_typing: is_typing,
                        is_llm_processing: is_llm_processing,
                        choices: choices,
                        on_choice_selected: {
                            let command_bus = command_bus.clone();
                            let mut dialogue_state = dialogue_state.clone();
                            move |choice_id: String| {
                                if let Err(e) = handle_choice_selected(&command_bus, &mut dialogue_state, &choice_id) {
                                    action_error.set(Some(e));
                                }
                            }
                        },
                        on_custom_input: {
                            let command_bus = command_bus.clone();
                            let mut dialogue_state = dialogue_state.clone();
                            move |text: String| {
                                if let Err(e) = handle_custom_input(&command_bus, &mut dialogue_state, &text) {
                                    action_error.set(Some(e));
                                }
                            }
                        },
                        on_advance: {
                            let mut dialogue_state = dialogue_state.clone();
                            move |_| {
                                handle_advance(&mut dialogue_state);
                            }
                        },
                    }
                } else {
                    EmptyDialogueBox {}
                }
            }

            // Action panel with scene interactions (disabled while LLM is processing)
            ActionPanel {
                interactions: interactions,
                disabled: is_llm_processing,
                on_interaction: {
                    let command_bus = command_bus.clone();
                    move |interaction: InteractionData| {
                        if let Err(e) = handle_interaction(&command_bus, &interaction) {
                            action_error.set(Some(e));
                        }
                    }
                },
                on_inventory: Some(EventHandler::new({
                    let game_state = game_state.clone();
                    let character_service = character_service.clone();
                    move |_| {
                        tracing::info!("Open inventory");
                        show_inventory_panel.set(true);
                        is_loading_inventory.set(true);

                        // Get the selected PC or first character
                        let characters = game_state.world.read().as_ref()
                            .map(|w| w.characters.clone())
                            .unwrap_or_default();
                        let char_id = selected_character_id.read().clone()
                            .or_else(|| characters.first().map(|c| c.id.clone()));

                        if let Some(cid) = char_id {
                            selected_character_id.set(Some(cid.clone()));
                            let char_svc = character_service.clone();
                            spawn_task(async move {
                                match char_svc.get_inventory(&cid).await {
                                    Ok(items) => {
                                        inventory_items.set(items);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to load inventory: {}", e);
                                        inventory_items.set(Vec::new());
                                    }
                                }
                                is_loading_inventory.set(false);
                            });
                        } else {
                            is_loading_inventory.set(false);
                        }
                    }
                })),
                on_character: Some(EventHandler::new({
                    let game_state = game_state.clone();
                    let world_service = world_service.clone();
                    let character_service = character_service.clone();
                    move |_| {
                        tracing::info!("Open character sheet");
                        // Show the modal first (loading state)
                        show_character_sheet.set(true);
                        is_loading_sheet.set(true);

                        // Get world ID and first available character
                        let world_id = game_state.world.read().as_ref()
                            .map(|w| w.world.id.clone());
                        let characters = game_state.world.read().as_ref()
                            .map(|w| w.characters.clone())
                            .unwrap_or_default();

                        // Auto-select first character if none selected
                        let char_id = selected_character_id.read().clone()
                            .or_else(|| characters.first().map(|c| c.id.clone()));

                        if let (Some(wid), Some(cid)) = (world_id, char_id.clone()) {
                            selected_character_id.set(Some(cid.clone()));
                            let world_svc = world_service.clone();
                            let char_svc = character_service.clone();
                            spawn_task(async move {
                                // Load schema
                                match world_svc.get_sheet_template(&wid).await {
                                    Ok(schema) => {
                                        character_sheet_schema.set(Some(schema));
                                    }
                                    Err(e) => tracing::warn!("Failed to load sheet schema: {}", e),
                                }
                                // Load character data
                                match char_svc.get_character(&cid).await {
                                    Ok(char_data) => {
                                        player_character_name.set(char_data.name);
                                        if let Some(sheet_data) = char_data.sheet_data {
                                            character_sheet_values.set(
                                                sheet_data
                                                    .values
                                                    .into_iter()
                                                    .collect::<HashMap<_, _>>()
                                            );
                                        }
                                    }
                                    Err(e) => tracing::warn!("Failed to load character: {}", e),
                                }
                                is_loading_sheet.set(false);
                            });
                        } else {
                            is_loading_sheet.set(false);
                        }
                    }
                })),
                on_map: Some(EventHandler::new({
                    let game_state = game_state.clone();
                    let location_service = location_service.clone();
                    move |_| {
                        tracing::info!("Open mini-map");
                        show_mini_map.set(true);
                        is_loading_map.set(true);

                        // Get current region to find location ID
                        let current_region = game_state.current_region.read().clone();

                        if let Some(region) = current_region {
                            let loc_svc = location_service.clone();
                            let location_id = region.location_id.clone();
                            spawn_task(async move {
                                match loc_svc.get_regions(&location_id).await {
                                    Ok(regions) => {
                                        // Convert to component data type
                                        let map_data: Vec<MapRegionData> = regions
                                            .into_iter()
                                            .map(|r| MapRegionData {
                                                id: r.id,
                                                name: r.name,
                                                description: r.description,
                                                backdrop_asset: r.backdrop_asset,
                                                bounds: r.map_bounds.map(|b| MapBounds {
                                                    x: b.x,
                                                    y: b.y,
                                                    width: b.width,
                                                    height: b.height,
                                                }),
                                                is_spawn_point: r.is_spawn_point,
                                            })
                                            .collect();
                                        map_regions.set(map_data);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to load regions for map: {}", e);
                                        map_regions.set(Vec::new());
                                    }
                                }
                                is_loading_map.set(false);
                            });
                        } else {
                            // No current region - fall back to navigation panel
                            show_mini_map.set(false);
                            show_navigation_panel.set(true);
                            is_loading_map.set(false);
                        }
                    }
                })),
                on_people: Some(EventHandler::new({
                    let game_state = game_state.clone();
                    let observation_service = observation_service.clone();
                    move |_| {
                        tracing::info!("Open known NPCs panel");
                        show_known_npcs_panel.set(true);
                        is_loading_npcs.set(true);

                        // Get the selected PC ID
                        let pc_id = game_state.selected_pc_id.read().clone();

                        if let Some(pid) = pc_id {
                            let obs_svc = observation_service.clone();
                            spawn_task(async move {
                                match obs_svc.list_observations(&pid).await {
                                    Ok(observations) => {
                                        // Convert to component data type
                                        let npc_data: Vec<NpcObservationData> = observations
                                            .into_iter()
                                            .map(|o| NpcObservationData {
                                                npc_id: o.npc_id,
                                                npc_name: o.npc_name,
                                                npc_portrait: o.npc_portrait,
                                                location_name: o.location_name,
                                                region_name: o.region_name,
                                                game_time: o.game_time,
                                                observation_type: o.observation_type,
                                                observation_type_icon: o.observation_type_icon,
                                                notes: o.notes,
                                            })
                                            .collect();
                                        known_npcs.set(npc_data);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to load observations: {}", e);
                                        known_npcs.set(Vec::new());
                                    }
                                }
                                is_loading_npcs.set(false);
                            });
                        } else {
                            is_loading_npcs.set(false);
                        }
                    }
                })),
                on_log: Some(EventHandler::new(move |_| {
                    tracing::info!("Open log");
                })),
                on_skills: Some(EventHandler::new({
                    let game_state = game_state.clone();
                    let skill_service = skill_service.clone();
                    let character_sheet_values = character_sheet_values;
                    move |_| {
                        tracing::info!("Open skills panel");
                        show_skills_panel.set(true);
                        is_loading_skills.set(true);

                        // Get world ID
                        let world_id = game_state.world.read().as_ref()
                            .map(|w| w.world.id.clone());

                        // Get character sheet values for modifiers
                        let sheet_values = character_sheet_values.read().clone();

                        if let Some(wid) = world_id {
                            let skill_svc = skill_service.clone();
                            spawn_task(async move {
                                match skill_svc.list_skills(&wid).await {
                                    Ok(skills) => {
                                        // Convert SkillData to PlayerSkillData
                                        let player_skill_data: Vec<PlayerSkillData> = skills
                                            .into_iter()
                                            .filter(|s| !s.is_hidden)
                                            .map(|s| {
                                                // Try to get modifier from character sheet
                                                // Look for fields like "skill_persuasion" or just the skill name
                                                let modifier = sheet_values
                                                    .get(&format!("skill_{}", s.name.to_lowercase().replace(' ', "_")))
                                                    .or_else(|| sheet_values.get(&s.name.to_lowercase()))
                                                    .and_then(|v| v.as_i64().map(|n| n as i32))
                                                    .unwrap_or(0);

                                                PlayerSkillData {
                                                    id: s.id,
                                                    name: s.name,
                                                    category: s.category.display_name().to_string(),
                                                    modifier,
                                                    proficient: modifier > 0, // Simple heuristic
                                                }
                                            })
                                            .collect();
                                        player_skills.set(player_skill_data);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to load skills: {}", e);
                                        player_skills.set(Vec::new());
                                    }
                                }
                                is_loading_skills.set(false);
                            });
                        } else {
                            is_loading_skills.set(false);
                        }
                    }
                })),
                on_region_items: Some(EventHandler::new(move |_| {
                    tracing::info!("Open region items panel");
                    show_region_items_panel.set(true);
                })),
                region_items_count: game_state.region_items.read().len(),
                has_conversation: dialogue_state.has_active_conversation(),
                on_end_conversation: Some(EventHandler::new({
                    let mut show_end_conversation_modal = show_end_conversation_modal.clone();
                    move |_| {
                        tracing::info!("Show end conversation confirmation");
                        show_end_conversation_modal.set(true);
                    }
                })),
            }

            // Character sheet viewer modal
            if *show_character_sheet.read() {
                if *is_loading_sheet.read() {
                    // Loading state
                    div {
                        class: "character-sheet-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-8",
                        onclick: move |_| show_character_sheet.set(false),

                        div {
                            class: "bg-dark-surface rounded-xl p-8 max-w-md text-center",
                            onclick: move |e| e.stop_propagation(),

                            div {
                                class: "text-gray-400 text-xl",
                                "Loading character sheet..."
                            }
                        }
                    }
                } else if let Some(schema) = character_sheet_schema.read().as_ref() {
                    CharacterSheetViewer {
                        character_name: player_character_name.read().clone(),
                        schema: schema.clone(),
                        values: character_sheet_values.read().clone(),
                        on_close: move |_| show_character_sheet.set(false),
                    }
                } else {
                    // No template loaded - show placeholder with character selection
                    {
                        // Read world state once to avoid multiple borrows
                        let world_read = game_state.world.read();
                        let characters = world_read.as_ref()
                            .map(|w| w.characters.clone())
                            .unwrap_or_default();
                        let world_id_for_create = world_read.as_ref()
                            .map(|w| w.world.id.clone());
                        drop(world_read); // Explicitly drop the borrow before rsx!
                        rsx! {
                            div {
                                class: "character-sheet-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-8",
                                onclick: move |_| show_character_sheet.set(false),

                                div {
                                    class: "bg-dark-surface rounded-xl p-8 max-w-md text-center",
                                    onclick: move |e| e.stop_propagation(),

                                    h2 {
                                        class: "text-gray-100 m-0 mb-4",
                                        "Character Sheet"
                                    }

                                    if characters.is_empty() {
                                        p {
                                            class: "text-gray-400 m-0 mb-6",
                                            "No player character created yet."
                                        }
                                        if let Some(wid) = world_id_for_create.clone() {
                                            button {
                                                onclick: move |_| {
                                                    navigator.push(crate::routes::Route::PCCreationRoute { world_id: wid.clone() });
                                                },
                                                class: "py-2 px-6 bg-blue-500 text-white border-0 rounded-lg cursor-pointer mb-4",
                                                "Create Character"
                                            }
                                        }
                                    } else {
                                        p {
                                            class: "text-gray-400 m-0 mb-6",
                                            "No character sheet template available for this world. The DM may need to configure the rule system."
                                        }
                                    }

                                    button {
                                        onclick: move |_| show_character_sheet.set(false),
                                        class: "py-2 px-6 bg-gray-700 text-white border-0 rounded-lg cursor-pointer",
                                        "Close"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Challenge roll modal (for active challenges you're rolling)
            if let Some(ref challenge) = active_challenge {
                ChallengeRollModal {
                    challenge_id: challenge.challenge_id.clone(),
                    challenge_name: challenge.challenge_name.clone(),
                    skill_name: challenge.skill_name.clone(),
                    difficulty_display: challenge.difficulty_display.clone(),
                    description: challenge.description.clone(),
                    character_modifier: challenge.character_modifier,
                    suggested_dice: challenge.suggested_dice.clone(),
                    rule_system_hint: challenge.rule_system_hint.clone(),
                    on_roll: {
                        let command_bus = command_bus.clone();
                        let challenge_id = challenge.challenge_id.clone();
                        move |input: DiceInput| {
                            if let Err(e) = send_challenge_roll_input(&command_bus, &challenge_id, input) {
                                action_error.set(Some(e));
                            }
                        }
                    },
                    on_close: {
                        let mut session_state = session_state.clone();
                        move |_| {
                            // Clear both active challenge and roll status
                            // This ensures modal state is fully reset when user closes via X button
                            session_state.clear_active_challenge();
                            session_state.clear_roll_status();
                        }
                    },
                }
            }

            // Challenge result popup (for received results without active challenge - Phase D)
            if let RollSubmissionStatus::ResultReady(result) = roll_status {
                if active_challenge.is_none() {
                    ChallengeResultPopup {
                        result: result.clone(),
                        on_dismiss: {
                            let mut session_state = session_state.clone();
                            move |_| {
                                session_state.dismiss_result();
                                session_state.clear_roll_status();
                            }
                        },
                    }
                }
            }

            // Navigation panel modal
            if *show_navigation_panel.read() {
                if let Some(ref nav) = navigation {
                    NavigationPanel {
                        navigation: nav.clone(),
                        current_region_name: current_region.as_ref().map(|r| r.name.clone()).unwrap_or_else(|| "Unknown".to_string()),
                        current_location_name: current_region.as_ref().map(|r| r.location_name.clone()).unwrap_or_else(|| "Unknown".to_string()),
                        disabled: is_llm_processing,
                        on_move_to_region: {
                            let command_bus = command_bus.clone();
                            let pc_id = selected_pc_id.clone();
                            move |region_id: String| {
                                if let Some(ref pc) = pc_id {
                                    if let Err(e) = send_move_to_region(&command_bus, pc, &region_id) {
                                        action_error.set(Some(e));
                                    } else {
                                        show_navigation_panel.set(false);
                                    }
                                } else {
                                    action_error.set(Some("No character selected".to_string()));
                                }
                            }
                        },
                        on_exit_to_location: {
                            let command_bus = command_bus.clone();
                            let pc_id = selected_pc_id.clone();
                            move |(location_id, arrival_region_id): (String, String)| {
                                if let Some(ref pc) = pc_id {
                                    if let Err(e) = send_exit_to_location(&command_bus, pc, &location_id, Some(&arrival_region_id)) {
                                        action_error.set(Some(e));
                                    } else {
                                        show_navigation_panel.set(false);
                                    }
                                } else {
                                    action_error.set(Some("No character selected".to_string()));
                                }
                            }
                        },
                        on_close: move |_| {
                            show_navigation_panel.set(false);
                        },
                    }
                }
            }

            // Inventory panel modal
            if *show_inventory_panel.read() {
                InventoryPanel {
                    character_name: player_character_name.read().clone(),
                    items: inventory_items.read().clone(),
                    is_loading: *is_loading_inventory.read(),
                    on_close: move |_| {
                        show_inventory_panel.set(false);
                    },
                    on_use_item: Some(EventHandler::new({
                        let command_bus = command_bus.clone();
                        move |item_id: String| {
                            tracing::info!("Use item: {}", item_id);
                            if let Err(e) = send_player_action(
                                &command_bus,
                                PlayerAction::use_item(&item_id, None),
                            ) {
                                action_error.set(Some(e));
                            }
                        }
                    })),
                    on_toggle_equip: Some(EventHandler::new({
                        let command_bus = command_bus.clone();
                        let inventory_items = inventory_items;
                        let pc_id = selected_pc_id.clone();
                        move |item_id: String| {
                            let Some(ref pc_id) = pc_id else {
                                action_error.set(Some("No character selected".to_string()));
                                return;
                            };
                            // Find the item to check its equipped state
                            let items = inventory_items.read();
                            let is_equipped = items.iter()
                                .find(|i| i.item.id == item_id)
                                .map(|i| i.equipped)
                                .unwrap_or(false);

                            if is_equipped {
                                tracing::info!("Unequip item: {}", item_id);
                                if let Err(e) = send_unequip_item(&command_bus, pc_id, &item_id) {
                                    action_error.set(Some(e));
                                }
                            } else {
                                tracing::info!("Equip item: {}", item_id);
                                if let Err(e) = send_equip_item(&command_bus, pc_id, &item_id) {
                                    action_error.set(Some(e));
                                }
                            }
                        }
                    })),
                    on_drop_item: Some(EventHandler::new({
                        let command_bus = command_bus.clone();
                        let pc_id = selected_pc_id.clone();
                        move |item_id: String| {
                            let Some(ref pc_id) = pc_id else {
                                action_error.set(Some("No character selected".to_string()));
                                return;
                            };
                            tracing::info!("Drop item: {}", item_id);
                            if let Err(e) = send_drop_item(&command_bus, pc_id, &item_id) {
                                action_error.set(Some(e));
                            }
                        }
                    })),
                }
            }

            // Known NPCs panel modal
            if *show_known_npcs_panel.read() {
                KnownNpcsPanel {
                    observations: known_npcs.read().clone(),
                    is_loading: *is_loading_npcs.read(),
                    on_close: move |_| {
                        show_known_npcs_panel.set(false);
                    },
                    on_npc_click: Some(EventHandler::new({
                        let command_bus = command_bus.clone();
                        move |npc_id: String| {
                            tracing::info!("Clicked NPC: {}", npc_id);
                            // Could open NPC details or start a talk action
                            if let Err(e) = send_player_action(
                                &command_bus,
                                PlayerAction::talk(&npc_id, None),
                            ) {
                                action_error.set(Some(e));
                            } else {
                                show_known_npcs_panel.set(false);
                            }
                        }
                    })),
                }
            }

            // Mini-map modal
            if *show_mini_map.read() {
                MiniMap {
                    location_name: current_region.as_ref().map(|r| r.location_name.clone()).unwrap_or_default(),
                    map_image: current_region.as_ref().and_then(|r| r.map_asset.clone()),
                    regions: map_regions.read().clone(),
                    current_region_id: current_region.as_ref().map(|r| r.id.clone()),
                    navigable_region_ids: navigation.as_ref()
                        .map(|n| n.connected_regions.iter()
                            .filter(|r| !r.is_locked)
                            .map(|r| r.region_id.clone())
                            .collect())
                        .unwrap_or_default(),
                    locked_region_ids: navigation.as_ref()
                        .map(|n| n.connected_regions.iter()
                            .filter(|r| r.is_locked)
                            .map(|r| r.region_id.clone())
                            .collect())
                        .unwrap_or_default(),
                    is_loading: *is_loading_map.read(),
                    on_region_click: {
                        let command_bus = command_bus.clone();
                        let selected_pc_id = selected_pc_id.clone();
                        move |region_id: String| {
                            if let Some(ref pc) = selected_pc_id {
                                if let Err(e) = send_move_to_region(&command_bus, pc, &region_id) {
                                    action_error.set(Some(e));
                                } else {
                                    show_mini_map.set(false);
                                }
                            } else {
                                action_error.set(Some("No character selected".to_string()));
                            }
                        }
                    },
                    on_close: move |_| show_mini_map.set(false),
                }
            }

            // Skills panel modal
            if *show_skills_panel.read() {
                if *is_loading_skills.read() {
                    // Loading state
                    div {
                        class: "fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center",
                        onclick: move |_| show_skills_panel.set(false),
                        div {
                            class: "bg-dark-surface rounded-xl p-8 text-center",
                            onclick: move |e| e.stop_propagation(),
                            div {
                                class: "text-gray-400 text-xl",
                                "Loading skills..."
                            }
                        }
                    }
                } else {
                    SkillsDisplay {
                        skills: player_skills.read().clone(),
                        on_close: move |_| show_skills_panel.set(false),
                    }
                }
            }

            // Region items panel modal
            if *show_region_items_panel.read() {
                RegionItemsPanel {
                    items: game_state.region_items.read().clone(),
                    on_close: move |_| show_region_items_panel.set(false),
                    on_pickup: {
                        let command_bus = command_bus.clone();
                        let selected_pc_id = selected_pc_id.clone();
                        move |item_id: String| {
                            if let Some(ref pc_id) = selected_pc_id {
                                tracing::info!("Pick up item: {} for PC: {}", item_id, pc_id);
                                if let Err(e) = send_pickup_item(&command_bus, pc_id, &item_id) {
                                    action_error.set(Some(e));
                                } else {
                                    show_region_items_panel.set(false);
                                }
                            } else {
                                action_error.set(Some("No character selected".to_string()));
                            }
                        }
                    },
                }
            }

            // Approach event overlay (NPC approaching player)
            if let Some(ref event) = approach_event {
                ApproachEventOverlay {
                    event: event.clone(),
                    on_dismiss: {
                        let mut game_state = game_state.clone();
                        move |_| {
                            game_state.clear_approach_event();
                        }
                    },
                }
            }

            // Location event banner
            if let Some(ref event) = location_event {
                LocationEventBanner {
                    event: event.clone(),
                    on_dismiss: {
                        let mut game_state = game_state.clone();
                        move |_| {
                            game_state.clear_location_event();
                        }
                    },
                }
            }

            // Time advance toast (when DM advances time)
            TimeAdvanceToast {}

            // End conversation confirmation modal
            if *show_end_conversation_modal.read() {
                EndConversationConfirmation {
                    npc_name: speaker_name.clone(),
                    on_confirm: {
                        let command_bus = command_bus.clone();
                        let mut show_end_conversation_modal = show_end_conversation_modal.clone();
                        let dialogue_state_for_closure = dialogue_state.clone();
                        move |_| {
                            let speaker_id = dialogue_state_for_closure.speaker_id.read().clone();
                            if let Some(npc_id) = speaker_id {
                                // Send EndConversation message
                                let msg = ClientMessageBuilder::end_conversation(&npc_id, None);
                                if let Err(e) = command_bus.send(msg) {
                                    tracing::error!("Failed to send EndConversation message: {}", e);
                                } else {
                                    tracing::info!("Sent EndConversation message for NPC: {}", npc_id);
                                }
                            } else {
                                tracing::warn!("No speaker ID available for ending conversation");
                            }
                            show_end_conversation_modal.set(false);
                        }
                    },
                    on_cancel: {
                        let mut show_end_conversation_modal = show_end_conversation_modal.clone();
                        move |_| {
                            show_end_conversation_modal.set(false);
                        }
                    },
                }
            }

            // Staging pending overlay (player waiting for DM to set the scene)
            if let Some(ref pending) = *game_state.staging_pending.read() {
                StagingPendingOverlay {
                    region_name: pending.region_name.clone(),
                    started_at_ms: pending.started_at_ms,
                    timeout_seconds: pending.timeout_seconds,
                }
            }
        }
    }
}

/// Overlay shown when player is waiting for DM to approve staging
#[component]
fn StagingPendingOverlay(region_name: String, started_at_ms: u64, timeout_seconds: u64) -> Element {
    let platform = use_context::<std::sync::Arc<dyn crate::ports::outbound::PlatformPort>>();
    let mut remaining_seconds = use_signal(|| {
        // Calculate initial remaining time
        let elapsed_ms = platform.now_millis().saturating_sub(started_at_ms);
        let elapsed_secs = elapsed_ms / 1000;
        timeout_seconds.saturating_sub(elapsed_secs)
    });

    // Timer effect to update countdown every second
    let platform_for_effect = platform.clone();
    use_effect(move || {
        if timeout_seconds == 0 {
            return; // No countdown if timeout is 0
        }

        let platform = platform_for_effect.clone();
        spawn_task(async move {
            loop {
                // Wait 1 second
                platform.sleep_ms(1000).await;

                // Recalculate remaining time
                let elapsed_ms = platform.now_millis().saturating_sub(started_at_ms);
                let elapsed_secs = elapsed_ms / 1000;
                let new_remaining = timeout_seconds.saturating_sub(elapsed_secs);

                remaining_seconds.set(new_remaining);

                // Stop updating if we've reached 0
                if new_remaining == 0 {
                    break;
                }
            }
        });
    });

    let remaining = *remaining_seconds.read();

    rsx! {
        div {
            class: "fixed inset-0 bg-black/70 flex items-center justify-center z-[900] backdrop-blur-sm",

            div {
                class: "bg-gradient-to-br from-dark-surface to-dark-bg p-8 rounded-2xl max-w-md text-center border border-amber-500/30",

                // Spinning loader
                div {
                    class: "mb-6",
                    div {
                        class: "w-16 h-16 mx-auto border-4 border-amber-500/30 border-t-amber-500 rounded-full animate-spin",
                    }
                }

                // Title
                h2 {
                    class: "text-2xl font-bold text-amber-400 mb-3",
                    "Setting the Scene..."
                }

                // Region name
                p {
                    class: "text-gray-300 text-lg mb-4",
                    "Entering {region_name}"
                }

                // Countdown timer (only show if timeout is set)
                if timeout_seconds > 0 {
                    div {
                        class: "mb-4",
                        p {
                            class: "text-gray-400 text-sm",
                            "Auto-continuing in"
                        }
                        p {
                            class: "text-2xl font-mono text-amber-300 mt-1",
                            "{remaining}s"
                        }
                    }
                }

                // Description
                p {
                    class: "text-gray-500 text-sm",
                    if timeout_seconds > 0 {
                        "Waiting for DM, or auto-continue when ready..."
                    } else {
                        "The DM is preparing the scene. Please wait..."
                    }
                }
            }
        }
    }
}

/// Standalone challenge result popup (Phase D)
/// Shown when a ChallengeResolved message is received without an active challenge modal.
#[component]
fn ChallengeResultPopup(
    result: crate::presentation::state::challenge_state::ChallengeResultData,
    on_dismiss: EventHandler<()>,
) -> Element {
    // Determine display colors and text based on outcome
    let (outcome_text, outcome_class, border_class) = match result.outcome.as_str() {
        "critical_success" => ("CRITICAL SUCCESS", "text-yellow-400", "border-yellow-400"),
        "success" => ("SUCCESS", "text-green-500", "border-green-500"),
        "failure" => ("FAILURE", "text-red-500", "border-red-500"),
        "critical_failure" => ("CRITICAL FAILURE", "text-red-700", "border-red-700"),
        _ => ("RESULT", "text-amber-500", "border-amber-500"),
    };

    rsx! {
        // Modal overlay
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| on_dismiss.call(()),

            // Modal content
            div {
                class: "bg-gradient-to-br from-dark-surface to-dark-bg p-8 rounded-2xl max-w-[450px] w-[90%] border-2 {border_class}",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "text-center mb-6",

                    h2 {
                        class: "text-2xl font-bold {outcome_class} mb-2",
                        "{outcome_text}"
                    }

                    p {
                        class: "text-gray-400 text-sm",
                        "{result.challenge_name}"
                    }

                    p {
                        class: "text-gray-500 text-xs",
                        "by {result.character_name}"
                    }
                }

                // Roll breakdown
                div {
                    class: "bg-black/30 rounded-lg p-4 mb-4",

                    div {
                        class: "flex justify-between mb-2",
                        span { class: "text-gray-400", "Roll" }
                        span { class: "text-white font-bold", "{result.roll}" }
                    }

                    div {
                        class: "flex justify-between mb-2",
                        span { class: "text-gray-400", "Modifier" }
                        span {
                            class: "text-blue-500 font-bold",
                            if result.modifier >= 0 { "+{result.modifier}" } else { "{result.modifier}" }
                        }
                    }

                    div {
                        class: "border-t border-white/10 pt-2 flex justify-between",
                        span { class: "text-gray-400 font-bold", "Total" }
                        span { class: "{outcome_class} font-bold text-xl", "{result.total}" }
                    }
                }

                // Optional roll breakdown string
                if let Some(breakdown) = &result.roll_breakdown {
                    p {
                        class: "text-gray-500 text-xs text-center mb-4 font-mono",
                        "{breakdown}"
                    }
                }

                // Outcome description
                if !result.outcome_description.is_empty() {
                    div {
                        class: "bg-black/20 rounded-lg p-4 mb-4",
                        p {
                            class: "text-gray-300 text-sm leading-relaxed italic",
                            "{result.outcome_description}"
                        }
                    }
                }

                // Dismiss button
                button {
                    onclick: move |_| on_dismiss.call(()),
                    class: "w-full p-3 bg-gradient-to-br from-amber-500 to-amber-600 text-white border-none rounded-lg cursor-pointer font-semibold",
                    "Continue"
                }
            }
        }
    }
}

/// Send a player action via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_player_action(command_bus: &CommandBus, action: PlayerAction) -> Result<(), String> {
    let msg = ClientMessageBuilder::player_action(
        action.action_type.as_str(),
        action.target.as_deref(),
        action.dialogue.as_deref(),
    );
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to send action: {}", e))
}

/// Handle a dialogue choice being selected
fn handle_choice_selected(
    command_bus: &CommandBus,
    dialogue_state: &mut crate::presentation::state::DialogueState,
    choice_id: &str,
) -> Result<(), String> {
    tracing::info!("Choice selected: {}", choice_id);

    // Clear awaiting state since we're making a choice
    dialogue_state.awaiting_input.set(false);

    // Send dialogue choice action to the server
    send_player_action(command_bus, PlayerAction::dialogue_choice(choice_id))
}

/// Handle custom text input
/// Returns Ok(()) on success, Err(message) on failure
fn handle_custom_input(
    command_bus: &CommandBus,
    dialogue_state: &mut crate::presentation::state::DialogueState,
    text: &str,
) -> Result<(), String> {
    tracing::info!("Custom input: {}", text);

    // Clear awaiting state
    dialogue_state.awaiting_input.set(false);

    // Send custom action to the server
    send_player_action(command_bus, PlayerAction::custom(text))
}

/// Handle advancing dialogue (clicking to continue or skipping typewriter)
fn handle_advance(dialogue_state: &mut crate::presentation::state::DialogueState) {
    if *dialogue_state.is_typing.read() {
        // Skip typewriter animation
        dialogue_state.skip_typewriter();
    } else {
        // If no choices and dialogue is done, the server will send next content
        if !dialogue_state.has_choices() {
            tracing::info!("Dialogue complete, awaiting server response");
        }
    }
}

/// Handle an interaction being selected from the action panel
/// Returns Ok(()) on success, Err(message) on failure
fn handle_interaction(
    command_bus: &CommandBus,
    interaction: &InteractionData,
) -> Result<(), String> {
    tracing::info!(
        "Selected interaction: {} ({})",
        interaction.name,
        interaction.interaction_type
    );

    // Convert interaction type to player action
    let action = match interaction.interaction_type.to_lowercase().as_str() {
        "talk" | "dialogue" | "speak" => PlayerAction::talk(&interaction.id, None),
        "examine" | "look" | "inspect" => PlayerAction::examine(&interaction.id),
        "travel" | "go" | "move" => PlayerAction::travel(&interaction.id),
        "use" | "interact" => {
            // Use the interaction ID as both item and target for generic "use"
            PlayerAction::use_item(&interaction.id, interaction.target_name.as_deref())
        }
        _ => {
            // For unknown interaction types, send as custom action
            PlayerAction::custom_targeted(&interaction.id, &interaction.name)
        }
    };

    send_player_action(command_bus, action)
}

/// Send a challenge roll with dice input via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_challenge_roll_input(
    command_bus: &CommandBus,
    challenge_id: &str,
    input: DiceInput,
) -> Result<(), String> {
    let msg = ClientMessageBuilder::challenge_roll_input(challenge_id, input.into());
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to submit roll: {}", e))
}

/// Send a move to region command via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_move_to_region(
    command_bus: &CommandBus,
    pc_id: &str,
    region_id: &str,
) -> Result<(), String> {
    let msg = ClientMessageBuilder::move_to_region(pc_id, region_id);
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to move: {}", e))
}

/// Send an exit to location command via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_exit_to_location(
    command_bus: &CommandBus,
    pc_id: &str,
    location_id: &str,
    arrival_region_id: Option<&str>,
) -> Result<(), String> {
    let msg = ClientMessageBuilder::exit_to_location(pc_id, location_id, arrival_region_id);
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to exit location: {}", e))
}

/// Send an equip item command via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_equip_item(command_bus: &CommandBus, pc_id: &str, item_id: &str) -> Result<(), String> {
    let msg = ClientMessageBuilder::equip_item(pc_id, item_id);
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to equip item: {}", e))
}

/// Send an unequip item command via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_unequip_item(command_bus: &CommandBus, pc_id: &str, item_id: &str) -> Result<(), String> {
    let msg = ClientMessageBuilder::unequip_item(pc_id, item_id);
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to unequip item: {}", e))
}

/// Send a drop item command via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_drop_item(command_bus: &CommandBus, pc_id: &str, item_id: &str) -> Result<(), String> {
    let msg = ClientMessageBuilder::drop_item(pc_id, item_id, 1);
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to drop item: {}", e))
}

/// Send a pickup item command via CommandBus
/// Returns Ok(()) on success, Err(message) on failure
fn send_pickup_item(command_bus: &CommandBus, pc_id: &str, item_id: &str) -> Result<(), String> {
    let msg = ClientMessageBuilder::pickup_item(pc_id, item_id);
    command_bus
        .send(msg)
        .map_err(|e| format!("Failed to pick up item: {}", e))
}
