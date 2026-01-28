//! Dialogue state management with typewriter effect
//!
//! Manages current dialogue display including typewriter animation.
//! Supports expression markers for three-tier emotional model.
//! Supports per-conversation state for dialogue scoping.

use dioxus::prelude::*;
use wrldbldr_domain::{parse_dialogue, DialogueMarker, ParsedDialogue};

use crate::application::dto::DialogueChoice;
use crate::infrastructure::spawn_task;
use crate::use_platform;

/// A marker with its trigger position in clean text
#[derive(Clone, Debug)]
pub struct PositionedMarker {
    /// Position in clean text where this marker triggers
    pub clean_position: usize,
    /// The original marker data
    pub marker: DialogueMarker,
}

/// Plain data for a conversation's dialogue state (no Signals)
#[derive(Clone, Debug)]
pub struct ConversationDialogue {
    /// Current speaker name
    pub speaker_name: String,
    /// Full dialogue text (original with markers)
    pub full_text: String,
    /// Clean text (markers stripped, for display)
    pub clean_text: String,
    /// Currently displayed text (typewriter progress)
    pub displayed_text: String,
    /// Whether typewriter is still animating
    pub is_typing: bool,
    /// Available dialogue choices
    pub choices: Vec<DialogueChoice>,
    /// Whether we're waiting for player input
    pub awaiting_input: bool,
    /// Custom input text (for custom response choices)
    pub custom_input: String,
    /// Speaker ID for targeting actions
    pub speaker_id: Option<String>,
    /// Version counter - increments when new dialogue arrives (for reactivity)
    pub dialogue_version: u32,

    // Expression system (Tier 3 of emotional model)
    /// Current expression for speaker (changes during typewriter)
    pub current_expression: Option<String>,
    /// Current mood tag to display next to speaker name (Tier 2)
    pub current_mood: Option<String>,
    /// Markers parsed from dialogue, with clean text positions
    pub positioned_markers: Vec<PositionedMarker>,
    /// Index of next marker to trigger
    pub next_marker_index: usize,
    /// Current action being performed (displayed as stage direction)
    pub current_action: Option<String>,
}

impl ConversationDialogue {
    /// Create a new conversation dialogue with default values
    pub fn new() -> Self {
        Self {
            speaker_name: String::new(),
            full_text: String::new(),
            clean_text: String::new(),
            displayed_text: String::new(),
            is_typing: false,
            choices: Vec::new(),
            awaiting_input: false,
            custom_input: String::new(),
            speaker_id: None,
            dialogue_version: 0,
            current_expression: None,
            current_mood: None,
            positioned_markers: Vec::new(),
            next_marker_index: 0,
            current_action: None,
        }
    }

    /// Check if the typewriter animation is complete (based on clean text)
    pub fn is_typing_complete(&self) -> bool {
        self.displayed_text.len() >= self.clean_text.len()
    }

    /// Get the delay for the next character based on punctuation
    pub fn get_char_delay(&self) -> u32 {
        if let Some(last_char) = self.displayed_text.chars().last() {
            match last_char {
                '.' | '!' | '?' => 150,
                ',' | ';' | ':' => 80,
                _ => 30,
            }
        } else {
            30
        }
    }
}

impl Default for ConversationDialogue {
    fn default() -> Self {
        Self::new()
    }
}

/// Dialogue state for visual novel UI (supports per-conversation scoping)
#[derive(Clone)]
pub struct DialogueState {
    /// Per-conversation dialogue data keyed by conversation_id (plain data, no Signals)
    pub dialogues_by_conversation: Signal<std::collections::HashMap<String, ConversationDialogue>>,
    /// Currently active conversation ID
    pub active_conversation_id: Signal<Option<String>>,
    /// Whether LLM is processing (show loading indicator)
    pub is_llm_processing: Signal<bool>,

    // Cached signals for the active conversation (for UI binding)
    pub speaker_name: Signal<String>,
    pub full_text: Signal<String>,
    pub clean_text: Signal<String>,
    pub displayed_text: Signal<String>,
    pub is_typing: Signal<bool>,
    pub choices: Signal<Vec<DialogueChoice>>,
    pub awaiting_input: Signal<bool>,
    pub custom_input: Signal<String>,
    pub speaker_id: Signal<Option<String>>,
    pub dialogue_version: Signal<u32>,
    pub current_expression: Signal<Option<String>>,
    pub current_mood: Signal<Option<String>>,
    pub positioned_markers: Signal<Vec<PositionedMarker>>,
    pub next_marker_index: Signal<usize>,
    pub current_action: Signal<Option<String>>,
}

