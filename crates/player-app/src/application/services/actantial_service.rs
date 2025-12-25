//! Actantial Service - Application service for managing NPC motivations
//!
//! This service provides use case implementations for managing wants, goals,
//! and actantial relationships. It uses HTTP for CRUD operations while
//! real-time updates are received via WebSocket.

use serde::{Deserialize, Serialize};

use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

// Re-export protocol types for convenience
pub use wrldbldr_protocol::{
    ActantialRoleData, ActorTypeData, WantVisibilityData,
    WantData, WantTargetData, GoalData,
    NpcActantialContextData, SocialViewsData, SocialRelationData,
    ActantialActorData, ActantialViewData,
};

/// Request to create a new want
#[derive(Clone, Debug, Serialize)]
pub struct CreateWantRequest {
    pub description: String,
    pub intensity: f32,
    pub priority: u32,
    pub visibility: WantVisibilityData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deflection_behavior: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tells: Option<String>,
}

/// Request to update an existing want
#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateWantRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<WantVisibilityData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deflection_behavior: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tells: Option<String>,
}

/// Request to set a want target
#[derive(Clone, Debug, Serialize)]
pub struct SetWantTargetRequest {
    pub target_id: String,
    pub target_type: String,
}

/// Request to add an actantial view
#[derive(Clone, Debug, Serialize)]
pub struct AddActantialViewRequest {
    pub want_id: String,
    pub actor_id: String,
    pub actor_type: ActorTypeData,
    pub role: ActantialRoleData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Request to remove an actantial view
#[derive(Clone, Debug, Serialize)]
pub struct RemoveActantialViewRequest {
    pub want_id: String,
    pub actor_id: String,
    pub actor_type: ActorTypeData,
}

/// Request to create a new goal
#[derive(Clone, Debug, Serialize)]
pub struct CreateGoalRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request to update an existing goal
#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateGoalRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response for want operations
#[derive(Clone, Debug, Deserialize)]
pub struct WantResponse {
    pub id: String,
    pub description: String,
    pub intensity: f32,
    pub priority: i32,
    pub visibility: WantVisibilityData,
    pub target: Option<WantTargetData>,
    pub deflection_behavior: Option<String>,
    pub tells: Option<String>,
}

/// Response for goal operations
#[derive(Clone, Debug, Deserialize)]
pub struct GoalResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Actantial service for managing NPC motivations
///
/// This service provides methods for want, goal, and actantial view operations
/// while depending only on the `ApiPort` trait, not concrete infrastructure.
pub struct ActantialService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> ActantialService<A> {
    /// Create a new ActantialService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    // === Want Operations ===

    /// List all wants for a character
    pub async fn list_wants(&self, character_id: &str) -> Result<Vec<WantResponse>, ApiError> {
        let path = format!("/api/characters/{}/wants", character_id);
        self.api.get(&path).await
    }

    /// Create a new want for a character
    pub async fn create_want(
        &self,
        character_id: &str,
        request: &CreateWantRequest,
    ) -> Result<WantResponse, ApiError> {
        let path = format!("/api/characters/{}/wants", character_id);
        self.api.post(&path, request).await
    }

    /// Update an existing want
    pub async fn update_want(
        &self,
        want_id: &str,
        request: &UpdateWantRequest,
    ) -> Result<WantResponse, ApiError> {
        let path = format!("/api/wants/{}", want_id);
        self.api.put(&path, request).await
    }

    /// Delete a want
    pub async fn delete_want(&self, want_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/wants/{}", want_id);
        self.api.delete(&path).await
    }

    /// Set a want's target
    pub async fn set_want_target(
        &self,
        want_id: &str,
        request: &SetWantTargetRequest,
    ) -> Result<(), ApiError> {
        let path = format!("/api/wants/{}/target", want_id);
        self.api.put_no_response(&path, request).await
    }

    /// Remove a want's target
    pub async fn remove_want_target(&self, want_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/wants/{}/target", want_id);
        self.api.delete(&path).await
    }

    // === Actantial Context Operations ===

    /// Get the full actantial context for a character
    pub async fn get_actantial_context(
        &self,
        character_id: &str,
    ) -> Result<NpcActantialContextData, ApiError> {
        let path = format!("/api/characters/{}/actantial-context", character_id);
        self.api.get(&path).await
    }

    /// Add an actantial view (helper/opponent/etc.) to a character
    pub async fn add_actantial_view(
        &self,
        character_id: &str,
        request: &AddActantialViewRequest,
    ) -> Result<(), ApiError> {
        let path = format!("/api/characters/{}/actantial-views", character_id);
        self.api.post(&path, request).await
    }

    /// Remove an actantial view from a character
    /// 
    /// Uses POST to a remove endpoint since DELETE with body is not universally supported
    pub async fn remove_actantial_view(
        &self,
        character_id: &str,
        request: &RemoveActantialViewRequest,
    ) -> Result<(), ApiError> {
        let path = format!("/api/characters/{}/actantial-views/remove", character_id);
        self.api.post_no_response(&path, request).await
    }

    // === Goal Operations ===

    /// List all goals for a world
    pub async fn list_goals(&self, world_id: &str) -> Result<Vec<GoalResponse>, ApiError> {
        let path = format!("/api/worlds/{}/goals", world_id);
        self.api.get(&path).await
    }

    /// Create a new goal for a world
    pub async fn create_goal(
        &self,
        world_id: &str,
        request: &CreateGoalRequest,
    ) -> Result<GoalResponse, ApiError> {
        let path = format!("/api/worlds/{}/goals", world_id);
        self.api.post(&path, request).await
    }

    /// Update an existing goal
    pub async fn update_goal(
        &self,
        goal_id: &str,
        request: &UpdateGoalRequest,
    ) -> Result<GoalResponse, ApiError> {
        let path = format!("/api/goals/{}", goal_id);
        self.api.put(&path, request).await
    }

    /// Delete a goal
    pub async fn delete_goal(&self, goal_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/goals/{}", goal_id);
        self.api.delete(&path).await
    }
}
