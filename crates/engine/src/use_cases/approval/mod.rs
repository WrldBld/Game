//! DM approval use cases.

use std::sync::Arc;

/// Container for approval use cases.
pub struct ApprovalUseCases {
    pub approve_staging: Arc<ApproveStaging>,
    pub approve_suggestion: Arc<ApproveSuggestion>,
}

impl ApprovalUseCases {
    pub fn new(
        approve_staging: Arc<ApproveStaging>,
        approve_suggestion: Arc<ApproveSuggestion>,
    ) -> Self {
        Self {
            approve_staging,
            approve_suggestion,
        }
    }
}

/// Approve staging use case.
pub struct ApproveStaging {
    #[allow(dead_code)]
    staging: Arc<crate::entities::Staging>,
}

impl ApproveStaging {
    pub fn new(staging: Arc<crate::entities::Staging>) -> Self {
        Self { staging }
    }

    pub async fn execute(
        &self,
        _staging_id: wrldbldr_domain::StagingId,
        _approved: bool,
    ) -> Result<(), ApprovalError> {
        todo!("Approve staging use case")
    }
}

/// Approve LLM suggestion use case.
pub struct ApproveSuggestion {
    // Dependencies would go here
}

impl ApproveSuggestion {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute(
        &self,
        _suggestion_id: uuid::Uuid,
        _approved: bool,
        _modifications: Option<String>,
    ) -> Result<(), ApprovalError> {
        todo!("Approve suggestion use case")
    }
}

impl Default for ApproveSuggestion {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("Item not found")]
    NotFound,
    #[error("Already processed")]
    AlreadyProcessed,
    #[error("Repository error: {0}")]
    Repo(#[from] crate::infrastructure::ports::RepoError),
}
