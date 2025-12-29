//! Use Cases Container - Port-based abstraction for game use cases
//!
//! This module provides `UseCases`, a container for all use cases used by WebSocket
//! handlers using **port traits** from `wrldbldr-engine-ports`.
//!
//! # Architecture
//!
//! This struct uses port traits from `wrldbldr-engine-ports::inbound`. This enables:
//!
//! - **Testability**: Easy mocking via port traits
//! - **Hexagonal purity**: Composition layer depends only on ports, not implementations
//! - **Flexibility**: Any implementation satisfying the port can be injected
//!
//! # Use Case Flow
//!
//! ```text
//! WebSocket Handler
//!       │
//!       ▼
//! AppState.use_cases.movement.move_to_region(...)
//!       │
//!       ├──> StagingStatePort (→ StagingStateAdapter)
//!       ├──> StagingServicePort (→ StagingServiceAdapter)
//!       └──> BroadcastPort (→ WebSocketBroadcastAdapter)
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use wrldbldr_engine_composition::UseCases;
//!
//! // Construct with implementations cast to appropriate types
//! let use_cases = UseCases::new(
//!     broadcast,
//!     movement_use_case,
//!     staging_use_case,
//!     inventory_use_case,
//!     player_action_use_case,
//!     observation_use_case,
//!     challenge_use_case,
//!     scene_use_case,
//!     connection_use_case,
//!     narrative_event_use_case,
//! );
//! ```

use std::sync::Arc;

use wrldbldr_engine_ports::inbound::{
    ChallengeUseCasePort, ConnectionUseCasePort, InventoryUseCasePort, MovementUseCasePort,
    NarrativeEventUseCasePort, ObservationUseCasePort, PlayerActionUseCasePort, SceneUseCasePort,
    StagingUseCasePort,
};
use wrldbldr_engine_ports::outbound::BroadcastPort;

/// Container for all use cases.
///
/// Use cases coordinate domain services to fulfill specific user intents.
/// They are called by WebSocket handlers and return domain result types.
///
/// All fields use `Arc<dyn ...>` for:
/// - Shared ownership across async handlers
/// - Dynamic dispatch enabling mock injection for tests
/// - Clean dependency inversion - depending on abstractions, not concretions
///
/// # Use Case Categories
///
/// ## Core Infrastructure
/// - `broadcast`: Broadcasting events to connected clients
///
/// ## Movement & Navigation
/// - `movement`: PC movement between regions and locations
///
/// ## Staging System
/// - `staging`: DM staging approval, regeneration, and pre-staging
///
/// ## Inventory & Items
/// - `inventory`: Item equip/unequip/drop/pickup operations
///
/// ## Player Actions
/// - `player_action`: Travel and queued action handling
///
/// ## NPC Interactions
/// - `observation`: NPC observation and event triggering
///
/// ## Challenge System
/// - `challenge`: Dice rolls and challenge resolution
///
/// ## Scene Management
/// - `scene`: Scene changes and directorial context
///
/// ## Connection Management
/// - `connection`: Join/leave world operations
///
/// ## Narrative Events
/// - `narrative_event`: DM approval of narrative events
#[derive(Clone)]
pub struct UseCases {
    /// Broadcast adapter for all use cases to share.
    ///
    /// Used to send events to connected WebSocket clients.
    pub broadcast: Arc<dyn BroadcastPort>,

    /// Movement use case for PC movement between regions/locations.
    ///
    /// Handles character movement, validates paths, and triggers staging.
    pub movement: Arc<dyn MovementUseCasePort>,

    /// Staging approval use case for DM staging operations.
    ///
    /// Manages NPC staging proposals, approvals, and regeneration.
    pub staging: Arc<dyn StagingUseCasePort>,

    /// Inventory use case for item equip/unequip/drop/pickup.
    ///
    /// Handles all item-related player actions.
    pub inventory: Arc<dyn InventoryUseCasePort>,

