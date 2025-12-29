//! NPC motivation tracking component
//!
//! Displays and allows editing of NPC demeanor and immediate goals.
//!
//! Note: "Demeanor" here is free-form emotional guidance for the DM,
//! distinct from DispositionLevel (per NPC-PC relationship tracking).

use dioxus::prelude::*;

use crate::presentation::components::dm_panel::director_generate_modal::DirectorGenerateModal;
use wrldbldr_player_app::application::dto::CharacterData as SceneCharacterState;

/// NPC motivation state
#[derive(Clone, PartialEq)]
pub struct Motivation {
    /// Current demeanor/emotional guidance (free-form, for DM reference)
    pub demeanor: String,
    /// Immediate goal the NPC is pursuing
    pub goal: String,
}

/// Props for the NPCMotivation component
#[derive(Props, Clone, PartialEq)]
pub struct NPCMotivationProps {
    /// World ID for asset generation
    pub world_id: String,
    /// The character being tracked
    pub character: SceneCharacterState,
    /// Current motivation state
    pub motivation: Motivation,
    /// Handler called when motivation is updated
    pub on_update: EventHandler<Motivation>,
    /// Optional character description for pre-populating generation prompts
    #[props(default)]
    pub character_description: Option<String>,
}

/// Demeanor options available for selection (motivational states, not disposition levels)
const DEMEANOR_OPTIONS: &[&str] = &[
    "Friendly",
    "Neutral",
    "Suspicious",
    "Hostile",
    "Curious",
    "Fearful",
    "Greedy",
    "Dutiful",
    "Conflicted",
    "Determined",
];

/// NPCMotivation component - Track NPC demeanor and goals
///
/// Displays NPC name with editable demeanor dropdown and goal text input.
/// Helps DMs manage NPC state during gameplay.
#[component]
pub fn NPCMotivation(props: NPCMotivationProps) -> Element {
    let char_name = props.character.name.clone();
    let char_id = props.character.id.clone();
    let motivation_demeanor = props.motivation.demeanor.clone();
    let motivation_goal = props.motivation.goal.clone();
    let mut show_generate_modal = use_signal(|| false);
    let mut generate_asset_type = use_signal(|| "portrait".to_string());

    // Clone for each closure to avoid move conflicts
    let motivation_for_demeanor = props.motivation.clone();
    let motivation_for_goal = props.motivation.clone();

    rsx! {
        div {
            class: "npc-motivation p-3 bg-dark-bg rounded-lg mb-2 border-l-4 border-purple-500",

            // Character name header
            h4 {
                class: "text-purple-500 text-sm m-0 mb-3",
                "{char_name}"
            }

            // Demeanor selector
            div {
                class: "mb-3",

                label {
                    class: "block text-gray-400 text-xs uppercase mb-1",
                    "Demeanor"
                }

                select {
                    value: "{motivation_demeanor}",
                    onchange: move |e| {
                        let mut updated = motivation_for_demeanor.clone();
                        updated.demeanor = e.value();
                        props.on_update.call(updated);
                    },
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm cursor-pointer",

                    for demeanor in DEMEANOR_OPTIONS.iter() {
                        option {
                            value: "{demeanor}",
                            "{demeanor}"
                        }
                    }
                    option {
                        value: "Custom",
                        "Custom..."
                    }
                }
            }

            // Goal input
            div {
                class: "mb-3",
                label {
                    class: "block text-gray-400 text-xs uppercase mb-1",
                    "Immediate Goal"
                }

                input {
                    r#type: "text",
                    value: "{motivation_goal}",
                    placeholder: "What does this NPC want right now?",
                    oninput: move |e| {
                        let mut updated = motivation_for_goal.clone();
                        updated.goal = e.value();
                        props.on_update.call(updated);
                    },
                    class: "w-full p-2 bg-dark-surface border border-gray-700 rounded-md text-white text-sm box-border transition-colors",
                }
            }

            // Generate asset buttons
            div {
                class: "flex gap-2 mt-3",
                button {
                    onclick: move |_| {
                        generate_asset_type.set("portrait".to_string());
                        show_generate_modal.set(true);
                    },
                    class: "flex-1 p-2 bg-purple-500 text-white border-0 rounded-md cursor-pointer text-xs font-medium",
                    "🎨 Generate Portrait"
                }
                button {
                    onclick: move |_| {
                        generate_asset_type.set("sprite".to_string());
                        show_generate_modal.set(true);
                    },
                    class: "flex-1 p-2 bg-purple-500 text-white border-0 rounded-md cursor-pointer text-xs font-medium",
                    "🖼️ Generate Sprite"
                }
            }
        }

        // Generate modal
        if *show_generate_modal.read() {
            DirectorGenerateModal {
                world_id: props.world_id.clone(),
                entity_type: "character".to_string(),
                entity_id: char_id.clone(),
                asset_type: generate_asset_type.read().clone(),
                character_name: char_name.clone(),
                initial_prompt: props.character_description.clone().unwrap_or_default(),
                on_close: move |_| show_generate_modal.set(false),
            }
        }
    }
}
