use serde::{Deserialize, Serialize};

use super::{CreateLoreChunkData, CreateLoreData, UpdateLoreChunkData, UpdateLoreData};

/// About target for wire format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AboutTargetData {
    Character { character_id: String },
    Location { location_id: String },
    Region { region_id: String },
    Item { item_id: String },
}

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

    // ========================================================================
    // P1: Lore linking + common lore backend
    // ========================================================================
    LinkLoreAbout {
        lore_id: String,
        target: AboutTargetData,
    },
    UnlinkLoreAbout {
        lore_id: String,
        target: AboutTargetData,
    },
    GetLoreAboutTargets {
        lore_id: String,
    },
    MarkCommonLore {
        location_id: String,
        lore_id: String,
        #[serde(default)]
        chunk_ids: Option<Vec<String>>,
    },
    UnmarkCommonLore {
        location_id: String,
        lore_id: String,
    },
    GetLocationCommonLore {
        location_id: String,
    },
}
