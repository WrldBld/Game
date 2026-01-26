use serde::{Deserialize, Serialize};

use super::{CreatePlayerCharacterData, UpdatePlayerCharacterData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerCharacterRequest {
    ListPlayerCharacters {
        world_id: String,
        #[serde(default)]
        limit: Option<u32>,
        #[serde(default)]
        offset: Option<u32>,
    },
    GetPlayerCharacter {
        pc_id: String,
    },
    CreatePlayerCharacter {
        world_id: String,
        data: CreatePlayerCharacterData,
    },
    UpdatePlayerCharacter {
        pc_id: String,
        data: UpdatePlayerCharacterData,
    },
    DeletePlayerCharacter {
        pc_id: String,
    },
    UpdatePlayerCharacterLocation {
        pc_id: String,
        region_id: String,
    },
    GetMyPlayerCharacter {
        world_id: String,
        user_id: String,
    },
}
