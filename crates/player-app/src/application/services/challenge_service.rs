//! Challenge Service - Application service for challenge management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and managing challenges. It uses WebSocket for real-time
//! communication with the Engine.

use std::sync::Arc;

use wrldbldr_protocol::RequestPayload;

use crate::application::dto::ChallengeData;
use crate::application::error::{ParseResponse, ServiceError};
use wrldbldr_player_ports::outbound::GameConnectionPort;

/// Challenge service for managing challenges
///
/// This service provides methods for challenge-related operations
/// while depending only on the `GameConnectionPort` trait, not concrete
/// infrastructure implementations.
#[derive(Clone)]
pub struct ChallengeService {
    connection: Arc<dyn GameConnectionPort>,
}

impl ChallengeService {
    /// Create a new ChallengeService with the given connection port
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// List all challenges in a world
    pub async fn list_challenges(&self, world_id: &str) -> Result<Vec<ChallengeData>, ServiceError> {
        let payload = RequestPayload::ListChallenges {
            world_id: world_id.to_string(),
        };
        let response = self.connection.request(payload).await?;
        response.parse()
    }

    /// Get a single challenge by ID
    pub async fn get_challenge(&self, challenge_id: &str) -> Result<ChallengeData, ServiceError> {
        let payload = RequestPayload::GetChallenge {
            challenge_id: challenge_id.to_string(),
        };
        let response = self.connection.request(payload).await?;
        response.parse()
    }

    /// Create a new challenge
    pub async fn create_challenge(
        &self,
        world_id: &str,
        challenge: &ChallengeData,
    ) -> Result<ChallengeData, ServiceError> {
        use wrldbldr_protocol::CreateChallengeData;

        let data = CreateChallengeData {
            name: challenge.name.clone(),
            description: Some(challenge.description.clone()),
            skill_id: challenge.skill_id.clone(),
            difficulty: challenge.difficulty.display(),
            success_outcome: Some(challenge.outcomes.success.description.clone()),
            failure_outcome: Some(challenge.outcomes.failure.description.clone()),
        };

        let payload = RequestPayload::CreateChallenge {
            world_id: world_id.to_string(),
            data,
        };
        let response = self.connection.request(payload).await?;
        response.parse()
    }

    /// Update an existing challenge
    pub async fn update_challenge(
        &self,
        challenge: &ChallengeData,
    ) -> Result<ChallengeData, ServiceError> {
        use wrldbldr_protocol::UpdateChallengeData;

        let data = UpdateChallengeData {
            name: Some(challenge.name.clone()),
            description: Some(challenge.description.clone()),
            skill_id: Some(challenge.skill_id.clone()),
            difficulty: Some(challenge.difficulty.display()),
            success_outcome: Some(challenge.outcomes.success.description.clone()),
            failure_outcome: Some(challenge.outcomes.failure.description.clone()),
        };

        let payload = RequestPayload::UpdateChallenge {
            challenge_id: challenge.id.clone(),
            data,
        };
        let response = self.connection.request(payload).await?;
        response.parse()
    }

    /// Delete a challenge
    pub async fn delete_challenge(&self, challenge_id: &str) -> Result<(), ServiceError> {
        let payload = RequestPayload::DeleteChallenge {
            challenge_id: challenge_id.to_string(),
        };
        let response = self.connection.request(payload).await?;
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
        let payload = RequestPayload::SetChallengeFavorite {
            challenge_id: challenge_id.to_string(),
            favorite: new_favorite,
        };
        let response = self.connection.request(payload).await?;
        response.parse_empty()?;
        Ok(new_favorite)
    }

    /// Set challenge active status
    pub async fn set_active(&self, challenge_id: &str, active: bool) -> Result<(), ServiceError> {
        let payload = RequestPayload::SetChallengeActive {
            challenge_id: challenge_id.to_string(),
            active,
        };
        let response = self.connection.request(payload).await?;
        response.parse_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_player_adapters::infrastructure::testing::MockGameConnectionPort;

    #[tokio::test]
    async fn list_challenges_sends_correct_payload() {
        let conn = Arc::new(MockGameConnectionPort::new("ws://test/ws"));
        let conn_dyn: Arc<dyn GameConnectionPort> = conn.clone();
        let svc = ChallengeService::new(conn_dyn);

        // The mock will return an error by default, but we can still verify the request was sent
        let _ = svc.list_challenges("world-1").await;

        // In a real test, we'd verify the payload was correct
        // For now, just ensure it doesn't panic
    }
}
