//! World Service - Application service for world management
//!
//! This service provides use case implementations for listing, loading,
//! and creating worlds. It abstracts away the HTTP client details from
//! the presentation layer.

use serde::{Deserialize, Serialize};

use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Summary of a world for list views
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Request to create a new world
#[derive(Clone, Debug, Serialize)]
pub struct CreateWorldRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_system: Option<serde_json::Value>,
}

/// Response from creating a world
#[derive(Clone, Debug, Deserialize)]
pub struct CreateWorldResponse {
    pub id: String,
    pub name: String,
}

/// Summary of an active session
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub world_id: String,
    pub dm_user_id: String,
    pub active_player_count: usize,
    pub created_at: i64,
}

/// Request to create or resume a session
#[derive(Clone, Debug, Serialize)]
pub struct CreateSessionRequest {
    pub dm_user_id: String,
}

/// Response from session creation/resumption
#[derive(Clone, Debug, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub world_id: String,
}

/// World service for managing worlds
///
/// This service provides methods for world-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct WorldService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> WorldService<A> {
    /// Create a new WorldService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all available worlds
    pub async fn list_worlds(&self) -> Result<Vec<WorldSummary>, ApiError> {
        self.api.get("/api/worlds").await
    }

    /// Get a world by ID (returns basic info)
    pub async fn get_world(&self, id: &str) -> Result<Option<WorldSummary>, ApiError> {
        let path = format!("/api/worlds/{}", id);
        self.api.get_optional(&path).await
    }

    /// Load a full world snapshot for gameplay
    ///
    /// Returns the raw JSON value which can be parsed by the caller
    /// into the appropriate WorldSnapshot type.
    pub async fn load_world_snapshot(&self, id: &str) -> Result<serde_json::Value, ApiError> {
        let path = format!("/api/worlds/{}/export/raw", id);
        self.api.get(&path).await
    }

    /// Create a new world
    ///
    /// # Arguments
    /// * `name` - The name of the world
    /// * `description` - Optional description
    /// * `rule_system` - Optional rule system configuration as JSON
    ///
    /// # Returns
    /// The ID of the created world
    pub async fn create_world(
        &self,
        name: &str,
        description: Option<&str>,
        rule_system: Option<serde_json::Value>,
    ) -> Result<String, ApiError> {
        let request = CreateWorldRequest {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            rule_system,
        };

        let response: CreateWorldResponse = self.api.post("/api/worlds", &request).await?;
        Ok(response.id)
    }

    /// Delete a world by ID
    pub async fn delete_world(&self, id: &str) -> Result<(), ApiError> {
        let path = format!("/api/worlds/{}", id);
        self.api.delete(&path).await
    }

    /// Fetch a rule system preset configuration
    ///
    /// # Arguments
    /// * `system_type` - The type (D20, D100, Narrative, Custom)
    /// * `variant` - The specific variant name
    pub async fn get_rule_system_preset(
        &self,
        system_type: &str,
        variant: &str,
    ) -> Result<serde_json::Value, ApiError> {
        let path = format!("/api/rule-systems/{}/presets/{}", system_type, variant);
        self.api.get(&path).await
    }

    /// Fetch the character sheet template for a world
    pub async fn get_sheet_template(&self, world_id: &str) -> Result<serde_json::Value, ApiError> {
        let path = format!("/api/worlds/{}/sheet-template", world_id);
        self.api.get(&path).await
    }

    /// List all active sessions across all worlds
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>, ApiError> {
        self.api.get("/api/sessions").await
    }

    /// Create or resume a session for a world
    ///
    /// # Arguments
    /// * `world_id` - The world ID
    /// * `dm_user_id` - The user ID of the dungeon master
    pub async fn create_session(
        &self,
        world_id: &str,
        dm_user_id: &str,
    ) -> Result<CreateSessionResponse, ApiError> {
        let path = format!("/api/worlds/{}/sessions", world_id);
        let request = CreateSessionRequest {
            dm_user_id: dm_user_id.to_string(),
        };
        self.api.post(&path, &request).await
    }
}

impl<A: ApiPort + Clone> Clone for WorldService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
