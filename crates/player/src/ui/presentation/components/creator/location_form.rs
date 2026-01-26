//! Location Form - Create and edit locations

use dioxus::prelude::*;

use super::asset_gallery::AssetGallery;
use super::suggestion_button::{SuggestionButton, SuggestionType};
use crate::application::services::LocationFormData;
use crate::application::services::SuggestionContext;
use crate::infrastructure::spawn_task;
use crate::presentation::components::common::FormField;
use crate::presentation::services::use_location_service;

/// Location types
const LOCATION_TYPES: &[&str] = &[
    "Interior",
    "Exterior",
    "Wilderness",
    "Urban",
    "Dungeon",
    "Castle",
    "Village",
    "City",
    "Forest",
    "Mountain",
    "Cave",
    "Temple",
];

/// Location form for creating/editing locations
#[component]
pub fn LocationForm(
    location_id: String,
    world_id: String,
    locations_signal: Signal<Vec<crate::application::services::location_service::LocationSummary>>,
    on_close: EventHandler<()>,
) -> Element {
    let is_new = location_id.is_empty();
    let loc_service = use_location_service();

    // Form state
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut location_type = use_signal(|| "Interior".to_string());
    let mut atmosphere = use_signal(String::new);
    let mut notable_features = use_signal(String::new);
    let mut hidden_secrets = use_signal(String::new);
    let mut parent_location_id: Signal<Option<String>> = use_signal(|| None);
    let mut parent_locations: Signal<Vec<LocationFormData>> = use_signal(Vec::new);
    let mut presence_cache_ttl_hours: Signal<i32> = use_signal(|| 3);
    let mut use_llm_presence: Signal<bool> = use_signal(|| true);
    let mut is_loading = use_signal(|| !is_new);
    let mut is_saving = use_signal(|| false);
    let mut success_message: Signal<Option<String>> = use_signal(|| None);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    // Load location data if editing existing location
    {
        let loc_id_for_effect = location_id.clone();
        let loc_svc = loc_service.clone();
        let world_id_for_effect = world_id.clone();
        use_effect(move || {
            let loc_id = loc_id_for_effect.clone();
            let load_existing = !loc_id.is_empty();
            let world_id_clone = world_id_for_effect.clone();
            let svc = loc_svc.clone();

            spawn_task(async move {
                // Load parent locations list
                if let Ok(parents) = svc.list_locations(&world_id_clone).await {
                    // Convert LocationSummary to LocationFormData for the dropdown
                    let parent_data: Vec<LocationFormData> = parents
                        .iter()
                        .map(|summary| LocationFormData {
                            id: Some(summary.id.clone()),
                            name: summary.name.clone(),
                            description: None,
                            location_type: summary.location_type.clone(),
                            atmosphere: None,
                            notable_features: None,
                            hidden_secrets: None,
                            parent_location_id: None,
                            backdrop_asset: None,
                            backdrop_regions: Vec::new(),
                            presence_cache_ttl_hours: None,
                            use_llm_presence: None,
                        })
                        .collect();
                    parent_locations.set(parent_data);
                }

                // Load location data if editing
                if load_existing {
                    match svc.get_location(&loc_id).await {
                        Ok(loc_data) => {
                            name.set(loc_data.name);
                            description.set(loc_data.description.unwrap_or_default());
                            location_type.set(
                                loc_data
                                    .location_type
                                    .unwrap_or_else(|| "Interior".to_string()),
                            );
                            atmosphere.set(loc_data.atmosphere.unwrap_or_default());
                            notable_features.set(loc_data.notable_features.unwrap_or_default());
                            hidden_secrets.set(loc_data.hidden_secrets.unwrap_or_default());
                            parent_location_id.set(loc_data.parent_location_id);
                            presence_cache_ttl_hours
                                .set(loc_data.presence_cache_ttl_hours.unwrap_or(3));
                            use_llm_presence.set(loc_data.use_llm_presence.unwrap_or(true));
                            is_loading.set(false);
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to load location: {}", e)));
                            is_loading.set(false);
                        }
                    }
                } else {
                    is_loading.set(false);
                }
            });
        });
    }

    rsx! {
        div {
            class: "location-form flex flex-col h-full bg-dark-surface rounded-lg overflow-hidden",

            // Header
            div {
                class: "form-header flex justify-between items-center p-4 border-b border-gray-700",

                h2 {
                    class: "text-white m-0 text-xl",
                    if is_new { "New Location" } else { "Edit Location" }
                }

                button {
                    onclick: move |_| on_close.call(()),
                    class: "px-2 py-1 bg-transparent text-gray-400 border-none cursor-pointer text-xl",
                    "×"
                }
            }

            // Error/Success messages
            if let Some(msg) = error_message.read().as_ref() {
                div {
                    class: "px-4 py-3 bg-red-500/10 border-b border-red-500/30 text-red-500 text-sm",
                    "{msg}"
                }
            }
            if let Some(msg) = success_message.read().as_ref() {
                div {
                    class: "px-4 py-3 bg-green-500/10 border-b border-green-500/30 text-green-500 text-sm",
                    "{msg}"
                }
            }

            // Form content (scrollable)
            div {
                class: "form-content flex-1 overflow-y-auto p-4 flex flex-col gap-4",

                if *is_loading.read() {
                    div {
                        class: "flex items-center justify-center p-8 text-gray-500",
                        "Loading location data..."
                    }
                } else {

                // Name field with suggest button
                FormField {
                    label: "Name",
                    required: true,
                    children: rsx! {
                        div { class: "flex gap-2",
                            input {
                                r#type: "text",
                                value: "{name}",
                                oninput: move |e| name.set(e.value()),
                                placeholder: "Enter location name...",
                                class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            }
                            SuggestionButton {
                                suggestion_type: SuggestionType::LocationName,
                                world_id: world_id.clone(),
                                context: SuggestionContext {
                                    entity_type: Some(location_type.read().clone()),
                                    ..Default::default()
                                },
                                on_select: move |value| name.set(value),
                            }
                        }
                    }
                }

                // Location type dropdown
                FormField {
                    label: "Type",
                    required: false,
                    children: rsx! {
                        select {
                            value: "{location_type}",
                            onchange: move |e| location_type.set(e.value()),
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",

                            for lt in LOCATION_TYPES {
                                option { value: "{lt}", "{lt}" }
                            }
                        }
                    }
                }

                // Description field
                FormField {
                    label: "Description",
                    required: false,
                    children: rsx! {
                        div { class: "flex flex-col gap-2",
                            textarea {
                                value: "{description}",
                                oninput: move |e| description.set(e.value()),
                                placeholder: "What does this place look like? What stands out?",
                                class: "w-full min-h-[80px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                            }
                            div { class: "flex justify-end",
                                SuggestionButton {
                                    suggestion_type: SuggestionType::LocationDescription,
                                    world_id: world_id.clone(),
                                    context: SuggestionContext {
                                        entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                        entity_type: Some(location_type.read().clone()),
                                        ..Default::default()
                                    },
                                    on_select: move |value| description.set(value),
                                }
                            }
                        }
                    }
                }

                // Atmosphere field
                FormField {
                    label: "Atmosphere",
                    required: false,
                    children: rsx! {
                        div { class: "flex gap-2",
                            input {
                                r#type: "text",
                                value: "{atmosphere}",
                                oninput: move |e| atmosphere.set(e.value()),
                                placeholder: "The mood and feeling of this place...",
                                class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                            }
                            SuggestionButton {
                                suggestion_type: SuggestionType::LocationAtmosphere,
                                world_id: world_id.clone(),
                                context: SuggestionContext {
                                    entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                    entity_type: Some(location_type.read().clone()),
                                    additional_context: if description.read().is_empty() { None } else { Some(description.read().clone()) },
                                    ..Default::default()
                                },
                                on_select: move |value| atmosphere.set(value),
                            }
                        }
                    }
                }

                // Notable features field
                FormField {
                    label: "Notable Features",
                    required: false,
                    children: rsx! {
                        div { class: "flex flex-col gap-2",
                            textarea {
                                value: "{notable_features}",
                                oninput: move |e| notable_features.set(e.value()),
                                placeholder: "Points of interest, interactable objects...",
                                class: "w-full min-h-[60px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                            }
                            div { class: "flex justify-end",
                                SuggestionButton {
                                    suggestion_type: SuggestionType::LocationFeatures,
                                    world_id: world_id.clone(),
                                    context: SuggestionContext {
                                        entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                        entity_type: Some(location_type.read().clone()),
                                        hints: if atmosphere.read().is_empty() { None } else { Some(atmosphere.read().clone()) },
                                        ..Default::default()
                                    },
                                    on_select: move |value| notable_features.set(value),
                                }
                            }
                        }
                    }
                }

                // Hidden secrets field
                FormField {
                    label: "Hidden Secrets",
                    required: false,
                    children: rsx! {
                        div { class: "flex flex-col gap-2",
                            textarea {
                                value: "{hidden_secrets}",
                                oninput: move |e| hidden_secrets.set(e.value()),
                                placeholder: "Things players might discover with investigation...",
                                class: "w-full min-h-[60px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                            }
                            div { class: "flex justify-end",
                                SuggestionButton {
                                    suggestion_type: SuggestionType::LocationSecrets,
                                    world_id: world_id.clone(),
                                    context: SuggestionContext {
                                        entity_name: if name.read().is_empty() { None } else { Some(name.read().clone()) },
                                        entity_type: Some(location_type.read().clone()),
                                        additional_context: if notable_features.read().is_empty() { None } else { Some(notable_features.read().clone()) },
                                        ..Default::default()
                                    },
                                    on_select: move |value| hidden_secrets.set(value),
                                }
                            }
                        }
                        }
                    }

                    // Staging Settings section
                    div {
                        class: "staging-settings mt-4",
                        h3 { class: "text-gray-400 text-sm uppercase mb-3", "Staging Settings" }

                        // Presence Cache TTL
                        FormField {
                            label: "Presence Cache TTL (hours)",
                            required: false,
                            children: rsx! {
                                div { class: "flex flex-col gap-2",
                                    input {
                                        r#type: "number",
                                        min: 0,
                                        max: 24,
                                        value: "{presence_cache_ttl_hours}",
                                        oninput: move |e| {
                                            if let Ok(val) = e.value().parse::<i32>() {
                                                presence_cache_ttl_hours.set(val.clamp(0, 24));
                                            }
                                        },
                                        class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                    }
                                    p {
                                        class: "text-xs text-gray-500",
                                        "How long NPC presence approvals remain valid (in game time). 0 = use world default."
                                    }
                                }
                            }
                        }

                        // Use LLM Presence toggle
                        FormField {
                            label: "Use LLM for Staging",
                            required: false,
                            children: rsx! {
                                div { class: "flex flex-col gap-2",
                                    label {
                                        class: "flex items-center gap-2 cursor-pointer",
                                        input {
                                            r#type: "checkbox",
                                            checked: *use_llm_presence.read(),
                                            onchange: move |e| use_llm_presence.set(e.checked()),
                                            class: "w-4 h-4 rounded border-gray-700 bg-dark-bg text-blue-500 focus:ring-blue-500",
                                        }
                                        "Enable AI-driven staging decisions for this location"
                                    }
                                    p {
                                        class: "text-xs text-gray-500 mt-1",
                                        "When enabled, the AI considers story context to suggest NPC presence. When disabled, only rule-based logic is used."
                                    }
                                }
                            }
                        }
                    }

                    // Parent location section
                    FormField {
                        label: "Parent Location",
                        required: false,
                        children: rsx! {
                            select {
                                value: parent_location_id.read().as_deref().unwrap_or(""),
                                onchange: move |e| {
                                    let val = e.value();
                                    parent_location_id.set(if val.is_empty() { None } else { Some(val) });
                                },
                                class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",

                                option { value: "", "None" }
                                for parent in parent_locations.read().iter() {
                                    // Don't allow selecting self as parent
                                    if parent.id.as_ref() != Some(&location_id) {
                                        option {
                                            value: "{parent.id.as_ref().unwrap_or(&String::new())}",
                                            "{parent.name}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Asset Gallery section
                    div {
                        class: "assets-section mt-4",

                        h3 { class: "text-gray-400 text-sm uppercase mb-3", "Assets" }

                        AssetGallery {
                            world_id: world_id.clone(),
                            entity_type: "location".to_string(),
                            entity_id: location_id.clone(),
                        }
                    }
                }
            }

            // Footer with action buttons
            div {
                class: "form-footer flex justify-end gap-2 p-4 border-t border-gray-700",

                button {
                    onclick: move |_| on_close.call(()),
                    class: "px-4 py-2 bg-transparent text-gray-400 border border-gray-700 rounded cursor-pointer",
                    disabled: *is_saving.read(),
                    "Cancel"
                }

                button {
                    class: format!(
                        "px-4 py-2 bg-green-500 text-white border-none rounded cursor-pointer font-medium {}",
                        if *is_saving.read() { "opacity-60" } else { "opacity-100" }
                    ),
                    disabled: *is_saving.read(),
                    onclick: {
                        let loc_svc = loc_service.clone();
                        move |_| {
                            let loc_name = name.read().clone();
                            if loc_name.is_empty() {
                                error_message.set(Some("Location name is required".to_string()));
                                return;
                            }

                            error_message.set(None);
                            success_message.set(None);
                            is_saving.set(true);

                            let loc_id = location_id.clone();
                            let on_close = on_close;
                            let svc = loc_svc.clone();
                            let world_id_clone = world_id.clone();

                            spawn_task(async move {
                                    let loc_data = LocationFormData {
                                        id: if is_new { None } else { Some(loc_id.clone()) },
                                        name: name.read().clone(),
                                        description: {
                                            let desc = description.read().clone();
                                            if desc.is_empty() { None } else { Some(desc) }
                                        },
                                        location_type: {
                                            let lt = location_type.read().clone();
                                            if lt.is_empty() { None } else { Some(lt) }
                                        },
                                        atmosphere: {
                                            let atm = atmosphere.read().clone();
                                            if atm.is_empty() { None } else { Some(atm) }
                                        },
                                        notable_features: {
                                            let nf = notable_features.read().clone();
                                            if nf.is_empty() { None } else { Some(nf) }
                                        },
                                        hidden_secrets: {
                                            let hs = hidden_secrets.read().clone();
                                            if hs.is_empty() { None } else { Some(hs) }
                                        },
                                         parent_location_id: parent_location_id.read().clone(),
                                         backdrop_asset: None,
                                         backdrop_regions: Vec::new(),
                                         presence_cache_ttl_hours: if *presence_cache_ttl_hours.read() == 0 {
                                             None  // 0 means use world default
                                         } else {
                                             Some(*presence_cache_ttl_hours.read())
                                         },
                                         use_llm_presence: Some(*use_llm_presence.read()),
                                    };

                                    match if is_new {
                                        svc.create_location(&world_id_clone, &loc_data).await
                                    } else {
                                        svc.update_location(&loc_id, &loc_data).await
                                    } {
                                        Ok(saved_location) => {
                                            // Update the locations signal reactively
                                            if is_new {
                                                // Add new location to list
                                                let summary = crate::application::services::location_service::LocationSummary {
                                                    id: saved_location.id.clone().unwrap_or_default(),
                                                    name: saved_location.name.clone(),
                                                    location_type: saved_location.location_type.clone(),
                                                };
                                                locations_signal.write().push(summary);
                                            } else {
                                                // Update existing location in list
                                                if let Some(id) = &saved_location.id {
                                                    let mut locs = locations_signal.write();
                                                    if let Some(existing) = locs.iter_mut().find(|l| l.id == *id) {
                                                        existing.name = saved_location.name.clone();
                                                        existing.location_type = saved_location.location_type.clone();
                                                    }
                                                }
                                            }

                                            success_message.set(Some(if is_new {
                                                "Location created successfully".to_string()
                                            } else {
                                                "Location saved successfully".to_string()
                                            }));
                                            is_saving.set(false);
                                            // Close form - let the user see the success message
                                            on_close.call(());
                                        }
                                        Err(e) => {
                                            error_message.set(Some(format!("Save failed: {}", e)));
                                            is_saving.set(false);
                                        }
                                }
                            });
                        }
                    },
                    if *is_saving.read() { "Saving..." } else { if is_new { "Create" } else { "Save" } }
                }
            }
        }
    }
}
