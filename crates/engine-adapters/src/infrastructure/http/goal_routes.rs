//! Goal API routes
//!
//! Endpoints for managing abstract goals within a world.
//! Goals are desire targets for NPC Wants (e.g., "Power", "Revenge", "Peace").
//!
//! # WebSocket Broadcasts (P3.6)
//!
//! State-modifying endpoints broadcast changes to connected WebSocket clients
//! for multiplayer consistency.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_domain::entities::Goal;
use wrldbldr_domain::{GoalId, WorldId};
use wrldbldr_engine_ports::outbound::GoalRepositoryPort;
use wrldbldr_protocol::{GoalData, ServerMessage};

use crate::infrastructure::state::AppState;
use crate::infrastructure::state_broadcast::broadcast_to_world_sessions;

// =============================================================================
// DTOs
// =============================================================================

/// Request to create a new goal
#[derive(Debug, Deserialize)]
pub struct CreateGoalRequest {
    pub name: String,
    pub description: Option<String>,
}

/// Request to update an existing goal
#[derive(Debug, Deserialize)]
pub struct UpdateGoalRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Response DTO for a goal
#[derive(Debug, Serialize)]
pub struct GoalResponse {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: Option<String>,
    /// Number of wants targeting this goal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_count: Option<u32>,
}

impl From<Goal> for GoalResponse {
    fn from(goal: Goal) -> Self {
        Self {
            id: goal.id.to_string(),
            world_id: goal.world_id.to_string(),
            name: goal.name,
            description: goal.description,
            usage_count: None,
        }
    }
}

impl GoalResponse {
    fn with_usage_count(mut self, count: u32) -> Self {
        self.usage_count = Some(count);
        self
    }
}

// =============================================================================
// Route Handlers
// =============================================================================

/// List all goals for a world
///
/// GET /api/worlds/{world_id}/goals
pub async fn list_goals(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<GoalResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let goal_repo = state.repository.goals();
    let goals = goal_repo
        .list_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch usage counts for each goal
    let mut responses = Vec::with_capacity(goals.len());
    for goal in goals {
        let count = goal_repo
            .get_targeting_want_count(goal.id)
            .await
            .unwrap_or(0);
        responses.push(GoalResponse::from(goal).with_usage_count(count));
    }

    Ok(Json(responses))
}

/// Create a new goal for a world
///
/// POST /api/worlds/{world_id}/goals
pub async fn create_goal(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(req): Json<CreateGoalRequest>,
) -> Result<(StatusCode, Json<GoalResponse>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Build the goal entity
    let mut goal = Goal::new(world_id, req.name);
    if let Some(desc) = req.description {
        goal = goal.with_description(desc);
    }

    // Persist
    let goal_repo = state.repository.goals();
    goal_repo
        .create(&goal)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Broadcast to connected clients
    let goal_data = GoalData {
        id: goal.id.to_string(),
        name: goal.name.clone(),
        description: goal.description.clone(),
        usage_count: 0, // New goal has no usage
    };
    let message = ServerMessage::GoalCreated {
        world_id: world_id.to_string(),
        goal: goal_data,
    };
    broadcast_to_world_sessions(&state.async_session_port, world_id, message).await;

    Ok((StatusCode::CREATED, Json(GoalResponse::from(goal))))
}

/// Get a specific goal by ID
///
/// GET /api/goals/{id}
pub async fn get_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GoalResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid goal ID".to_string()))?;
    let goal_id = GoalId::from_uuid(uuid);

    let goal_repo = state.repository.goals();
    let goal = goal_repo
        .get(goal_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Goal not found".to_string()))?;

    let count = goal_repo
        .get_targeting_want_count(goal_id)
        .await
        .unwrap_or(0);

    Ok(Json(GoalResponse::from(goal).with_usage_count(count)))
}

/// Update a goal
///
/// PUT /api/goals/{id}
pub async fn update_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateGoalRequest>,
) -> Result<Json<GoalResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid goal ID".to_string()))?;
    let goal_id = GoalId::from_uuid(uuid);

    let goal_repo = state.repository.goals();
    
    // Get existing goal
    let mut goal = goal_repo
        .get(goal_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Goal not found".to_string()))?;

    let world_id = goal.world_id;

    // Apply updates
    if let Some(name) = req.name {
        goal.name = name;
    }
    if let Some(desc) = req.description {
        goal.description = if desc.is_empty() { None } else { Some(desc) };
    }

    // Persist
    goal_repo
        .update(&goal)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Broadcast to connected clients
    let usage_count = goal_repo.get_targeting_want_count(goal_id).await.unwrap_or(0);
    let goal_data = GoalData {
        id: goal.id.to_string(),
        name: goal.name.clone(),
        description: goal.description.clone(),
        usage_count,
    };
    let message = ServerMessage::GoalUpdated { goal: goal_data };
    broadcast_to_world_sessions(&state.async_session_port, world_id, message).await;

    Ok(Json(GoalResponse::from(goal)))
}

/// Delete a goal
///
/// DELETE /api/goals/{id}
pub async fn delete_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid goal ID".to_string()))?;
    let goal_id = GoalId::from_uuid(uuid);

    let goal_repo = state.repository.goals();

    // Check if goal exists
    let goal = goal_repo
        .get(goal_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Goal not found".to_string()))?;

    let world_id = goal.world_id;

    // Check if any wants target this goal
    let usage_count = goal_repo
        .get_targeting_want_count(goal_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if usage_count > 0 {
        return Err((
            StatusCode::CONFLICT,
            format!(
                "Cannot delete goal '{}' - it is targeted by {} want(s)",
                goal.name, usage_count
            ),
        ));
    }

    // Delete
    goal_repo
        .delete(goal_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Broadcast to connected clients
    let message = ServerMessage::GoalDeleted {
        goal_id: goal_id.to_string(),
    };
    broadcast_to_world_sessions(&state.async_session_port, world_id, message).await;

    Ok(StatusCode::NO_CONTENT)
}
