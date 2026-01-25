//! DM panel components - Directorial controls for gameplay
//!
//! Provides reusable components for DM view including scene preview,
//! directorial notes, NPC motivation tracking, LLM response approval,
//! staging approval, challenge management, and time controls.

pub mod active_conversations;
pub mod adhoc_challenge_modal;
pub mod approval_popup;
pub mod challenge_library;
pub mod challenge_outcome_approval;
pub mod character_perspective;
pub mod conversation_details;
pub mod conversation_log;
pub mod decision_queue;
pub mod director_generate_modal;
pub mod director_queue_panel;
pub mod directorial_notes;
pub mod location_navigator;
pub mod location_preview_modal;
pub mod location_staging;
pub mod log_entry;
pub mod npc_disposition_panel;
pub mod npc_motivation;
pub mod pc_management;
pub mod scene_preview;
pub mod split_party_banner;
pub mod staging_approval;
pub mod time_control;
pub mod tone_selector;
pub mod trigger_challenge_modal;
pub mod visual_state_dropdown;
pub mod visual_state_details_modal;
pub mod visual_state_generation_modal;
pub mod visual_state_preview;

// Re-export key types for external use
pub use active_conversations::ActiveConversationsPanel;
pub use challenge_outcome_approval::{ChallengeOutcomeApprovalCard, ChallengeOutcomesSection};
pub use conversation_details::ConversationDetailsPanel;
pub use conversation_log::{ChallengeResultInfo, ConversationLog, ConversationTurn};
pub use location_preview_modal::LocationPreviewModal;
pub use location_staging::{LocationStagingPanel, PreStageApprovalData, RegionStagingInfo, StagingStatus};
pub use npc_disposition_panel::{
    DispositionChangeEvent, NpcDispositionListPanel, NpcDispositionPanel, RelationshipChangeEvent,
    SceneNpcInfo, DISPOSITION_OPTIONS, RELATIONSHIP_OPTIONS,
};
pub use split_party_banner::SplitPartyBanner;
pub use staging_approval::{StagingApprovalPopup, StagingApprovalResult, StagingRegenerateRequest};
pub use time_control::TimeControlPanel;
pub use visual_state_dropdown::VisualStateDropdown;
pub use visual_state_details_modal::VisualStateDetailData;
pub use visual_state_details_modal::VisualStateDetailsModal;
pub use visual_state_generation_modal::GeneratedStateResult;
pub use visual_state_generation_modal::VisualStateGenerationModal;
pub use visual_state_preview::VisualStatePreview;