impl DialogueState {
    /// Create a new DialogueState with empty values
    pub fn new() -> Self {
        let state = Self {
            dialogues_by_conversation: Signal::new(std::collections::HashMap::new()),
            active_conversation_id: Signal::new(None),
            is_llm_processing: Signal::new(false),
            speaker_name: Signal::new(String::new()),
            full_text: Signal::new(String::new()),
            clean_text: Signal::new(String::new()),
            displayed_text: Signal::new(String::new()),
            is_typing: Signal::new(false),
            choices: Signal::new(Vec::new()),
            awaiting_input: Signal::new(false),
            custom_input: Signal::new(String::new()),
            speaker_id: Signal::new(None),
            dialogue_version: Signal::new(0),
            current_expression: Signal::new(None),
            current_mood: Signal::new(None),
            positioned_markers: Signal::new(Vec::new()),
            next_marker_index: Signal::new(0),
            current_action: Signal::new(None),
        };
        // refresh_active_cache will be called after initialization via the struct methods
        state
    }

    /// Set the active conversation ID (for tracking multi-turn conversations)
    pub fn set_conversation_id(&mut self, conversation_id: Option<String>) {
        let convo = conversation_id.clone();
        self.active_conversation_id.set(convo);
        self.refresh_active_cache();
    }

    /// Get the current conversation ID
    pub fn get_conversation_id(&self) -> Option<String> {
        self.active_conversation_id.read().clone()
    }

    /// Refresh the cache of signals from the active conversation
    ///
    /// This must be called when active_conversation_id changes to ensure
    /// cached signal subscriptions are updated to point to the new conversation.
    fn refresh_active_cache(&mut self) {
        let active_id = self.active_conversation_id.read();
        let dialogues = self.dialogues_by_conversation.read();

        let dialogue = match active_id.as_ref().and_then(|id| dialogues.get(id)) {
            Some(d) => d,
            None => return, // No active conversation, keep cache empty
        };

        self.speaker_name.set(dialogue.speaker_name.clone());
        self.full_text.set(dialogue.full_text.clone());
        self.clean_text.set(dialogue.clean_text.clone());
        self.displayed_text.set(dialogue.displayed_text.clone());
        self.is_typing.set(dialogue.is_typing);
        self.choices.set(dialogue.choices.clone());
        self.awaiting_input.set(dialogue.awaiting_input);
        self.custom_input.set(dialogue.custom_input.clone());
        self.speaker_id.set(dialogue.speaker_id.clone());
        self.dialogue_version.set(dialogue.dialogue_version);
        self.current_expression.set(dialogue.current_expression.clone());
        self.current_mood.set(dialogue.current_mood.clone());
        self.positioned_markers.set(dialogue.positioned_markers.clone());
        self.next_marker_index.set(dialogue.next_marker_index);
        self.current_action.set(dialogue.current_action.clone());
    }

    /// Apply a new dialogue response (starts typewriter animation)
    ///
    /// Parses expression markers from text and prepares for typewriter animation.
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
    /// * `initial_expression` - Starting expression for sprite
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

        // Build the new dialogue data
        let dialogue = ConversationDialogue {
            speaker_id: Some(speaker_id),
            speaker_name,
            full_text: text,
            clean_text: parsed.clean_text.clone(),
            displayed_text: String::new(),
            choices,
            is_typing: true,
            awaiting_input: false,
            custom_input: String::new(),
            current_mood: initial_mood,
            current_expression: initial_expression,
            positioned_markers: positioned,
            next_marker_index: 0,
            current_action: None,
            dialogue_version: 1, // Reset version for new dialogue
        };

