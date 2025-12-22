//! Skills Display Component
//!
//! Displays player's skills with modifiers in a compact panel.

use dioxus::prelude::*;

/// A player skill with its properties
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSkillData {
    /// Unique skill ID
    pub id: String,
    /// Human-readable skill name
    pub name: String,
    /// Skill category (e.g., "Combat", "Magic", "Social")
    pub category: String,
    /// Skill modifier (positive or negative)
    pub modifier: i32,
    /// Whether character is proficient in this skill
    pub proficient: bool,
}

/// Props for SkillsDisplay component
#[derive(Props, Clone, PartialEq)]
pub struct SkillsDisplayProps {
    /// List of player skills to display
    pub skills: Vec<PlayerSkillData>,
    /// Called when closing the skills panel
    pub on_close: EventHandler<()>,
}

/// Skills Display Component
///
/// Shows a compact panel of the player's skills, grouped by category,
/// with modifiers highlighted and proficient skills marked.
#[component]
pub fn SkillsDisplay(props: SkillsDisplayProps) -> Element {
    // Group skills by category
    let mut categories: std::collections::BTreeMap<String, Vec<PlayerSkillData>> =
        std::collections::BTreeMap::new();

    for skill in props.skills.iter() {
        categories
            .entry(skill.category.clone())
            .or_insert_with(Vec::new)
            .push(skill.clone());
    }

    rsx! {
        // Modal overlay
        div {
            id: "skills-overlay",
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| {
                props.on_close.call(());
            },

            // Modal content
            div {
                id: "skills-modal",
                class: "bg-gradient-to-br from-dark-surface to-dark-bg p-8 rounded-2xl max-w-2xl w-[90%] max-h-[80vh] overflow-y-auto border-2 border-blue-500 shadow-[0_20px_60px_rgba(59,130,246,0.2)]",
                onclick: move |e: dioxus::prelude::MouseEvent| {
                    e.stop_propagation();
                },

                // Header
                div {
                    class: "flex justify-between items-center mb-6",

                    h2 {
                        class: "text-blue-500 m-0 text-2xl",
                        "Skills"
                    }

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "bg-transparent border-0 text-gray-400 cursor-pointer text-2xl p-0 hover:text-gray-300",
                        "×"
                    }
                }

                // Skills by category
                if categories.is_empty() {
                    p {
                        class: "text-gray-400 text-center my-8",
                        "No skills available"
                    }
                } else {
                    {
                        categories.iter().map(|(category, skills)| {
                            rsx! {
                                div {
                                    key: "{category}",
                                    class: "mb-6",

                                    // Category header
                                    h3 {
                                        class: "text-amber-500 text-sm uppercase tracking-wide m-0 mb-3",
                                        "{category}"
                                    }

                                    // Skills in this category
                                    div {
                                        class: "flex flex-col gap-2",

                                        {
                                            skills.iter().map(|skill| {
                                                // CRITICAL: Extract conditional classes BEFORE rsx! - no inline if in class strings
                                                let modifier_color_class = if skill.modifier >= 0 {
                                                    "text-green-500"
                                                } else {
                                                    "text-red-500"
                                                };

                                                let border_color_class = if skill.proficient {
                                                    "border-l-amber-500"
                                                } else {
                                                    "border-l-transparent"
                                                };

                                                rsx! {
                                                    div {
                                                        key: "{skill.id}",
                                                        class: "flex items-center justify-between px-3 py-3 bg-black/30 rounded-lg border-l-[3px] {border_color_class}",

                                                        // Skill name
                                                        div {
                                                            class: "flex items-center gap-2",

                                                            span {
                                                                class: "text-white flex-1",
                                                                "{skill.name}"
                                                            }

                                                            // Proficiency indicator
                                                            if skill.proficient {
                                                                span {
                                                                    class: "text-amber-500 text-xs font-bold",
                                                                    "Prof"
                                                                }
                                                            }
                                                        }

                                                        // Modifier
                                                        div {
                                                            class: "{modifier_color_class} font-bold text-base min-w-[3rem] text-right",
                                                            if skill.modifier >= 0 {
                                                                "+{skill.modifier}"
                                                            } else {
                                                                "{skill.modifier}"
                                                            }
                                                        }
                                                    }
                                                }
                                            })
                                        }
                                    }
                                }
                            }
                        })
                    }
                }

                // Footer
                div {
                    class: "mt-6 pt-4 border-t border-white/10",

                    button {
                        onclick: move |_| props.on_close.call(()),
                        class: "w-full py-3 bg-gray-700 text-white border-0 rounded-lg cursor-pointer text-sm hover:bg-gray-600",
                        "Close"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_core::NoOpMutations;

    #[test]
    fn skills_display_smoke_renders() {
        #[component]
        fn App() -> Element {
            rsx! {
                SkillsDisplay {
                    skills: vec![],
                    on_close: move |_| {},
                }
            }
        }

        let mut dom = VirtualDom::new(App);
        let mut muts = NoOpMutations;
        dom.rebuild(&mut muts);
    }
}
