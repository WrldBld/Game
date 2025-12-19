//! Role selection view

use dioxus::prelude::*;

use wrldbldr_protocol::ParticipantRole;

#[component]
pub fn RoleSelect(on_select_role: EventHandler<ParticipantRole>) -> Element {
    rsx! {
        div {
            class: "role-select flex flex-col items-center justify-center h-full bg-gradient-to-br from-dark-surface to-dark-gradient-end",

            h2 {
                class: "text-white mb-8 text-3xl",
                "Select Your Role"
            }

            div {
                class: "flex gap-6 flex-wrap justify-center",

                // Dungeon Master card
                RoleCard {
                    title: "Dungeon Master",
                    description: "Direct the story, approve NPC responses, and manage the game world.",
                    icon: "🎭",
                    color: "#ef4444",
                    on_click: move |_| on_select_role.call(ParticipantRole::DungeonMaster)
                }

                // Player card
                RoleCard {
                    title: "Player",
                    description: "Experience the story as a player character in a visual novel style.",
                    icon: "⚔️",
                    color: "#3b82f6",
                    on_click: move |_| on_select_role.call(ParticipantRole::Player)
                }

                // Spectator card
                RoleCard {
                    title: "Spectator",
                    description: "Watch the game unfold without participating.",
                    icon: "👁️",
                    color: "#8b5cf6",
                    on_click: move |_| on_select_role.call(ParticipantRole::Spectator)
                }
            }
        }
    }
}

#[component]
fn RoleCard(
    title: &'static str,
    description: &'static str,
    icon: &'static str,
    color: &'static str,
    on_click: EventHandler<()>,
) -> Element {
    // Extract conditional classes before rsx! block
    let (border_class, title_class) = match color {
        "#ef4444" => ("border-red-500", "text-red-500"),
        "#3b82f6" => ("border-blue-500", "text-blue-500"),
        "#8b5cf6" => ("border-purple-500", "text-purple-500"),
        _ => ("border-gray-500", "text-gray-500"),
    };

    let button_classes = format!("bg-dark-bg border-2 {} rounded-xl p-8 w-[250px] cursor-pointer transition-all duration-200 hover:scale-105 hover:shadow-xl", border_class);
    let title_classes = format!("mb-2 text-xl {}", title_class);

    rsx! {
        button {
            onclick: move |_| on_click.call(()),
            class: "{button_classes}",

            div {
                class: "text-5xl mb-4",
                "{icon}"
            }
            h3 {
                class: "{title_classes}",
                "{title}"
            }
            p {
                class: "text-gray-400 text-sm leading-6",
                "{description}"
            }
        }
    }
}
