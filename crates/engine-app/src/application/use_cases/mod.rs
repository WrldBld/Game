//! Use Cases - Application layer orchestration
//!
//! Use cases coordinate domain services to fulfill specific user intents.
//! They are transport-agnostic and return domain results, not protocol messages.
//!
//! # Scope Clarification
//!
//! Use cases handle **WebSocket message handlers** (gameplay events):
//! - movement.rs, staging.rs, challenge.rs, inventory.rs, etc.
//!
//! They do NOT replace **AppRequestHandler** which handles:
//! - Request/Response CRUD operations via `ClientMessage::Request { request_id, payload }`
//! - Located in `engine-app/src/application/handlers/request_handler.rs`
//!
//! The distinction:
//! - **Use cases**: Complex workflows with side-effects (broadcasts, state changes)
//! - **AppRequestHandler**: Simple CRUD operations that return a single response
//!
//! # Port Ownership
//!
//! Use cases depend on **outbound ports** defined in `engine-ports::outbound`.
//! These are dependency contracts that adapters implement.
//!
//! - `ChallengeResolutionPort`, `ChallengeOutcomeApprovalPort`, `ChallengeDmApprovalQueuePort`
//! - `ConnectionManagerPort`, `DirectorialContextQueryPort`, `PlayerCharacterDtoPort`, `WorldSnapshotJsonPort`
//! - `WorldStatePort`, `DirectorialContextDtoRepositoryPort`, `SceneDmActionQueuePort`
//! - `PlayerActionQueuePort`, `DmNotificationPort`
//! - `StagingStatePort`, `StagingQueryPort`, `StagingStateExtPort`, `StagingMutationPort`
//!
//! Types used by these ports are defined in `engine-ports::outbound::use_case_types`.
//!
//! **Other infrastructure ports** (also in engine-ports::outbound):
//! - Repository traits (`CharacterRepositoryPort`, `LocationRepositoryPort`, etc.)
//! - External service traits (`LlmPort`, `ComfyUIPort`, `BroadcastPort`)
//!
//! See: `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`
//!
//! # Architecture Rules
//!
//! 1. Use cases must NOT import `wrldbldr_protocol::ServerMessage`
//! 2. Use cases return domain result types (enums, structs)
//! 3. Use cases use `BroadcastPort` for side-effect notifications
//! 4. Use cases orchestrate domain services, they don't replace them
//! 5. Use cases are the transaction/workflow boundary
//! 6. Use cases import `UseCaseContext` from `engine-ports::inbound`
//!
//! # Handler Pattern
//!
//! Handlers should call use cases like this:
//!
//! ```rust,ignore
//! let ctx = HandlerContext::extract(state, client_id).await?;
//! match state.use_cases.movement.move_to_region(ctx.into(), input).await {
//!     Ok(result) => Some(result.into_server_message()),
//!     Err(e) => Some(e.into_server_error()),
//! }
//! ```
//!
//! # Use Case Implementation Status
//!
//! Phase 3.1-3.4: Infrastructure
//! - [x] mod.rs (this file)
//! - [x] errors.rs - Error types with ErrorCode trait
//! - [x] builders/scene_builder.rs - Shared scene building logic
//!
//! Phase 3.5+: Use Cases
//! - [x] movement.rs - MovementUseCase
//! - [x] staging.rs - StagingApprovalUseCase
//! - [x] inventory.rs - InventoryUseCase
//! - [x] challenge.rs - ChallengeUseCase
//! - [x] observation.rs - ObservationUseCase
//! - [x] scene.rs - SceneUseCase
//! - [x] connection.rs - ConnectionUseCase
//! - [x] player_action.rs - PlayerActionUseCase

mod builders;
mod challenge;
mod connection;
mod errors;
mod inventory;
mod movement;
mod narrative_event;
mod observation;
mod player_action;
mod scene;
mod staging;

// Re-export UseCaseContext from ports (defined there to avoid circular deps)
pub use wrldbldr_engine_ports::inbound::UseCaseContext;

// Re-export error types (explicit)
pub use errors::{
    ActionError, ChallengeError, ConnectionError, ErrorCode, InventoryError, MovementError,
    NarrativeEventError, ObservationError, SceneError, StagingError,
};

// Re-export builders (explicit)
pub use builders::SceneBuilder;

// Re-export use cases (explicit where possible)
pub use challenge::{
    AdHocOutcomes, AdHocResult, ApprovalItem, ChallengeUseCase, CreateAdHocInput, DiceInputType,
    DiscardChallengeInput, DiscardResult,
    OutcomeDecision as ChallengeOutcomeDecision, OutcomeDecisionInput, OutcomeDecisionResult,
    OutcomeDetail, RegenerateOutcomeInput, RegenerateResult as ChallengeRegenerateResult,
    RequestBranchesInput, RequestSuggestionInput, RollResult, SelectBranchInput,
    SubmitDiceInputInput, SubmitRollInput, SuggestionDecisionInput, TriggerChallengeInput,
    TriggerInfo, TriggerResult,
};

pub use connection::{
    ConnectedUser, ConnectionInfo, ConnectionManagerPort, ConnectionUseCase, JoinWorldInput,
    JoinWorldResult, LeaveWorldResult, PcData, SetSpectateTargetInput, SpectateTargetResult,
    UserJoinedEvent, WorldRole,
};

pub use inventory::{
    DropInput, DropResult, EquipInput, EquipResult, InventoryUseCase, PickupInput, PickupResult,
    UnequipInput, UnequipResult,
};
pub use movement::{
    ExitToLocationInput, MoveToRegionInput, MovementResult, MovementUseCase, PendingStagingData,
    SelectCharacterInput, SelectCharacterResult, StagingProposalData, StagingServicePort,
    StagingStatePort,
};
pub use observation::{
    ApproachEventData, LocationEventData, ObservationUseCase, ShareNpcLocationInput,
    ShareNpcLocationResult, TriggerApproachInput, TriggerApproachResult, TriggerLocationEventInput,
    TriggerLocationEventResult,
};
pub use player_action::{
    ActionResult, DmNotificationPort, PlayerActionInput, PlayerActionUseCase,
};

pub use scene::{
    CharacterData as SceneCharacterData, CharacterEntity, DirectorialContextData,
    DirectorialContextDtoRepositoryPort, DirectorialUpdateResult, DmAction,
    DmActionQueuePort as SceneDmActionQueuePort, InteractionData as SceneInteractionData,
    InteractionEntity, InteractionTarget, LocationEntity, NpcMotivation, RequestSceneChangeInput,
    SceneApprovalDecision, SceneApprovalDecisionInput, SceneApprovalDecisionResult,
    SceneChangeResult, SceneData, SceneEntity, SceneUseCase, TimeContext, UpdateDirectorialInput,
    UseCaseSceneWithRelations, WorldStatePort,
};

pub use staging::{
    ApproveInput, ApproveResult, ApprovedNpc, ApprovedNpcData, PendingStagingInfo, PreStageInput,
    PreStageResult, ProposedNpc, RegenerateInput, RegenerateResult as StagingRegenerateResult,
    RegeneratedNpc, StagingApprovalSource, StagingApprovalUseCase, StagingServiceExtPort,
    StagingStateExtPort, WaitingPcInfo,
};

pub use narrative_event::{
    DecisionResult as NarrativeEventDecisionResult, NarrativeEventUseCase,
    SuggestionDecisionInput as NarrativeEventSuggestionDecisionInput,
};
