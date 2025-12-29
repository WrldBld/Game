//! Spectator View - Watch the game without participating
//!
//! Displays the same scene as PC view but without interaction capabilities.
//! Spectators can see the backdrop, characters, and dialogue but cannot make choices.

use dioxus::prelude::*;

use crate::presentation::components::visual_novel::{Backdrop, CharacterLayer, EmptyDialogueBox};
use crate::presentation::state::{use_dialogue_state, use_game_state, use_typewriter_effect};

/// Spectator View - read-only view of the game
///
/// Connection handling and back navigation are provided by WorldSessionLayout wrapper.
#[component]
pub fn SpectatorView() -> Element {
    // Get global state from context
    let game_state = use_game_state();
    let mut dialogue_state = use_dialogue_state();

    // Run typewriter effect for read-only dialogue display
    use_typewriter_effect(&mut dialogue_state);

    // Read scene characters from game state (reactive)
    let scene_characters = game_state.scene_characters.read().clone();

    // Get conversation history for the log
    let mut conversation_log = use_signal(|| Vec::<ConversationEntry>::new());

    // Track dialogue updates to add to log
    {
        let dialogue_state_clone = dialogue_state.clone();
        use_effect(move || {
            let is_typing = *dialogue_state_clone.is_typing.read();
            let has_dialogue = !dialogue_state_clone.full_text.read().is_empty();
            let current_speaker = dialogue_state_clone.speaker_name.read().clone();
            let current_text = dialogue_state_clone.displayed_text.read().clone();

            // Only add to log once typing is complete
            if !is_typing && has_dialogue && !current_text.is_empty() {
                let mut log = conversation_log.write();

                // Check if we should add a new entry (different speaker or new dialogue)
                let should_add = log.is_empty()
                    || log.last().map(|e| &e.speaker) != Some(&current_speaker)
                    || log.last().map(|e| &e.text) != Some(&current_text);

                if should_add {
                    log.push(ConversationEntry {
                        speaker: current_speaker,
                        text: current_text,
                    });
                }
            }
        });
    }

    // Read current state for rendering
    let speaker_name = dialogue_state.speaker_name.read().clone();
    let displayed_text = dialogue_state.displayed_text.read().clone();
    let is_typing = *dialogue_state.is_typing.read();
    let has_dialogue = dialogue_state.has_dialogue();
    let is_llm_processing = *dialogue_state.is_llm_processing.read();

    rsx! {
        div {
            class: "spectator-view h-full flex flex-col relative bg-gradient-to-b from-dark-surface to-dark-purple-end",

            // Spectator badge (top right)
            div {
                class: "absolute top-4 right-4 z-[100] px-4 py-2 bg-purple-500/20 text-purple-300 border border-purple-500 rounded-lg text-sm",
                "Spectating"
            }

            // Visual novel stage (2.3.1 - Scene display)
            Backdrop {
                image_url: game_state.backdrop_url(),

                // Character layer with real scene characters
                CharacterLayer {
                    characters: scene_characters,
                    on_character_click: None, // Spectators cannot interact
                }
            }

            // Dialogue box (fixed at bottom) - 2.3.2 Read-only dialogue display
            div {
                class: "dialogue-container absolute bottom-0 left-0 right-0 z-10",

                if has_dialogue {
                    SpectatorDialogueBox {
                        speaker_name: speaker_name.clone(),
                        dialogue_text: displayed_text.clone(),
                        is_typing: is_typing,
                        is_llm_processing: is_llm_processing,
                    }
                } else {
                    EmptyDialogueBox {}
                }
            }

            // Conversation log (2.3.3 - Scrollable history) - only show if log has entries
            if !conversation_log.read().is_empty() {
                ConversationLog {
                    entries: conversation_log.read().clone(),
                }
            }
        }
    }
}

/// A read-only dialogue box for spectators (no choices)
///
/// Displays the speaker name and dialogue text with typewriter animation,
/// but does not show choice buttons or custom input fields.
#[derive(Props, Clone, PartialEq)]
pub struct SpectatorDialogueBoxProps {
    /// Speaker name
    pub speaker_name: String,
    /// Dialogue text to display
    pub dialogue_text: String,
    /// Whether typewriter is still animating
    #[props(default = false)]
    pub is_typing: bool,
    /// Whether NPC is currently thinking
    #[props(default = false)]
    pub is_llm_processing: bool,
}

/// Spectator-specific dialogue box (no interaction)
#[component]
fn SpectatorDialogueBox(props: SpectatorDialogueBoxProps) -> Element {
    let has_speaker = !props.speaker_name.is_empty();

    rsx! {
        div {
            class: "spectator-dialogue-box bg-black/85 border-t-2 border-purple-500 p-4 max-h-[200px]",

            // Speaker name plate
            if has_speaker {
                div {
                    class: "spectator-character-name text-purple-300 font-semibold text-sm mb-2 uppercase tracking-wider",
                    "{props.speaker_name}"
                }
            }

            // Dialogue text with typewriter cursor
            div {
                class: "spectator-dialogue-text-container min-h-[60px] overflow-hidden",

                if props.is_llm_processing {
                    p {
                        class: "spectator-dialogue-text text-gray-400 italic text-[0.95rem] leading-6 m-0",

                        "NPC is thinking"

                        // Animated ellipsis
                        span {
                            class: "animate-[ellipsis_1.5s_steps(4,end)_infinite]",
                            "..."
                        }
                    }
                } else {
                    p {
                        class: "spectator-dialogue-text text-gray-200 text-[0.95rem] leading-6 m-0",

                        "{props.dialogue_text}"

                        // Blinking cursor during typing
                        if props.is_typing {
                            span {
                                class: "typewriter-cursor ml-0.5 animate-[blink_0.7s_step-end_infinite]",
                                "▌"
                            }
                        }
                    }
                }
            }

            // Spectator indicator (instead of choices)
            div {
                class: "mt-3 pt-3 border-t border-gray-700 text-purple-500 text-xs text-center italic",
                "Spectating - No choices available"
            }
        }
    }
}

/// A conversation log entry
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConversationEntry {
    /// Speaker name
    speaker: String,
    /// Dialogue text
    text: String,
}

/// Conversation log component - scrollable history
///
/// Shows a history of all dialogue exchanges in chronological order.
#[derive(Props, Clone, PartialEq)]
pub struct ConversationLogProps {
    /// Log entries
    pub(crate) entries: Vec<ConversationEntry>,
}

#[component]
fn ConversationLog(props: ConversationLogProps) -> Element {
    rsx! {
        div {
            class: "conversation-log absolute bottom-[220px] left-0 right-0 h-[180px] bg-black/70 border-t border-b border-gray-700 overflow-y-auto p-4 text-[0.85rem] leading-snug",

            for (idx, entry) in props.entries.iter().enumerate() {
                div {
                    key: "{idx}",
                    class: "mb-2 pb-2 border-b border-gray-800",

                    div {
                        class: "text-purple-300 font-semibold text-xs uppercase tracking-wider",
                        "{entry.speaker}"
                    }

                    div {
                        class: "text-gray-300 mt-1 break-words",
                        "{entry.text}"
                    }
                }
            }
        }
    }
}