        // Store in the map
        let active_id = self.active_conversation_id.read().clone();
        if let Some(ref id) = active_id.as_ref() {
            let mut dialogues = self.dialogues_by_conversation.write();
            dialogues.insert(id.to_string(), dialogue);
            drop(dialogues); // Release write lock before refresh
            self.refresh_active_cache();
        } else {
            // No active conversation, apply directly to cache
            self.speaker_name.set(dialogue.speaker_name.clone());
            self.full_text.set(dialogue.full_text.clone());
            self.clean_text.set(dialogue.clean_text.clone());
            self.displayed_text.set(dialogue.displayed_text.clone());
            self.is_typing.set(dialogue.is_typing);
            self.choices.set(dialogue.choices.clone());
            self.awaiting_input.set(dialogue.awaiting_input);
            self.custom_input.set(dialogue.custom_input.clone());
            self.speaker_id.set(dialogue.speaker_id.clone());
            self.dialogue_version.set(dialogue.dialogue_version);
            self.current_expression.set(dialogue.current_expression.clone());
            self.current_mood.set(dialogue.current_mood.clone());
            self.positioned_markers.set(dialogue.positioned_markers.clone());
            self.next_marker_index.set(dialogue.next_marker_index);
            self.current_action.set(dialogue.current_action.clone());
        }
    }

    /// Skip to the end of the typewriter animation for the active conversation
    pub fn skip_typewriter(&mut self) {
        let active_id = self.active_conversation_id.read().clone();
        if active_id.is_none() {
            return;
        }

        let mut dialogues = self.dialogues_by_conversation.write();
        if let Some(id) = active_id.as_ref() {
            if let Some(dialogue) = dialogues.get_mut(id) {
                // Skip typewriter
                let clean = dialogue.clean_text.clone();
                dialogue.displayed_text = clean;
                dialogue.is_typing = false;
                dialogue.awaiting_input = true;

                // Apply all remaining markers
                if let Some(last_marker) = dialogue.positioned_markers.last() {
                    if let Some(expr) = last_marker.marker.expression_value() {
                        dialogue.current_expression = Some(expr.to_string());
                    }
                }
                dialogue.current_action = None; // Clear action when skipping
            }
        }

        // Refresh cache after modification
        drop(dialogues); // Release write lock
        self.refresh_active_cache();
    }

    /// Clear the conversation state for a specific conversation ID
    pub fn clear_conversation(&mut self, conversation_id: &str) {
        let conv_id = conversation_id.to_string();

        // Check if we're clearing the active conversation
        let should_clear_cache = {
            let active_id = self.active_conversation_id.read();
            active_id.as_ref().map(|id| id == &conv_id).unwrap_or(false)
        };

        // Remove the conversation from the map
        let mut dialogues = self.dialogues_by_conversation.write();
        dialogues.remove(&conv_id);

        // If we cleared the active conversation, set it to None and refresh cache
        if should_clear_cache {
            self.active_conversation_id.set(None);
            drop(dialogues); // Release write lock before refresh
            self.refresh_active_cache();
        }
    }

    /// Clear all dialogue state (when scene changes or disconnects)
    pub fn clear(&mut self) {
        {
            self.dialogues_by_conversation.write().clear();
        }
        self.active_conversation_id.set(None);
        self.is_llm_processing.set(false);
        self.refresh_active_cache();
    }

    /// Check and trigger any markers at the current position
    ///
    /// Called during typewriter animation to update expression/action.
    /// Returns true if a marker was triggered.
    pub fn check_markers(&mut self, current_position: usize) -> bool {
        let active_id = self.active_conversation_id.read().clone();
        if active_id.is_none() {
            return false;
        }

        let mut dialogues = self.dialogues_by_conversation.write();
        if let Some(id) = active_id.as_ref() {
            if let Some(dialogue) = dialogues.get_mut(id) {
                let mut triggered = false;
                let markers = dialogue.positioned_markers.clone(); // Clone for iteration
                let mut next_idx = dialogue.next_marker_index;

                while next_idx < markers.len() && markers[next_idx].clean_position <= current_position {
                    let marker = &markers[next_idx];

                    if let Some(expr) = marker.marker.expression_value() {
                        dialogue.current_expression = Some(expr.to_string());
                    }

                    if let Some(action) = marker.marker.action_value() {
                        dialogue.current_action = Some(action.to_string());
                    } else {
                        // Clear action when we hit a non-action marker
                        dialogue.current_action = None;
                    }

                    next_idx += 1;
                    triggered = true;
                }

                dialogue.next_marker_index = next_idx;
                return triggered;
            }
        }
        false
    }

    /// Check if the typewriter animation is complete (based on clean text)
    pub fn is_typing_complete(&self) -> bool {
        self.clean_text.read().is_empty()
            || self.displayed_text.read().len() >= self.clean_text.read().len()
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

    /// Check if there's an active conversation
    ///
    /// Returns true if a conversation_id is set, indicating an active conversation
    /// that can be ended.
    pub fn has_active_conversation(&self) -> bool {
        self.active_conversation_id.read().is_some()
    }

    /// Clear the conversation (when conversation ends) - legacy compatibility
    pub fn clear_conversation_legacy(&mut self) {
        self.clear();
    }
}

/// Calculate marker positions in clean text
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

        // Adjust the offset for this marker's length
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
    let dialogue_version_signal = dialogue_state.dialogue_version;
    let dialogue_version = *dialogue_version_signal.read();

    // Cache the signal references to use them in the effect (top-level signals only)
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

        // Check if we should start typing (read the current signal value)
        if !*is_typing_signal.read() {
            return;
        }

        let text = clean_text.read().clone();
        if text.is_empty() {
            return;
        }

        // Spawn the async typewriter animation
        let platform = platform.clone();
        spawn_task(async move {
            let mut current = String::new();

            for (char_index, ch) in text.chars().enumerate() {
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

                        if let Some(expr) = marker.marker.expression_value() {
                            current_expression.set(Some(expr.to_string()));
                        }

                        if let Some(action) = marker.marker.action_value() {
                            current_action.set(Some(action.to_string()));
                        } else {
                            current_action.set(None);
                        }

                        idx += 1;
                    }

                    next_marker_index.set(idx);
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

            // Mark as complete (only if we weren't interrupted AND version still matches)
            if *is_typing_signal.read() && *dialogue_version_signal.read() == started_version {
                is_typing_signal.set(false);
                awaiting_signal.set(true);
                current_action.set(None); // Clear action when done
            }
        });
    });
}
