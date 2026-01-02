//! Abstracted Queue Services Container
//!
//! This module provides an abstracted container for all queue-related services
//! using port traits instead of concrete implementations. This follows hexagonal
//! architecture principles where the composition layer depends only on ports,
//! not on concrete adapters.
//!
//! # Architecture
//!
//! This struct mirrors `engine-adapters::infrastructure::state::QueueServices`
//! but uses `Arc<dyn Trait>` for all fields instead of concrete types with
//! generic parameters. This enables:
//!
//! - Clean separation between composition and adapter layers
//! - Easy testing with mock implementations
//! - Runtime polymorphism for queue backends
//!
//! # Example
//!
//! ```ignore
//! use wrldbldr_engine_composition::QueueServices;
//!
//! let queue_services = QueueServices::new(
//!     player_action_queue,
//!     dm_action_queue,
//!     llm_queue,
//!     asset_generation_queue,
//!     dm_approval_queue,
//!     challenge_outcome_queue,
//! );
//! ```

use std::sync::Arc;

// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::DmActionQueueServicePort;
// True outbound ports (adapter-implemented infrastructure)
use wrldbldr_engine_ports::outbound::{
    AssetGenerationQueueServicePort, ChallengeOutcomeData, DmApprovalQueueServicePort,
    LlmQueueServicePort, PlayerActionQueueServicePort, QueuePort,
};

/// Queue processing services for asynchronous operations.
///
/// This struct groups all queue-related services that handle background
/// processing of player actions, DM actions, LLM requests, asset generation,
/// and approval workflows.
///
/// All fields use trait objects (`Arc<dyn Trait>`) to maintain clean
/// architectural boundaries between the composition layer and concrete
/// adapter implementations.
///
/// # Queue Types
///
/// - **Player Action Queue**: Handles player input actions (talk, examine, move, etc.)
/// - **DM Action Queue**: High-priority queue for Dungeon Master actions
/// - **LLM Queue**: Processes requests to the language model (NPC responses, suggestions)
/// - **Asset Generation Queue**: Handles ComfyUI image generation requests
/// - **DM Approval Queue**: Manages approval workflow for DM review
/// - **Challenge Outcome Queue**: Stores challenge results awaiting DM approval
#[derive(Clone)]
pub struct QueueServices {
    /// Service for enqueueing and processing player actions.
    ///
    /// Player actions (talk, examine, move, etc.) are enqueued here and
    /// processed to generate LLM requests for NPC responses.
    pub player_action_queue_service: Arc<dyn PlayerActionQueueServicePort>,

    /// Service for enqueueing and processing DM actions.
    ///
    /// DM actions have high priority and are processed before player actions.
    /// Includes approval decisions, direct NPC control, and scene transitions.
    pub dm_action_queue_service: Arc<dyn DmActionQueueServicePort>,

    /// Service for enqueueing and processing LLM requests.
    ///
    /// Handles both NPC response generation and suggestion generation.
    /// Processed items are sent to Ollama or similar LLM backend.
    pub llm_queue_service: Arc<dyn LlmQueueServicePort>,

    /// Service for enqueueing and processing asset generation requests.
    ///
    /// Handles ComfyUI workflow execution for character portraits,
    /// location images, and other visual assets.
    pub asset_generation_queue_service: Arc<dyn AssetGenerationQueueServicePort>,

    /// Service for managing DM approval workflow.
    ///
    /// NPC responses and tool calls are queued here for DM review.
    /// DMs can accept, modify, reject, or take over responses.
    pub dm_approval_queue_service: Arc<dyn DmApprovalQueueServicePort>,

    /// Queue for challenge outcomes awaiting DM approval.
    ///
    /// When a challenge (skill check, combat, etc.) is resolved,
    /// the outcome is stored here for DM review before being applied.
    pub challenge_outcome_queue: Arc<dyn QueuePort<ChallengeOutcomeData>>,
}

impl QueueServices {
    /// Creates a new `QueueServices` instance with all queue processing services.
    ///
    /// # Arguments
    ///
    /// * `player_action_queue_service` - Service for player action processing
    /// * `dm_action_queue_service` - Service for DM action processing
    /// * `llm_queue_service` - Service for LLM request processing
    /// * `asset_generation_queue_service` - Service for asset generation
    /// * `dm_approval_queue_service` - Service for DM approval workflow
    /// * `challenge_outcome_queue` - Queue for challenge outcome storage
    ///
    /// # Example
    ///
    /// ```ignore
    /// let queue_services = QueueServices::new(
    ///     Arc::new(player_action_service),
    ///     Arc::new(dm_action_service),
    ///     Arc::new(llm_service),
    ///     Arc::new(asset_service),
    ///     Arc::new(approval_service),
    ///     Arc::new(challenge_queue),
    /// );
    /// ```
    pub fn new(
        player_action_queue_service: Arc<dyn PlayerActionQueueServicePort>,
        dm_action_queue_service: Arc<dyn DmActionQueueServicePort>,
        llm_queue_service: Arc<dyn LlmQueueServicePort>,
        asset_generation_queue_service: Arc<dyn AssetGenerationQueueServicePort>,
        dm_approval_queue_service: Arc<dyn DmApprovalQueueServicePort>,
        challenge_outcome_queue: Arc<dyn QueuePort<ChallengeOutcomeData>>,
    ) -> Self {
        Self {
            player_action_queue_service,
            dm_action_queue_service,
            llm_queue_service,
            asset_generation_queue_service,
            dm_approval_queue_service,
            challenge_outcome_queue,
        }
    }
}

impl std::fmt::Debug for QueueServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueServices")
            .field(
                "player_action_queue_service",
                &"Arc<dyn PlayerActionQueueServicePort>",
            )
            .field(
                "dm_action_queue_service",
                &"Arc<dyn DmActionQueueServicePort>",
            )
            .field("llm_queue_service", &"Arc<dyn LlmQueueServicePort>")
            .field(
                "asset_generation_queue_service",
                &"Arc<dyn AssetGenerationQueueServicePort>",
            )
            .field(
                "dm_approval_queue_service",
                &"Arc<dyn DmApprovalQueueServicePort>",
            )
            .field(
                "challenge_outcome_queue",
                &"Arc<dyn QueuePort<ChallengeOutcomeData>>",
            )
            .finish()
    }
}
