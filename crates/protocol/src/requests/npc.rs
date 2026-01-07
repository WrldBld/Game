use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NpcRequest {
    SetNpcDisposition {
        npc_id: String,
        pc_id: String,
        disposition: String,
        #[serde(default)]
        reason: Option<String>,
    },
    SetNpcRelationship { npc_id: String, pc_id: String, relationship: String },
    GetNpcDispositions { pc_id: String },

    SetNpcMood {
        npc_id: String,
        region_id: String,
        mood: String,
        #[serde(default)]
        reason: Option<String>,
    },
    GetNpcMood { npc_id: String, region_id: String },

    // Character-Region Relationship Operations
    ListCharacterRegionRelationships { character_id: String },
    SetCharacterHomeRegion { character_id: String, region_id: String },
    SetCharacterWorkRegion { character_id: String, region_id: String },
    RemoveCharacterRegionRelationship {
        character_id: String,
        region_id: String,
        relationship_type: String,
    },
    ListRegionNpcs { region_id: String },
}
