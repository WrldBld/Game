//! Challenge list view components

use dioxus::prelude::*;
use std::collections::HashMap;
use wrldbldr_player_app::application::dto::{ChallengeData, ChallengeType};

/// Section for a challenge type
#[derive(Props, Clone, PartialEq)]
pub struct ChallengeTypeSectionProps {
    pub challenge_type: ChallengeType,
    pub challenges: Vec<ChallengeData>,
    pub skills_map: HashMap<String, String>,
    pub on_toggle_favorite: EventHandler<String>,
    pub on_toggle_active: EventHandler<String>,
    pub on_edit: EventHandler<ChallengeData>,
    pub on_delete: EventHandler<String>,
    pub on_trigger: Option<EventHandler<ChallengeData>>,
}

#[component]
pub fn ChallengeTypeSection(props: ChallengeTypeSectionProps) -> Element {
    let mut is_collapsed = use_signal(|| false);
    let arrow_icon = if *is_collapsed.read() { "▶" } else { "▼" };

    rsx! {
        div {
            class: "bg-black/20 rounded-lg overflow-hidden",

            // Section header
            div {
                class: "flex justify-between items-center px-4 py-3 bg-black/30 cursor-pointer",
                onclick: move |_| {
                    let current = *is_collapsed.read();
                    is_collapsed.set(!current);
                },

                div { class: "flex items-center gap-2",
                    h3 { class: "text-gray-200 m-0 text-sm font-semibold",
                        "{props.challenge_type.display_name()}"
                    }
                    span { class: "text-gray-500 text-xs",
                        "({props.challenges.len()})"
                    }
                }

                span { class: "text-gray-500",
                    "{arrow_icon}"
                }
            }

            // Challenge cards
            if !*is_collapsed.read() {
                div { class: "p-3 flex flex-col gap-2",
                    for challenge in props.challenges.iter() {
                        ChallengeCard {
                            key: "{challenge.id}",
                            challenge: challenge.clone(),
                            skill_name: props.skills_map.get(&challenge.skill_id).cloned().unwrap_or_else(|| "Unknown".to_string()),
                            on_toggle_favorite: props.on_toggle_favorite,
                            on_toggle_active: props.on_toggle_active,
                            on_edit: props.on_edit,
                            on_delete: props.on_delete,
                            on_trigger: props.on_trigger,
                        }
                    }
                }
            }
        }
    }
}

/// Individual challenge card
#[derive(Props, Clone, PartialEq)]
pub struct ChallengeCardProps {
    pub challenge: ChallengeData,
    pub skill_name: String,
    pub on_toggle_favorite: EventHandler<String>,
    pub on_toggle_active: EventHandler<String>,
    pub on_edit: EventHandler<ChallengeData>,
    pub on_delete: EventHandler<String>,
    pub on_trigger: Option<EventHandler<ChallengeData>>,
}

#[component]
pub fn ChallengeCard(props: ChallengeCardProps) -> Element {
    let challenge = props.challenge.clone();
    let id = challenge.id.clone();
    let id_for_favorite = id.clone();
    let id_for_active = id.clone();
    let id_for_delete = id.clone();
    let challenge_for_edit = challenge.clone();
    let challenge_for_trigger = challenge.clone();

    let opacity_class = if challenge.active {
        "opacity-100"
    } else {
        "opacity-60"
    };
    let border_class = if challenge.is_favorite {
        "border-amber-500"
    } else {
        "border-gray-700"
    };
    let star_icon = if challenge.is_favorite { "⭐" } else { "☆" };
    let active_bg = if challenge.active {
        "bg-emerald-500"
    } else {
        "bg-gray-700"
    };
    let active_text = if challenge.active {
        "Active"
    } else {
        "Inactive"
    };
    let extra_tags = if challenge.tags.len() > 2 {
        challenge.tags.len() - 2
    } else {
        0
    };

    rsx! {
        div {
            class: "flex items-center gap-3 p-3 bg-dark-bg border {border_class} rounded {opacity_class}",

            // Favorite star
            button {
                onclick: move |_| props.on_toggle_favorite.call(id_for_favorite.clone()),
                class: "bg-transparent border-0 cursor-pointer text-base p-0",
                "{star_icon}"
            }

            // Main info
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-2 mb-1",
                    span { class: "text-white font-medium whitespace-nowrap overflow-hidden text-ellipsis",
                        "{challenge.name}"
                    }
                    span { class: "text-gray-400 text-xs",
                        "{challenge.difficulty.display()}"
                    }
                }
                div { class: "flex gap-2 flex-wrap",
                    span { class: "text-blue-400 text-xs",
                        "{props.skill_name}"
                    }
                    if !challenge.description.is_empty() {
                        span { class: "text-gray-500 text-xs overflow-hidden text-ellipsis whitespace-nowrap max-w-[200px]",
                            "{challenge.description}"
                        }
                    }
                }
            }

            // Tags
            div { class: "flex gap-1 flex-wrap",
                for tag in challenge.tags.iter().take(2) {
                    span {
                        class: "px-1.5 py-0.5 bg-gray-700 text-gray-400 text-[0.625rem] rounded",
                        "{tag}"
                    }
                }
                if extra_tags > 0 {
                    span { class: "text-gray-500 text-[0.625rem]", "+{extra_tags}" }
                }
            }

            // Active toggle
            button {
                onclick: move |_| props.on_toggle_active.call(id_for_active.clone()),
                class: "px-2 py-1 {active_bg} text-white border-0 rounded cursor-pointer text-[0.625rem]",
                "{active_text}"
            }

            // Actions
            div { class: "flex gap-1",
                // Trigger button (only if handler provided)
                if let Some(ref on_trigger) = props.on_trigger {
                    button {
                        onclick: {
                            let trigger = *on_trigger;
                            let c = challenge_for_trigger.clone();
                            move |_| trigger.call(c.clone())
                        },
                        class: "px-2 py-1.5 bg-purple-500 text-white border-0 rounded cursor-pointer text-xs",
                        "▶"
                    }
                }

                button {
                    onclick: move |_| props.on_edit.call(challenge_for_edit.clone()),
                    class: "px-2 py-1.5 bg-blue-500 text-white border-0 rounded cursor-pointer text-xs",
                    "Edit"
                }

                button {
                    onclick: move |_| props.on_delete.call(id_for_delete.clone()),
                    class: "px-2 py-1.5 bg-red-500 text-white border-0 rounded cursor-pointer text-xs",
                    "×"
                }
            }
        }
    }
}
