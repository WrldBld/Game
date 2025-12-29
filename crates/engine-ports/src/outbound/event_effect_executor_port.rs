//! Event effect executor port - Interface for executing narrative event outcome effects
//!
//! This port abstracts the execution of `EventEffect` items that are defined in
//! narrative event outcomes. When a DM approves a narrative event, the selected
//! outcome's effects are executed through this service.
//!
//! # Effects Supported
//!
//! - `SetFlag` - Sets a game flag (stored in session state)
//! - `EnableChallenge` / `DisableChallenge` - Toggles challenge availability
//! - `EnableEvent` / `DisableEvent` - Toggles narrative event availability
//! - `RevealInformation` - Reveals info to players (logged, optionally journaled)
//! - `GiveItem` / `TakeItem` - Modifies player inventory (logged for DM to narrate)
//! - `ModifyRelationship` - Changes NPC relationship sentiment
//! - `ModifyStat` - Changes character stat value
//! - `TriggerScene` - Initiates scene transition
//! - `StartCombat` - Initiates combat encounter
//! - `AddReward` - Grants experience or rewards
//! - `Custom` - Logs for DM action
//!
//! # Architecture
//!
//! The service follows hexagonal architecture, depending on repository ports.
//! Conversation history logging is handled by the caller via WorldStateManager.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::EventEffect;
use wrldbldr_domain::WorldId;

/// Result of executing a single effect
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectExecutionResult {
    /// Description of what happened
    pub description: String,
    /// Whether the effect was fully executed (vs logged for DM action)
    pub was_executed: bool,
    /// Any warning or note
    pub note: Option<String>,
}

/// Result of executing all effects from an outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeExecutionResult {
    /// Individual effect results
    pub effects: Vec<EffectExecutionResult>,
    /// Total effects attempted
    pub total: usize,
    /// Effects that were fully executed
    pub executed_count: usize,
    /// Effects logged for DM action
    pub logged_count: usize,
}

impl OutcomeExecutionResult {
    /// Create an empty result (no effects executed)
    pub fn empty() -> Self {
        Self {
            effects: Vec::new(),
            total: 0,
            executed_count: 0,
            logged_count: 0,
        }
    }
}

/// Port for event effect executor operations
///
/// This trait defines the application use cases for executing narrative event
/// outcome effects. It takes a list of `EventEffect` items and executes them,
/// making the necessary repository calls.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait EventEffectExecutorPort: Send + Sync {
    /// Execute all effects from an outcome
    ///
    /// # Arguments
    ///
    /// * `effects` - The list of effects to execute
    /// * `world_id` - The world where effects should be applied
    ///
    /// # Returns
    ///
    /// An `OutcomeExecutionResult` summarizing what was done.
    ///
    /// # Note
    ///
    /// Some effects are "logged for DM action" rather than executed directly.
    /// For example, `GiveItem` logs that an item should be given, but the DM
    /// narrates the actual acquisition. Check `was_executed` on individual
    /// results to see what was directly applied vs logged.
    async fn execute_effects(
        &self,
        effects: &[EventEffect],
        world_id: WorldId,
    ) -> OutcomeExecutionResult;
}
