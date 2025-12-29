//! Player Action Use Case
//!
//! Handles player actions including immediate travel and queued actions.
//!
//! # Responsibilities
//!
//! - Process travel actions immediately (update location, resolve scene)
//! - Queue non-travel actions for LLM processing
//! - Coordinate with MovementUseCase for travel
//! - Notify DM of queued actions
//!
//! # Architecture Note
//!
//! Travel actions are special - they bypass the queue and execute immediately.
//! This use case delegates travel to MovementUseCase to avoid duplicating
//! movement logic.

use std::sync::Arc;
use tracing::{debug, info};

use wrldbldr_domain::{ActionId, LocationId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::BroadcastPort;

use super::errors::ActionError;
use super::movement::{ExitToLocationInput, MoveToRegionInput, MovementResult, MovementUseCase};

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{ActionResult, PlayerActionInput};

// =============================================================================
// Player Action Queue Port
// =============================================================================

/// Port for player action queue operations
#[async_trait::async_trait]
pub trait PlayerActionQueuePort: Send + Sync {
    /// Enqueue an action
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        player_id: String,
        pc_id: Option<PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<ActionId, String>;

    /// Get current queue depth
    async fn depth(&self) -> Result<usize, String>;
}

/// Port for sending messages to DM
#[async_trait::async_trait]
pub trait DmNotificationPort: Send + Sync {
    /// Send action queued notification to DM
    async fn notify_action_queued(
        &self,
        world_id: &WorldId,
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    );
}

// =============================================================================
// Player Action Use Case
// =============================================================================

/// Use case for handling player actions
///
/// Handles both immediate actions (travel) and queued actions (speak, interact).
/// Delegates travel to MovementUseCase to avoid duplicating movement logic.
pub struct PlayerActionUseCase {
    /// Movement use case for travel actions
    movement: Arc<MovementUseCase>,
    /// Queue service for non-immediate actions
    action_queue: Arc<dyn PlayerActionQueuePort>,
    /// DM notification port
    dm_notification: Arc<dyn DmNotificationPort>,
    /// Broadcast port for side-effect notifications
    broadcast: Arc<dyn BroadcastPort>,
}

impl PlayerActionUseCase {
    /// Create a new PlayerActionUseCase with all dependencies
    pub fn new(
        movement: Arc<MovementUseCase>,
        action_queue: Arc<dyn PlayerActionQueuePort>,
        dm_notification: Arc<dyn DmNotificationPort>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            movement,
            action_queue,
            dm_notification,
            broadcast,
        }
    }

    /// Handle a player action
    ///
    /// Travel actions are processed immediately.
    /// All other actions are queued for LLM/DM processing.
    pub async fn handle_action(
        &self,
        ctx: UseCaseContext,
        input: PlayerActionInput,
    ) -> Result<ActionResult, ActionError> {
        debug!(
            action_type = %input.action_type,
            target = ?input.target,
            "Handling player action"
        );

        let action_id = ActionId::new();

        // Travel actions are special - they execute immediately
        if input.action_type == "travel" {
            return self.handle_travel(ctx, action_id, input.target).await;
        }

        // Non-travel actions go through the queue
        self.queue_action(ctx, action_id, input).await
    }

    /// Handle a travel action
    async fn handle_travel(
        &self,
        ctx: UseCaseContext,
        action_id: ActionId,
        target: Option<String>,
    ) -> Result<ActionResult, ActionError> {
        let target_str = target.ok_or(ActionError::MissingTarget)?;
        let pc_id = ctx.pc_id.ok_or(ActionError::NoPcSelected)?;

        debug!(
            pc_id = %pc_id,
            target = %target_str,
            "Processing travel action"
        );

        // Try to parse as region ID first (movement within location)
        if let Ok(region_uuid) = uuid::Uuid::parse_str(&target_str) {
            let region_id = RegionId::from_uuid(region_uuid);

            let movement_input = MoveToRegionInput {
                pc_id,
                target_region_id: region_id,
            };

            match self
                .movement
                .move_to_region(ctx.clone(), movement_input)
                .await
            {
                Ok(MovementResult::SceneChanged(event)) => {
                    return Ok(ActionResult::TravelCompleted {
                        action_id: action_id.to_string(),
                        scene: event,
                    });
                }
                Ok(MovementResult::StagingPending {
                    region_id,
                    region_name,
                }) => {
                    return Ok(ActionResult::TravelPending {
                        action_id: action_id.to_string(),
                        region_id,
                        region_name,
                    });
                }
                Ok(MovementResult::Blocked { reason }) => {
                    return Err(ActionError::MovementBlocked(reason));
                }
                Err(e) => {
                    // Maybe it's a location ID, not a region ID
                    debug!(error = %e, "Region movement failed, trying as location exit");
                }
            }
        }

        // Try to parse as location ID (exit to different location)
        if let Ok(location_uuid) = uuid::Uuid::parse_str(&target_str) {
            let location_id = LocationId::from_uuid(location_uuid);

            let exit_input = ExitToLocationInput {
                pc_id,
                target_location_id: location_id,
                arrival_region_id: None,
            };

            match self.movement.exit_to_location(ctx, exit_input).await {
                Ok(MovementResult::SceneChanged(event)) => {
                    return Ok(ActionResult::TravelCompleted {
                        action_id: action_id.to_string(),
                        scene: event,
                    });
                }
                Ok(MovementResult::StagingPending {
                    region_id,
                    region_name,
                }) => {
                    return Ok(ActionResult::TravelPending {
                        action_id: action_id.to_string(),
                        region_id,
                        region_name,
                    });
                }
                Ok(MovementResult::Blocked { reason }) => {
                    return Err(ActionError::MovementBlocked(reason));
                }
                Err(e) => {
                    return Err(ActionError::MovementFailed(e.to_string()));
                }
            }
        }

        // Invalid target ID
        Err(ActionError::MovementFailed(format!(
            "Invalid travel target: {}",
            target_str
        )))
    }

    /// Queue a non-travel action
    async fn queue_action(
        &self,
        ctx: UseCaseContext,
        action_id: ActionId,
        input: PlayerActionInput,
    ) -> Result<ActionResult, ActionError> {
        let player_id = ctx.user_id.clone();

        // Enqueue the action
        self.action_queue
            .enqueue_action(
                &ctx.world_id,
                player_id.clone(),
                ctx.pc_id,
                input.action_type.clone(),
                input.target,
                input.dialogue,
            )
            .await
            .map_err(|e| ActionError::QueueFailed(e))?;

        // Get queue depth
        let depth = self.action_queue.depth().await.unwrap_or(0);

        // Notify DM
        self.dm_notification
            .notify_action_queued(
                &ctx.world_id,
                action_id.to_string(),
                player_id,
                input.action_type,
                depth,
            )
            .await;

        info!(
            action_id = %action_id,
            queue_depth = depth,
            "Action queued"
        );

        Ok(ActionResult::Queued {
            action_id: action_id.to_string(),
            queue_depth: depth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_result_variants() {
        let action_id = ActionId::new().to_string();

        let queued = ActionResult::Queued {
            action_id: action_id.clone(),
            queue_depth: 5,
        };

        let pending = ActionResult::TravelPending {
            action_id: action_id.clone(),
            region_id: RegionId::from_uuid(uuid::Uuid::new_v4()),
            region_name: "Town Square".to_string(),
        };

        match queued {
            ActionResult::Queued { queue_depth, .. } => assert_eq!(queue_depth, 5),
            _ => panic!("Expected Queued variant"),
        }

        match pending {
            ActionResult::TravelPending { region_name, .. } => {
                assert_eq!(region_name, "Town Square")
            }
            _ => panic!("Expected TravelPending variant"),
        }
    }

    #[test]
    fn test_player_action_input() {
        let travel = PlayerActionInput {
            action_type: "travel".to_string(),
            target: Some("room-123".to_string()),
            dialogue: None,
        };

        let speak = PlayerActionInput {
            action_type: "speak".to_string(),
            target: Some("npc-456".to_string()),
            dialogue: Some("Hello there!".to_string()),
        };

        assert_eq!(travel.action_type, "travel");
        assert!(travel.dialogue.is_none());

        assert_eq!(speak.action_type, "speak");
        assert!(speak.dialogue.is_some());
    }
}
