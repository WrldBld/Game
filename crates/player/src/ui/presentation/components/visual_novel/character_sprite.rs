//! Character sprite component for visual novel scenes
//!
//! Displays character sprites at different positions on screen.
//! Supports expression-based sprite swapping and mood badges from the
//! three-tier emotional model.

use dioxus::prelude::*;

use crate::application::application::dto::{
    CharacterData as SceneCharacterState, CharacterPosition,
};

/// Props for the CharacterSprite component
#[derive(Props, Clone, PartialEq)]
pub struct CharacterSpriteProps {
    /// Character data including position and sprite asset
    pub character: SceneCharacterState,
    /// Optional click handler
    #[props(default)]
    pub on_click: Option<EventHandler<String>>,
    /// Override expression (from dialogue typewriter)
    #[props(default)]
    pub override_expression: Option<String>,
}

/// Build an expression-specific sprite URL from the base sprite
///
/// Given a base sprite like "assets/marcus.png" and expression "happy",
/// returns "assets/marcus_happy.png"
fn build_expression_sprite_url(base_url: &str, expression: &str) -> String {
    if let Some(dot_pos) = base_url.rfind('.') {
        let (name, ext) = base_url.split_at(dot_pos);
        format!("{}_{}{}", name, expression.to_lowercase(), ext)
    } else {
        format!("{}_{}", base_url, expression.to_lowercase())
    }
}

/// Character sprite component - displays a character at their position
///
/// Uses `.sprite-left`, `.sprite-center`, `.sprite-right` Tailwind classes.
/// Characters who are speaking are highlighted with brightness and scale.
///
/// Supports the three-tier emotional model:
/// - Expression (Tier 3): Swaps sprite based on expression markers
/// - Mood (Tier 2): Shows mood badge overlay on sprite
#[component]
pub fn CharacterSprite(props: CharacterSpriteProps) -> Element {
    // Don't render off-screen or unknown position characters
    if matches!(
        props.character.position,
        CharacterPosition::OffScreen | CharacterPosition::Unknown
    ) {
        return rsx! {};
    }

    let position_class = match props.character.position {
        CharacterPosition::Left => "sprite-left",
        CharacterPosition::Center => "sprite-center",
        CharacterPosition::Right => "sprite-right",
        // OffScreen and Unknown are already handled above, but match must be exhaustive
        CharacterPosition::OffScreen | CharacterPosition::Unknown => return rsx! {},
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
    let full_style = format!(
        "{} transition: filter 0.3s, transform 0.3s; cursor: {};",
        speaking_style, cursor_style
    );

    // Determine the sprite URL based on expression
    // Priority: override_expression > character.expression > base sprite
    let expression = props
        .override_expression
        .as_ref()
        .or(props.character.expression.as_ref());

    let sprite_url = props.character.sprite_asset.as_ref().map(|base_url| {
        if let Some(expr) = expression {
            build_expression_sprite_url(base_url, expr)
        } else {
            base_url.clone()
        }
    });

    // Get mood for badge display
    let mood = props.character.mood.as_ref();

    rsx! {
        div {
            class: "character-sprite {position_class} relative",
            style: "{full_style}",
            onclick: move |_| {
                if let Some(ref handler) = props.on_click {
                    handler.call(character_id.clone());
                }
            },

            if let Some(ref url) = sprite_url {
                // Sprite image with fallback handling
                SpriteImage {
                    url: url.clone(),
                    fallback_url: props.character.sprite_asset.clone(),
                    alt: character_name.clone(),
                }

                // Mood badge overlay (Tier 2)
                if let Some(mood_text) = mood {
                    div {
                        class: "absolute top-2 right-2 px-2 py-0.5 rounded text-xs font-medium bg-amber-900/80 text-amber-200 border border-amber-700/50 backdrop-blur-sm",
                        "*{mood_text}*"
                    }
                }
            } else {
                // Placeholder sprite when no image is available
                PlaceholderSprite {
                    name: props.character.name.clone(),
                    is_speaking: props.character.is_speaking,
                    mood: mood.cloned(),
                }
            }
        }
    }
}

/// Sprite image with fallback support
///
/// Tries to load the expression-specific sprite, falls back to base sprite on error.
#[component]
fn SpriteImage(url: String, fallback_url: Option<String>, alt: String) -> Element {
    let mut current_url = use_signal(|| url.clone());
    let mut tried_fallback = use_signal(|| false);

    let fallback = fallback_url.clone();

    rsx! {
        img {
            src: "{current_url}",
            alt: "{alt}",
            class: "max-h-[400px] object-contain pointer-events-none",
            onerror: move |_| {
                // If we haven't tried the fallback yet and have one available
                if !*tried_fallback.read() {
                    if let Some(ref fb) = fallback {
                        current_url.set(fb.clone());
                        tried_fallback.set(true);
                    }
                }
            },
        }
    }
}

/// Placeholder sprite for characters without images
#[component]
fn PlaceholderSprite(name: String, is_speaking: bool, mood: Option<String>) -> Element {
    let border_class = if is_speaking {
        "border-[#d4af37]"
    } else {
        "border-gray-700"
    };

    rsx! {
        div {
            class: "w-[180px] h-[280px] bg-white/10 rounded-lg border-2 {border_class} flex flex-col items-center justify-center text-gray-400 relative",

            // Mood badge (if present)
            if let Some(ref mood_text) = mood {
                div {
                    class: "absolute top-2 right-2 px-2 py-0.5 rounded text-xs font-medium bg-amber-900/80 text-amber-200 border border-amber-700/50",
                    "*{mood_text}*"
                }
            }

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
                    on_click: props.on_character_click,
                }
            }
        }
    }
}
