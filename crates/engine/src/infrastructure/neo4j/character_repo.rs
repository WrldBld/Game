//! Neo4j character repository implementation.
//!
//! # Graph-First Design
//!
//! The following relationships are stored as Neo4j edges:
//! - Wants: `(Character)-[:HAS_WANT]->(Want)` + `(Want)-[:TARGETS]->(target)`
//! - Inventory: `(Character)-[:POSSESSES]->(Item)`
//! - Location relationships: `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS`
//! - Actantial views: `VIEWS_AS_HELPER`, `VIEWS_AS_OPPONENT`, etc.
//!
//! Archetype history and stats remain as JSON (acceptable per ADR - complex nested non-relational)

use async_trait::async_trait;
use neo4rs::{query, Graph, Row};
use uuid::Uuid;
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{CharacterRepo, NpcRegionRelationType, NpcRegionRelationship, NpcWithRegionInfo, RepoError};

// =============================================================================
// Stored Types for JSON serialization
// =============================================================================

/// Stored representation of StatBlock for Neo4j persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StatBlockStored {
    stats: std::collections::HashMap<String, i32>,
    current_hp: Option<i32>,
    max_hp: Option<i32>,
}

impl From<StatBlock> for StatBlockStored {
    fn from(value: StatBlock) -> Self {
        Self {
            stats: value.stats,
            current_hp: value.current_hp,
            max_hp: value.max_hp,
        }
    }
}

impl From<StatBlockStored> for StatBlock {
    fn from(value: StatBlockStored) -> Self {
        Self {
            stats: value.stats,
            current_hp: value.current_hp,
            max_hp: value.max_hp,
        }
    }
}

/// Stored representation of ArchetypeChange for Neo4j persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ArchetypeChangeStored {
    from: String,
    to: String,
    reason: String,
    timestamp: String,
}

impl From<ArchetypeChange> for ArchetypeChangeStored {
    fn from(value: ArchetypeChange) -> Self {
        Self {
            from: format!("{:?}", value.from),
            to: format!("{:?}", value.to),
            reason: value.reason,
            timestamp: value.timestamp.to_rfc3339(),
        }
    }
}

impl From<ArchetypeChangeStored> for ArchetypeChange {
    fn from(value: ArchetypeChangeStored) -> Self {
        let timestamp = wrldbldr_domain::common::parse_datetime_or(
            &value.timestamp,
            chrono::DateTime::UNIX_EPOCH,
        );
        Self {
            from: value.from.parse().unwrap_or(CampbellArchetype::Ally),
            to: value.to.parse().unwrap_or(CampbellArchetype::Ally),
            reason: value.reason,
            timestamp,
        }
    }
}

// =============================================================================
// Repository Implementation
// =============================================================================

/// Repository for Character operations.
pub struct Neo4jCharacterRepo {
    graph: Graph,
}

impl Neo4jCharacterRepo {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Convert a Neo4j row to a Character entity.
    fn row_to_character(&self, row: Row) -> Result<Character, RepoError> {
        let node: neo4rs::Node = row
            .get("c")
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let id: CharacterId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
        let world_id: WorldId =
            parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let description: String = node
            .get("description")
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let base_archetype_str: String = node
            .get("base_archetype")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let current_archetype_str: String = node
            .get("current_archetype")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let default_disposition_str: String = node
            .get("default_disposition")
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let base_archetype: CampbellArchetype = base_archetype_str
            .parse()
            .unwrap_or(CampbellArchetype::Ally);
        let current_archetype: CampbellArchetype = current_archetype_str
            .parse()
            .unwrap_or(CampbellArchetype::Ally);

        let archetype_history: Vec<ArchetypeChange> = node
            .get_json::<Vec<ArchetypeChangeStored>>("archetype_history")
            .map_err(|e| RepoError::Database(e.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect();

        let stats: StatBlock = node
            .get_json::<StatBlockStored>("stats")
            .map_err(|e| RepoError::Database(e.to_string()))?
            .into();

        let default_disposition = default_disposition_str
            .parse()
            .map_err(|e: String| RepoError::Database(e))?;

        // Parse default_mood from stored string (falls back to Calm)
        let default_mood_str: String = node
            .get("default_mood")
            .unwrap_or_else(|_| "calm".to_string());
        let default_mood: MoodState = default_mood_str.parse().unwrap_or(MoodState::Calm);

        // Parse expression_config from JSON (falls back to default)
        let expression_config: ExpressionConfig = node
            .get::<String>("expression_config")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

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
            default_mood,
            expression_config,
        })
    }

