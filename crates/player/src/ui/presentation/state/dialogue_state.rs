//! Dialogue state management with typewriter effect
//!
//! Manages the current dialogue display including typewriter animation.
//! Supports expression markers for the three-tier emotional model.

use dioxus::prelude::*;
use wrldbldr_domain::{parse_dialogue, DialogueMarker, ParsedDialogue};

use crate::application::dto::DialogueChoice;
use crate::use_platform;

/// A marker with its trigger position in the clean text
#[derive(Clone, Debug)]
pub struct PositionedMarker {
    /// Position in clean text where this marker triggers
    pub clean_position: usize,
    /// The original marker data
    pub marker: DialogueMarker,
}

/// Dialogue state for the visual novel UI
#[derive(Clone)]
pub struct DialogueState {
    /// Current speaker name
    pub speaker_name: Signal<String>,
    /// Full dialogue text (original with markers)
    pub full_text: Signal<String>,
    /// Clean text (markers stripped, for display)
    pub clean_text: Signal<String>,
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
    /// Version counter - increments when new dialogue arrives (for reactivity)
    pub dialogue_version: Signal<u32>,

    // Expression system (Tier 3 of emotional model)
    /// Current expression for the speaker (changes during typewriter)
    pub current_expression: Signal<Option<String>>,
    /// Current mood tag to display next to speaker name (Tier 2)
    pub current_mood: Signal<Option<String>>,
    /// Markers parsed from the dialogue, with clean text positions
    pub positioned_markers: Signal<Vec<PositionedMarker>>,
    /// Index of the next marker to trigger
    pub next_marker_index: Signal<usize>,
    /// Current action being performed (displayed as stage direction)
    pub current_action: Signal<Option<String>>,
    /// Current conversation ID for tracking multi-turn conversations
    pub conversation_id: Signal<Option<String>>,
}

impl DialogueState {
    /// Create a new DialogueState with empty values
    pub fn new() -> Self {
        Self {
            speaker_name: Signal::new(String::new()),
            full_text: Signal::new(String::new()),
            clean_text: Signal::new(String::new()),
            displayed_text: Signal::new(String::new()),
            is_typing: Signal::new(false),
            choices: Signal::new(Vec::new()),
            awaiting_input: Signal::new(false),
            custom_input: Signal::new(String::new()),
            speaker_id: Signal::new(None),
            is_llm_processing: Signal::new(false),
            dialogue_version: Signal::new(0),
            current_expression: Signal::new(None),
            current_mood: Signal::new(None),
            positioned_markers: Signal::new(Vec::new()),
            next_marker_index: Signal::new(0),
            current_action: Signal::new(None),
            conversation_id: Signal::new(None),
        }
    }

    /// Apply a new dialogue response (starts typewriter animation)
    ///
    /// Parses expression markers from the text and prepares for typewriter animation.
    pub fn apply_dialogue(
        &mut self,
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
    ) {
        self.apply_dialogue_with_mood(speaker_id, speaker_name, text, choices, None, None);
    }

    /// Apply dialogue with initial mood and expression
    ///
    /// # Arguments
    /// * `speaker_id` - ID of the speaking character
    /// * `speaker_name` - Display name of the speaker
    /// * `text` - Dialogue text (may contain expression markers)
    /// * `choices` - Available dialogue choices
    /// * `initial_mood` - NPC's current mood (Tier 2, shown as tag)
    /// * `initial_expression` - Starting expression for the sprite
    pub fn apply_dialogue_with_mood(
        &mut self,
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
        initial_mood: Option<String>,
        initial_expression: Option<String>,
    ) {
        // Parse the dialogue to extract markers and clean text
        let parsed = parse_dialogue(&text);
        let positioned = calculate_marker_positions(&parsed);

        self.speaker_id.set(Some(speaker_id));
        self.speaker_name.set(speaker_name);
        self.full_text.set(text);
        self.clean_text.set(parsed.clean_text);
        self.displayed_text.set(String::new());
        self.choices.set(choices);
        self.is_typing.set(true);
        self.awaiting_input.set(false);
        self.custom_input.set(String::new());
        self.is_llm_processing.set(false);

        // Expression system
        self.current_mood.set(initial_mood);
        self.current_expression.set(initial_expression);
        self.positioned_markers.set(positioned);
        self.next_marker_index.set(0);
        self.current_action.set(None);

        // Increment version to trigger typewriter restart
        let current_version = *self.dialogue_version.read();
        self.dialogue_version.set(current_version.wrapping_add(1));
    }

    /// Skip to the end of the typewriter animation
    pub fn skip_typewriter(&mut self) {
        let clean = self.clean_text.read().clone();
        self.displayed_text.set(clean);
        self.is_typing.set(false);
        self.awaiting_input.set(true);

        // Apply all remaining markers
        let markers = self.positioned_markers.read();
        if let Some(last_marker) = markers.last() {
            if let Some(expr) = &last_marker.marker.expression {
                self.current_expression.set(Some(expr.clone()));
            }
        }
        self.current_action.set(None); // Clear action when skipping
    }

    /// Set the conversation ID for tracking multi-turn conversations
    pub fn set_conversation_id(&mut self, conversation_id: Option<String>) {
        self.conversation_id.set(conversation_id);
    }

    /// Get the current conversation ID
    pub fn get_conversation_id(&self) -> Option<String> {
        self.conversation_id.read().clone()
    }

    /// Clear the conversation (when conversation ends)
    pub fn clear_conversation(&mut self) {
        self.conversation_id.set(None);
    }

