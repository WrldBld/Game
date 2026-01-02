//! Narrative Event Use Case Port
//!
//! Defines the inbound port for narrative event operations. This trait abstracts
//! the `NarrativeEventUseCase` from `engine-app`, allowing adapters to handle
//! DM approval of narrative event suggestions without depending on the application layer.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        ADAPTER LAYER                                    │
//! │                                                                         │
//! │  narrative_event_handler.rs                                             │
//! │      │                                                                  │
//! │      └──> dyn NarrativeEventUseCasePort                                 │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                        PORTS LAYER                                      │
//! │                                                                         │
//! │  NarrativeEventUseCasePort trait (this file)                            │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                     APPLICATION LAYER                                   │
//! │                                                                         │
//! │  NarrativeEventUseCase implements NarrativeEventUseCasePort             │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Operations
//!
//! - **handle_suggestion_decision**: DM approves or rejects a narrative event suggestion
//!
//! # DM-Only Operations
//!
//! This port handles DM approval workflow for narrative events. When approved,
//! the event is triggered and broadcast to all players in the world.

use async_trait::async_trait;

use super::UseCaseContext;
use crate::outbound::NarrativeEventError;
use crate::outbound::{NarrativeEventDecisionResult, NarrativeEventSuggestionDecisionInput};

/// Port for narrative event operations
///
/// This port abstracts narrative event use case operations, allowing adapters to
/// invoke DM approval handling without depending on the application layer directly.
///
/// # Implementors
///
/// - `NarrativeEventUseCase` in `engine-app`
///
/// # Example
///
/// ```ignore
/// async fn handle_decision(
///     narrative_port: Arc<dyn NarrativeEventUseCasePort>,
///     ctx: UseCaseContext,
///     request_id: String,
///     event_id: String,
///     approved: bool,
/// ) -> Result<NarrativeEventDecisionResult, NarrativeEventError> {
///     let input = NarrativeEventSuggestionDecisionInput {
///         request_id,
///         event_id,
///         approved,
///         selected_outcome: None,
///     };
///     narrative_port.handle_suggestion_decision(ctx, input).await
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait NarrativeEventUseCasePort: Send + Sync {
    /// Handle DM's decision on a narrative event suggestion
    ///
    /// Processes the DM's approval or rejection of a narrative event suggestion.
    /// If approved, triggers the event and broadcasts to all players in the world.
    /// If rejected, no broadcast is sent.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info (must be DM)
    /// * `input` - Contains request ID, event ID, approval status, and optional selected outcome
    ///
    /// # Returns
    ///
    /// * `Ok(NarrativeEventDecisionResult { triggered: true })` - Event approved and broadcast
    /// * `Ok(NarrativeEventDecisionResult { triggered: false })` - Event rejected
    /// * `Err(NarrativeEventError::Unauthorized)` - User is not the DM
    /// * `Err(NarrativeEventError::ApprovalFailed)` - Approval service encountered an error
    async fn handle_suggestion_decision(
        &self,
        ctx: UseCaseContext,
        input: NarrativeEventSuggestionDecisionInput,
    ) -> Result<NarrativeEventDecisionResult, NarrativeEventError>;
}
