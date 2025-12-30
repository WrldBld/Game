//! Player Character Service - Application service for player character management
//!
//! This service provides use case implementations for creating, updating,
//! and fetching player characters. It abstracts away the HTTP client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::application::dto::FieldValue;
use crate::application::ports::outbound::{ApiError, ApiPort};

/// Character sheet data from API
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetDataApi {
    #[serde(default)]
    pub values: HashMap<String, FieldValue>,
}

/// Full player character data
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerCharacterData {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub world_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataApi>,
    pub current_location_id: String,
    pub starting_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
    pub created_at: String,
    pub last_active_at: String,
}

/// Request to create a player character
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreatePlayerCharacterRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub starting_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataApi>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
}

/// Request to update a player character
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdatePlayerCharacterRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<CharacterSheetDataApi>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
}

/// Request to update a player character's location
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateLocationRequest {
    pub location_id: String,
}

/// Response from location update
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateLocationResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
}

/// Player character service for managing player characters
///
/// This service provides methods for player character-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct PlayerCharacterService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> PlayerCharacterService<A> {
    /// Create a new PlayerCharacterService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// Create a new player character
    pub async fn create_pc(
        &self,
        session_id: &str,
        request: &CreatePlayerCharacterRequest,
    ) -> Result<PlayerCharacterData, ApiError> {
        let path = format!("/api/sessions/{}/player-characters", session_id);
        self.api.post(&path, request).await
    }

    /// Get the current user's player character for a session
    pub async fn get_my_pc(
        &self,
        session_id: &str,
    ) -> Result<Option<PlayerCharacterData>, ApiError> {
        let path = format!("/api/sessions/{}/player-characters/me", session_id);
        match self.api.get::<PlayerCharacterData>(&path).await {
            Ok(pc) => Ok(Some(pc)),
            Err(ApiError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get a player character by ID
    pub async fn get_pc(
        &self,
        pc_id: &str,
    ) -> Result<PlayerCharacterData, ApiError> {
        let path = format!("/api/player-characters/{}", pc_id);
        self.api.get(&path).await
    }

    /// List all player characters in a session
    pub async fn list_pcs(
        &self,
        session_id: &str,
    ) -> Result<Vec<PlayerCharacterData>, ApiError> {
        let path = format!("/api/sessions/{}/player-characters", session_id);
        self.api.get(&path).await
    }

    /// Update a player character
    pub async fn update_pc(
        &self,
        pc_id: &str,
        request: &UpdatePlayerCharacterRequest,
    ) -> Result<PlayerCharacterData, ApiError> {
        let path = format!("/api/player-characters/{}", pc_id);
        self.api.put(&path, request).await
    }

    /// Update a player character's location
    pub async fn update_location(
        &self,
        pc_id: &str,
        location_id: &str,
    ) -> Result<UpdateLocationResponse, ApiError> {
        let path = format!("/api/player-characters/{}/location", pc_id);
        let request = UpdateLocationRequest {
            location_id: location_id.to_string(),
        };
        self.api.put(&path, &request).await
    }

    /// Delete a player character
    pub async fn delete_pc(&self, pc_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/player-characters/{}", pc_id);
        self.api.delete(&path).await
    }
}

