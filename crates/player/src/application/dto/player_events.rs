//! Player events - re-exported from ports layer
//!
//! The PlayerEvent types and their From implementations are defined in
//! player-ports/inbound as they represent the inbound port contract.
//! This module re-exports them for backwards compatibility.

// Re-export all types from the ports layer
pub use crate::ports::outbound::player_events::{
    ActantialViewData, ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, CharacterData,
    CharacterPosition, ConnectedUser, DialogueChoice, EntityChangedData, GameTime, GoalData,
    InteractionData, JoinError, NarrativeEventSuggestionInfo, NavigationData, NavigationExit,
    NavigationTarget, NpcDispositionData, NpcPresenceData, NpcPresentInfo, OutcomeBranchData,
    OutcomeDetailData, PlayerEvent, PreviousStagingInfo, ProposedToolInfo, RegionData,
    RegionItemData, ResponseResult, SceneData, SplitPartyLocation, StagedNpcInfo, WaitingPcInfo,
    WantData, WantTargetData, WorldRole,
};
