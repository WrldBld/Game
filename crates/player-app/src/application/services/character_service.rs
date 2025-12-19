//! Character Service - Application service for character management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and fetching characters. It abstracts away the HTTP client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::application::dto::{FieldValue, InventoryItemData};
use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Character summary for list views
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: Option<String>,
}

/// Character sheet data from API
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetDataApi {
    #[serde(default)]
    pub values: HashMap<String, FieldValue>,
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
    pub sheet_data: Option<CharacterSheetDataApi>,
}

/// Character service for managing characters
///
/// This service provides methods for character-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct CharacterService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> CharacterService<A> {
    /// Create a new CharacterService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all characters in a world
    pub async fn list_characters(&self, world_id: &str) -> Result<Vec<CharacterSummary>, ApiError> {
        let path = format!("/api/worlds/{}/characters", world_id);
        self.api.get(&path).await
    }

    /// Get a single character by ID
    pub async fn get_character(
        &self,
        character_id: &str,
    ) -> Result<CharacterFormData, ApiError> {
        // Engine exposes GET /api/characters/{id} for single-character fetch
        let path = format!("/api/characters/{}", character_id);
        self.api.get(&path).await
    }

    /// Create a new character
    pub async fn create_character(
        &self,
        world_id: &str,
        character: &CharacterFormData,
    ) -> Result<CharacterFormData, ApiError> {
        let path = format!("/api/worlds/{}/characters", world_id);
        self.api.post(&path, character).await
    }

    /// Update an existing character
    pub async fn update_character(
        &self,
        character_id: &str,
        character: &CharacterFormData,
    ) -> Result<CharacterFormData, ApiError> {
        let path = format!("/api/characters/{}", character_id);
        self.api.put(&path, character).await
    }

    /// Delete a character
    pub async fn delete_character(&self, character_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/characters/{}", character_id);
        self.api.delete(&path).await
    }

    /// Change a character's archetype
    pub async fn change_archetype(
        &self,
        character_id: &str,
        new_archetype: &str,
        reason: &str,
    ) -> Result<(), ApiError> {
        #[derive(Serialize)]
        struct ArchetypeRequest {
            archetype: String,
            reason: String,
        }

        let path = format!("/api/characters/{}/archetype", character_id);
        let request = ArchetypeRequest {
            archetype: new_archetype.to_string(),
            reason: reason.to_string(),
        };
        self.api.post_no_response(&path, &request).await
    }

    /// Get a character's inventory
    pub async fn get_inventory(
        &self,
        character_id: &str,
    ) -> Result<Vec<InventoryItemData>, ApiError> {
        let path = format!("/api/characters/{}/inventory", character_id);
        self.api.get(&path).await
    }
}

impl<A: ApiPort + Clone> Clone for CharacterService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
