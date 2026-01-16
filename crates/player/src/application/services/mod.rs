//! Application services
//!
//! This module contains application services that implement use cases
//! for the WrldBldr Player. Services depend on port traits, not concrete
//! infrastructure implementations.

pub mod actantial_service;
pub mod action_service;
pub mod asset_service;
pub mod challenge_service;
pub mod character_service;
pub mod character_sheet_service;
pub mod event_chain_service;
pub mod generation_service;
pub mod location_service;
pub mod narrative_event_service;
pub mod observation_service;
pub mod player_character_service;
pub mod session_command_service;
pub mod session_service;
pub mod settings_service;
pub mod skill_service;
pub mod story_event_service;
pub mod suggestion_service;
pub mod user_service;
pub mod workflow_service;
pub mod world_service;

// Re-export action service
pub use action_service::ActionService;

// Re-export session command service
pub use session_command_service::SessionCommandService;

// Re-export session service types
pub use session_service::{connection_state_to_status, DEFAULT_ENGINE_URL};

pub use session_service::{SessionEvent, SessionService};

// Re-export world service types
pub use world_service::WorldService;

// Re-export character service types
pub use character_service::{CharacterFormData, CharacterService, CharacterSummary};

// Re-export player character service types
pub use player_character_service::{
    CreatePlayerCharacterRequest, PlayerCharacterData, PlayerCharacterService,
    UpdatePlayerCharacterRequest,
};

// Re-export location service types
pub use location_service::{LocationFormData, LocationService, LocationSummary};
// Map-related types from protocol
pub use wrldbldr_shared::{MapBoundsData, RegionListItemData};

// Re-export skill service types
pub use skill_service::{CreateSkillRequest, SkillService, UpdateSkillRequest};

// Re-export challenge service types
pub use challenge_service::ChallengeService;

// Re-export story event service types
pub use story_event_service::{CreateDmMarkerRequest, StoryEventService};

// Re-export narrative event service types
pub use narrative_event_service::NarrativeEventService;

// Re-export workflow service types
pub use workflow_service::{
    AnalyzeWorkflowResponse, InputDefault, PromptMapping, TestWorkflowResponse, WorkflowAnalysis,
    WorkflowConfig, WorkflowInput, WorkflowService, WorkflowSlotCategory, WorkflowSlotStatus,
};

// Re-export asset service types
pub use asset_service::{Asset, AssetService, GenerateRequest};

// Re-export suggestion service types
pub use crate::application::dto::requests::SuggestionContext;
pub use suggestion_service::SuggestionService;

// Re-export event chain service types
pub use event_chain_service::{
    CreateEventChainRequest, EventChainData, EventChainService, UpdateEventChainRequest,
};

// Re-export generation service types
pub use generation_service::GenerationService;

// Re-export settings service types
pub use settings_service::SettingsService;

// Re-export observation service types
pub use observation_service::{ObservationService, ObservationSummary};

// Re-export actantial service types
pub use actantial_service::{
    ActantialService, AddActantialViewRequest, CreateGoalRequest, CreateWantRequest, GoalResponse,
    RemoveActantialViewRequest, SetWantTargetRequest, UpdateGoalRequest, UpdateWantRequest,
    WantResponse,
};

// Re-export user service types
pub use user_service::UserService;

// Re-export character sheet service types
pub use character_sheet_service::{
    CharacterSheetService, CompleteCreationResponse, GameSystemInfo, GetSheetResponse,
    StartCreationResponse, UpdateFieldResponse,
};
