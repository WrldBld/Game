use serde::{Deserialize, Serialize};

use super::{CreateLoreChunkData, CreateLoreData, UpdateLoreChunkData, UpdateLoreData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LoreRequest {
    ListLore {
        world_id: String,
    },
    GetLore {
        lore_id: String,
    },
    CreateLore {
        world_id: String,
        data: CreateLoreData,
    },
    UpdateLore {
        lore_id: String,
        data: UpdateLoreData,
    },
    DeleteLore {
        lore_id: String,
    },

    AddLoreChunk {
        lore_id: String,
        data: CreateLoreChunkData,
    },
    UpdateLoreChunk {
        chunk_id: String,
        data: UpdateLoreChunkData,
    },
    DeleteLoreChunk {
        chunk_id: String,
    },

    GrantLoreKnowledge {
        character_id: String,
        lore_id: String,
        #[serde(default)]
        chunk_ids: Option<Vec<String>>,
        discovery_source: crate::types::LoreDiscoverySourceData,
    },
    RevokeLoreKnowledge {
        character_id: String,
        lore_id: String,
        #[serde(default)]
        chunk_ids: Option<Vec<String>>,
    },

    GetCharacterLore {
        character_id: String,
    },
    GetLoreKnowers {
        lore_id: String,
    },
}
