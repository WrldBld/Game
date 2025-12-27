//! Actantial Service - Application service for managing NPC motivations
//!
//! This service provides use case implementations for managing wants, goals,
//! and actantial relationships via WebSocket request/response pattern.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use wrldbldr_player_ports::outbound::GameConnectionPort;
use wrldbldr_protocol::messages::{CreateGoalData, CreateWantData, UpdateGoalData, UpdateWantData};
use wrldbldr_protocol::RequestPayload;

// Re-export protocol types for convenience
pub use wrldbldr_protocol::{
    ActantialActorData, ActantialRoleData, ActantialViewData, ActorTypeData, GoalData,
    NpcActantialContextData, SocialRelationData, SocialViewsData, WantData, WantTargetData,
    WantTargetTypeData, WantVisibilityData,
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
    pub target_type: WantTargetTypeData,
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
    pub role: ActantialRoleData,
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
/// using WebSocket request/response pattern via the `GameConnectionPort`.
#[derive(Clone)]
pub struct ActantialService {
    connection: Arc<dyn GameConnectionPort>,
}

impl ActantialService {
    /// Create a new ActantialService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    // === Want Operations ===

    /// List all wants for a character
    pub async fn list_wants(&self, character_id: &str) -> Result<Vec<WantResponse>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::ListWants {
                    character_id: character_id.to_string(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Create a new want for a character
    pub async fn create_want(
        &self,
        character_id: &str,
        request: &CreateWantRequest,
    ) -> Result<WantResponse, ServiceError> {
        let data = CreateWantData {
            description: request.description.clone(),
            intensity: request.intensity,
            priority: request.priority,
            visibility: request.visibility.clone(),
            target_id: request.target_id.clone(),
            target_type: request.target_type.as_ref().and_then(|t| {
                // Convert string to WantTargetTypeData
                match t.as_str() {
                    "Character" => Some(WantTargetTypeData::Character),
                    "Item" => Some(WantTargetTypeData::Item),
                    "Goal" => Some(WantTargetTypeData::Goal),
                    _ => None,
                }
            }),
            deflection_behavior: request.deflection_behavior.clone(),
            tells: request.tells.clone().map(|t| vec![t]).unwrap_or_default(),
        };

        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::CreateWant {
                    character_id: character_id.to_string(),
                    data,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update an existing want
    pub async fn update_want(
        &self,
        want_id: &str,
        request: &UpdateWantRequest,
    ) -> Result<WantResponse, ServiceError> {
        let data = UpdateWantData {
            description: request.description.clone(),
            intensity: request.intensity,
            priority: request.priority,
            visibility: request.visibility.clone(),
            deflection_behavior: request.deflection_behavior.clone(),
            tells: request.tells.clone().map(|t| vec![t]),
        };

        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::UpdateWant {
                    want_id: want_id.to_string(),
                    data,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Delete a want
    pub async fn delete_want(&self, want_id: &str) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::DeleteWant {
                    want_id: want_id.to_string(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Set a want's target
    pub async fn set_want_target(
        &self,
        want_id: &str,
        request: &SetWantTargetRequest,
    ) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::SetWantTarget {
                    want_id: want_id.to_string(),
                    target_id: request.target_id.clone(),
                    target_type: request.target_type.clone(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Remove a want's target
    pub async fn remove_want_target(&self, want_id: &str) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::RemoveWantTarget {
                    want_id: want_id.to_string(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    // === Actantial Context Operations ===

    /// Get the full actantial context for a character
    pub async fn get_actantial_context(
        &self,
        character_id: &str,
    ) -> Result<NpcActantialContextData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::GetActantialContext {
                    character_id: character_id.to_string(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Add an actantial view (helper/opponent/etc.) to a character
    pub async fn add_actantial_view(
        &self,
        character_id: &str,
        request: &AddActantialViewRequest,
    ) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::AddActantialView {
                    character_id: character_id.to_string(),
                    want_id: request.want_id.clone(),
                    target_id: request.actor_id.clone(),
                    target_type: request.actor_type.clone(),
                    role: request.role.clone(),
                    reason: request.reason.clone().unwrap_or_default(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Remove an actantial view from a character
    pub async fn remove_actantial_view(
        &self,
        character_id: &str,
        request: &RemoveActantialViewRequest,
    ) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::RemoveActantialView {
                    character_id: character_id.to_string(),
                    want_id: request.want_id.clone(),
                    target_id: request.actor_id.clone(),
                    target_type: request.actor_type.clone(),
                    role: request.role.clone(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    // === Goal Operations ===

    /// List all goals for a world
    pub async fn list_goals(&self, world_id: &str) -> Result<Vec<GoalResponse>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::ListGoals {
                    world_id: world_id.to_string(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Create a new goal for a world
    pub async fn create_goal(
        &self,
        world_id: &str,
        request: &CreateGoalRequest,
    ) -> Result<GoalResponse, ServiceError> {
        let data = CreateGoalData {
            name: request.name.clone(),
            description: request.description.clone(),
        };

        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::CreateGoal {
                    world_id: world_id.to_string(),
                    data,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update an existing goal
    pub async fn update_goal(
        &self,
        goal_id: &str,
        request: &UpdateGoalRequest,
    ) -> Result<GoalResponse, ServiceError> {
        let data = UpdateGoalData {
            name: request.name.clone(),
            description: request.description.clone(),
        };

        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::UpdateGoal {
                    goal_id: goal_id.to_string(),
                    data,
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Delete a goal
    pub async fn delete_goal(&self, goal_id: &str) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::DeleteGoal {
                    goal_id: goal_id.to_string(),
                },
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }
}