    /// Check and trigger any markers at the current position
    ///
    /// Called during typewriter animation to update expression/action.
    /// Returns true if a marker was triggered.
    pub fn check_markers(&mut self, current_position: usize) -> bool {
        let markers = self.positioned_markers.read();
        let mut next_idx = *self.next_marker_index.read();
        let mut triggered = false;

        while next_idx < markers.len() && markers[next_idx].clean_position <= current_position {
            let marker = &markers[next_idx];

            if let Some(expr) = &marker.marker.expression {
                self.current_expression.set(Some(expr.clone()));
            }

            if let Some(action) = &marker.marker.action {
                self.current_action.set(Some(action.clone()));
            } else {
                // Clear action when we hit a non-action marker
                self.current_action.set(None);
            }

            next_idx += 1;
            triggered = true;
        }

        self.next_marker_index.set(next_idx);
        triggered
    }

    /// Check if typewriter is complete (based on clean text)
    pub fn is_typing_complete(&self) -> bool {
        let clean_len = self.clean_text.read().len();
        let displayed_len = self.displayed_text.read().len();
        displayed_len >= clean_len
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
        self.clean_text.set(String::new());
        self.displayed_text.set(String::new());
        self.is_typing.set(false);
        self.choices.set(Vec::new());
        self.awaiting_input.set(false);
        self.custom_input.set(String::new());
        self.is_llm_processing.set(false);
        self.current_expression.set(None);
        self.current_mood.set(None);
        self.positioned_markers.set(Vec::new());
        self.next_marker_index.set(0);
        self.current_action.set(None);
        self.conversation_id.set(None);
    }

    /// Check if there's active dialogue to display
    pub fn has_dialogue(&self) -> bool {
        !self.clean_text.read().is_empty()
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

/// Calculate marker positions in the clean text
///
/// Markers have positions in the original text, but we need to know
/// where they trigger relative to the clean text (what's being displayed).
fn calculate_marker_positions(parsed: &ParsedDialogue) -> Vec<PositionedMarker> {
    let mut positioned = Vec::new();
    let mut offset_adjustment = 0;

    for marker in &parsed.markers {
        // The clean position is the original position minus all the marker text
        // that came before this one
        let clean_position = marker.start_offset.saturating_sub(offset_adjustment);

        positioned.push(PositionedMarker {
            clean_position,
            marker: marker.clone(),
        });

        // Adjust offset for this marker's length
        offset_adjustment += marker.len();
    }

    positioned
}

impl Default for DialogueState {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook for running the typewriter effect with expression support
///
/// Call this in a component to drive the typewriter animation.
/// Updates expressions and actions as markers are encountered.
/// Restarts automatically when new dialogue arrives (detected via dialogue_version).
pub fn use_typewriter_effect(dialogue_state: &mut DialogueState) {
    let platform = use_platform();

    // Read the version to establish dependency - effect re-runs when this changes
    let dialogue_version = *dialogue_state.dialogue_version.read();
    // Also keep a signal reference to check for version changes in the async loop
    let dialogue_version_signal = dialogue_state.dialogue_version;

    let clean_text = dialogue_state.clean_text;
    let mut displayed_text = dialogue_state.displayed_text;
    let mut is_typing_signal = dialogue_state.is_typing;
    let mut awaiting_signal = dialogue_state.awaiting_input;
    let positioned_markers = dialogue_state.positioned_markers;
    let mut next_marker_index = dialogue_state.next_marker_index;
    let mut current_expression = dialogue_state.current_expression;
    let mut current_action = dialogue_state.current_action;

    // Use use_effect with dialogue_version as dependency to restart on new dialogue
    use_effect(move || {
        // Capture the version this effect was started with
        let started_version = dialogue_version;

        // Check if we should start typing (read current signal value)
        if !*is_typing_signal.read() {
            return;
        }

        let text = clean_text.read().clone();
        if text.is_empty() {
            return;
        }

        // Spawn the async typewriter animation
        let platform = platform.clone();
        spawn(async move {
            let mut current = String::new();
            let mut char_index = 0;

            for ch in text.chars() {
                // Check if we should stop:
                // 1. User skipped (is_typing set to false)
                // 2. New dialogue arrived (version changed)
                if !*is_typing_signal.read() {
                    break;
                }
                if *dialogue_version_signal.read() != started_version {
                    // New dialogue arrived, stop this animation immediately
                    break;
                }

                // Check for markers at this position
                {
                    let markers = positioned_markers.read();
                    let mut idx = *next_marker_index.read();

                    while idx < markers.len() && markers[idx].clean_position <= char_index {
                        let marker = &markers[idx];

                        if let Some(expr) = &marker.marker.expression {
                            current_expression.set(Some(expr.clone()));
                        }

                        if let Some(action) = &marker.marker.action {
                            current_action.set(Some(action.clone()));
                        } else {
                            current_action.set(None);
                        }

                        idx += 1;
                    }

                    next_marker_index.set(idx);
                }

                current.push(ch);
                displayed_text.set(current.clone());
                char_index += 1;

                // Variable delay based on punctuation
                let delay = match ch {
                    '.' | '!' | '?' => 150,
                    ',' | ';' | ':' => 80,
                    _ => 30,
                };

                platform.sleep_ms(delay).await;
            }

            // Mark as complete (only if we weren't interrupted AND version still matches)
            if *is_typing_signal.read() && *dialogue_version_signal.read() == started_version {
                is_typing_signal.set(false);
                awaiting_signal.set(true);
                current_action.set(None); // Clear action when done
            }
        });
    });
}
