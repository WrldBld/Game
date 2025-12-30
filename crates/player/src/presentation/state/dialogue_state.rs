//! Dialogue state management with typewriter effect
//!
//! Manages the current dialogue display including typewriter animation.

use dioxus::prelude::*;

use crate::application::dto::DialogueChoice;
use crate::application::ports::outbound::Platform;

/// Dialogue state for the visual novel UI
#[derive(Clone)]
pub struct DialogueState {
    /// Current speaker name
    pub speaker_name: Signal<String>,
    /// Full dialogue text (target for typewriter)
    pub full_text: Signal<String>,
    /// Currently displayed text (typewriter progress)
    pub displayed_text: Signal<String>,
    /// Whether typewriter is still animating
    pub is_typing: Signal<bool>,
    /// Available dialogue choices
    pub choices: Signal<Vec<DialogueChoice>>,
    /// Whether we're waiting for player input
    pub awaiting_input: Signal<bool>,
    /// Custom input text (for custom response choices)
    pub custom_input: Signal<String>,
    /// Speaker ID for targeting actions
    pub speaker_id: Signal<Option<String>>,
    /// Whether LLM is processing (show loading indicator)
    pub is_llm_processing: Signal<bool>,
}

impl DialogueState {
    /// Create a new DialogueState with empty values
    pub fn new() -> Self {
        Self {
            speaker_name: Signal::new(String::new()),
            full_text: Signal::new(String::new()),
            displayed_text: Signal::new(String::new()),
            is_typing: Signal::new(false),
            choices: Signal::new(Vec::new()),
            awaiting_input: Signal::new(false),
            custom_input: Signal::new(String::new()),
            speaker_id: Signal::new(None),
            is_llm_processing: Signal::new(false),
        }
    }

    /// Apply a new dialogue response (starts typewriter animation)
    pub fn apply_dialogue(
        &mut self,
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
    ) {
        self.speaker_id.set(Some(speaker_id));
        self.speaker_name.set(speaker_name);
        self.full_text.set(text);
        self.displayed_text.set(String::new());
        self.choices.set(choices);
        self.is_typing.set(true);
        self.awaiting_input.set(false);
        self.custom_input.set(String::new());
        self.is_llm_processing.set(false); // Clear processing indicator when response arrives
    }

    /// Skip to the end of the typewriter animation
    pub fn skip_typewriter(&mut self) {
        let full = self.full_text.read().clone();
        self.displayed_text.set(full);
        self.is_typing.set(false);
        self.awaiting_input.set(true);
    }

    /// Called when a character is typed (for manual typewriter control)
    pub fn type_character(&mut self) {
        let full = self.full_text.read();
        let mut displayed = self.displayed_text.read().clone();

        if displayed.len() < full.len() {
            if let Some(next_char) = full.chars().nth(displayed.len()) {
                displayed.push(next_char);
                self.displayed_text.set(displayed);
            }
        } else {
            // Finished typing
            self.is_typing.set(false);
            self.awaiting_input.set(true);
        }
    }

    /// Check if typewriter is complete
    pub fn is_typing_complete(&self) -> bool {
        let full_len = self.full_text.read().len();
        let displayed_len = self.displayed_text.read().len();
        displayed_len >= full_len
    }

    /// Get the delay for the next character based on punctuation
    pub fn get_char_delay(&self) -> u32 {
        let displayed = self.displayed_text.read();
        if let Some(last_char) = displayed.chars().last() {
            match last_char {
                '.' | '!' | '?' => 150,
                ',' | ';' | ':' => 80,
                _ => 30,
            }
        } else {
            30
        }
    }

    /// Clear the dialogue state
    pub fn clear(&mut self) {
        self.speaker_id.set(None);
        self.speaker_name.set(String::new());
        self.full_text.set(String::new());
        self.displayed_text.set(String::new());
        self.is_typing.set(false);
        self.choices.set(Vec::new());
        self.awaiting_input.set(false);
        self.custom_input.set(String::new());
        self.is_llm_processing.set(false);
    }

    /// Check if there's active dialogue to display
    pub fn has_dialogue(&self) -> bool {
        !self.full_text.read().is_empty()
    }

    /// Check if there are choices available
    pub fn has_choices(&self) -> bool {
        !self.choices.read().is_empty()
    }

    /// Check if custom input is available (any choice with is_custom_input)
    pub fn has_custom_input(&self) -> bool {
        self.choices.read().iter().any(|c| c.is_custom_input)
    }
}

impl Default for DialogueState {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook for running the typewriter effect
///
/// Call this in a component to drive the typewriter animation.
/// Returns true while typing is in progress.
pub fn use_typewriter_effect(dialogue_state: &mut DialogueState) {
    let platform = use_context::<Platform>();
    let is_typing = *dialogue_state.is_typing.read();
    let full_text = dialogue_state.full_text.clone();
    let displayed_text = dialogue_state.displayed_text.clone();
    let is_typing_signal = dialogue_state.is_typing.clone();
    let awaiting_signal = dialogue_state.awaiting_input.clone();

    use_future(move || {
        let platform = platform.clone();
        let full_text = full_text.clone();
        let mut displayed_text = displayed_text.clone();
        let mut is_typing_signal = is_typing_signal.clone();
        let mut awaiting_signal = awaiting_signal.clone();

        async move {
            if !is_typing {
                return;
            }

            let text = full_text.read().clone();
            let mut current = String::new();

            for ch in text.chars() {
                // Check if we should stop (user skipped)
                if !*is_typing_signal.read() {
                    break;
                }

                current.push(ch);
                displayed_text.set(current.clone());

                // Variable delay based on punctuation
                let delay = match ch {
                    '.' | '!' | '?' => 150,
                    ',' | ';' | ':' => 80,
                    _ => 30,
                };

                platform.sleep_ms(delay).await;
            }

            // Mark as complete
            is_typing_signal.set(false);
            awaiting_signal.set(true);
        }
    });
}
