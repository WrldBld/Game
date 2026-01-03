//! Challenge use cases.

use std::sync::Arc;

/// Container for challenge use cases.
pub struct ChallengeUseCases {
    pub roll: Arc<RollChallenge>,
    pub resolve: Arc<ResolveOutcome>,
}

impl ChallengeUseCases {
    pub fn new(roll: Arc<RollChallenge>, resolve: Arc<ResolveOutcome>) -> Self {
        Self { roll, resolve }
    }
}

/// Roll a challenge use case.
pub struct RollChallenge {
    #[allow(dead_code)]
    challenge: Arc<crate::entities::Challenge>,
}

impl RollChallenge {
    pub fn new(challenge: Arc<crate::entities::Challenge>) -> Self {
        Self { challenge }
    }

    pub async fn execute(
        &self,
        _challenge_id: wrldbldr_domain::ChallengeId,
        _modifier: i32,
    ) -> Result<RollResult, ChallengeError> {
        todo!("Roll challenge use case")
    }
}

/// Resolve challenge outcome use case.
pub struct ResolveOutcome {
    #[allow(dead_code)]
    challenge: Arc<crate::entities::Challenge>,
}

impl ResolveOutcome {
    pub fn new(challenge: Arc<crate::entities::Challenge>) -> Self {
        Self { challenge }
    }

    pub async fn execute(
        &self,
        _challenge_id: wrldbldr_domain::ChallengeId,
        _outcome: Outcome,
    ) -> Result<(), ChallengeError> {
        todo!("Resolve outcome use case")
    }
}

#[derive(Debug)]
pub struct RollResult {
    pub roll: i32,
    pub total: i32,
    pub success: bool,
}

#[derive(Debug)]
pub enum Outcome {
    Success,
    Failure,
    CriticalSuccess,
    CriticalFailure,
}

#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Challenge not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] crate::infrastructure::ports::RepoError),
}
