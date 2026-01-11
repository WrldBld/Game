//! PC Creation View - Multi-step form for creating a player character

use dioxus::prelude::*;
use std::collections::HashMap;

use crate::infrastructure::spawn_task;

use crate::application::dto::CharacterSheetSchema;
use crate::application::services::CreatePlayerCharacterRequest;
use crate::presentation::components::schema_character_sheet::SchemaCharacterSheet;
use crate::presentation::services::{
    use_character_sheet_service, use_location_service, use_player_character_service,
    use_world_service,
};
use crate::presentation::state::use_session_state;
use crate::use_platform;

/// Wizard step enum
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum CreationStep {
    #[default]
    Basics,
    CharacterSheet,
    StartingLocation,
    Review,
}

/// Props for PC creation view
#[derive(Props, Clone, PartialEq)]
pub struct PCCreationProps {
    pub world_id: String,
}

/// PC Creation View - Multi-step form for creating a player character
#[component]
pub fn PCCreationView(props: PCCreationProps) -> Element {
    let navigator = use_navigator();
    let platform = use_platform();
    let _session_state = use_session_state();
    let pc_service = use_player_character_service();
    let location_service = use_location_service();
    let world_service = use_world_service();
    let sheet_service = use_character_sheet_service();

    // Step tracking
    let mut current_step = use_signal(|| CreationStep::Basics);

    // Form state - Step 1: Basics
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);

    // Form state - Step 2: Character Sheet
    let mut sheet_schema: Signal<Option<CharacterSheetSchema>> = use_signal(|| None);
    let sheet_values: Signal<HashMap<String, serde_json::Value>> = use_signal(HashMap::new);
    let mut sheet_loading = use_signal(|| false);

    // Form state - Step 3: Starting Location
    let mut available_locations: Signal<Vec<crate::application::services::LocationSummary>> =
        use_signal(Vec::new);
    let mut selected_location_id: Signal<Option<String>> = use_signal(|| None);
    let mut locations_loading = use_signal(|| false);

    // General state
    let mut is_creating = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    // Load character sheet schema from the game system
    {
        let world_id = props.world_id.clone();
        let world_svc = world_service.clone();
        let sheet_svc = sheet_service.clone();
        let platform_clone = platform.clone();
        use_effect(move || {
            let world_svc = world_svc.clone();
            let sheet_svc = sheet_svc.clone();
            let world_id_clone = world_id.clone();
            let plat = platform_clone.clone();
            sheet_loading.set(true);
            spawn_task(async move {
                // Try to get the schema from the world's sheet template endpoint
                match world_svc.get_sheet_template(&world_id_clone).await {
                    Ok(schema_json) => {
                        match serde_json::from_value::<CharacterSheetSchema>(schema_json) {
                            Ok(schema) => {
                                plat.log_info(&format!(
                                    "Loaded character sheet schema for system: {}",
                                    schema.system_id
                                ));
                                sheet_schema.set(Some(schema));
                            }
                            Err(e) => {
                                plat.log_warn(&format!("Failed to parse schema: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        // Fallback: try listing systems and use the first available
                        plat.log_info(&format!(
                            "Sheet template not available ({}), trying system list",
                            e
                        ));
                        if let Ok(systems) = sheet_svc.list_systems().await {
                            if let Some(system) =
                                systems.iter().find(|s| s.has_sheet_schema)
                            {
                                if let Ok(schema_json) = sheet_svc.get_schema(&system.id).await {
                                    if let Ok(schema) =
                                        serde_json::from_value::<CharacterSheetSchema>(schema_json)
                                    {
                                        plat.log_info(&format!(
                                            "Using fallback schema: {}",
                                            schema.system_id
                                        ));
                                        sheet_schema.set(Some(schema));
                                    }
                                }
                            }
                        }
                    }
                }
                sheet_loading.set(false);
            });
        });
    }

    // Load available locations
    {
        let world_id = props.world_id.clone();
        let loc_svc = location_service.clone();
        let platform_clone = platform.clone();
        use_effect(move || {
            let svc = loc_svc.clone();
            let world_id_clone = world_id.clone();
            let plat = platform_clone.clone();
            locations_loading.set(true);
            spawn_task(async move {
                match svc.list_locations(&world_id_clone).await {
                    Ok(locations) => {
                        available_locations.set(locations);
                    }
                    Err(e) => {
                        plat.log_error(&format!("Failed to load locations: {}", e));
                    }
                }
                locations_loading.set(false);
            });
        });
    }

    // Navigation handlers
    let go_next = move |_| {
        let step = *current_step.read();
        match step {
            CreationStep::Basics => {
                if name.read().trim().is_empty() {
                    error_message.set(Some("Character name is required".to_string()));
                    return;
                }
                current_step.set(CreationStep::CharacterSheet);
            }
            CreationStep::CharacterSheet => {
                current_step.set(CreationStep::StartingLocation);
            }
            CreationStep::StartingLocation => {
                if selected_location_id.read().is_none() {
                    error_message.set(Some("Please select a starting location".to_string()));
                    return;
                }
                current_step.set(CreationStep::Review);
            }
            CreationStep::Review => {
                // Will be handled by create button
            }
        }
        error_message.set(None);
    };

    let go_back = move |_| {
        let step = *current_step.read();
        match step {
            CreationStep::Basics => {
                navigator.go_back();
            }
            CreationStep::CharacterSheet => {
                current_step.set(CreationStep::Basics);
            }
            CreationStep::StartingLocation => {
                current_step.set(CreationStep::CharacterSheet);
            }
            CreationStep::Review => {
                current_step.set(CreationStep::StartingLocation);
            }
        }
        error_message.set(None);
    };

    let create_character = move |_| {
        let name_val = name.read().clone();
        let _desc_val = description.read().clone();
        let location_id = selected_location_id.read().clone();
        let sheet_vals = sheet_values.read().clone();
        let pc_svc = pc_service.clone();
        let nav = navigator;
        let world_id = props.world_id.clone();

        if name_val.trim().is_empty() {
            error_message.set(Some("Character name is required".to_string()));
            return;
        }

        if location_id.is_none() {
            error_message.set(Some("Please select a starting location".to_string()));
            return;
        }

        is_creating.set(true);
        error_message.set(None);

        spawn_task(async move {
            // Convert sheet values to JSON if present
            let sheet_data = if sheet_vals.is_empty() {
                None
            } else {
                serde_json::to_value(&sheet_vals).ok()
            };

            let starting_region_id = match location_id {
                Some(id) => Some(id),
                None => {
                    error_message.set(Some("Please select a starting location".to_string()));
                    is_creating.set(false);
                    return;
                }
            };

            let request = CreatePlayerCharacterRequest {
                name: name_val,
                user_id: None, // Will be set by the server from the connection context
                starting_region_id,
                sheet_data,
            };

            match pc_svc.create_pc(&world_id, &request).await {
                Ok(_pc) => {
                    // Navigate to PC View
                    nav.push(crate::routes::Route::PCViewRoute {
                        world_id: world_id.clone(),
                    });
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to create character: {}", e)));
                    is_creating.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "h-screen flex flex-col bg-dark-bg text-white",

            // Header
            div {
                class: "p-6 border-b border-gray-700",
                h1 {
                    class: "m-0 text-2xl text-white",
                    "Create Your Character"
                }
                p {
                    class: "mt-2 mb-0 text-gray-400 text-sm",
                    "Set up your character to join the adventure"
                }
            }

            // Progress indicator
            div {
                class: "flex gap-2 px-6 py-4 bg-black/20 border-b border-gray-700",
                StepIndicator {
                    number: 1,
                    label: "Basics",
                    is_active: *current_step.read() == CreationStep::Basics,
                    is_complete: *current_step.read() != CreationStep::Basics,
                }
                StepIndicator {
                    number: 2,
                    label: "Character Sheet",
                    is_active: *current_step.read() == CreationStep::CharacterSheet,
                    is_complete: matches!(*current_step.read(), CreationStep::StartingLocation | CreationStep::Review),
                }
                StepIndicator {
                    number: 3,
                    label: "Starting Location",
                    is_active: *current_step.read() == CreationStep::StartingLocation,
                    is_complete: *current_step.read() == CreationStep::Review,
                }
                StepIndicator {
                    number: 4,
                    label: "Review",
                    is_active: *current_step.read() == CreationStep::Review,
                    is_complete: false,
                }
            }

            // Content area
            div {
                class: "flex-1 overflow-y-auto p-8",

                // Error message
                if let Some(err) = error_message.read().as_ref() {
                    div {
                        class: "py-3 px-4 bg-red-500/10 border border-red-500 rounded-lg text-red-500 mb-4",
                        "{err}"
                    }
                }

                match *current_step.read() {
                    CreationStep::Basics => rsx! {
                        BasicsStep {
                            name: name.read().clone(),
                            description: description.read().clone(),
                            on_name_change: move |n| name.set(n),
                            on_description_change: move |d| description.set(d),
                        }
                    },
                    CreationStep::CharacterSheet => rsx! {
                        CharacterSheetStep {
                            schema: sheet_schema.read().clone(),
                            values: sheet_values,
                            loading: *sheet_loading.read(),
                        }
                    },
                    CreationStep::StartingLocation => rsx! {
                        StartingLocationStep {
                            locations: available_locations.read().clone(),
                            selected: selected_location_id.read().clone(),
                            loading: *locations_loading.read(),
                            on_select: move |id| selected_location_id.set(Some(id)),
                        }
                    },
                    CreationStep::Review => {
                        // Read signals before the iterator to avoid nested borrows
                        let selected_id = selected_location_id.read().clone();
                        let location_name = available_locations.read().iter()
                            .find(|l| Some(l.id.as_str()) == selected_id.as_deref())
                            .map(|l| l.name.clone())
                            .unwrap_or_default();
                        rsx! {
                            ReviewStep {
                                name: name.read().clone(),
                                description: description.read().clone(),
                                location: location_name,
                                has_sheet: !sheet_values.read().is_empty(),
                            }
                        }
                    }
                }
            }

            // Footer with navigation
            div {
                class: "py-4 px-6 border-t border-gray-700 flex justify-between",
                button {
                    onclick: go_back,
                    class: "py-2 px-4 bg-gray-700 text-white border-0 rounded-lg cursor-pointer",
                    if *current_step.read() == CreationStep::Basics {
                        "Cancel"
                    } else {
                        "Back"
                    }
                }
                div {
                    class: "flex gap-2",
                    if *current_step.read() == CreationStep::Review {
                        button {
                            onclick: create_character,
                            disabled: *is_creating.read(),
                            class: "py-2 px-6 bg-green-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                            if *is_creating.read() {
                                "Creating..."
                            } else {
                                "Create Character"
                            }
                        }
                    } else {
                        button {
                            onclick: go_next,
                            class: "py-2 px-6 bg-blue-500 text-white border-0 rounded-lg cursor-pointer font-medium",
                            "Next"
                        }
                    }
                }
            }
        }
    }
}

/// Step indicator component
#[component]
fn StepIndicator(number: u8, label: &'static str, is_active: bool, is_complete: bool) -> Element {
    // Extract conditional classes before rsx! block
    let bg_classes = if is_active || is_complete {
        "w-8 h-8 rounded-full flex items-center justify-center text-sm text-white font-semibold bg-blue-500"
    } else {
        "w-8 h-8 rounded-full flex items-center justify-center text-sm text-white font-semibold bg-gray-700"
    };

    let text_classes = if is_active {
        "text-sm font-medium text-white"
    } else if is_complete {
        "text-sm font-medium text-gray-400"
    } else {
        "text-sm font-medium text-gray-500"
    };

    rsx! {
        div {
            class: "flex items-center gap-2",
            div {
                class: "{bg_classes}",
                if is_complete { "✓" } else { "{number}" }
            }
            span {
                class: "{text_classes}",
                "{label}"
            }
            if number < 4 {
                div {
                    class: "w-[60px] h-0.5 bg-gray-700 mx-2",
                }
            }
        }
    }
}

/// Step 1: Character Basics
#[derive(Props, Clone, PartialEq)]
struct BasicsStepProps {
    name: String,
    description: String,
    on_name_change: EventHandler<String>,
    on_description_change: EventHandler<String>,
}

#[component]
fn BasicsStep(props: BasicsStepProps) -> Element {
    rsx! {
        div {
            class: "max-w-[600px] mx-auto",
            h2 {
                class: "mb-6 text-xl text-white",
                "Character Basics"
            }
            div {
                class: "flex flex-col gap-6",
                div {
                    label {
                        class: "block mb-2 text-gray-400 text-sm font-medium",
                        "Character Name *"
                    }
                    input {
                        r#type: "text",
                        value: "{props.name}",
                        oninput: move |e| props.on_name_change.call(e.value()),
                        placeholder: "Enter your character's name",
                        class: "w-full p-3 bg-dark-surface border border-gray-700 rounded-lg text-white text-base",
                    }
                }
                div {
                    label {
                        class: "block mb-2 text-gray-400 text-sm font-medium",
                        "Description (Optional)"
                    }
                    textarea {
                        value: "{props.description}",
                        oninput: move |e| props.on_description_change.call(e.value()),
                        placeholder: "Describe your character...",
                        rows: 4,
                        class: "w-full p-3 bg-dark-surface border border-gray-700 rounded-lg text-white text-base resize-y",
                    }
                }
            }
        }
    }
}

/// Step 2: Character Sheet
#[derive(Props, Clone, PartialEq)]
struct CharacterSheetStepProps {
    /// Character sheet schema from game system
    schema: Option<CharacterSheetSchema>,
    /// Signal for field values - child components read/write directly
    values: Signal<HashMap<String, serde_json::Value>>,
    loading: bool,
}

#[component]
fn CharacterSheetStep(props: CharacterSheetStepProps) -> Element {
    rsx! {
        div {
            class: "max-w-[800px] mx-auto",
            h2 {
                class: "mb-6 text-xl text-white",
                "Character Sheet"
            }
            if props.loading {
                div {
                    class: "text-center p-8 text-gray-400",
                    "Loading character sheet..."
                }
            } else if let Some(schema) = props.schema.as_ref() {
                SchemaCharacterSheet {
                    schema: schema.clone(),
                    values: props.values,
                }
            } else {
                div {
                    class: "p-6 bg-dark-surface rounded-lg border border-gray-700 text-center text-gray-400",
                    "No character sheet schema available for this world. You can skip this step."
                }
            }
        }
    }
}

/// Step 3: Starting Location
#[derive(Props, Clone, PartialEq)]
struct StartingLocationStepProps {
    locations: Vec<crate::application::services::LocationSummary>,
    selected: Option<String>,
    loading: bool,
    on_select: EventHandler<String>,
}

#[component]
fn StartingLocationStep(props: StartingLocationStepProps) -> Element {
    let locations = props.locations.clone();
    let selected = props.selected.clone();

    rsx! {
        div {
            class: "max-w-[800px] mx-auto",
            h2 {
                class: "mb-6 text-xl text-white",
                "Choose Starting Location"
            }
            if props.loading {
                div {
                    class: "text-center p-8 text-gray-400",
                    "Loading locations..."
                }
            } else if locations.is_empty() {
                div {
                    class: "p-6 bg-dark-surface rounded-lg border border-gray-700 text-center text-gray-400",
                    "No locations available. Please contact your DM."
                }
            } else {
                div {
                    class: "grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4",
                    {locations.into_iter().map(|location| {
                        let loc_id = location.id.clone();
                        let sel = selected.as_ref().map(|s| s == &loc_id).unwrap_or(false);
                        rsx! {
                            LocationCard {
                                location,
                                is_selected: sel,
                                on_select: move |_| props.on_select.call(loc_id.clone()),
                            }
                        }
                    })}
                }
            }
        }
    }
}

/// Location card component
#[derive(Props, Clone, PartialEq)]
struct LocationCardProps {
    location: crate::application::services::LocationSummary,
    is_selected: bool,
    on_select: EventHandler<()>,
}

#[component]
fn LocationCard(props: LocationCardProps) -> Element {
    // Extract conditional classes before rsx! block
    let card_classes = if props.is_selected {
        "p-6 border-2 rounded-lg cursor-pointer transition-all duration-200 bg-blue-500/10 border-blue-500"
    } else {
        "p-6 border-2 rounded-lg cursor-pointer transition-all duration-200 bg-dark-surface border-gray-700"
    };

    rsx! {
        div {
            onclick: move |_| props.on_select.call(()),
            class: "{card_classes}",
            h3 {
                class: "m-0 mb-2 text-white text-base",
                "{props.location.name}"
            }
            if let Some(loc_type) = props.location.location_type.as_ref() {
                p {
                    class: "m-0 text-gray-400 text-sm leading-6",
                    "{loc_type}"
                }
            }
            if props.is_selected {
                div {
                    class: "mt-3 text-blue-500 text-sm font-medium",
                    "✓ Selected"
                }
            }
        }
    }
}

/// Step 4: Review
#[derive(Props, Clone, PartialEq)]
struct ReviewStepProps {
    name: String,
    description: String,
    location: String,
    has_sheet: bool,
}

#[component]
fn ReviewStep(props: ReviewStepProps) -> Element {
    rsx! {
        div {
            class: "max-w-[600px] mx-auto",
            h2 {
                class: "mb-6 text-xl text-white",
                "Review Your Character"
            }
            div {
                class: "flex flex-col gap-6 p-6 bg-dark-surface rounded-lg border border-gray-700",
                div {
                    div {
                        class: "text-gray-400 text-sm mb-1",
                        "Name"
                    }
                    div {
                        class: "text-white text-base font-medium",
                        "{props.name}"
                    }
                }
                if !props.description.is_empty() {
                    div {
                        div {
                            class: "text-gray-400 text-sm mb-1",
                            "Description"
                        }
                        div {
                            class: "text-white text-sm leading-6",
                            "{props.description}"
                        }
                    }
                }
                div {
                    div {
                        class: "text-gray-400 text-sm mb-1",
                        "Starting Location"
                    }
                    div {
                        class: "text-white text-base font-medium",
                        "{props.location}"
                    }
                }
                div {
                    div {
                        class: "text-gray-400 text-sm mb-1",
                        "Character Sheet"
                    }
                    div {
                        class: "text-white text-sm",
                        if props.has_sheet {
                            "✓ Configured"
                        } else {
                            "Not configured"
                        }
                    }
                }
            }
            div {
                class: "mt-6 p-4 bg-blue-500/10 border border-blue-500 rounded-lg text-gray-400 text-sm",
                "Ready to create your character! Click 'Create Character' to begin your adventure."
            }
        }
    }
}
