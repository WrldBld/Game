//! DM panel components - Directorial controls for gameplay
//!
//! Provides reusable components for the DM view including scene preview,
//! directorial notes, NPC motivation tracking, LLM response approval,
//! staging approval, and challenge management.

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
pub mod location_preview_modal;
pub mod location_staging;
pub mod log_entry;
pub mod npc_mood_panel;
pub mod npc_motivation;
pub mod pc_management;
pub mod scene_preview;
pub mod split_party_banner;
pub mod staging_approval;
pub mod tone_selector;
pub mod trigger_challenge_modal;

// Re-export key types for external use
pub use challenge_outcome_approval::{ChallengeOutcomeApprovalCard, ChallengeOutcomesSection};
pub use conversation_log::{ChallengeResultInfo, ConversationLog, ConversationTurn};
pub use location_staging::{LocationStagingPanel, RegionStagingInfo, StagingStatus};
pub use npc_mood_panel::{
    MoodChangeEvent, NpcMoodListPanel, NpcMoodPanel, RelationshipChangeEvent, SceneNpcInfo,
    MOOD_OPTIONS, RELATIONSHIP_OPTIONS,
};
pub use location_preview_modal::LocationPreviewModal;
pub use split_party_banner::SplitPartyBanner;
pub use staging_approval::{StagingApprovalPopup, StagingApprovalResult, StagingRegenerateRequest};