    /// Convert a Neo4j row to an Item entity.
    fn row_to_item(&self, row: Row) -> Result<Item, RepoError> {
        let node: neo4rs::Node = row
            .get("i")
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let id: ItemId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
        let world_id: WorldId =
            parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(Item {
            id,
            world_id,
            name,
            description: node.get_optional_string("description"),
            item_type: node.get_optional_string("item_type"),
            is_unique: node.get_bool_or("is_unique", false),
            properties: node.get_optional_string("properties"),
            can_contain_items: node.get_bool_or("can_contain_items", false),
            container_limit: node.get_positive_i64("container_limit"),
        })
    }
}

#[async_trait]
impl CharacterRepo for Neo4jCharacterRepo {
    // =========================================================================
    // CRUD Operations
    // =========================================================================

    async fn get(&self, id: CharacterId) -> Result<Option<Character>, RepoError> {
        let q = query("MATCH (c:Character {id: $id}) RETURN c").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            Ok(Some(self.row_to_character(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, character: &Character) -> Result<(), RepoError> {
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let expression_config_json = serde_json::to_string(&character.expression_config)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        // MERGE to handle both create and update, with CONTAINS_CHARACTER edge
        let q = query(
            "MATCH (w:World {id: $world_id})
            MERGE (c:Character {id: $id})
            SET c.world_id = $world_id,
                c.name = $name,
                c.description = $description,
                c.sprite_asset = $sprite_asset,
                c.portrait_asset = $portrait_asset,
                c.base_archetype = $base_archetype,
                c.current_archetype = $current_archetype,
                c.archetype_history = $archetype_history,
                c.stats = $stats,
                c.is_alive = $is_alive,
                c.is_active = $is_active,
                c.default_disposition = $default_disposition,
                c.default_mood = $default_mood,
                c.expression_config = $expression_config
            MERGE (w)-[:CONTAINS_CHARACTER]->(c)
            RETURN c.id as id",
        )
        .param("id", character.id.to_string())
        .param("world_id", character.world_id.to_string())
        .param("name", character.name.clone())
        .param("description", character.description.clone())
        .param(
            "sprite_asset",
            character.sprite_asset.clone().unwrap_or_default(),
        )
        .param(
            "portrait_asset",
            character.portrait_asset.clone().unwrap_or_default(),
        )
        .param("base_archetype", format!("{:?}", character.base_archetype))
        .param(
            "current_archetype",
            format!("{:?}", character.current_archetype),
        )
        .param("archetype_history", archetype_history_json)
        .param("stats", stats_json)
        .param("is_alive", character.is_alive)
        .param("is_active", character.is_active)
        .param(
            "default_disposition",
            character.default_disposition.to_string(),
        )
        .param("default_mood", character.default_mood.to_string())
        .param("expression_config", expression_config_json);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        // Verify the operation succeeded
        if result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
            .is_none()
        {
            tracing::warn!(
                "save failed: World {} not found for character {}",
                character.world_id,
                character.id
            );
            return Err(RepoError::NotFound);
        }

        tracing::debug!("Saved character: {}", character.name);
        Ok(())
    }

    async fn delete(&self, id: CharacterId) -> Result<(), RepoError> {
        // Delete the character and all connected Want nodes in a single atomic query
        // Using OPTIONAL MATCH ensures we don't fail if there are no wants
        let q = query(
            "MATCH (c:Character {id: $id})
            OPTIONAL MATCH (c)-[:HAS_WANT]->(w:Want)
            DETACH DELETE w, c",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted character: {}", id);
        Ok(())
    }

    // =========================================================================
    // Query Operations
    // =========================================================================

    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Character>, RepoError> {
        // Characters currently staged in a region via STAGED_IN edge
        let q = query(
            "MATCH (c:Character)-[:STAGED_IN]->(r:Region {id: $region_id})
            RETURN c
            ORDER BY c.name",
        )
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut characters = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            characters.push(self.row_to_character(row)?);
        }

        Ok(characters)
    }

    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Character>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHARACTER]->(c:Character)
            RETURN c
            ORDER BY c.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut characters = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            characters.push(self.row_to_character(row)?);
        }

