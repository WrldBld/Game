//! Scene preview component for DM view
//!
//! Shows a smaller version of what players see including backdrop and character sprites.

use dioxus::prelude::*;
use wrldbldr_player_app::application::dto::CharacterData as SceneCharacterState;

use crate::presentation::utils::CharacterPositionStyle;

/// Scene preview state for UI display (view model)
#[derive(Clone, PartialEq)]
pub struct ScenePreviewState {
    /// Scene name/title
    pub name: String,
    /// Backdrop image URL
    pub backdrop_url: Option<String>,
    /// Current dialogue text being shown
    pub dialogue_text: String,
    /// Speaker of the dialogue
    pub speaker_name: String,
}

/// Props for the ScenePreview component
#[derive(Props, Clone, PartialEq)]
pub struct ScenePreviewProps {
    /// Optional scene data to display
    #[props(default)]
    pub scene: Option<ScenePreviewState>,
    /// Characters currently visible in the scene
    #[props(default)]
    pub characters: Vec<SceneCharacterState>,
}

/// ScenePreview component - Shows smaller version of PC view
///
/// Displays a compact version of the current scene with backdrop and character sprites,
/// along with current dialogue. Useful for DMs to see what players are experiencing.
#[component]
pub fn ScenePreview(props: ScenePreviewProps) -> Element {
    // Extract background style before rsx! block
    let bg_style = match &props.scene {
        Some(scene) => match &scene.backdrop_url {
            Some(url) => format!(
                "background-image: url('{}'); background-size: cover; background-position: center;",
                url
            ),
            None => "background: linear-gradient(to bottom, #1a1a2e, #2d1b3d);".to_string(),
        },
        None => "background: linear-gradient(to bottom, #1a1a2e, #2d1b3d);".to_string(),
    };

    let has_dialogue = props
        .scene
        .as_ref()
        .map(|s| !s.dialogue_text.is_empty())
        .unwrap_or(false);

    rsx! {
        div {
            class: "scene-preview h-full w-full relative overflow-hidden rounded-lg",

            // Backdrop
            div {
                class: "absolute inset-0 bg-cover bg-center",
                style: "{bg_style}",
            }

            // Vignette effect
            div {
                class: "absolute inset-0 pointer-events-none shadow-[inset_0_0_100px_rgba(0,0,0,0.4)]",
            }

            // Character sprites
            div {
                class: "absolute bottom-[20%] w-full flex justify-around items-end px-8",

                for character in props.characters.iter() {
                    CharacterSpritePreview {
                        character: character.clone(),
                    }
                }
            }

            // Dialogue box overlay (small version)
            if has_dialogue {
                if let Some(scene) = &props.scene {
                    div {
                        class: "absolute bottom-0 left-0 right-0 p-4 border-t border-gray-700 bg-gradient-to-t from-dark-surface to-dark-surface/80",

                        // Speaker name
                        if !scene.speaker_name.is_empty() {
                            div {
                                class: "text-blue-500 text-sm font-semibold mb-1",
                                "{scene.speaker_name}"
                            }
                        }

                        // Dialogue text
                        p {
                            class: "text-white text-sm leading-snug m-0",
                            "{scene.dialogue_text}"
                        }
                    }
                }
            }

            // Empty state
            if props.scene.is_none() {
                div {
                    class: "absolute inset-0 flex items-center justify-center text-gray-500 text-sm",
                    "No scene loaded"
                }
            }
        }
    }
}

/// Character sprite preview component
#[component]
fn CharacterSpritePreview(character: SceneCharacterState) -> Element {
    let position_classes = character.position.as_tailwind_classes();

    let sprite_content = match &character.sprite_asset {
        Some(url) => rsx! {
            img {
                src: "{url}",
                alt: "{character.name}",
                class: "w-full h-full object-contain",
            }
        },
        None => rsx! {
            div {
                class: "w-full h-full bg-blue-500 bg-opacity-20 rounded flex items-center justify-center text-xs text-blue-500",
                "[{character.name}]"
            }
        },
    };

    rsx! {
        div {
            class: "relative w-20 h-30 flex flex-col items-center {position_classes}",

            // Sprite container
            div {
                class: "w-full h-full relative overflow-visible",
                {sprite_content}
            }

            // Name label
            div {
                class: "absolute -bottom-6 left-1/2 -translate-x-1/2 whitespace-nowrap text-xs text-gray-400 bg-black bg-opacity-50 py-1 px-2 rounded",
                "{character.name}"
            }

            // Mood indicator (Tier 2 emotional state)
            if let Some(mood) = &character.mood {
                if !mood.is_empty() {
                    div {
                        class: "absolute -top-2 -right-2 bg-amber-600 text-white text-xs py-1 px-2 rounded-md whitespace-nowrap",
                        "{mood}"
                    }
                }
            }

            // Expression indicator (Tier 3 transient expression)
            if let Some(expression) = &character.expression {
                if !expression.is_empty() {
                    div {
                        class: "absolute -bottom-8 left-1/2 -translate-x-1/2 bg-purple-500 text-white text-xs py-0.5 px-2 rounded whitespace-nowrap",
                        "[{expression}]"
                    }
                }
            }
        }
    }
}
