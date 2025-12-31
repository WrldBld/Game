//! Common helpers for character repository operations

use anyhow::Result;
use neo4rs::Row;
use wrldbldr_domain::entities::Character;
use wrldbldr_domain::value_objects::{ArchetypeChange, CampbellArchetype};
use wrldbldr_domain::{CharacterId, WorldId};

use super::stored_types::{ArchetypeChangeStored, StatBlockStored};

/// Convert a Neo4j row to a Character entity
pub(crate) fn row_to_character(row: Row) -> Result<Character> {
    let node: neo4rs::Node = row.get("c")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description")?;
    let sprite_asset: String = node.get("sprite_asset")?;
    let portrait_asset: String = node.get("portrait_asset")?;
    let base_archetype_str: String = node.get("base_archetype")?;
    let current_archetype_str: String = node.get("current_archetype")?;
    let archetype_history_json: String = node.get("archetype_history")?;
    let stats_json: String = node.get("stats")?;
    let is_alive: bool = node.get("is_alive")?;
    let is_active: bool = node.get("is_active")?;
    let default_disposition_str: String = node.get("default_disposition")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;
    let base_archetype: CampbellArchetype = base_archetype_str
        .parse()
        .unwrap_or(CampbellArchetype::Ally);
    let current_archetype: CampbellArchetype = current_archetype_str
        .parse()
        .unwrap_or(CampbellArchetype::Ally);
    let archetype_history: Vec<ArchetypeChange> =
        serde_json::from_str::<Vec<ArchetypeChangeStored>>(&archetype_history_json)?
            .into_iter()
            .map(Into::into)
            .collect();
    let stats = serde_json::from_str::<StatBlockStored>(&stats_json)?.into();
    let default_disposition = default_disposition_str
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    Ok(Character {
        id: CharacterId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        description,
        sprite_asset: if sprite_asset.is_empty() {
            None
        } else {
            Some(sprite_asset)
        },
        portrait_asset: if portrait_asset.is_empty() {
            None
        } else {
            Some(portrait_asset)
        },
        base_archetype,
        current_archetype,
        archetype_history,
        stats,
        is_alive,
        is_active,
        default_disposition,
    })
}
