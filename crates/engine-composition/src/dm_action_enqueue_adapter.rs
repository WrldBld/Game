//! DM Action Enqueue Adapter - Bridges DmActionEnqueuePort to DmActionQueueServicePort
//!
//! This adapter implements the `DmActionEnqueuePort` outbound port by delegating
//! to the internal `DmActionQueueServicePort` service trait.
//!
//! # Architecture
//!
//! This adapter lives in `engine-composition` (not `engine-adapters`) because:
//! - It needs to bridge between a port (`DmActionEnqueuePort`) and an internal
//!   service trait (`DmActionQueueServicePort`)
//! - `engine-composition` is allowed to depend on `engine-app` for DI wiring
//! - `engine-adapters` should NOT depend on `engine-app`
//!
//! The composition root creates this adapter and provides it to other adapters
//! that need to enqueue DM actions.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use wrldbldr_engine_app::application::services::internal::{
    DmAction as InternalDmAction, DmActionQueueServicePort, DmActionType as InternalDmActionType,
    DmDecision as InternalDmDecision,
};
use wrldbldr_engine_ports::outbound::{
    DmActionEnqueuePort, DmActionEnqueueRequest, DmActionEnqueueType, DmEnqueueDecision, QueueError,
};

/// Adapter that implements DmActionEnqueuePort by delegating to DmActionQueueServicePort
pub struct DmActionEnqueueAdapter {
    dm_action_queue_service: Arc<dyn DmActionQueueServicePort>,
}

impl DmActionEnqueueAdapter {
    /// Create a new adapter wrapping a DmActionQueueServicePort
    pub fn new(dm_action_queue_service: Arc<dyn DmActionQueueServicePort>) -> Self {
        Self {
            dm_action_queue_service,
        }
    }
}

#[async_trait]
impl DmActionEnqueuePort for DmActionEnqueueAdapter {
    async fn enqueue(&self, request: DmActionEnqueueRequest) -> Result<Uuid, QueueError> {
        // Convert port DTO to internal DTO
        let internal_action_type = match request.action_type {
            DmActionEnqueueType::ApprovalDecision {
                request_id,
                decision,
            } => {
                let internal_decision = match decision {
                    DmEnqueueDecision::Approve => InternalDmDecision::Accept,
                    DmEnqueueDecision::Reject { reason } => {
                        InternalDmDecision::Reject { feedback: reason }
                    }
                    DmEnqueueDecision::ApproveWithEdits { modified_text } => {
                        InternalDmDecision::TakeOver {
                            dm_response: modified_text,
                        }
                    }
                };
                InternalDmActionType::ApprovalDecision {
                    request_id,
                    decision: internal_decision,
                }
            }
        };

        // Create internal DM action
        let internal_action = InternalDmAction {
            world_id: *request.world_id.as_uuid(),
            dm_id: request.dm_id,
            action: internal_action_type,
            timestamp: request.timestamp,
        };

        // Delegate to internal service
        self.dm_action_queue_service
            .enqueue(internal_action)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))
    }
}
