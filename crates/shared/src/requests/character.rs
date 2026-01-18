use serde::{Deserialize, Serialize};

use super::{ChangeArchetypeData, CreateCharacterData, UpdateCharacterData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CharacterRequest {
    ListCharacters {
        world_id: String,
    },
    GetCharacter {
        character_id: String,
    },
    CreateCharacter {
        world_id: String,
        data: CreateCharacterData,
    },
    UpdateCharacter {
        character_id: String,
        data: UpdateCharacterData,
    },
    DeleteCharacter {
        character_id: String,
    },
    ChangeArchetype {
        character_id: String,
        data: ChangeArchetypeData,
    },
    GetCharacterInventory {
        character_id: String,
    },
}
