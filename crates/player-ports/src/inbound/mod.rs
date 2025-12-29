//! Inbound ports - Data types for server-to-application communication
//!
//! These types define the contract for how server events are represented
//! in the application layer. The adapters layer produces these types,
//! the application and UI layers consume them.

pub mod player_events;

// Re-export all types for convenience
pub use player_events::{
    ActantialViewData, ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, CharacterData,
    CharacterPosition, ConnectedUser, DialogueChoice, EntityChangedData, GameTime, GoalData,
    InteractionData, JoinError, NarrativeEventSuggestionInfo, NavigationData, NavigationExit,
    NavigationTarget, NpcDispositionData, NpcPresenceData, NpcPresentInfo, OutcomeBranchData,
    OutcomeDetailData, PlayerEvent, PreviousStagingInfo, ProposedToolInfo, RegionData,
    RegionItemData, ResponseResult, SceneData, SplitPartyLocation, StagedNpcInfo, WaitingPcInfo,
    WantData, WantTargetData, WorldRole,
};
