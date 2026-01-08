//! Application state management
//!
//! Central state management using Dioxus signals and context.

pub mod approval_state;
pub mod challenge_state;
pub mod connection_state;
pub mod dialogue_state;
pub mod game_state;
pub mod generation_state;
pub mod lore_state;
pub mod session_state;

// Export individual substates
pub use approval_state::{ConversationLogEntry, PendingApproval, PendingChallengeOutcome};
pub use challenge_state::RollSubmissionStatus;
pub use connection_state::ConnectionStatus;
pub use dialogue_state::{use_typewriter_effect, DialogueState};
pub use game_state::{
    ApproachEventData, GameState, LocationEventData, TimeMode, TimeSuggestionData, ViewMode,
};
pub use generation_state::{
    BatchStatus, GenerationBatch, GenerationState, SuggestionStatus, SuggestionTask,
};
pub use lore_state::{KnownLoreEntry, LoreState};

// SessionState is the facade that composes the substates (backward-compatible)
pub use session_state::SessionState;

use dioxus::prelude::*;

/// Get the game state from context
///
/// # Panics
/// Panics if GameState has not been provided via use_context_provider
pub fn use_game_state() -> GameState {
    use_context::<GameState>()
}

/// Get the session state from context
///
/// # Panics
/// Panics if SessionState has not been provided via use_context_provider
pub fn use_session_state() -> SessionState {
    use_context::<SessionState>()
}

/// Get the dialogue state from context
///
/// # Panics
/// Panics if DialogueState has not been provided via use_context_provider
pub fn use_dialogue_state() -> DialogueState {
    use_context::<DialogueState>()
}

/// Get the generation state from context
///
/// # Panics
/// Panics if GenerationState has not been provided via use_context_provider
pub fn use_generation_state() -> GenerationState {
    use_context::<GenerationState>()
}

/// Get the lore state from context
///
/// # Panics
/// Panics if LoreState has not been provided via use_context_provider
pub fn use_lore_state() -> LoreState {
    use_context::<LoreState>()
}
