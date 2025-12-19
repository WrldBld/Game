//! DM panel components - Directorial controls for gameplay
//!
//! Provides reusable components for the DM view including scene preview,
//! directorial notes, NPC motivation tracking, LLM response approval,
//! and challenge management.

pub mod adhoc_challenge_modal;
pub mod approval_popup;
pub mod challenge_library;
pub mod challenge_outcome_approval;
pub mod character_perspective;
pub mod conversation_log;
pub mod decision_queue;
pub mod directorial_notes;
pub mod director_generate_modal;
pub mod director_queue_panel;
pub mod location_navigator;
pub mod log_entry;
pub mod npc_motivation;
pub mod pc_management;
pub mod scene_preview;
pub mod tone_selector;
pub mod trigger_challenge_modal;

// Re-export key types for external use
pub use challenge_outcome_approval::{ChallengeOutcomeApprovalCard, ChallengeOutcomesSection};
pub use conversation_log::{ChallengeResultInfo, ConversationLog, ConversationTurn};
