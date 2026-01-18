//! Challenge Service - Application service for challenge management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and managing challenges. It uses WebSocket for real-time
//! communication with the Engine.

use wrldbldr_shared::{ChallengeRequest, RequestPayload};

use crate::application::dto::ChallengeData;
use crate::application::error::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;

/// Challenge service for managing challenges
///
/// This service provides methods for challenge-related operations
/// while depending only on the `CommandBus`, not concrete
/// infrastructure implementations. The `GameRequestPort` methods are
/// available via blanket implementation.
#[derive(Clone)]
pub struct ChallengeService {
    commands: CommandBus,
}

impl ChallengeService {
    /// Create a new ChallengeService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// List all challenges in a world
    pub async fn list_challenges(
        &self,
        world_id: &str,
    ) -> Result<Vec<ChallengeData>, ServiceError> {
        let payload = RequestPayload::Challenge(ChallengeRequest::ListChallenges {
            world_id: world_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }

    /// Get a single challenge by ID
    pub async fn get_challenge(&self, challenge_id: &str) -> Result<ChallengeData, ServiceError> {
        let payload = RequestPayload::Challenge(ChallengeRequest::GetChallenge {
            challenge_id: challenge_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }

    /// Create a new challenge
    pub async fn create_challenge(
        &self,
        world_id: &str,
        challenge: &ChallengeData,
    ) -> Result<ChallengeData, ServiceError> {
        let data = wrldbldr_shared::CreateChallengeData {
            name: challenge.name.clone(),
            description: Some(challenge.description.clone()),
            skill_id: challenge.skill_id.clone(),
            difficulty: challenge.difficulty.display(),
            success_outcome: Some(challenge.outcomes.success.description.clone()),
            failure_outcome: Some(challenge.outcomes.failure.description.clone()),
        };

        let payload = RequestPayload::Challenge(ChallengeRequest::CreateChallenge {
            world_id: world_id.to_string(),
            data,
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }

    /// Update an existing challenge
    pub async fn update_challenge(
        &self,
        challenge: &ChallengeData,
    ) -> Result<ChallengeData, ServiceError> {
        let data = wrldbldr_shared::UpdateChallengeData {
            name: Some(challenge.name.clone()),
            description: Some(challenge.description.clone()),
            skill_id: Some(challenge.skill_id.clone()),
            difficulty: Some(challenge.difficulty.display()),
            success_outcome: Some(challenge.outcomes.success.description.clone()),
            failure_outcome: Some(challenge.outcomes.failure.description.clone()),
        };

        let payload = RequestPayload::Challenge(ChallengeRequest::UpdateChallenge {
            challenge_id: challenge.id.clone(),
            data,
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }

    /// Delete a challenge
    pub async fn delete_challenge(&self, challenge_id: &str) -> Result<(), ServiceError> {
        let payload = RequestPayload::Challenge(ChallengeRequest::DeleteChallenge {
            challenge_id: challenge_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse_empty()
    }

    /// Toggle challenge favorite status
    ///
    /// Returns the new favorite state after toggling
    pub async fn toggle_favorite(&self, challenge_id: &str) -> Result<bool, ServiceError> {
        // First get current state
        let challenge = self.get_challenge(challenge_id).await?;
        let new_favorite = !challenge.is_favorite;

        // Set new state
        let payload = RequestPayload::Challenge(ChallengeRequest::SetChallengeFavorite {
            challenge_id: challenge_id.to_string(),
            favorite: new_favorite,
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse_empty()?;
        Ok(new_favorite)
    }

    /// Set challenge active status
    pub async fn set_active(&self, challenge_id: &str, active: bool) -> Result<(), ServiceError> {
        let payload = RequestPayload::Challenge(ChallengeRequest::SetChallengeActive {
            challenge_id: challenge_id.to_string(),
            active,
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::messaging::{BusMessage, PendingRequests};
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    fn create_test_command_bus() -> (CommandBus, mpsc::Receiver<BusMessage>) {
        let (tx, rx) = mpsc::channel(10);
        let pending = Arc::new(Mutex::new(PendingRequests::default()));
        (CommandBus::new(tx, pending), rx)
    }

    #[tokio::test]
    async fn list_challenges_sends_correct_payload() {
        let (commands, mut rx) = create_test_command_bus();
        let svc = ChallengeService::new(commands);

        // The request will timeout since there's no server, but we can verify a message was sent
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            svc.list_challenges("world-1"),
        )
        .await;

        // Verify that a request message was sent
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, BusMessage::Request { .. }));
    }
}
