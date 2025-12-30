//! Challenge editor form component

use crate::presentation::services::use_challenge_service;
use dioxus::prelude::*;
use wrldbldr_player_app::application::dto::{
    ChallengeData, ChallengeDifficulty, ChallengeOutcomes, ChallengeType, SkillData,
};

/// Props for ChallengeFormModal
#[derive(Props, Clone, PartialEq)]
pub struct ChallengeFormModalProps {
    pub world_id: String,
    pub challenge: Option<ChallengeData>,
    pub skills: Vec<SkillData>,
    pub on_save: EventHandler<ChallengeData>,
    pub on_close: EventHandler<()>,
}

/// Modal for creating/editing a challenge
#[component]
pub fn ChallengeFormModal(props: ChallengeFormModalProps) -> Element {
    let is_edit = props.challenge.is_some();
    let initial = props
        .challenge
        .clone()
        .unwrap_or_default_challenge(&props.world_id);

    let mut name = use_signal(|| initial.name.clone());
    let mut description = use_signal(|| initial.description.clone());
    let mut skill_id = use_signal(|| initial.skill_id.clone());
    let mut challenge_type = use_signal(|| initial.challenge_type);
    let mut difficulty = use_signal(|| initial.difficulty.clone());
    let mut success_desc = use_signal(|| initial.outcomes.success.description.clone());
    let mut failure_desc = use_signal(|| initial.outcomes.failure.description.clone());
    let mut tags_str = use_signal(|| initial.tags.join(", "));
    let mut is_saving = use_signal(|| false);
    let mut save_error: Signal<Option<String>> = use_signal(|| None);
    let mut validation_errors: Signal<Vec<String>> = use_signal(Vec::new);

    let challenge_id = initial.id.clone();
    let world_id = props.world_id.clone();

    // Get challenge service
    let challenge_service = use_challenge_service();

    let world_id_for_save = world_id.clone();
    let challenge_id_for_save = challenge_id.clone();
    let challenge_service_for_save = challenge_service.clone();

    let handle_save = move |_| {
        // Validate inputs
        let mut errors = Vec::new();

        if name.read().trim().is_empty() {
            errors.push("Challenge name is required".to_string());
        }

        if skill_id.read().is_empty() {
            errors.push("Skill is required".to_string());
        }

        // Validate difficulty-specific values
        match &*difficulty.read() {
            ChallengeDifficulty::Dc { value } => {
                if *value == 0 {
                    errors.push("DC value must be greater than 0".to_string());
                }
            }
            ChallengeDifficulty::Percentage { value } => {
                if *value > 100 {
                    errors.push("Percentage must be between 0 and 100".to_string());
                }
            }
            _ => {}
        }

        if !errors.is_empty() {
            validation_errors.set(errors);
            return;
        }

        validation_errors.set(Vec::new());
        is_saving.set(true);
        save_error.set(None);

        let challenge_data = ChallengeData {
            id: challenge_id_for_save.clone(),
            world_id: world_id_for_save.clone(),
            scene_id: None,
            name: name.read().clone(),
            description: description.read().clone(),
            challenge_type: *challenge_type.read(),
            skill_id: skill_id.read().clone(),
            difficulty: difficulty.read().clone(),
            outcomes: ChallengeOutcomes {
                success: wrldbldr_player_app::application::dto::Outcome {
                    description: success_desc.read().clone(),
                    triggers: vec![],
                },
                failure: wrldbldr_player_app::application::dto::Outcome {
                    description: failure_desc.read().clone(),
                    triggers: vec![],
                },
                partial: None,
                critical_success: None,
                critical_failure: None,
            },
            trigger_conditions: vec![],
            prerequisite_challenges: vec![],
            active: true,
            order: 0,
            is_favorite: false,
            tags: tags_str
                .read()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        };

        let on_save = props.on_save;
        let is_edit = is_edit;
        let service = challenge_service_for_save.clone();
        let wid = world_id_for_save.clone();

        spawn(async move {
            let result = if is_edit {
                service.update_challenge(&challenge_data).await
            } else {
                service.create_challenge(&wid, &challenge_data).await
            };

            match result {
                Ok(saved) => {
                    on_save.call(saved);
                }
                Err(e) => {
                    save_error.set(Some(format!("Failed to save challenge: {}", e)));
                    is_saving.set(false);
                }
            }
        });
    };

    let header_text = if is_edit {
        "Edit Challenge"
    } else {
        "New Challenge"
    };
    let is_disabled = *is_saving.read() || name.read().is_empty() || skill_id.read().is_empty();
    let opacity_class = if is_disabled {
        "opacity-50"
    } else {
        "opacity-100"
    };
    let button_text = if *is_saving.read() {
        "Saving..."
    } else if is_edit {
        "Update"
    } else {
        "Create"
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/90 flex items-center justify-center z-[1100]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-dark-surface rounded-xl w-[90%] max-w-[600px] max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center px-6 py-4 border-b border-gray-700",
                    h3 { class: "text-white m-0",
                        "{header_text}"
                    }
                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-0 text-gray-400 text-2xl cursor-pointer",
                        "×"
                    }
                }

                // Form
                div {
                    class: "p-6 flex flex-col gap-4",

                    // Validation errors
                    if !validation_errors.read().is_empty() {
                        div {
                            class: "px-4 py-3 bg-red-500/10 border-l-3 border-l-red-500 rounded text-red-500 text-sm",
                            for error in validation_errors.read().iter() {
                                div { class: "mb-1", "• {error}" }
                            }
                        }
                    }

                    // Name
                    div {
                        label { class: "block text-gray-400 text-xs mb-1", "Name *" }
                        input {
                            r#type: "text",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                            placeholder: "e.g., Investigate the Crime Scene",
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                        }
                    }

                    // Description
                    div {
                        label { class: "block text-gray-400 text-xs mb-1", "Description" }
                        textarea {
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                            placeholder: "What this challenge represents...",
                            rows: "2",
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y box-border",
                        }
                    }

                    // Type and Skill row
                    div { class: "grid grid-cols-2 gap-4",
                        // Type
                        div {
                            label { class: "block text-gray-400 text-xs mb-1", "Type" }
                            select {
                                value: "{challenge_type.read():?}",
                                onchange: move |e| {
                                    let val = e.value();
                                    challenge_type.set(match val.as_str() {
                                        "SkillCheck" => ChallengeType::SkillCheck,
                                        "AbilityCheck" => ChallengeType::AbilityCheck,
                                        "SavingThrow" => ChallengeType::SavingThrow,
                                        "OpposedCheck" => ChallengeType::OpposedCheck,
                                        "ComplexChallenge" => ChallengeType::ComplexChallenge,
                                        _ => ChallengeType::SkillCheck,
                                    });
                                },
                                class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                for ct in ChallengeType::all() {
                                    option { value: "{ct:?}", "{ct.display_name()}" }
                                }
                            }
                        }

                        // Skill
                        div {
                            label { class: "block text-gray-400 text-xs mb-1", "Skill *" }
                            select {
                                value: "{skill_id}",
                                onchange: move |e| skill_id.set(e.value()),
                                class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                option { value: "", "Select a skill..." }
                                for skill in props.skills.iter() {
                                    option { value: "{skill.id}", "{skill.name}" }
                                }
                            }
                        }
                    }

                    // Difficulty
                    div {
                        label { class: "block text-gray-400 text-xs mb-1", "Difficulty" }
                        div { class: "flex gap-2",
                            select {
                                value: match &*difficulty.read() {
                                    ChallengeDifficulty::Dc { .. } => "dc",
                                    ChallengeDifficulty::Percentage { .. } => "percentage",
                                    ChallengeDifficulty::Descriptor { .. } => "descriptor",
                                    ChallengeDifficulty::Opposed => "opposed",
                                    ChallengeDifficulty::Custom { .. } => "custom",
                                },
                                onchange: move |e| {
                                    difficulty.set(match e.value().as_str() {
                                        "dc" => ChallengeDifficulty::Dc { value: 10 },
                                        "percentage" => ChallengeDifficulty::Percentage { value: 50 },
                                        "descriptor" => ChallengeDifficulty::Descriptor { value: "Moderate".to_string() },
                                        "opposed" => ChallengeDifficulty::Opposed,
                                        "custom" => ChallengeDifficulty::Custom { value: "".to_string() },
                                        _ => ChallengeDifficulty::Dc { value: 10 },
                                    });
                                },
                                class: "p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                option { value: "dc", "DC" }
                                option { value: "percentage", "Percentage" }
                                option { value: "descriptor", "Descriptor" }
                                option { value: "opposed", "Opposed" }
                                option { value: "custom", "Custom" }
                            }

                            // Value input (for DC/Percentage/Descriptor/Custom)
                            match &*difficulty.read() {
                                ChallengeDifficulty::Dc { value } => rsx! {
                                    input {
                                        r#type: "number",
                                        value: "{value}",
                                        oninput: move |e| {
                                            if let Ok(v) = e.value().parse() {
                                                difficulty.set(ChallengeDifficulty::Dc { value: v });
                                            }
                                        },
                                        class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                    }
                                },
                                ChallengeDifficulty::Percentage { value } => rsx! {
                                    input {
                                        r#type: "number",
                                        value: "{value}",
                                        min: "0",
                                        max: "100",
                                        oninput: move |e| {
                                            if let Ok(v) = e.value().parse() {
                                                difficulty.set(ChallengeDifficulty::Percentage { value: v });
                                            }
                                        },
                                        class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                    }
                                },
                                ChallengeDifficulty::Descriptor { value } => rsx! {
                                    select {
                                        value: "{value}",
                                        onchange: move |e| {
                                            difficulty.set(ChallengeDifficulty::Descriptor { value: e.value() });
                                        },
                                        class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                        option { value: "Trivial", "Trivial" }
                                        option { value: "Easy", "Easy" }
                                        option { value: "Routine", "Routine" }
                                        option { value: "Moderate", "Moderate" }
                                        option { value: "Challenging", "Challenging" }
                                        option { value: "Hard", "Hard" }
                                        option { value: "Very Hard", "Very Hard" }
                                        option { value: "Extreme", "Extreme" }
                                    }
                                },
                                ChallengeDifficulty::Custom { value } => rsx! {
                                    input {
                                        r#type: "text",
                                        value: "{value}",
                                        placeholder: "Custom difficulty...",
                                        oninput: move |e| {
                                            difficulty.set(ChallengeDifficulty::Custom { value: e.value() });
                                        },
                                        class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white",
                                    }
                                },
                                ChallengeDifficulty::Opposed => rsx! {
                                    span { class: "text-gray-500 p-2", "Roll vs opponent" }
                                },
                            }
                        }
                    }

                    // Success outcome
                    div {
                        label { class: "block text-emerald-500 text-xs mb-1", "Success Outcome" }
                        textarea {
                            value: "{success_desc}",
                            oninput: move |e| success_desc.set(e.value()),
                            placeholder: "What happens on success...",
                            rows: "2",
                            class: "w-full p-2 bg-dark-bg border border-emerald-500 rounded text-white resize-y box-border",
                        }
                    }

                    // Failure outcome
                    div {
                        label { class: "block text-red-500 text-xs mb-1", "Failure Outcome" }
                        textarea {
                            value: "{failure_desc}",
                            oninput: move |e| failure_desc.set(e.value()),
                            placeholder: "What happens on failure...",
                            rows: "2",
                            class: "w-full p-2 bg-dark-bg border border-red-500 rounded text-white resize-y box-border",
                        }
                    }

                    // Tags
                    div {
                        label { class: "block text-gray-400 text-xs mb-1", "Tags (comma-separated)" }
                        input {
                            r#type: "text",
                            value: "{tags_str}",
                            oninput: move |e| tags_str.set(e.value()),
                            placeholder: "investigation, social, combat",
                            class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white box-border",
                        }
                    }

                    // Error
                    if let Some(err) = save_error.read().as_ref() {
                        div { class: "text-red-500 text-sm", "{err}" }
                    }

                    // Actions
                    div { class: "flex gap-3 justify-end mt-2",
                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "px-4 py-2 bg-gray-700 text-white border-0 rounded-lg cursor-pointer",
                            "Cancel"
                        }
                        button {
                            onclick: handle_save,
                            disabled: is_disabled,
                            class: "px-4 py-2 bg-blue-500 text-white border-0 rounded-lg cursor-pointer {opacity_class}",
                            "{button_text}"
                        }
                    }
                }
            }
        }
    }
}

// Helper trait for default challenge
trait DefaultChallenge {
    fn unwrap_or_default_challenge(self, world_id: &str) -> ChallengeData;
}

impl DefaultChallenge for Option<ChallengeData> {
    fn unwrap_or_default_challenge(self, world_id: &str) -> ChallengeData {
        self.unwrap_or(ChallengeData {
            id: uuid::Uuid::new_v4().to_string(),
            world_id: world_id.to_string(),
            scene_id: None,
            name: String::new(),
            description: String::new(),
            challenge_type: ChallengeType::SkillCheck,
            skill_id: String::new(),
            difficulty: ChallengeDifficulty::default(),
            outcomes: ChallengeOutcomes::default(),
            trigger_conditions: vec![],
            prerequisite_challenges: vec![],
            active: true,
            order: 0,
            is_favorite: false,
            tags: vec![],
        })
    }
}
