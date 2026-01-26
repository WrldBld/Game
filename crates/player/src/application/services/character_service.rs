//! Character Service - Application service for character management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and fetching characters. It abstracts away the WebSocket client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};

use crate::application::dto::requests::{
    ChangeArchetypeRequest, CreateCharacterRequest, UpdateCharacterRequest,
};
use crate::application::dto::InventoryItemData;
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_shared::character_sheet::CharacterSheetValues;
use wrldbldr_shared::{CharacterRequest, RequestPayload};

/// Character summary for list views
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: Option<String>,
}

/// Full character data for create/edit forms via API
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterFormData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archetype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wants: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fears: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backstory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite_asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portrait_asset: Option<String>,
    #[serde(default)]
    pub sheet_data: Option<CharacterSheetValues>,
}

/// Character service for managing characters
///
/// This service provides methods for character-related operations
/// while depending only on the `CommandBus`, not concrete
/// infrastructure implementations.
#[derive(Clone)]
pub struct CharacterService {
    commands: CommandBus,
}

impl CharacterService {
    /// Create a new CharacterService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// List all characters in a world
    pub async fn list_characters(
        &self,
        world_id: &str,
    ) -> Result<Vec<CharacterSummary>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::ListCharacters {
                    world_id: world_id.to_string(),
                    limit: None,
                    offset: None,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Get a single character by ID
    pub async fn get_character(
        &self,
        character_id: &str,
    ) -> Result<CharacterFormData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::GetCharacter {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Create a new character
    pub async fn create_character(
        &self,
        world_id: &str,
        character: &CharacterFormData,
    ) -> Result<CharacterFormData, ServiceError> {
        let request = CreateCharacterRequest {
            name: character.name.clone(),
            description: character.description.clone(),
            archetype: character.archetype.clone(),
            sprite_asset: character.sprite_asset.clone(),
            portrait_asset: character.portrait_asset.clone(),
        };

        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::CreateCharacter {
                    world_id: world_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Update an existing character
    pub async fn update_character(
        &self,
        character_id: &str,
        character: &CharacterFormData,
    ) -> Result<CharacterFormData, ServiceError> {
        let request = UpdateCharacterRequest {
            name: Some(character.name.clone()),
            description: character.description.clone(),
            sprite_asset: character.sprite_asset.clone(),
            portrait_asset: character.portrait_asset.clone(),
            is_alive: None,
            is_active: None,
        };

        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::UpdateCharacter {
                    character_id: character_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Delete a character
    pub async fn delete_character(&self, character_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::DeleteCharacter {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Change a character's archetype
    pub async fn change_archetype(
        &self,
        character_id: &str,
        new_archetype: &str,
        reason: &str,
    ) -> Result<(), ServiceError> {
        let request = ChangeArchetypeRequest {
            new_archetype: new_archetype.to_string(),
            reason: reason.to_string(),
        };

        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::ChangeArchetype {
                    character_id: character_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Get a character's inventory
    pub async fn get_inventory(
        &self,
        character_id: &str,
    ) -> Result<Vec<InventoryItemData>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Character(CharacterRequest::GetCharacterInventory {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }
}
