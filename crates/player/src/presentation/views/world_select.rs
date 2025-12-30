//! World Selection View - Select or create a world before entering gameplay
//!
//! This view appears after role selection and before the game views.
//! - DM: Can create new worlds or continue existing ones
//! - Player: Can join existing worlds
//! - Spectator: Can watch existing worlds

use dioxus::prelude::*;

use crate::application::dto::{
    DiceSystem, RuleSystemConfig, RuleSystemPresetDetails, RuleSystemType, RuleSystemVariant,
    StatDefinition, SuccessComparison, SessionWorldSnapshot,
};
use crate::application::services::world_service::{WorldSummary, SessionInfo};
use crate::application::ports::outbound::Platform;
use crate::presentation::services::use_world_service;
use crate::presentation::state::GameState;
use crate::UserRole;

/// Props for WorldSelectView
#[derive(Props, Clone, PartialEq)]
pub struct WorldSelectViewProps {
    /// The role the user selected
    pub role: UserRole,
    /// Called when a world is selected and loaded
    pub on_world_selected: EventHandler<String>,
    /// Called when user wants to go back to role selection
    pub on_back: EventHandler<()>,
}

/// World Selection View component
#[component]
pub fn WorldSelectView(props: WorldSelectViewProps) -> Element {
    let game_state = use_context::<GameState>();
    let platform = use_context::<Platform>();
    let world_service = use_world_service();
    let mut worlds: Signal<Vec<WorldSummary>> = use_signal(Vec::new);
    let mut sessions: Signal<Vec<SessionInfo>> = use_signal(Vec::new);
    let mut is_loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut show_create_form = use_signal(|| false);
    let mut world_to_load: Signal<Option<String>> = use_signal(|| None);

    let is_dm = props.role == UserRole::DungeonMaster;

    // Clone services for use in effects
    let world_service_for_list = world_service.clone();
    let world_service_for_load = world_service.clone();

    // Fetch worlds on mount
    use_effect(move || {
        let svc = world_service_for_list.clone();
        spawn(async move {
            match svc.list_worlds().await {
                Ok(list) => {
                    worlds.set(list);
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    is_loading.set(false);
                }
            }
        });
    });

    // Fetch active sessions for all worlds (for DM "Continue" and Player/Spectator views)
    let user_id = platform.get_user_id();
    let world_service_for_sessions = world_service.clone();
    use_effect(move || {
        let svc = world_service_for_sessions.clone();
        spawn(async move {
            match svc.list_sessions().await {
                Ok(list) => {
                    sessions.set(list);
                }
                Err(e) => {
                    tracing::error!("Failed to load sessions: {}", e);
                }
            }
        });
    });

    // Effect to load world when world_to_load is set
    use_effect(move || {
        if let Some(world_id) = world_to_load.read().clone() {
            let mut game_state = game_state.clone();
            let world_id_for_callback = world_id.clone();
            let svc = world_service_for_load.clone();
            spawn(async move {
                is_loading.set(true);
                match svc.load_world_snapshot(&world_id).await {
                    Ok(snapshot_json) => {
                        // Parse the JSON value into SessionWorldSnapshot
                        match serde_json::from_value::<SessionWorldSnapshot>(snapshot_json) {
                            Ok(snapshot) => {
                                game_state.load_world(snapshot);
                                // Signal that world is loaded - parent will navigate
                                props.on_world_selected.call(world_id_for_callback);
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to parse world snapshot: {}", e)));
                                is_loading.set(false);
                            }
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load world: {}", e)));
                        is_loading.set(false);
                    }
                }
                world_to_load.set(None);
            });
        }
    });

    // Title based on role
    let title = match props.role {
        UserRole::DungeonMaster => "Select World",
        UserRole::Player => "Join a World",
        UserRole::Spectator => "Watch a World",
    };

    let subtitle = match props.role {
        UserRole::DungeonMaster => "Continue an existing campaign or create a new one",
        UserRole::Player => "Choose a world to join as a player",
        UserRole::Spectator => "Choose a world to watch",
    };

    let action_label = match props.role {
        UserRole::DungeonMaster => "Continue",
        UserRole::Player => "Join",
        UserRole::Spectator => "Watch",
    };

    // Snapshot worlds and sessions for rendering
    let worlds_val_snapshot = worlds.read().clone();
    let sessions_val_snapshot = sessions.read().clone();

    // For Players/Spectators, only show worlds that currently have an active session.
    // For DMs, show all worlds (they can start new sessions).
    let filtered_worlds: Vec<WorldSummary> = if is_dm {
        worlds_val_snapshot.clone()
    } else {
        worlds_val_snapshot
            .iter()
            .filter(|w| sessions_val_snapshot.iter().any(|s| s.world_id == w.id))
            .cloned()
            .collect()
    };

    rsx! {
        div {
            class: "world-select-view h-full flex flex-col items-center justify-center p-8 bg-gradient-to-br from-dark-surface to-dark-gradient-end",

            div {
                class: "max-w-[700px] w-full",

                // Back button
                button {
                    onclick: move |_| props.on_back.call(()),
                    class: "mb-6 px-4 py-2 bg-transparent text-gray-400 border border-gray-700 rounded-md cursor-pointer text-sm",
                    "← Back to Role Selection"
                }

                h1 {
                    class: "text-white text-center mb-2 text-3xl",
                    "{title}"
                }
                p {
                    class: "text-gray-400 text-center mb-8",
                    "{subtitle}"
                }

                // Error message
                if let Some(err) = error.read().as_ref() {
                    div {
                        class: "p-4 bg-red-500/10 border border-red-500/30 rounded-lg text-red-500 mb-4",
                        "{err}"
                    }
                }

                // Loading state
                if *is_loading.read() {
                    div {
                        class: "text-center text-gray-500 p-8",
                        "Loading worlds..."
                    }
                } else if *show_create_form.read() && is_dm {
                    // Create form (DM only)
                    CreateWorldForm {
                        on_created: move |world_id: String| {
                            show_create_form.set(false);
                            world_to_load.set(Some(world_id));
                        },
                        on_cancel: move |_| show_create_form.set(false),
                    }
                } else {
                    // World list
                    div {
                        class: "bg-dark-surface rounded-lg overflow-hidden",

                        // Header with create button (DM only)
                        div {
                            class: "flex justify-between items-center p-4 border-b border-gray-700",

                            h2 { class: "text-gray-400 text-sm uppercase m-0",
                                if is_dm { "Your Worlds" } else { "Available Worlds" }
                            }

                            if is_dm {
                                button {
                                    onclick: move |_| show_create_form.set(true),
                                    class: "px-4 py-2 bg-purple-500 text-white border-0 rounded cursor-pointer text-sm",
                                    "+ Create New World"
                                }
                            }
                        }

                        // World list
                        div {
                            class: "max-h-[400px] overflow-y-auto",

                            if filtered_worlds.is_empty() {
                                div {
                                    class: "p-8 text-center text-gray-500",
                                    if is_dm {
                                        p { "No worlds yet." }
                                        p { class: "text-sm", "Create your first world to get started!" }
                                    } else {
                                        p { "No worlds available." }
                                        p { class: "text-sm", "Ask your DM to create a world first." }
                                    }
                                }
                            }

                            for world in filtered_worlds.iter() {
                                WorldCard {
                                    key: "{world.id}",
                                    world: world.clone(),
                                    action_label: action_label,
                                    is_dm: is_dm,
                                    has_dm_session: if is_dm {
                                        sessions_val_snapshot.iter().any(|s| {
                                            s.world_id == world.id && s.dm_user_id == user_id
                                        })
                                    } else {
                                        false
                                    },
                                    on_select: {
                                        let mut world_to_load = world_to_load.clone();
                                        let user_id = user_id.clone();
                                        let svc = world_service.clone();
                                        move |id: String| {
                                            if is_dm {
                                                // For DMs, ensure there is an active deterministic session
                                                // for this world. We optimistically start loading the world,
                                                // and fire-and-forget the session creation.
                                                world_to_load.set(Some(id.clone()));
                                                let world_id = id.clone();
                                                let dm_id = user_id.clone();
                                                let svc_for_async = svc.clone();
                                                spawn(async move {
                                                    if let Err(e) =
                                                        svc_for_async.create_session(&world_id, &dm_id).await
                                                    {
                                                        tracing::error!(
                                                            "Failed to create/resume DM session: {}",
                                                            e
                                                        );
                                                    }
                                                });
                                            } else {
                                                world_to_load.set(Some(id));
                                            }
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// World card in the list
#[component]
fn WorldCard(
    world: WorldSummary,
    action_label: &'static str,
    is_dm: bool,
    has_dm_session: bool,
    on_select: EventHandler<String>,
) -> Element {
    let world_id = world.id.clone();

    let button_label = if is_dm {
        if has_dm_session {
            "Continue Session →"
        } else {
            "Start Session →"
        }
    } else {
        action_label
    };

    rsx! {
        div {
            class: "p-4 border-b border-gray-700 flex justify-between items-center",

            div {
                class: "flex-1",
                h3 { class: "text-white m-0 mb-1 text-base", "{world.name}" }
                if let Some(desc) = &world.description {
                    p { class: "text-gray-400 m-0 text-sm leading-snug", "{desc}" }
                }
            }

            button {
                onclick: move |_| on_select.call(world_id.clone()),
                class: "px-4 py-2 bg-blue-500 text-white border-0 rounded cursor-pointer text-sm whitespace-nowrap",
                "{button_label}"
            }
        }
    }
}

/// Form for creating a new world (DM only)
#[component]
fn CreateWorldForm(on_created: EventHandler<String>, on_cancel: EventHandler<()>) -> Element {
    let world_service = use_world_service();
    let mut name = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut selected_type = use_signal(|| RuleSystemType::D20);
    let mut selected_variant: Signal<Option<RuleSystemVariant>> =
        use_signal(|| Some(RuleSystemVariant::Dnd5e));
    let mut is_creating = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut created_world_id: Signal<Option<String>> = use_signal(|| None);

    // Full rule system config (loaded when variant changes)
    let mut rule_config: Signal<Option<RuleSystemConfig>> = use_signal(|| None);
    let mut show_advanced = use_signal(|| false);
    let mut is_loading_preset = use_signal(|| false);

    // Update variant when type changes
    let current_type = *selected_type.read();
    let available_variants = RuleSystemVariant::variants_for_type(current_type);

    // Clone services for use in effect and handler
    let world_service_for_preset = world_service.clone();
    let world_service_for_create = world_service.clone();

    // Effect to call on_created when world is created
    use_effect(move || {
        let world_id = created_world_id.read().clone();
        if let Some(id) = world_id {
            on_created.call(id);
            created_world_id.set(None);
        }
    });

    // Effect to load preset when variant changes
    let variant_for_effect = selected_variant.read().clone();
    use_effect(move || {
        if let Some(variant) = variant_for_effect.clone() {
            let svc = world_service_for_preset.clone();
            spawn(async move {
                is_loading_preset.set(true);
                // Determine system type for the variant
                let system_type = match variant {
                    RuleSystemVariant::Dnd5e | RuleSystemVariant::Pathfinder2e | RuleSystemVariant::GenericD20 => "D20",
                    RuleSystemVariant::CallOfCthulhu7e | RuleSystemVariant::RuneQuest | RuleSystemVariant::GenericD100 => "D100",
                    RuleSystemVariant::KidsOnBikes | RuleSystemVariant::FateCore | RuleSystemVariant::PoweredByApocalypse => "Narrative",
                    RuleSystemVariant::Custom(_) => "Custom",
                };
                let variant_str = format!("{:?}", variant);

                match svc.get_rule_system_preset(system_type, &variant_str).await {
                    Ok(config_json) => {
                        // Engine returns RuleSystemPresetDetails { variant, config }
                        match serde_json::from_value::<RuleSystemPresetDetails>(config_json) {
                            Ok(preset_details) => {
                                rule_config.set(Some(preset_details.config));
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to parse preset: {}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load preset: {}", e)));
                    }
                }
                is_loading_preset.set(false);
            });
        } else {
            rule_config.set(None);
        }
    });

    let mut handle_type_change = move |type_str: String| {
        let new_type = match type_str.as_str() {
            "D20" => RuleSystemType::D20,
            "D100" => RuleSystemType::D100,
            "Narrative" => RuleSystemType::Narrative,
            _ => RuleSystemType::Custom,
        };
        selected_type.set(new_type);
        // Set default variant for the new type
        let variants = RuleSystemVariant::variants_for_type(new_type);
        selected_variant.set(variants.into_iter().next());
    };

    let mut handle_variant_change = move |variant_str: String| {
        let variant = match variant_str.as_str() {
            "Dnd5e" => Some(RuleSystemVariant::Dnd5e),
            "Pathfinder2e" => Some(RuleSystemVariant::Pathfinder2e),
            "GenericD20" => Some(RuleSystemVariant::GenericD20),
            "CallOfCthulhu7e" => Some(RuleSystemVariant::CallOfCthulhu7e),
            "RuneQuest" => Some(RuleSystemVariant::RuneQuest),
            "GenericD100" => Some(RuleSystemVariant::GenericD100),
            "KidsOnBikes" => Some(RuleSystemVariant::KidsOnBikes),
            "FateCore" => Some(RuleSystemVariant::FateCore),
            "PoweredByApocalypse" => Some(RuleSystemVariant::PoweredByApocalypse),
            _ => None,
        };
        selected_variant.set(variant);
    };

    let handle_create = move |_| {
        let name_val = name.read().clone();
        if name_val.trim().is_empty() {
            error.set(Some("World name is required".to_string()));
            return;
        }

        let desc_val = description.read().clone();
        let config = rule_config.read().clone();
        let svc = world_service_for_create.clone();

        spawn(async move {
            is_creating.set(true);
            error.set(None);

            // Convert RuleSystemConfig to JSON
            let rule_system_json = config.and_then(|c| serde_json::to_value(c).ok());

            match svc.create_world(
                &name_val,
                if desc_val.is_empty() { None } else { Some(&desc_val) },
                rule_system_json,
            )
            .await
            {
                Ok(world_id) => {
                    created_world_id.set(Some(world_id));
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    is_creating.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "bg-dark-surface rounded-lg p-6 max-h-[80vh] overflow-y-auto",

            h2 { class: "text-white m-0 mb-4", "Create New World" }

            // Error
            if let Some(err) = error.read().as_ref() {
                div {
                    class: "p-3 bg-red-500/10 rounded text-red-500 mb-4 text-sm",
                    "{err}"
                }
            }

            // Name field
            div { class: "mb-4",
                label { class: "block text-gray-400 text-sm mb-1", "Name *" }
                input {
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                    placeholder: "The Dragon's Bane",
                    disabled: *is_creating.read(),
                    class: "w-full p-3 bg-dark-bg border border-gray-700 rounded text-white box-border",
                }
            }

            // Description field
            div { class: "mb-4",
                label { class: "block text-gray-400 text-sm mb-1", "Description" }
                textarea {
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                    placeholder: "A dark fantasy campaign in the realm of Valdris...",
                    disabled: *is_creating.read(),
                    class: "w-full min-h-[80px] p-3 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                }
            }

            // Rule System Selection
            div { class: "mb-4 p-4 bg-dark-bg rounded-lg border border-gray-700",
                h3 { class: "text-gray-400 text-sm uppercase m-0 mb-3", "Rule System" }

                // System Type dropdown
                div { class: "mb-3",
                    label { class: "block text-gray-500 text-xs mb-1", "System Type" }
                    select {
                        value: "{current_type:?}",
                        onchange: move |e| handle_type_change(e.value()),
                        disabled: *is_creating.read(),
                        class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white",
                        option { value: "D20", "D20 System (D&D, Pathfinder)" }
                        option { value: "D100", "D100 System (Call of Cthulhu)" }
                        option { value: "Narrative", "Narrative (Kids on Bikes, FATE)" }
                        option { value: "Custom", "Custom" }
                    }
                }

                // Variant/Preset dropdown (only show if variants available)
                if !available_variants.is_empty() {
                    {
                        let current_variant_value = selected_variant.read()
                            .as_ref()
                            .map(|v| format!("{:?}", v))
                            .unwrap_or_default();

                        rsx! {
                            div { class: "mb-2",
                                label { class: "block text-gray-500 text-xs mb-1", "Preset" }
                                select {
                                    value: "{current_variant_value}",
                                    onchange: move |e| handle_variant_change(e.value()),
                                    disabled: *is_creating.read() || *is_loading_preset.read(),
                                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white",
                                    for variant in available_variants.iter() {
                                        option {
                                            value: "{variant:?}",
                                            "{variant.display_name()}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Show selected preset description
                if let Some(variant) = selected_variant.read().as_ref() {
                    p { class: "text-gray-500 text-xs mt-2 mb-0",
                        "{variant.description()}"
                    }
                } else {
                    p { class: "text-gray-500 text-xs mt-2 mb-0",
                        "{current_type.description()}"
                    }
                }

                // Loading indicator
                if *is_loading_preset.read() {
                    p { class: "text-purple-500 text-xs mt-2 mb-0",
                        "Loading preset..."
                    }
                }
            }

            // Advanced Configuration (collapsible)
            if rule_config.read().is_some() {
                div { class: "mb-4",
                    {
                        let is_expanded = *show_advanced.read();
                        rsx! {
                            button {
                                onclick: move |_| show_advanced.set(!is_expanded),
                                class: "w-full p-3 bg-dark-bg border border-gray-700 rounded text-gray-400 cursor-pointer text-left flex justify-between items-center",
                                span { "Advanced Configuration" }
                                span { if is_expanded { "▼" } else { "▶" } }
                            }
                        }
                    }

                    if *show_advanced.read() {
                        {
                            let Some(config) = rule_config.read().clone() else {
                                return rsx! {};
                            };
                            rsx! {
                                RuleSystemConfigEditor {
                                    config: config,
                                    on_change: move |new_config| rule_config.set(Some(new_config)),
                                    disabled: *is_creating.read(),
                                }
                            }
                        }
                    }
                }
            }

            // Buttons
            div { class: "flex gap-3",
                button {
                    onclick: handle_create,
                    disabled: *is_creating.read() || *is_loading_preset.read(),
                    class: "flex-1 p-3 bg-purple-500 text-white border-0 rounded cursor-pointer font-semibold",
                    if *is_creating.read() { "Creating..." } else { "Create World" }
                }
                button {
                    onclick: move |_| on_cancel.call(()),
                    disabled: *is_creating.read(),
                    class: "py-3 px-6 bg-gray-700 text-white border-0 rounded cursor-pointer",
                    "Cancel"
                }
            }
        }
    }
}

/// Editor for RuleSystemConfig - shows all fields
#[component]
fn RuleSystemConfigEditor(
    config: RuleSystemConfig,
    on_change: EventHandler<RuleSystemConfig>,
    disabled: bool,
) -> Element {
    let mut local_config = use_signal(|| config.clone());

    // Update local config when prop changes
    let config_clone = config.clone();
    use_effect(move || {
        local_config.set(config_clone.clone());
    });

    let config_read = local_config.read();
    let dice_str = match &config_read.dice_system {
        DiceSystem::D20 => "D20",
        DiceSystem::D100 => "D100",
        DiceSystem::Fate => "Fate",
        DiceSystem::DicePool { .. } => "DicePool",
        DiceSystem::Custom(_) => "Custom",
    };
    let comparison_str = match &config_read.success_comparison {
        SuccessComparison::GreaterOrEqual => "GreaterOrEqual",
        SuccessComparison::LessOrEqual => "LessOrEqual",
        SuccessComparison::Narrative => "Narrative",
    };

    rsx! {
        div { class: "p-4 bg-dark-bg border border-gray-700 rounded-b border-t-0",

            // Rule System Name
            div { class: "mb-3",
                label { class: "block text-gray-500 text-xs mb-1", "Rule System Name" }
                input {
                    r#type: "text",
                    value: "{config_read.name}",
                    oninput: move |e| {
                        let mut cfg = local_config.read().clone();
                        cfg.name = e.value();
                        local_config.set(cfg.clone());
                        on_change.call(cfg);
                    },
                    disabled: disabled,
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white box-border",
                }
            }

            // Rule System Description
            div { class: "mb-3",
                label { class: "block text-gray-500 text-xs mb-1", "Description" }
                textarea {
                    value: "{config_read.description}",
                    oninput: move |e| {
                        let mut cfg = local_config.read().clone();
                        cfg.description = e.value();
                        local_config.set(cfg.clone());
                        on_change.call(cfg);
                    },
                    disabled: disabled,
                    class: "w-full min-h-[60px] p-2 bg-dark-surface border border-gray-700 rounded text-white resize-y box-border",
                }
            }

            // Dice System
            div { class: "mb-3",
                label { class: "block text-gray-500 text-xs mb-1", "Dice System" }
                select {
                    value: "{dice_str}",
                    onchange: move |e| {
                        let mut cfg = local_config.read().clone();
                        cfg.dice_system = match e.value().as_str() {
                            "D20" => DiceSystem::D20,
                            "D100" => DiceSystem::D100,
                            "Fate" => DiceSystem::Fate,
                            _ => DiceSystem::Custom("Custom".to_string()),
                        };
                        local_config.set(cfg.clone());
                        on_change.call(cfg);
                    },
                    disabled: disabled,
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white",
                    option { value: "D20", "D20 (d20 + modifier)" }
                    option { value: "D100", "D100 (percentile)" }
                    option { value: "Fate", "Fate Dice (4dF)" }
                    option { value: "Custom", "Custom" }
                }
            }

            // Success Comparison
            div { class: "mb-3",
                label { class: "block text-gray-500 text-xs mb-1", "Success Comparison" }
                select {
                    value: "{comparison_str}",
                    onchange: move |e| {
                        let mut cfg = local_config.read().clone();
                        cfg.success_comparison = match e.value().as_str() {
                            "GreaterOrEqual" => SuccessComparison::GreaterOrEqual,
                            "LessOrEqual" => SuccessComparison::LessOrEqual,
                            _ => SuccessComparison::Narrative,
                        };
                        local_config.set(cfg.clone());
                        on_change.call(cfg);
                    },
                    disabled: disabled,
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white",
                    option { value: "GreaterOrEqual", "Roll >= Target (D20 style)" }
                    option { value: "LessOrEqual", "Roll <= Target (D100 style)" }
                    option { value: "Narrative", "Narrative (story-driven)" }
                }
            }

            // Skill Check Formula
            div { class: "mb-3",
                label { class: "block text-gray-500 text-xs mb-1", "Skill Check Formula" }
                input {
                    r#type: "text",
                    value: "{config_read.skill_check_formula}",
                    oninput: move |e| {
                        let mut cfg = local_config.read().clone();
                        cfg.skill_check_formula = e.value();
                        local_config.set(cfg.clone());
                        on_change.call(cfg);
                    },
                    disabled: disabled,
                    placeholder: "e.g., 1d20 + modifier vs DC",
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded text-white box-border",
                }
            }

            // Stats Section
            div { class: "mt-4",
                h4 { class: "text-gray-400 text-xs uppercase m-0 mb-2",
                    "Character Stats ({config_read.stat_definitions.len()})"
                }

                // Stats header row
                div { class: "grid grid-cols-[1fr_60px_60px_60px_60px_30px] gap-2 mb-1 px-1",
                    span { class: "text-gray-500 text-[0.625rem] uppercase", "Name" }
                    span { class: "text-gray-500 text-[0.625rem] uppercase text-center", "Abbr" }
                    span { class: "text-gray-500 text-[0.625rem] uppercase text-center", "Min" }
                    span { class: "text-gray-500 text-[0.625rem] uppercase text-center", "Max" }
                    span { class: "text-gray-500 text-[0.625rem] uppercase text-center", "Default" }
                    span {}
                }

                for (i, stat) in config_read.stat_definitions.iter().enumerate() {
                    div {
                        key: "{i}",
                        class: "grid grid-cols-[1fr_60px_60px_60px_60px_30px] gap-2 items-center mb-2",

                        input {
                            r#type: "text",
                            value: "{stat.name}",
                            oninput: move |e| {
                                let mut cfg = local_config.read().clone();
                                if let Some(s) = cfg.stat_definitions.get_mut(i) {
                                    s.name = e.value();
                                }
                                local_config.set(cfg.clone());
                                on_change.call(cfg);
                            },
                            disabled: disabled,
                            class: "p-1.5 bg-dark-surface border border-gray-700 rounded text-white",
                        }
                        input {
                            r#type: "text",
                            value: "{stat.abbreviation}",
                            oninput: move |e| {
                                let mut cfg = local_config.read().clone();
                                if let Some(s) = cfg.stat_definitions.get_mut(i) {
                                    s.abbreviation = e.value();
                                }
                                local_config.set(cfg.clone());
                                on_change.call(cfg);
                            },
                            disabled: disabled,
                            class: "p-1.5 bg-dark-surface border border-gray-700 rounded text-white text-center",
                        }
                        input {
                            r#type: "number",
                            value: "{stat.min_value}",
                            oninput: move |e| {
                                let mut cfg = local_config.read().clone();
                                if let Some(s) = cfg.stat_definitions.get_mut(i) {
                                    s.min_value = e.value().parse().unwrap_or(0);
                                }
                                local_config.set(cfg.clone());
                                on_change.call(cfg);
                            },
                            disabled: disabled,
                            class: "p-1.5 bg-dark-surface border border-gray-700 rounded text-white text-center",
                        }
                        input {
                            r#type: "number",
                            value: "{stat.max_value}",
                            oninput: move |e| {
                                let mut cfg = local_config.read().clone();
                                if let Some(s) = cfg.stat_definitions.get_mut(i) {
                                    s.max_value = e.value().parse().unwrap_or(20);
                                }
                                local_config.set(cfg.clone());
                                on_change.call(cfg);
                            },
                            disabled: disabled,
                            class: "p-1.5 bg-dark-surface border border-gray-700 rounded text-white text-center",
                        }
                        input {
                            r#type: "number",
                            value: "{stat.default_value}",
                            oninput: move |e| {
                                let mut cfg = local_config.read().clone();
                                if let Some(s) = cfg.stat_definitions.get_mut(i) {
                                    s.default_value = e.value().parse().unwrap_or(10);
                                }
                                local_config.set(cfg.clone());
                                on_change.call(cfg);
                            },
                            disabled: disabled,
                            class: "p-1.5 bg-dark-surface border border-gray-700 rounded text-white text-center",
                        }
                        button {
                            onclick: move |_| {
                                let mut cfg = local_config.read().clone();
                                cfg.stat_definitions.remove(i);
                                local_config.set(cfg.clone());
                                on_change.call(cfg);
                            },
                            disabled: disabled,
                            class: "py-1 px-2 bg-red-500 text-white border-0 rounded cursor-pointer text-xs",
                            "X"
                        }
                    }
                }

                // Add stat button
                button {
                    onclick: move |_| {
                        let mut cfg = local_config.read().clone();
                        cfg.stat_definitions.push(StatDefinition {
                            name: "New Stat".to_string(),
                            abbreviation: "NEW".to_string(),
                            min_value: 1,
                            max_value: 20,
                            default_value: 10,
                        });
                        local_config.set(cfg.clone());
                        on_change.call(cfg);
                    },
                    disabled: disabled,
                    class: "py-2 px-4 bg-gray-700 text-white border-0 rounded cursor-pointer text-xs",
                    "+ Add Stat"
                }
            }
        }
    }
}
