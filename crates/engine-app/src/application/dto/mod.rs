//! Data Transfer Objects (DTOs) - Application layer data structures
//!
//! DTOs are used for data transfer between layers (HTTP routes, services, etc.)
//! They provide a stable API that is decoupled from domain entities.
//!
//! NOTE: Many DTOs have been consolidated into `wrldbldr_protocol` as the single
//! source of truth for wire-format types. Import directly from protocol for:
//! - Asset DTOs (GalleryAssetResponseDto, GenerationBatchResponseDto, etc.)
//! - Export DTOs (ExportQueryDto)
//! - Rule System DTOs
//! - Most Workflow DTOs

mod challenge;
mod character;
mod comfyui_config;
mod event_chain;
mod interaction;
mod item;
mod location;
mod narrative_event;
mod player_character;
mod rule_system;
mod scene;

mod sheet_template;
mod skill;
mod story_event;
mod suggestion;
mod workflow;
mod world;
mod world_snapshot;

// NOTE: Approval DTOs moved to wrldbldr_domain::value_objects::queue_data
// NOTE: Queue item types moved to wrldbldr_domain::value_objects::queue_data

// Challenge DTOs
pub use challenge::{
    ChallengeOutcomePendingNotification, ChallengeResolvedNotification, ChallengeResponseDto,
    ChallengeRollSubmittedNotification, CreateChallengeRequestDto, DifficultyRequestDto,
    OutcomeBranchDto, OutcomeBranchResponse, OutcomeBranchSelectionRequest,
    OutcomeBranchesReadyNotification, OutcomeRequestDto, OutcomeSuggestionReadyNotification,
    OutcomeSuggestionRequest, OutcomeSuggestionResponse, OutcomeTriggerRequestDto,
    OutcomesRequestDto, PendingChallengeResolutionDto, TriggerConditionRequestDto,
    UpdateChallengeRequestDto,
};

// Rule system DTOs
pub use rule_system::{
    parse_system_type, parse_variant, RuleSystemPresetDetailsDto, RuleSystemPresetSummaryDto,
    RuleSystemSummaryDto, RuleSystemTypeDetailsDto,
};

// Workflow DTOs - conversion functions that need WorkflowService
pub use workflow::{
    workflow_config_from_export_dto, workflow_config_to_export_dto,
    workflow_config_to_full_response_dto, workflow_config_to_response_dto,
};
// Re-export WorkflowConfigExportDto for service-layer use (avoids direct protocol import)
pub use wrldbldr_protocol::WorkflowConfigExportDto;

// Scene DTOs
pub use scene::{CreateSceneRequestDto, SceneResponseDto, UpdateNotesRequestDto};

// Story event DTOs
pub use story_event::{
    CreateDmMarkerRequestDto, ListStoryEventsQueryDto, PaginatedStoryEventsResponseDto,
    StoryEventResponseDto, UpdateStoryEventRequestDto,
};

// Narrative event DTOs
pub use narrative_event::{
    CreateNarrativeEventRequestDto, ListNarrativeEventsQueryDto, NarrativeEventResponseDto,
    UpdateNarrativeEventRequestDto,
};

// Event chain DTOs
pub use event_chain::{
    AddEventRequestDto, ChainStatusResponseDto, CreateEventChainRequestDto, EventChainResponseDto,
    UpdateEventChainRequestDto,
};

// Location DTOs
pub use location::{
    parse_location_type, ConnectionResponseDto, CreateConnectionRequestDto,
    CreateLocationRequestDto, CreateRegionRequestDto, LocationResponseDto, MapBoundsDto,
    RegionResponseDto,
};

// Character DTOs
pub use character::{
    parse_archetype, parse_relationship_type, ChangeArchetypeRequestDto, CharacterResponseDto,
    CreateCharacterRequestDto, CreateRelationshipRequestDto, CreatedIdResponseDto,
};

// Item DTOs
pub use item::{
    parse_acquisition_method, AddInventoryItemRequestDto, CreateItemRequestDto,
    InventoryItemResponseDto, ItemResponseDto, UpdateInventoryItemRequestDto,
};

// ComfyUI config DTO
pub use comfyui_config::ComfyUIConfigDto;

// Skill DTOs
pub use skill::{CreateSkillRequestDto, SkillResponseDto, UpdateSkillRequestDto};

// Interaction DTOs
pub use interaction::{
    parse_interaction_type, parse_target, CreateInteractionRequestDto, InteractionResponseDto,
    SetAvailabilityRequestDto,
};

// Sheet template DTOs
pub use sheet_template::{
    CreateFieldRequestDto, CreateSectionRequestDto, SheetTemplateResponseDto,
    SheetTemplateStorageDto, SheetTemplateSummaryDto,
};

// Player Character DTOs
pub use player_character::{
    CreatePlayerCharacterRequestDto, PlayerCharacterResponseDto, UpdatePlayerCharacterRequestDto,
};

// Suggestion DTOs
pub use suggestion::{SuggestionRequestDto, UnifiedSuggestionRequestDto};

// World DTOs
pub use world::{
    parse_monomyth_stage, ActResponseDto, CreateActRequestDto, CreateWorldRequestDto,
    UpdateWorldRequestDto, WorldResponseDto,
};

// World snapshot DTO (for session management)
pub use world_snapshot::WorldSnapshot;
