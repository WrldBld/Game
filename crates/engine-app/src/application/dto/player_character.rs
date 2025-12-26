//! Player Character DTOs

use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::{CharacterSheetData, PlayerCharacter};

/// Request to create a player character
#[derive(Debug, Deserialize)]
pub struct CreatePlayerCharacterRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub starting_location_id: String,
    #[serde(default)]
    pub starting_region_id: Option<String>,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
    #[serde(default)]
    pub sheet_data: Option<CharacterSheetData>,
}

/// Request to update a player character
#[derive(Debug, Deserialize)]
pub struct UpdatePlayerCharacterRequestDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
    #[serde(default)]
    pub sheet_data: Option<CharacterSheetData>,
}

/// Player character response
#[derive(Debug, Serialize)]
pub struct PlayerCharacterResponseDto {
    pub id: String,
    pub world_id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub current_location_id: String,
    pub current_region_id: Option<String>,
    pub starting_location_id: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub sheet_data: Option<CharacterSheetData>,
    pub created_at: String,
    pub last_active_at: String,
}

impl From<PlayerCharacter> for PlayerCharacterResponseDto {
    fn from(pc: PlayerCharacter) -> Self {
        Self {
            id: pc.id.to_string(),
            world_id: pc.world_id.to_string(),
            user_id: pc.user_id,
            name: pc.name,
            description: pc.description,
            current_location_id: pc.current_location_id.to_string(),
            current_region_id: pc.current_region_id.map(|r| r.to_string()),
            starting_location_id: pc.starting_location_id.to_string(),
            sprite_asset: pc.sprite_asset,
            portrait_asset: pc.portrait_asset,
            sheet_data: pc.sheet_data,
            created_at: pc.created_at.to_rfc3339(),
            last_active_at: pc.last_active_at.to_rfc3339(),
        }
    }
}