    /// Player action use case for travel and queued actions.
    ///
    /// Manages the player action queue and processing.
    pub player_action: Arc<dyn PlayerActionUseCasePort>,

    /// Observation use case for NPC observation and event triggering.
    ///
    /// Handles what happens when PCs observe NPCs and vice versa.
    pub observation: Arc<dyn ObservationUseCasePort>,

    /// Challenge use case for dice rolls and challenge resolution.
    ///
    /// Manages the full challenge workflow from triggering to outcome.
    pub challenge: Arc<dyn ChallengeUseCasePort>,

    /// Scene use case for scene changes and directorial context.
    ///
    /// Handles scene transitions and DM directorial tools.
    pub scene: Arc<dyn SceneUseCasePort>,

    /// Connection use case for join/leave world operations.
    ///
    /// Manages player connection lifecycle within worlds.
    pub connection: Arc<dyn ConnectionUseCasePort>,

    /// Narrative event use case for DM approval of narrative events.
    ///
    /// Handles the narrative event approval workflow.
    pub narrative_event: Arc<dyn NarrativeEventUseCasePort>,
}

impl UseCases {
    /// Creates a new `UseCases` instance with all use case implementations.
    ///
    /// # Arguments
    ///
    /// * `broadcast` - Implementation of [`BroadcastPort`] for event broadcasting
    /// * `movement` - Implementation of [`MovementUseCasePort`]
    /// * `staging` - Implementation of [`StagingUseCasePort`]
    /// * `inventory` - Implementation of [`InventoryUseCasePort`]
    /// * `player_action` - Implementation of [`PlayerActionUseCasePort`]
    /// * `observation` - Implementation of [`ObservationUseCasePort`]
    /// * `challenge` - Implementation of [`ChallengeUseCasePort`]
    /// * `scene` - Implementation of [`SceneUseCasePort`]
    /// * `connection` - Implementation of [`ConnectionUseCasePort`]
    /// * `narrative_event` - Implementation of [`NarrativeEventUseCasePort`]
    ///
    /// # Example
    ///
    /// ```ignore
    /// let use_cases = UseCases::new(
    ///     Arc::new(broadcast_adapter) as Arc<dyn BroadcastPort>,
    ///     Arc::new(movement_impl) as Arc<dyn MovementUseCasePort>,
    ///     Arc::new(staging_impl) as Arc<dyn StagingUseCasePort>,
    ///     Arc::new(inventory_impl) as Arc<dyn InventoryUseCasePort>,
    ///     Arc::new(player_action_impl) as Arc<dyn PlayerActionUseCasePort>,
    ///     Arc::new(observation_impl) as Arc<dyn ObservationUseCasePort>,
    ///     Arc::new(challenge_impl) as Arc<dyn ChallengeUseCasePort>,
    ///     Arc::new(scene_impl) as Arc<dyn SceneUseCasePort>,
    ///     Arc::new(connection_impl) as Arc<dyn ConnectionUseCasePort>,
    ///     Arc::new(narrative_event_impl) as Arc<dyn NarrativeEventUseCasePort>,
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        broadcast: Arc<dyn BroadcastPort>,
        movement: Arc<dyn MovementUseCasePort>,
        staging: Arc<dyn StagingUseCasePort>,
        inventory: Arc<dyn InventoryUseCasePort>,
        player_action: Arc<dyn PlayerActionUseCasePort>,
        observation: Arc<dyn ObservationUseCasePort>,
        challenge: Arc<dyn ChallengeUseCasePort>,
        scene: Arc<dyn SceneUseCasePort>,
        connection: Arc<dyn ConnectionUseCasePort>,
        narrative_event: Arc<dyn NarrativeEventUseCasePort>,
    ) -> Self {
        Self {
            broadcast,
            movement,
            staging,
            inventory,
            player_action,
            observation,
            challenge,
            scene,
            connection,
            narrative_event,
        }
    }

    /// Get a reference to the broadcast port.
    ///
    /// This allows use cases and services to broadcast events without
    /// needing a direct reference to the concrete adapter.
    pub fn broadcast(&self) -> &Arc<dyn BroadcastPort> {
        &self.broadcast
    }
}

