//! DM Action Enqueue Port - Outbound port for enqueueing DM actions
//!
//! This port provides a simplified interface for adapters that need to enqueue
//! DM actions without depending on the full internal service implementation.
//!
//! # Architecture
//!
//! This is an outbound port that:
//! - Is implemented by a bridge adapter in `engine-composition`
//! - Is depended upon by `SceneDmActionQueueAdapter` in `engine-adapters`
//! - Abstracts away the internal `DmActionQueueServicePort` from adapters

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wrldbldr_domain::WorldId;

use super::QueueError;

// ============================================================================
// Port DTOs
// ============================================================================

/// Request to enqueue a DM action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmActionEnqueueRequest {
    /// World where the action occurs
    pub world_id: WorldId,
    /// DM who initiated the action
    pub dm_id: String,
    /// The specific action to perform
    pub action_type: DmActionEnqueueType,
    /// When the action was submitted
    pub timestamp: DateTime<Utc>,
}

/// Types of DM actions that can be enqueued
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DmActionEnqueueType {
    /// Process an approval decision
    ApprovalDecision {
        request_id: String,
        decision: DmEnqueueDecision,
    },
}

/// DM decision types for enqueue port
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum DmEnqueueDecision {
    /// Approve the request as-is
    Approve,
    /// Reject with reason
    Reject { reason: String },
    /// Approve with modifications
    ApproveWithEdits { modified_text: String },
}

// ============================================================================
// Port Trait
// ============================================================================

/// Outbound port for enqueueing DM actions
///
/// This port provides a simplified interface for adapters to enqueue DM actions
/// without depending on the internal `DmActionQueueServicePort`.
#[async_trait]
pub trait DmActionEnqueuePort: Send + Sync {
    /// Enqueue a DM action for processing
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, request: DmActionEnqueueRequest) -> Result<Uuid, QueueError>;
}
