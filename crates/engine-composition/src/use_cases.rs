//! Use Cases Container - Port-based abstraction for game use cases
//!
//! This module provides `UseCases`, a container for all use cases used by WebSocket
//! handlers using **port traits** from `wrldbldr-engine-ports`.
//!
//! # Architecture
//!
//! Unlike `engine-adapters/src/infrastructure/state/use_cases.rs` which uses
//! concrete use case implementations from `engine-app`, this struct uses only
//! port traits and placeholder types. This enables:
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
//! # Implementation Status
//!
//! Use cases without dedicated port traits use `Box<dyn Any + Send + Sync>` as a
//! placeholder until proper port traits are defined:
//!
//! - [ ] MovementUseCasePort - PC movement between regions/locations
//! - [ ] StagingApprovalUseCasePort - DM staging approval, regeneration, pre-staging
//! - [ ] InventoryUseCasePort - Item management
//! - [ ] PlayerActionUseCasePort - Player action handling
//! - [ ] ObservationUseCasePort - NPC observation events
//! - [ ] ChallengeUseCasePort - Challenge resolution
//! - [ ] SceneUseCasePort - Scene management
//! - [ ] ConnectionUseCasePort - World connection management
//! - [ ] NarrativeEventUseCasePort - Narrative event approval
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

use std::any::Any;
use std::sync::Arc;

use wrldbldr_engine_ports::outbound::BroadcastPort;

/// Placeholder type for use cases that don't have port traits yet.
///
/// This allows the composition layer to be defined now, with proper port
/// traits to be added incrementally as each use case is refactored.
///
/// When a proper port trait is added, the corresponding field in `UseCases`
/// will be changed from `Arc<dyn UseCasePlaceholder>` to `Arc<dyn TheNewPort>`.
pub trait UseCasePlaceholder: Any + Send + Sync {
    /// Downcast to the concrete type if needed.
    ///
    /// This is a workaround until proper port traits are defined.
    fn as_any(&self) -> &dyn Any;
}

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
    pub movement: Arc<dyn UseCasePlaceholder>,

    /// Staging approval use case for DM staging operations.
    ///
    /// Manages NPC staging proposals, approvals, and regeneration.
    pub staging: Arc<dyn UseCasePlaceholder>,

    /// Inventory use case for item equip/unequip/drop/pickup.
    ///
    /// Handles all item-related player actions.
    pub inventory: Arc<dyn UseCasePlaceholder>,

    /// Player action use case for travel and queued actions.
    ///
    /// Manages the player action queue and processing.
    pub player_action: Arc<dyn UseCasePlaceholder>,

    /// Observation use case for NPC observation and event triggering.
    ///
    /// Handles what happens when PCs observe NPCs and vice versa.
    pub observation: Arc<dyn UseCasePlaceholder>,

    /// Challenge use case for dice rolls and challenge resolution.
    ///
    /// Manages the full challenge workflow from triggering to outcome.
    pub challenge: Arc<dyn UseCasePlaceholder>,

    /// Scene use case for scene changes and directorial context.
    ///
    /// Handles scene transitions and DM directorial tools.
    pub scene: Arc<dyn UseCasePlaceholder>,

    /// Connection use case for join/leave world operations.
    ///
    /// Manages player connection lifecycle within worlds.
    pub connection: Arc<dyn UseCasePlaceholder>,

    /// Narrative event use case for DM approval of narrative events.
    ///
    /// Handles the narrative event approval workflow.
    pub narrative_event: Arc<dyn UseCasePlaceholder>,
}

impl UseCases {
    /// Creates a new `UseCases` instance with all use case implementations.
    ///
    /// # Arguments
    ///
    /// * `broadcast` - Implementation of [`BroadcastPort`] for event broadcasting
    /// * `movement` - Movement use case implementation
    /// * `staging` - Staging approval use case implementation
    /// * `inventory` - Inventory use case implementation
    /// * `player_action` - Player action use case implementation
    /// * `observation` - Observation use case implementation
    /// * `challenge` - Challenge use case implementation
    /// * `scene` - Scene use case implementation
    /// * `connection` - Connection use case implementation
    /// * `narrative_event` - Narrative event use case implementation
    ///
    /// # Example
    ///
    /// ```ignore
    /// let use_cases = UseCases::new(
    ///     Arc::new(broadcast_adapter) as Arc<dyn BroadcastPort>,
    ///     Arc::new(movement_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(staging_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(inventory_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(player_action_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(observation_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(challenge_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(scene_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(connection_impl) as Arc<dyn UseCasePlaceholder>,
    ///     Arc::new(narrative_event_impl) as Arc<dyn UseCasePlaceholder>,
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        broadcast: Arc<dyn BroadcastPort>,
        movement: Arc<dyn UseCasePlaceholder>,
        staging: Arc<dyn UseCasePlaceholder>,
        inventory: Arc<dyn UseCasePlaceholder>,
        player_action: Arc<dyn UseCasePlaceholder>,
        observation: Arc<dyn UseCasePlaceholder>,
        challenge: Arc<dyn UseCasePlaceholder>,
        scene: Arc<dyn UseCasePlaceholder>,
        connection: Arc<dyn UseCasePlaceholder>,
        narrative_event: Arc<dyn UseCasePlaceholder>,
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
            .field("movement", &"Arc<dyn UseCasePlaceholder>")
            .field("staging", &"Arc<dyn UseCasePlaceholder>")
            .field("inventory", &"Arc<dyn UseCasePlaceholder>")
            .field("player_action", &"Arc<dyn UseCasePlaceholder>")
            .field("observation", &"Arc<dyn UseCasePlaceholder>")
            .field("challenge", &"Arc<dyn UseCasePlaceholder>")
            .field("scene", &"Arc<dyn UseCasePlaceholder>")
            .field("connection", &"Arc<dyn UseCasePlaceholder>")
            .field("narrative_event", &"Arc<dyn UseCasePlaceholder>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_engine_ports::outbound::MockBroadcastPort;

    /// Simple mock placeholder for use cases
    struct MockUseCasePlaceholder;

    impl UseCasePlaceholder for MockUseCasePlaceholder {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_use_cases_construction() {
        let use_cases = UseCases::new(
            Arc::new(MockBroadcastPort::new()),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
        );

        // Verify debug output works
        let debug_str = format!("{:?}", use_cases);
        assert!(debug_str.contains("UseCases"));
        assert!(debug_str.contains("BroadcastPort"));
    }

    #[test]
    fn test_use_cases_clone() {
        let use_cases = UseCases::new(
            Arc::new(MockBroadcastPort::new()),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
        );

        // Clone should work (important for sharing across async tasks)
        let _cloned = use_cases.clone();
    }

    #[test]
    fn test_broadcast_accessor() {
        let use_cases = UseCases::new(
            Arc::new(MockBroadcastPort::new()),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
            Arc::new(MockUseCasePlaceholder),
        );

        // broadcast() accessor should return the same Arc
        let _broadcast = use_cases.broadcast();
    }
}
