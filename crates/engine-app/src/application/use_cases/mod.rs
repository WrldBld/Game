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
//! # Port Ownership in engine-app
//!
//! Some ports are defined here rather than in `engine-ports` because they depend on
//! DTOs defined within this crate. This is valid hexagonal architecture - use-case
//! specific ports can live in the application layer.
//!
//! **Use-case ports** (valid in engine-app):
//! - `WorldStatePort` - Depends on `DirectorialContextData`
//! - `ConnectionManagerPort` - Depends on `ConnectionInfo`, `UserJoinedEvent`, etc.
//! - `StagingStatePort` - Depends on staging-specific DTOs
//! - `DmNotificationPort`, `WorldMessagePort` - Use-case specific notification semantics
//! - `ChallengeResolutionPort`, `SceneServicePort`, etc. - Service facade ports
//!
//! **Infrastructure ports** (should be in engine-ports):
//! - Repository traits (`CharacterRepositoryPort`, `LocationRepositoryPort`, etc.)
//! - External service traits (`LlmPort`, `ComfyUIPort`, `BroadcastPort`)
//!
//! See: `docs/plans/HEXAGONAL_GAP_REMEDIATION_PLAN.md` Appendix B
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
    AdHocOutcomes, AdHocResult, ApprovalItem, ChallengeOutcomeApprovalPort,
    ChallengeResolutionPort, ChallengeUseCase, CreateAdHocInput, DiceInputType,
    DiscardChallengeInput, DiscardResult, DmApprovalQueuePort as ChallengeDmApprovalQueuePort,
    OutcomeDecision as ChallengeOutcomeDecision, OutcomeDecisionInput, OutcomeDecisionResult,
    OutcomeDetail, RegenerateOutcomeInput, RegenerateResult as ChallengeRegenerateResult,
    RequestBranchesInput, RequestSuggestionInput, RollResult, SelectBranchInput,
    SubmitDiceInputInput, SubmitRollInput, SuggestionDecisionInput, TriggerChallengeInput,
    TriggerInfo, TriggerResult,
};

pub use connection::{
    ConnectedUser, ConnectionInfo, ConnectionManagerPort, ConnectionUseCase,
    DirectorialContextPort, JoinWorldInput, JoinWorldResult, LeaveWorldResult, PcData,
    PlayerCharacterServicePort, SetSpectateTargetInput, SpectateTargetResult, UserJoinedEvent,
    UserLeftEvent, WorldRole, WorldServicePort,
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
    TriggerLocationEventResult, WorldMessagePort,
};
pub use player_action::{
    ActionResult, DmNotificationPort, PlayerActionInput, PlayerActionQueuePort, PlayerActionUseCase,
};

pub use scene::{
    ApprovalDecision as SceneApprovalDecision, ApprovalDecisionInput as SceneApprovalDecisionInput,
    ApprovalDecisionResult as SceneApprovalDecisionResult, CharacterData as SceneCharacterData,
    CharacterEntity, DirectorialContextData, DirectorialContextRepositoryPort,
    DirectorialUpdateResult, DmAction, DmActionQueuePort as SceneDmActionQueuePort,
    InteractionData as SceneInteractionData, InteractionEntity, InteractionServicePort,
    InteractionTarget, LocationEntity, NpcMotivation, RequestSceneChangeInput, SceneChangeResult,
    SceneData, SceneEntity, SceneServicePort, SceneUseCase, SceneWithRelations, TimeContext,
    UpdateDirectorialInput, WorldStatePort,
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
