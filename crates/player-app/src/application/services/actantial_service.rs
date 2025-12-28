//! Actantial Service - Application service for managing NPC motivations
//!
//! This service provides use case implementations for managing wants, goals,
//! and actantial relationships via WebSocket request/response pattern.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use wrldbldr_player_ports::outbound::GameConnectionPort;
// Note: Actantial enum types (WantVisibilityData, ActantialRoleData, etc.) are imported
// as shared value objects. These are essentially protocol primitives used in DTOs.
// This is a documented exception in the hexagonal architecture.
use wrldbldr_protocol::{
    ActantialRoleData, ActorTypeData, NpcActantialContextData, RequestPayload,
    WantTargetData, WantTargetTypeData, WantVisibilityData,
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

// From impls for protocol conversion at the boundary
impl CreateWantRequest {
    fn to_protocol_data(&self) -> wrldbldr_protocol::messages::CreateWantData {
        wrldbldr_protocol::messages::CreateWantData {
            description: self.description.clone(),
            intensity: self.intensity,
            priority: self.priority,
            visibility: self.visibility.clone(),
            target_id: self.target_id.clone(),
            target_type: self.target_type.as_ref().and_then(|t| {
                match t.as_str() {
                    "Character" => Some(WantTargetTypeData::Character),
                    "Item" => Some(WantTargetTypeData::Item),
                    "Goal" => Some(WantTargetTypeData::Goal),
                    _ => None,
                }
            }),
            deflection_behavior: self.deflection_behavior.clone(),
            tells: self.tells.clone().map(|t| vec![t]).unwrap_or_default(),
        }
    }
}

impl UpdateWantRequest {
    fn to_protocol_data(&self) -> wrldbldr_protocol::messages::UpdateWantData {
        wrldbldr_protocol::messages::UpdateWantData {
            description: self.description.clone(),
            intensity: self.intensity,
            priority: self.priority,
            visibility: self.visibility.clone(),
            deflection_behavior: self.deflection_behavior.clone(),
            tells: self.tells.clone().map(|t| vec![t]),
        }
    }
}

impl From<&CreateGoalRequest> for wrldbldr_protocol::messages::CreateGoalData {
    fn from(req: &CreateGoalRequest) -> Self {
        Self {
            name: req.name.clone(),
            description: req.description.clone(),
        }
    }
}

impl From<&UpdateGoalRequest> for wrldbldr_protocol::messages::UpdateGoalData {
    fn from(req: &UpdateGoalRequest) -> Self {
        Self {
            name: req.name.clone(),
            description: req.description.clone(),
        }
    }
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
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::CreateWant {
                    character_id: character_id.to_string(),
                    data: request.to_protocol_data(),
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
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::UpdateWant {
                    want_id: want_id.to_string(),
                    data: request.to_protocol_data(),
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
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::CreateGoal {
                    world_id: world_id.to_string(),
                    data: request.into(),
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
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::UpdateGoal {
                    goal_id: goal_id.to_string(),
                    data: request.into(),
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
