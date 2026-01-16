//! Staging use cases.
//!
//! Handles staging approval requests, regeneration, and approval application.

mod approve;
mod auto_approve;
mod regenerate;
mod request_approval;
mod suggestions;
mod types;

#[cfg(test)]
mod llm_integration_tests;

use std::sync::Arc;

use wrldbldr_domain::WorldId;

use crate::infrastructure::app_settings::AppSettings;
use crate::infrastructure::ports::RepoError;
use crate::repositories::Settings;

// Re-export types
pub use crate::infrastructure::ports::{PendingStagingRequest, TimeSuggestion};
pub use crate::repositories::{PendingStaging, TimeSuggestionStore};
pub use approve::{ApproveStagingInput, ApproveStagingRequest, StagingReadyPayload};
pub use auto_approve::AutoApproveStagingTimeout;
pub use regenerate::RegenerateStagingSuggestions;
pub use request_approval::{RequestStagingApproval, StagingApprovalContext, StagingApprovalInput};
pub use types::{
    ApprovedNpc, GameTimeData, NpcPresent, PreviousStagingData, StagedNpc, StagingApprovalData,
    StagingPendingData, StagingRequestResult, WaitingPc,
};

/// Timeout in seconds before a pending staging request auto-approves.
/// This is the delay shown to players while waiting for DM approval.
/// Not to be confused with TTL (time-to-live), which controls how long
/// approved staging remains valid (configured via `default_presence_cache_ttl_hours`).
pub const DEFAULT_STAGING_TIMEOUT_SECONDS: u64 = 30;

/// Fetches world settings with graceful fallback to defaults.
///
/// Returns `AppSettings::default()` if:
/// - No world-specific settings exist (Ok(None))
/// - Settings fetch fails (logs warning and uses defaults)
///
/// This ensures staging operations never fail due to settings unavailability.
async fn get_settings_with_fallback(
    settings: &Settings,
    world_id: WorldId,
    operation: &str,
) -> AppSettings {
    match settings.get_for_world(world_id).await {
        Ok(settings) => settings,
        Err(e) => {
            tracing::warn!(
                error = %e,
                world_id = %world_id,
                "Failed to load world settings for {}, using defaults",
                operation
            );
            AppSettings::default()
        }
    }
}

/// Container for staging use cases.
pub struct StagingUseCases {
    pub request_approval: Arc<RequestStagingApproval>,
    pub regenerate: Arc<RegenerateStagingSuggestions>,
    pub approve: Arc<ApproveStagingRequest>,
    pub auto_approve_timeout: Arc<AutoApproveStagingTimeout>,
}

impl StagingUseCases {
    pub fn new(
        request_approval: Arc<RequestStagingApproval>,
        regenerate: Arc<RegenerateStagingSuggestions>,
        approve: Arc<ApproveStagingRequest>,
        auto_approve_timeout: Arc<AutoApproveStagingTimeout>,
    ) -> Self {
        Self {
            request_approval,
            regenerate,
            approve,
            auto_approve_timeout,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StagingError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Region not found")]
    RegionNotFound,
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