impl std::fmt::Debug for UseCases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UseCases")
            .field("broadcast", &"Arc<dyn BroadcastPort>")
            .field("movement", &"Arc<dyn MovementUseCasePort>")
            .field("staging", &"Arc<dyn StagingUseCasePort>")
            .field("inventory", &"Arc<dyn InventoryUseCasePort>")
            .field("player_action", &"Arc<dyn PlayerActionUseCasePort>")
            .field("observation", &"Arc<dyn ObservationUseCasePort>")
            .field("challenge", &"Arc<dyn ChallengeUseCasePort>")
            .field("scene", &"Arc<dyn SceneUseCasePort>")
            .field("connection", &"Arc<dyn ConnectionUseCasePort>")
            .field("narrative_event", &"Arc<dyn NarrativeEventUseCasePort>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_engine_ports::inbound::{
        MockChallengeUseCasePort, MockConnectionUseCasePort, MockInventoryUseCasePort,
        MockMovementUseCasePort, MockNarrativeEventUseCasePort, MockObservationUseCasePort,
        MockPlayerActionUseCasePort, MockSceneUseCasePort, MockStagingUseCasePort,
    };
    use wrldbldr_engine_ports::outbound::MockBroadcastPort;

    #[test]
    fn test_use_cases_construction() {
        let use_cases = UseCases::new(
            Arc::new(MockBroadcastPort::new()),
            Arc::new(MockMovementUseCasePort::new()),
            Arc::new(MockStagingUseCasePort::new()),
            Arc::new(MockInventoryUseCasePort::new()),
            Arc::new(MockPlayerActionUseCasePort::new()),
            Arc::new(MockObservationUseCasePort::new()),
            Arc::new(MockChallengeUseCasePort::new()),
            Arc::new(MockSceneUseCasePort::new()),
            Arc::new(MockConnectionUseCasePort::new()),
            Arc::new(MockNarrativeEventUseCasePort::new()),
        );

        // Verify debug output works
        let debug_str = format!("{:?}", use_cases);
        assert!(debug_str.contains("UseCases"));
        assert!(debug_str.contains("BroadcastPort"));
        assert!(debug_str.contains("MovementUseCasePort"));
        assert!(debug_str.contains("StagingUseCasePort"));
    }

    #[test]
    fn test_use_cases_clone() {
        let use_cases = UseCases::new(
            Arc::new(MockBroadcastPort::new()),
            Arc::new(MockMovementUseCasePort::new()),
            Arc::new(MockStagingUseCasePort::new()),
            Arc::new(MockInventoryUseCasePort::new()),
            Arc::new(MockPlayerActionUseCasePort::new()),
            Arc::new(MockObservationUseCasePort::new()),
            Arc::new(MockChallengeUseCasePort::new()),
            Arc::new(MockSceneUseCasePort::new()),
            Arc::new(MockConnectionUseCasePort::new()),
            Arc::new(MockNarrativeEventUseCasePort::new()),
        );

        // Clone should work (important for sharing across async tasks)
        let _cloned = use_cases.clone();
    }

    #[test]
    fn test_broadcast_accessor() {
        let use_cases = UseCases::new(
            Arc::new(MockBroadcastPort::new()),
            Arc::new(MockMovementUseCasePort::new()),
            Arc::new(MockStagingUseCasePort::new()),
            Arc::new(MockInventoryUseCasePort::new()),
            Arc::new(MockPlayerActionUseCasePort::new()),
            Arc::new(MockObservationUseCasePort::new()),
            Arc::new(MockChallengeUseCasePort::new()),
            Arc::new(MockSceneUseCasePort::new()),
            Arc::new(MockConnectionUseCasePort::new()),
            Arc::new(MockNarrativeEventUseCasePort::new()),
        );

        // broadcast() accessor should return the same Arc
        let _broadcast = use_cases.broadcast();
    }
}
