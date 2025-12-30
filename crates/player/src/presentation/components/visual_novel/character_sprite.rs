//! Character sprite component for visual novel scenes
//!
//! Displays character sprites at different positions on screen.

use dioxus::prelude::*;

use crate::application::dto::websocket_messages::{SceneCharacterState, CharacterPosition};

/// Props for the CharacterSprite component
#[derive(Props, Clone, PartialEq)]
pub struct CharacterSpriteProps {
    /// Character data including position and sprite asset
    pub character: SceneCharacterState,
    /// Optional click handler
    #[props(default)]
    pub on_click: Option<EventHandler<String>>,
}

/// Character sprite component - displays a character at their position
///
/// Uses `.sprite-left`, `.sprite-center`, `.sprite-right` Tailwind classes.
/// Characters who are speaking are highlighted with brightness and scale.
#[component]
pub fn CharacterSprite(props: CharacterSpriteProps) -> Element {
    // Don't render off-screen characters
    if props.character.position == CharacterPosition::OffScreen {
        return rsx! {};
    }

    let position_class = match props.character.position {
        CharacterPosition::Left => "sprite-left",
        CharacterPosition::Center => "sprite-center",
        CharacterPosition::Right => "sprite-right",
        CharacterPosition::OffScreen => return rsx! {},
    };

    // Speaking characters get highlighted
    let speaking_style = if props.character.is_speaking {
        "filter: brightness(1.1) drop-shadow(0 0 10px rgba(212, 175, 55, 0.5)); transform: scale(1.02);"
    } else {
        "filter: brightness(0.85);"
    };

    let character_id = props.character.id.clone();
    let character_name = props.character.name.clone();
    let has_click = props.on_click.is_some();
    let cursor_style = if has_click { "pointer" } else { "default" };
    let full_style = format!("{} transition: filter 0.3s, transform 0.3s; cursor: {};", speaking_style, cursor_style);

    rsx! {
        div {
            class: "character-sprite {position_class}",
            style: "{full_style}",
            onclick: move |_| {
                if let Some(ref handler) = props.on_click {
                    handler.call(character_id.clone());
                }
            },

            if let Some(ref sprite_url) = props.character.sprite_asset {
                img {
                    src: "{sprite_url}",
                    alt: "{character_name}",
                    class: "max-h-[400px] object-contain pointer-events-none",
                }
            } else {
                // Placeholder sprite when no image is available
                PlaceholderSprite {
                    name: props.character.name.clone(),
                    is_speaking: props.character.is_speaking,
                }
            }
        }
    }
}

/// Placeholder sprite for characters without images
#[component]
fn PlaceholderSprite(name: String, is_speaking: bool) -> Element {
    let border_class = if is_speaking {
        "border-[#d4af37]"
    } else {
        "border-gray-700"
    };

    rsx! {
        div {
            class: "w-[180px] h-[280px] bg-white/10 rounded-lg border-2 {border_class} flex flex-col items-center justify-center text-gray-400",

            // Character silhouette icon
            div {
                class: "text-6xl mb-4 opacity-50",
                "👤"
            }

            // Character name
            div {
                class: "text-sm text-center px-2",
                "{name}"
            }
        }
    }
}

/// Character layer component - container for all character sprites
///
/// Provides proper z-indexing and positioning context for sprites.
#[derive(Props, Clone, PartialEq)]
pub struct CharacterLayerProps {
    /// Characters to display
    pub characters: Vec<SceneCharacterState>,
    /// Optional click handler for characters
    #[props(default)]
    pub on_character_click: Option<EventHandler<String>>,
}

#[component]
pub fn CharacterLayer(props: CharacterLayerProps) -> Element {
    rsx! {
        div {
            class: "character-layer absolute inset-0 pointer-events-none z-[1]",

            for character in props.characters.iter() {
                CharacterSprite {
                    key: "{character.id}",
                    character: character.clone(),
                    on_click: props.on_character_click.clone(),
                }
            }
        }
    }
}
