//! Common helpers for character repository operations

use anyhow::Result;
use neo4rs::Row;
use wrldbldr_domain::entities::Character;
use wrldbldr_domain::value_objects::{ArchetypeChange, CampbellArchetype};
use wrldbldr_domain::{CharacterId, WorldId};

use super::super::neo4j_helpers::{parse_typed_id, NodeExt};
use super::stored_types::{ArchetypeChangeStored, StatBlockStored};

/// Convert a Neo4j row to a Character entity
pub(crate) fn row_to_character(row: Row) -> Result<Character> {
    let node: neo4rs::Node = row.get("c")?;

    let id: CharacterId = parse_typed_id(&node, "id")?;
    let world_id: WorldId = parse_typed_id(&node, "world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description")?;
    let base_archetype_str: String = node.get("base_archetype")?;
    let current_archetype_str: String = node.get("current_archetype")?;
    let default_disposition_str: String = node.get("default_disposition")?;

    let base_archetype: CampbellArchetype = base_archetype_str
        .parse()
        .unwrap_or(CampbellArchetype::Ally);
    let current_archetype: CampbellArchetype = current_archetype_str
        .parse()
        .unwrap_or(CampbellArchetype::Ally);
    let archetype_history: Vec<ArchetypeChange> = node
        .get_json::<Vec<ArchetypeChangeStored>>("archetype_history")?
        .into_iter()
        .map(Into::into)
        .collect();
    let stats = node.get_json::<StatBlockStored>("stats")?.into();
    let default_disposition = default_disposition_str
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    Ok(Character {
        id,
        world_id,
        name,
        description,
        sprite_asset: node.get_optional_string("sprite_asset"),
        portrait_asset: node.get_optional_string("portrait_asset"),
        base_archetype,
        current_archetype,
        archetype_history,
        stats,
        is_alive: node.get_bool_or("is_alive", true),
        is_active: node.get_bool_or("is_active", true),
        default_disposition,
    })
}