        Ok(characters)
    }

    async fn list_npcs_in_world(&self, world_id: WorldId) -> Result<Vec<Character>, RepoError> {
        // All characters in a world are NPCs (PlayerCharacters are separate)
        self.list_in_world(world_id).await
    }

    async fn update_position(
        &self,
        id: CharacterId,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
        // Remove any existing STAGED_IN edges and create new one
        let q = query(
            "MATCH (c:Character {id: $id})
            OPTIONAL MATCH (c)-[old:STAGED_IN]->()
            DELETE old
            WITH c
            MATCH (r:Region {id: $region_id})
            CREATE (c)-[:STAGED_IN]->(r)
            RETURN c.id as id",
        )
        .param("id", id.to_string())
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        // Verify the operation succeeded
        if result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
            .is_none()
        {
            tracing::warn!(
                "update_position failed: Character {} or Region {} not found",
                id,
                region_id
            );
            return Err(RepoError::NotFound);
        }

        tracing::debug!("Updated character {} position to region {}", id, region_id);
        Ok(())
    }

    // =========================================================================
    // Relationship Operations
    // =========================================================================

    async fn get_relationships(&self, id: CharacterId) -> Result<Vec<Relationship>, RepoError> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:RELATES_TO]->(other:Character)
            RETURN r.id as rel_id, other.id as other_id, r.relationship_type as rel_type, 
                   r.sentiment as sentiment, r.known_to_player as known_to_player",
        )
        .param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut relationships = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let rel_id_str: String = row
                .get("rel_id")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let rel_id = RelationshipId::from(
                Uuid::parse_str(&rel_id_str).map_err(|e| RepoError::Database(e.to_string()))?,
            );

            let other_id_str: String = row
                .get("other_id")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let other_id = CharacterId::from(
                Uuid::parse_str(&other_id_str).map_err(|e| RepoError::Database(e.to_string()))?,
            );

            let rel_type_str: String = row
                .get("rel_type")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let relationship_type: RelationshipType = rel_type_str.parse().unwrap_or(RelationshipType::Custom(rel_type_str));

            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let known_to_player: bool = row.get("known_to_player").unwrap_or(true);

            relationships.push(Relationship {
                id: rel_id,
                from_character: id,
                to_character: other_id,
                relationship_type,
                sentiment: sentiment as f32,
                history: Vec::new(), // History is stored separately if needed
                known_to_player,
            });
        }

        Ok(relationships)
    }

    async fn save_relationship(&self, relationship: &Relationship) -> Result<(), RepoError> {
        // Convert relationship type to string for storage
        let rel_type_str = match &relationship.relationship_type {
            RelationshipType::Family(family) => format!("family:{:?}", family),
            RelationshipType::Romantic => "romantic".to_string(),
            RelationshipType::Professional => "professional".to_string(),
            RelationshipType::Rivalry => "rivalry".to_string(),
            RelationshipType::Friendship => "friendship".to_string(),
            RelationshipType::Mentorship => "mentorship".to_string(),
            RelationshipType::Enmity => "enmity".to_string(),
            RelationshipType::Custom(s) => s.clone(),
        };

        let q = query(
            "MATCH (from:Character {id: $from_id})
            MATCH (to:Character {id: $to_id})
            MERGE (from)-[r:RELATES_TO {id: $rel_id}]->(to)
            SET r.relationship_type = $rel_type,
                r.sentiment = $sentiment,
                r.known_to_player = $known_to_player",
        )
        .param("from_id", relationship.from_character.to_string())
        .param("to_id", relationship.to_character.to_string())
        .param("rel_id", relationship.id.to_string())
        .param("rel_type", rel_type_str)
        .param("sentiment", relationship.sentiment as f64)
        .param("known_to_player", relationship.known_to_player);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!(
            "Saved relationship from {} to {}",
            relationship.from_character,
            relationship.to_character
        );
        Ok(())
    }

    // =========================================================================
    // Inventory Operations
    // =========================================================================

    async fn get_inventory(&self, id: CharacterId) -> Result<Vec<Item>, RepoError> {
        let q = query(
            "MATCH (c:Character {id: $id})-[:POSSESSES]->(i:Item)
            RETURN i",
        )
        .param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut items = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            items.push(self.row_to_item(row)?);
        }

        Ok(items)
    }

    async fn add_to_inventory(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Character {id: $character_id})
            MATCH (i:Item {id: $item_id})
            MERGE (c)-[:POSSESSES]->(i)",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Added item {} to character {}", item_id, character_id);
        Ok(())
    }

    async fn remove_from_inventory(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Removed item {} from character {}", item_id, character_id);
        Ok(())
    }

    // =========================================================================
    // Wants/Goals Operations
    // =========================================================================

    async fn get_wants(&self, id: CharacterId) -> Result<Vec<Want>, RepoError> {
        let q = query(
            "MATCH (c:Character {id: $id})-[:HAS_WANT]->(w:Want)
            RETURN w",
        )
        .param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut wants = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let node: neo4rs::Node = row
                .get("w")
                .map_err(|e| RepoError::Database(e.to_string()))?;

            let want_id: WantId =
                parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
            let description: String = node.get_string_or("description", "");
            let intensity: f64 = node.get_f64_or("intensity", 0.5);
            let visibility_str: String = node.get_string_or("visibility", "Hidden");
            let created_at = node.get_datetime_or("created_at", chrono::Utc::now());
            let deflection_behavior = node.get_optional_string("deflection_behavior");
            let tells: Vec<String> = node.get_json_or_default("tells");

            let visibility = match visibility_str.as_str() {
                "Known" => WantVisibility::Known,
                "Suspected" => WantVisibility::Suspected,
                _ => WantVisibility::Hidden,
            };

            wants.push(Want {
                id: want_id,
                description,
                intensity: intensity as f32,
                visibility,
                created_at,
                deflection_behavior,
                tells,
            });
        }

        Ok(wants)
    }

    async fn save_want(&self, character_id: CharacterId, want: &Want) -> Result<(), RepoError> {
        let visibility_str = match want.visibility {
            WantVisibility::Known => "Known",
            WantVisibility::Suspected => "Suspected",
            WantVisibility::Hidden => "Hidden",
        };
        let tells_json = serde_json::to_string(&want.tells)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MATCH (c:Character {id: $character_id})
            MERGE (w:Want {id: $want_id})
            SET w.description = $description,
                w.intensity = $intensity,
                w.visibility = $visibility,
                w.created_at = $created_at,
                w.deflection_behavior = $deflection_behavior,
                w.tells = $tells
            MERGE (c)-[:HAS_WANT]->(w)",
        )
        .param("character_id", character_id.to_string())
        .param("want_id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("visibility", visibility_str)
        .param("created_at", want.created_at.to_rfc3339())
        .param(
            "deflection_behavior",
            want.deflection_behavior.clone().unwrap_or_default(),
        )
        .param("tells", tells_json);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Saved want {} for character {}", want.id, character_id);
        Ok(())
    }

    // =========================================================================
    // Disposition Operations
    // =========================================================================

    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>, RepoError> {
        let q = query(
            "MATCH (npc:Character {id: $npc_id})-[d:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            RETURN d.disposition as disposition, d.relationship as relationship, 
                   d.sentiment as sentiment, d.updated_at as updated_at,
                   d.disposition_reason as disposition_reason, d.relationship_points as relationship_points",
        )
        .param("npc_id", npc_id.to_string())
        .param("pc_id", pc_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let disposition_str: String = row
                .get("disposition")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let disposition: DispositionLevel = disposition_str
                .parse()
                .map_err(|e: String| RepoError::Database(e))?;

            let relationship_str: String = row.get("relationship").unwrap_or_else(|_| "Stranger".to_string());
            let relationship: RelationshipLevel = relationship_str.parse().unwrap_or(RelationshipLevel::Stranger);

            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let updated_at = wrldbldr_domain::common::parse_datetime_or(&updated_at_str, chrono::Utc::now());
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            Ok(Some(NpcDispositionState {
                npc_id,
                pc_id,
                disposition,
                relationship,
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            }))
        } else {
            Ok(None)
        }
    }

    async fn save_disposition(&self, disposition: &NpcDispositionState) -> Result<(), RepoError> {
        let q = query(
            "MATCH (npc:Character {id: $npc_id})
            MATCH (pc:PlayerCharacter {id: $pc_id})
            MERGE (npc)-[d:DISPOSITION_TOWARD]->(pc)
            SET d.disposition = $disposition,
                d.relationship = $relationship,
                d.sentiment = $sentiment,
                d.updated_at = $updated_at,
                d.disposition_reason = $disposition_reason,
                d.relationship_points = $relationship_points",
        )
        .param("npc_id", disposition.npc_id.to_string())
        .param("pc_id", disposition.pc_id.to_string())
        .param("disposition", disposition.disposition.to_string())
        .param("relationship", format!("{:?}", disposition.relationship))
        .param("sentiment", disposition.sentiment as f64)
        .param("updated_at", disposition.updated_at.to_rfc3339())
        .param(
            "disposition_reason",
            disposition.disposition_reason.clone().unwrap_or_default(),
        )
        .param("relationship_points", disposition.relationship_points as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!(
            "Saved disposition from NPC {} toward PC {}",
            disposition.npc_id,
            disposition.pc_id
        );
        Ok(())
    }

    // =========================================================================
    // Actantial Context Operations
    // =========================================================================

    async fn get_actantial_context(
        &self,
        id: CharacterId,
    ) -> Result<Option<ActantialContext>, RepoError> {
        // Get character info first
        let char_q = query("MATCH (c:Character {id: $id}) RETURN c.name as name")
            .param("id", id.to_string());

        let mut char_result = self
            .graph
            .execute(char_q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let character_name: String = if let Some(row) = char_result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            row.get("name")
                .map_err(|e| RepoError::Database(e.to_string()))?
        } else {
            return Ok(None); // Character not found
        };

        // ActantialContext is an LLM context structure that aggregates wants and social views.
        // For basic persistence, we return an empty context with the character ID/name.
        // Full context building is done at the service layer.
        Ok(Some(ActantialContext::new(
            Uuid::from(id),
            character_name,
        )))
    }

    async fn save_actantial_context(
        &self,
        _id: CharacterId,
        _context: &ActantialContext,
    ) -> Result<(), RepoError> {
        // ActantialContext is an LLM-ready aggregated view, not directly persisted.
        // The underlying data (wants, actantial views) are persisted separately.
        // This method is a no-op since the context is derived from other persisted data.
        tracing::debug!(
            "save_actantial_context called - this is an aggregated view, underlying data should be saved via save_want and actantial edge operations"
        );
        Ok(())
    }

    // =========================================================================
    // NPC-Region Relationship Operations
    // =========================================================================

    async fn get_region_relationships(&self, id: CharacterId) -> Result<Vec<NpcRegionRelationship>, RepoError> {
        let q = query(
            "MATCH (c:Character {id: $id})
            OPTIONAL MATCH (c)-[h:HOME_REGION]->(hr:Region)
            OPTIONAL MATCH (c)-[w:WORKS_AT_REGION]->(wr:Region)
            OPTIONAL MATCH (c)-[f:FREQUENTS_REGION]->(fr:Region)
            OPTIONAL MATCH (c)-[a:AVOIDS_REGION]->(ar:Region)
            RETURN 
                collect(DISTINCT {region_id: hr.id, type: 'HOME_REGION'}) as home,
                collect(DISTINCT {region_id: wr.id, type: 'WORKS_AT_REGION', shift: w.shift}) as works,
                collect(DISTINCT {region_id: fr.id, type: 'FREQUENTS_REGION', frequency: f.frequency, time_of_day: f.time_of_day}) as frequents,
                collect(DISTINCT {region_id: ar.id, type: 'AVOIDS_REGION', reason: a.reason}) as avoids",
        )
        .param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut relationships = Vec::new();

        if let Some(row) = result.next().await.map_err(|e| RepoError::Database(e.to_string()))? {
            // Parse home regions
            if let Ok(homes) = row.get::<Vec<serde_json::Value>>("home") {
                for h in homes {
                    if let Some(region_id_str) = h.get("region_id").and_then(|v| v.as_str()) {
                        if !region_id_str.is_empty() {
                            if let Ok(uuid) = Uuid::parse_str(region_id_str) {
                                relationships.push(NpcRegionRelationship {
                                    region_id: RegionId::from_uuid(uuid),
                                    relationship_type: NpcRegionRelationType::HomeRegion,
                                    shift: None,
                                    frequency: None,
                                    time_of_day: None,
                                    reason: None,
                                });
                            }
                        }
                    }
                }
            }

            // Parse work regions
            if let Ok(works) = row.get::<Vec<serde_json::Value>>("works") {
                for w in works {
                    if let Some(region_id_str) = w.get("region_id").and_then(|v| v.as_str()) {
                        if !region_id_str.is_empty() {
                            if let Ok(uuid) = Uuid::parse_str(region_id_str) {
                                relationships.push(NpcRegionRelationship {
                                    region_id: RegionId::from_uuid(uuid),
                                    relationship_type: NpcRegionRelationType::WorksAt,
                                    shift: w.get("shift").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    frequency: None,
                                    time_of_day: None,
                                    reason: None,
                                });
                            }
                        }
                    }
                }
            }

            // Parse frequents regions
            if let Ok(frequents) = row.get::<Vec<serde_json::Value>>("frequents") {
                for f in frequents {
                    if let Some(region_id_str) = f.get("region_id").and_then(|v| v.as_str()) {
                        if !region_id_str.is_empty() {
                            if let Ok(uuid) = Uuid::parse_str(region_id_str) {
                                relationships.push(NpcRegionRelationship {
                                    region_id: RegionId::from_uuid(uuid),
                                    relationship_type: NpcRegionRelationType::Frequents,
                                    shift: None,
                                    frequency: f.get("frequency").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    time_of_day: f.get("time_of_day").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    reason: None,
                                });
                            }
                        }
                    }
                }
            }

            // Parse avoids regions
            if let Ok(avoids) = row.get::<Vec<serde_json::Value>>("avoids") {
                for a in avoids {
                    if let Some(region_id_str) = a.get("region_id").and_then(|v| v.as_str()) {
                        if !region_id_str.is_empty() {
                            if let Ok(uuid) = Uuid::parse_str(region_id_str) {
                                relationships.push(NpcRegionRelationship {
                                    region_id: RegionId::from_uuid(uuid),
                                    relationship_type: NpcRegionRelationType::Avoids,
                                    shift: None,
                                    frequency: None,
                                    time_of_day: None,
                                    reason: a.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(relationships)
    }

    async fn set_home_region(&self, id: CharacterId, region_id: RegionId) -> Result<(), RepoError> {
        // Remove existing HOME_REGION and create new one
        let q = query(
            "MATCH (c:Character {id: $id})
            OPTIONAL MATCH (c)-[old:HOME_REGION]->()
            DELETE old
            WITH c
            MATCH (r:Region {id: $region_id})
            CREATE (c)-[:HOME_REGION]->(r)",
        )
        .param("id", id.to_string())
        .param("region_id", region_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Set home region for character {} to {}", id, region_id);
        Ok(())
    }

    async fn set_work_region(&self, id: CharacterId, region_id: RegionId, shift: Option<String>) -> Result<(), RepoError> {
        // Remove existing WORKS_AT_REGION and create new one
        let q = query(
            "MATCH (c:Character {id: $id})
            OPTIONAL MATCH (c)-[old:WORKS_AT_REGION]->()
            DELETE old
            WITH c
            MATCH (r:Region {id: $region_id})
            CREATE (c)-[:WORKS_AT_REGION {shift: $shift}]->(r)",
        )
        .param("id", id.to_string())
        .param("region_id", region_id.to_string())
        .param("shift", shift.unwrap_or_else(|| "always".to_string()));

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Set work region for character {} to {}", id, region_id);
        Ok(())
    }

    async fn add_frequents_region(&self, id: CharacterId, region_id: RegionId, frequency: String, time_of_day: Option<String>) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Character {id: $id})
            MATCH (r:Region {id: $region_id})
            MERGE (c)-[f:FREQUENTS_REGION]->(r)
            SET f.frequency = $frequency, f.time_of_day = $time_of_day",
        )
        .param("id", id.to_string())
        .param("region_id", region_id.to_string())
        .param("frequency", frequency)
        .param("time_of_day", time_of_day.unwrap_or_default());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Added frequents region for character {} to {}", id, region_id);
        Ok(())
    }

    async fn add_avoids_region(&self, id: CharacterId, region_id: RegionId, reason: Option<String>) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Character {id: $id})
            MATCH (r:Region {id: $region_id})
            MERGE (c)-[a:AVOIDS_REGION]->(r)
            SET a.reason = $reason",
        )
        .param("id", id.to_string())
        .param("region_id", region_id.to_string())
        .param("reason", reason.unwrap_or_default());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Added avoids region for character {} to {}", id, region_id);
        Ok(())
    }

    async fn remove_region_relationship(&self, id: CharacterId, region_id: RegionId, relationship_type: &str) -> Result<(), RepoError> {
        // Use static queries per relationship type to avoid any format-based injection risk
        let (cypher, rel_type_name) = match relationship_type.to_uppercase().as_str() {
            "HOME_REGION" | "HOME" => (
                "MATCH (c:Character {id: $id})-[r:HOME_REGION]->(region:Region {id: $region_id}) DELETE r",
                "HOME_REGION"
            ),
            "WORKS_AT_REGION" | "WORKS_AT" | "WORK" => (
                "MATCH (c:Character {id: $id})-[r:WORKS_AT_REGION]->(region:Region {id: $region_id}) DELETE r",
                "WORKS_AT_REGION"
            ),
            "FREQUENTS_REGION" | "FREQUENTS" => (
                "MATCH (c:Character {id: $id})-[r:FREQUENTS_REGION]->(region:Region {id: $region_id}) DELETE r",
                "FREQUENTS_REGION"
            ),
            "AVOIDS_REGION" | "AVOIDS" => (
                "MATCH (c:Character {id: $id})-[r:AVOIDS_REGION]->(region:Region {id: $region_id}) DELETE r",
                "AVOIDS_REGION"
            ),
            _ => return Err(RepoError::Database(format!("Unknown relationship type: {}", relationship_type))),
        };

        let q = query(cypher)
            .param("id", id.to_string())
            .param("region_id", region_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Removed {} relationship from character {} to region {}", rel_type_name, id, region_id);
        Ok(())
    }

    async fn get_npcs_for_region(&self, region_id: RegionId) -> Result<Vec<NpcWithRegionInfo>, RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})
            OPTIONAL MATCH (c1:Character)-[h:HOME_REGION]->(r)
            OPTIONAL MATCH (c2:Character)-[w:WORKS_AT_REGION]->(r)
            OPTIONAL MATCH (c3:Character)-[f:FREQUENTS_REGION]->(r)
            OPTIONAL MATCH (c4:Character)-[a:AVOIDS_REGION]->(r)
            WITH r,
                collect(DISTINCT {c: c1, type: 'HOME_REGION', shift: null, frequency: null, time_of_day: null, reason: null}) as homes,
                collect(DISTINCT {c: c2, type: 'WORKS_AT_REGION', shift: w.shift, frequency: null, time_of_day: null, reason: null}) as works,
                collect(DISTINCT {c: c3, type: 'FREQUENTS_REGION', shift: null, frequency: f.frequency, time_of_day: f.time_of_day, reason: null}) as frequents,
                collect(DISTINCT {c: c4, type: 'AVOIDS_REGION', shift: null, frequency: null, time_of_day: null, reason: a.reason}) as avoids
            UNWIND (homes + works + frequents + avoids) as item
            WHERE item.c IS NOT NULL
            RETURN DISTINCT item.c.id as character_id, item.c.name as name, 
                   item.c.sprite_asset as sprite_asset, item.c.portrait_asset as portrait_asset,
                   item.type as relationship_type, item.shift as shift, 
                   item.frequency as frequency, item.time_of_day as time_of_day, item.reason as reason,
                   COALESCE(item.c.default_mood, 'calm') as default_mood",
        )
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut npcs = Vec::new();

        while let Some(row) = result.next().await.map_err(|e| RepoError::Database(e.to_string()))? {
            let character_id_str: String = row.get("character_id").map_err(|e| RepoError::Database(e.to_string()))?;
            let character_id = CharacterId::from_uuid(
                Uuid::parse_str(&character_id_str).map_err(|e| RepoError::Database(e.to_string()))?
            );
            let name: String = row.get("name").map_err(|e| RepoError::Database(e.to_string()))?;
            let sprite_asset: Option<String> = row.get("sprite_asset").ok();
            let portrait_asset: Option<String> = row.get("portrait_asset").ok();
            let rel_type_str: String = row.get("relationship_type").map_err(|e| RepoError::Database(e.to_string()))?;
            let shift: Option<String> = row.get("shift").ok();
            let frequency: Option<String> = row.get("frequency").ok();
            let time_of_day: Option<String> = row.get("time_of_day").ok();
            let reason: Option<String> = row.get("reason").ok();

            let relationship_type = match rel_type_str.as_str() {
                "HOME_REGION" => NpcRegionRelationType::HomeRegion,
                "WORKS_AT_REGION" => NpcRegionRelationType::WorksAt,
                "FREQUENTS_REGION" => NpcRegionRelationType::Frequents,
                "AVOIDS_REGION" => NpcRegionRelationType::Avoids,
                _ => continue,
            };

            // Parse default_mood from string
            let default_mood_str: String = row.get("default_mood").unwrap_or_else(|_| "calm".to_string());
            let default_mood: MoodState = default_mood_str.parse().unwrap_or(MoodState::Calm);

            npcs.push(NpcWithRegionInfo {
                character_id,
                name,
                sprite_asset,
                portrait_asset,
                relationship_type,
                shift,
                frequency,
                time_of_day,
                reason,
                default_mood,
            });
        }

        Ok(npcs)
    }
}
