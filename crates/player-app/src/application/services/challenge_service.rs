//! Challenge Service - Application service for challenge management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and managing challenges. It abstracts away the HTTP client
//! details from the presentation layer.

use crate::application::dto::ChallengeData;
use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Challenge service for managing challenges
///
/// This service provides methods for challenge-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct ChallengeService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> ChallengeService<A> {
    /// Create a new ChallengeService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all challenges in a world
    pub async fn list_challenges(&self, world_id: &str) -> Result<Vec<ChallengeData>, ApiError> {
        let path = format!("/api/worlds/{}/challenges", world_id);
        self.api.get(&path).await
    }

    /// Get a single challenge by ID
    pub async fn get_challenge(&self, challenge_id: &str) -> Result<ChallengeData, ApiError> {
        let path = format!("/api/challenges/{}", challenge_id);
        self.api.get(&path).await
    }

    /// Create a new challenge
    pub async fn create_challenge(
        &self,
        world_id: &str,
        challenge: &ChallengeData,
    ) -> Result<ChallengeData, ApiError> {
        let path = format!("/api/worlds/{}/challenges", world_id);
        self.api.post(&path, challenge).await
    }

    /// Update an existing challenge
    pub async fn update_challenge(
        &self,
        challenge: &ChallengeData,
    ) -> Result<ChallengeData, ApiError> {
        let path = format!("/api/challenges/{}", challenge.id);
        self.api.put(&path, challenge).await
    }

    /// Delete a challenge
    pub async fn delete_challenge(&self, challenge_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/challenges/{}", challenge_id);
        self.api.delete(&path).await
    }

    /// Toggle challenge favorite status
    pub async fn toggle_favorite(&self, challenge_id: &str) -> Result<bool, ApiError> {
        let path = format!("/api/challenges/{}/favorite", challenge_id);
        self.api.put_empty_with_response(&path).await
    }

    /// Set challenge active status
    pub async fn set_active(&self, challenge_id: &str, active: bool) -> Result<(), ApiError> {
        let path = format!("/api/challenges/{}/active", challenge_id);
        self.api.put_no_response(&path, &active).await
    }
}

impl<A: ApiPort + Clone> Clone for ChallengeService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::testing::MockApiPort;
    use crate::infrastructure::testing::fixtures::api_request_failed;

    #[tokio::test]
    async fn list_challenges_hits_expected_path() {
        let api = MockApiPort::new();
        api.when_get_err(
            "/api/worlds/world-1/challenges",
            api_request_failed("boom"),
        );

        let svc = ChallengeService::new(api.clone());
        let _ = svc.list_challenges("world-1").await;

        let reqs = api.requests();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].method, "GET");
        assert_eq!(reqs[0].path, "/api/worlds/world-1/challenges");
    }
}
