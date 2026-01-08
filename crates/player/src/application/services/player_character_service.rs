//! Player Character Service - Application service for player character management
//!
//! This service provides use case implementations for creating, updating,
//! and fetching player characters via WebSocket request/response pattern.
//! All operations use the GameConnectionPort for real-time communication.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::dto::CharacterSheetDataApi;
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::ports::outbound::GameConnectionPort;
use wrldbldr_protocol::{PlayerCharacterRequest, RequestPayload};

/// Full player character data
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerCharacterData {
    pub id: String,
    #[serde(default)]
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
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_region_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<serde_json::Value>,
}

/// Request to update a player character
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdatePlayerCharacterRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_data: Option<serde_json::Value>,
}

/// Response from location update
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateLocationResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
}

// From impls for protocol conversion at the boundary
impl From<&CreatePlayerCharacterRequest> for wrldbldr_protocol::CreatePlayerCharacterData {
    fn from(req: &CreatePlayerCharacterRequest) -> Self {
        Self {
            name: req.name.clone(),
            user_id: req.user_id.clone(),
            starting_region_id: req.starting_region_id.clone(),
            sheet_data: req.sheet_data.clone(),
        }
    }
}

impl From<&UpdatePlayerCharacterRequest> for wrldbldr_protocol::UpdatePlayerCharacterData {
    fn from(req: &UpdatePlayerCharacterRequest) -> Self {
        Self {
            name: req.name.clone(),
            sheet_data: req.sheet_data.clone(),
        }
    }
}

/// Player character service for managing player characters
///
/// This service provides methods for player character-related operations
/// using WebSocket request/response pattern via the `GameConnectionPort`.
#[derive(Clone)]
pub struct PlayerCharacterService {
    connection: Arc<dyn GameConnectionPort>,
}

impl PlayerCharacterService {
    /// Create a new PlayerCharacterService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// Create a new player character
    pub async fn create_pc(
        &self,
        world_id: &str,
        request: &CreatePlayerCharacterRequest,
    ) -> Result<PlayerCharacterData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(PlayerCharacterRequest::CreatePlayerCharacter {
                    world_id: world_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Get the current user's player character for a world
    pub async fn get_my_pc(
        &self,
        world_id: &str,
        user_id: &str,
    ) -> Result<Option<PlayerCharacterData>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(PlayerCharacterRequest::GetMyPlayerCharacter {
                    world_id: world_id.to_string(),
                    user_id: user_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_optional()
    }

    /// Get a player character by ID
    pub async fn get_pc(&self, pc_id: &str) -> Result<PlayerCharacterData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(PlayerCharacterRequest::GetPlayerCharacter {
                    pc_id: pc_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// List all player characters in a world
    pub async fn list_pcs(&self, world_id: &str) -> Result<Vec<PlayerCharacterData>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(PlayerCharacterRequest::ListPlayerCharacters {
                    world_id: world_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update a player character
    pub async fn update_pc(
        &self,
        pc_id: &str,
        request: &UpdatePlayerCharacterRequest,
    ) -> Result<PlayerCharacterData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(PlayerCharacterRequest::UpdatePlayerCharacter {
                    pc_id: pc_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update a player character's location (move to a region)
    pub async fn update_location(
        &self,
        pc_id: &str,
        region_id: &str,
    ) -> Result<UpdateLocationResponse, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(
                    PlayerCharacterRequest::UpdatePlayerCharacterLocation {
                        pc_id: pc_id.to_string(),
                        region_id: region_id.to_string(),
                    },
                ),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Delete a player character
    pub async fn delete_pc(&self, pc_id: &str) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::PlayerCharacter(PlayerCharacterRequest::DeletePlayerCharacter {
                    pc_id: pc_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }
}
