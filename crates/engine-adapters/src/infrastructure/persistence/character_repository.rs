//! Character repository implementation for Neo4j
//!
//! # Graph-First Design (Phase 0.C)
//!
//! This repository uses Neo4j edges for all relationships:
//! - Wants: `(Character)-[:HAS_WANT]->(Want)` + `(Want)-[:TARGETS]->(target)`
//! - Inventory: `(Character)-[:POSSESSES]->(Item)`
//! - Location: `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS`
//! - Actantial: `VIEWS_AS_HELPER`, `VIEWS_AS_OPPONENT`, etc.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use super::converters::{row_to_item, row_to_want};
use wrldbldr_domain::entities::{
    AcquisitionMethod, ActantialRole, ActantialView, Character, CharacterWant, FrequencyLevel,
    InventoryItem, StatBlock, Want, WantVisibility,
};
use wrldbldr_domain::value_objects::{
    ActantialTarget, ArchetypeChange, CampbellArchetype, DispositionLevel, NpcDispositionState,
    RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift, RelationshipLevel,
    WantTarget,
};
use wrldbldr_domain::PlayerCharacterId;
use wrldbldr_domain::{CharacterId, ItemId, LocationId, RegionId, SceneId, WantId, WorldId};
use wrldbldr_engine_ports::outbound::CharacterRepositoryPort;

/// Repository for Character operations
pub struct Neo4jCharacterRepository {
    connection: Neo4jConnection,
}

impl Neo4jCharacterRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    // =========================================================================
    // Core CRUD
    // =========================================================================

    /// Create a new character
    pub async fn create(&self, character: &Character) -> Result<()> {
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (c:Character {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                sprite_asset: $sprite_asset,
                portrait_asset: $portrait_asset,
                base_archetype: $base_archetype,
                current_archetype: $current_archetype,
                archetype_history: $archetype_history,
                stats: $stats,
                is_alive: $is_alive,
                is_active: $is_active,
                default_disposition: $default_disposition
            })
            CREATE (w)-[:CONTAINS_CHARACTER]->(c)
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
        );

        self.connection.graph().run(q).await?;
        tracing::debug!("Created character: {}", character.name);
        Ok(())
    }

    /// Get a character by ID
    pub async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        let q = query(
            "MATCH (c:Character {id: $id})
            RETURN c",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_character(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all characters in a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Character>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHARACTER]->(c:Character)
            RETURN c
            ORDER BY c.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    /// Get all characters featured in a scene (via FEATURES_CHARACTER edge)
    pub async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[:FEATURES_CHARACTER]->(c:Character)
            RETURN c
            ORDER BY c.name",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    /// Update a character
    pub async fn update(&self, character: &Character) -> Result<()> {
        let stats_json = serde_json::to_string(&StatBlockStored::from(character.stats.clone()))?;
        let archetype_history_json = serde_json::to_string(
            &character
                .archetype_history
                .iter()
                .cloned()
                .map(ArchetypeChangeStored::from)
                .collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (c:Character {id: $id})
            SET c.name = $name,
                c.description = $description,
                c.sprite_asset = $sprite_asset,
                c.portrait_asset = $portrait_asset,
                c.base_archetype = $base_archetype,
                c.current_archetype = $current_archetype,
                c.archetype_history = $archetype_history,
                c.stats = $stats,
                c.is_alive = $is_alive,
                c.is_active = $is_active,
                c.default_disposition = $default_disposition
            RETURN c.id as id",
        )
        .param("id", character.id.to_string())
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
        );

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated character: {}", character.name);
        Ok(())
    }

    /// Delete a character (cascading deletes wants, actantial views, inventory edges)
    pub async fn delete(&self, id: CharacterId) -> Result<()> {
        // First delete all Want nodes connected to this character
        let delete_wants = query(
            "MATCH (c:Character {id: $id})-[:HAS_WANT]->(w:Want)
            DETACH DELETE w",
        )
        .param("id", id.to_string());
        self.connection.graph().run(delete_wants).await?;

        // Then delete the character itself
        let q = query(
            "MATCH (c:Character {id: $id})
            DETACH DELETE c",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted character: {}", id);
        Ok(())
    }

    /// Change a character's archetype
    pub async fn change_archetype(
        &self,
        id: CharacterId,
        new_archetype: CampbellArchetype,
        reason: &str,
    ) -> Result<()> {
        if let Some(mut character) = self.get(id).await? {
            character.change_archetype(new_archetype, reason, Utc::now());
            self.update(&character).await?;
        }
        Ok(())
    }

    // =========================================================================
    // Wants
    // =========================================================================

    /// Create a want and attach it to a character
    pub async fn create_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()> {
        let tells_json = serde_json::to_string(&want.tells)?;
        let visibility_str = match want.visibility {
            WantVisibility::Known => "Known",
            WantVisibility::Suspected => "Suspected",
            WantVisibility::Hidden => "Hidden",
        };

        let q = query(
            "MATCH (c:Character {id: $character_id})
            CREATE (w:Want {
                id: $id,
                description: $description,
                intensity: $intensity,
                visibility: $visibility,
                created_at: $created_at,
                deflection_behavior: $deflection_behavior,
                tells: $tells
            })
            CREATE (c)-[:HAS_WANT {
                priority: $priority,
                acquired_at: $acquired_at
            }]->(w)
            RETURN w.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("visibility", visibility_str)
        .param("created_at", want.created_at.to_rfc3339())
        .param(
            "deflection_behavior",
            want.deflection_behavior.clone().unwrap_or_default(),
        )
        .param("tells", tells_json)
        .param("priority", priority as i64)
        .param("acquired_at", Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Created want for character {}: {}",
            character_id,
            want.description
        );
        Ok(())
    }

    /// Get all wants for a character
    pub async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:HAS_WANT]->(w:Want)
            RETURN w, r.priority as priority, r.acquired_at as acquired_at
            ORDER BY r.priority",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut wants = Vec::new();

        while let Some(row) = result.next().await? {
            let want = row_to_want(&row)?;
            let priority: i64 = row.get("priority")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquired_at = DateTime::parse_from_rfc3339(&acquired_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            wants.push(CharacterWant {
                want,
                priority: priority as u32,
                acquired_at,
            });
        }

        Ok(wants)
    }

    /// Update a want
    pub async fn update_want(&self, want: &Want) -> Result<()> {
        let tells_json = serde_json::to_string(&want.tells)?;
        let visibility_str = match want.visibility {
            WantVisibility::Known => "Known",
            WantVisibility::Suspected => "Suspected",
            WantVisibility::Hidden => "Hidden",
        };

        let q = query(
            "MATCH (w:Want {id: $id})
            SET w.description = $description,
                w.intensity = $intensity,
                w.visibility = $visibility,
                w.deflection_behavior = $deflection_behavior,
                w.tells = $tells
            RETURN w.id as id",
        )
        .param("id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("visibility", visibility_str)
        .param(
            "deflection_behavior",
            want.deflection_behavior.clone().unwrap_or_default(),
        )
        .param("tells", tells_json);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete a want
    pub async fn delete_want(&self, want_id: WantId) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})
            DETACH DELETE w",
        )
        .param("id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set a want's target (creates TARGETS edge)
    pub async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()> {
        // First remove any existing target
        self.remove_want_target(want_id).await?;

        // Create the new TARGETS edge based on target type
        let cypher = match target_type {
            "Character" => {
                "MATCH (w:Want {id: $want_id}), (t:Character {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            "Item" => {
                "MATCH (w:Want {id: $want_id}), (t:Item {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            "Goal" => {
                "MATCH (w:Want {id: $want_id}), (t:Goal {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            _ => return Err(anyhow::anyhow!("Invalid target type: {}", target_type)),
        };

        let q = query(cypher)
            .param("want_id", want_id.to_string())
            .param("target_id", target_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a want's target
    pub async fn remove_want_target(&self, want_id: WantId) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})-[r:TARGETS]->()
            DELETE r",
        )
        .param("id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    // =========================================================================
    // Actantial Views
    // =========================================================================

    /// Add an actantial view
    pub async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}}), (t:Character {{id: $target_id}})
            CREATE (s)-[:{} {{
                want_id: $want_id,
                reason: $reason,
                assigned_at: $assigned_at
            }}]->(t)",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", view.want_id.to_string())
            .param("reason", view.reason.clone())
            .param("assigned_at", view.assigned_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get all actantial views for a character (toward both NPCs and PCs)
    pub async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, ActantialTarget, ActantialView)>> {
        // Query views toward NPCs
        let q_npc = query(
            "MATCH (s:Character {id: $id})-[r]->(t:Character)
            WHERE type(r) IN ['VIEWS_AS_HELPER', 'VIEWS_AS_OPPONENT', 'VIEWS_AS_SENDER', 'VIEWS_AS_RECEIVER']
            RETURN type(r) as role_type, t.id as target_id, 'NPC' as target_type,
                   r.want_id as want_id, r.reason as reason, r.assigned_at as assigned_at",
        )
        .param("id", character_id.to_string());

        // Query views toward PCs
        let q_pc = query(
            "MATCH (s:Character {id: $id})-[r]->(t:PlayerCharacter)
            WHERE type(r) IN ['VIEWS_AS_HELPER', 'VIEWS_AS_OPPONENT', 'VIEWS_AS_SENDER', 'VIEWS_AS_RECEIVER']
            RETURN type(r) as role_type, t.id as target_id, 'PC' as target_type,
                   r.want_id as want_id, r.reason as reason, r.assigned_at as assigned_at",
        )
        .param("id", character_id.to_string());

        let mut views = Vec::new();

        // Process NPC views
        let mut result = self.connection.graph().execute(q_npc).await?;
        while let Some(row) = result.next().await? {
            if let Some(view) = self.parse_actantial_view_row(&row)? {
                views.push(view);
            }
        }

        // Process PC views
        let mut result = self.connection.graph().execute(q_pc).await?;
        while let Some(row) = result.next().await? {
            if let Some(view) = self.parse_actantial_view_row(&row)? {
                views.push(view);
            }
        }

        Ok(views)
    }

    /// Helper to parse actantial view row
    fn parse_actantial_view_row(
        &self,
        row: &Row,
    ) -> Result<Option<(ActantialRole, ActantialTarget, ActantialView)>> {
        let role_type: String = row.get("role_type")?;
        let target_id_str: String = row.get("target_id")?;
        let target_type: String = row.get("target_type")?;
        let want_id_str: String = row.get("want_id")?;
        let reason: String = row.get("reason")?;
        let assigned_at_str: String = row.get("assigned_at")?;

        let role = match role_type.as_str() {
            "VIEWS_AS_HELPER" => ActantialRole::Helper,
            "VIEWS_AS_OPPONENT" => ActantialRole::Opponent,
            "VIEWS_AS_SENDER" => ActantialRole::Sender,
            "VIEWS_AS_RECEIVER" => ActantialRole::Receiver,
            _ => return Ok(None),
        };

        let target_uuid = uuid::Uuid::parse_str(&target_id_str)?;
        let target = match target_type.as_str() {
            "NPC" => ActantialTarget::Npc(target_uuid),
            "PC" => ActantialTarget::Pc(target_uuid),
            _ => return Ok(None),
        };

        let want_id = WantId::from_uuid(uuid::Uuid::parse_str(&want_id_str)?);
        let assigned_at = DateTime::parse_from_rfc3339(&assigned_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some((
            role,
            target,
            ActantialView {
                want_id,
                reason,
                assigned_at,
            },
        )))
    }

    /// Remove an actantial view
    pub async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}})-[r:{} {{want_id: $want_id}}]->(t:Character {{id: $target_id}})
            DELETE r",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an actantial view toward a PC
    pub async fn add_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}}), (t:PlayerCharacter {{id: $target_id}})
            CREATE (s)-[:{} {{
                want_id: $want_id,
                reason: $reason,
                assigned_at: $assigned_at
            }}]->(t)",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", view.want_id.to_string())
            .param("reason", view.reason.clone())
            .param("assigned_at", view.assigned_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an actantial view toward a PC
    pub async fn remove_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        want_id: WantId,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}})-[r:{} {{want_id: $want_id}}]->(t:PlayerCharacter {{id: $target_id}})
            DELETE r",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get the resolved target of a want
    pub async fn get_want_target(&self, want_id: WantId) -> Result<Option<WantTarget>> {
        // Query for TARGETS edge to any of the possible target types
        let q = query(
            "MATCH (w:Want {id: $want_id})-[:TARGETS]->(target)
            RETURN labels(target) as labels, target.id as id, target.name as name,
                   target.description as description",
        )
        .param("want_id", want_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let labels: Vec<String> = row.get("labels")?;
            let id_str: String = row.get("id")?;
            let name: String = row.get("name")?;
            let description: Option<String> = row
                .get("description")
                .ok()
                .filter(|s: &String| !s.is_empty());

            let id = uuid::Uuid::parse_str(&id_str)?;

            // Determine target type from labels
            if labels.contains(&"Character".to_string()) {
                Ok(Some(WantTarget::Character { id, name }))
            } else if labels.contains(&"Item".to_string()) {
                Ok(Some(WantTarget::Item { id, name }))
            } else if labels.contains(&"Goal".to_string()) {
                Ok(Some(WantTarget::Goal {
                    id,
                    name,
                    description,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    // =========================================================================
    // Inventory
    // =========================================================================

    /// Add an item to character's inventory
    pub async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()> {
        let method_str = acquisition_method
            .map(|m| m.to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (c:Character {id: $character_id}), (i:Item {id: $item_id})
            CREATE (c)-[:POSSESSES {
                quantity: $quantity,
                equipped: $equipped,
                acquired_at: $acquired_at,
                acquisition_method: $acquisition_method
            }]->(i)
            RETURN i.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("equipped", equipped)
        .param("acquired_at", Utc::now().to_rfc3339())
        .param("acquisition_method", method_str);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get character's inventory
    pub async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item)
            RETURN i, r.quantity as quantity, r.equipped as equipped, 
                   r.acquired_at as acquired_at, r.acquisition_method as acquisition_method",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut inventory = Vec::new();

        while let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            let quantity: i64 = row.get("quantity")?;
            let equipped: bool = row.get("equipped")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquisition_method_str: String = row.get("acquisition_method").unwrap_or_default();

            let acquired_at = DateTime::parse_from_rfc3339(&acquired_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let acquisition_method = if acquisition_method_str.is_empty() {
                None
            } else {
                acquisition_method_str.parse().ok()
            };

            inventory.push(InventoryItem {
                item,
                quantity: quantity as u32,
                equipped,
                acquired_at,
                acquisition_method,
            });
        }

        Ok(inventory)
    }

    /// Get a single inventory item by ID
    pub async fn get_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            RETURN i, r.quantity as quantity, r.equipped as equipped, 
                   r.acquired_at as acquired_at, r.acquisition_method as acquisition_method",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            let quantity: i64 = row.get("quantity")?;
            let equipped: bool = row.get("equipped")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquisition_method_str: String = row.get("acquisition_method").unwrap_or_default();

            let acquired_at = DateTime::parse_from_rfc3339(&acquired_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let acquisition_method = if acquisition_method_str.is_empty() {
                None
            } else {
                acquisition_method_str.parse().ok()
            };

            Ok(Some(InventoryItem {
                item,
                quantity: quantity as u32,
                equipped,
                acquired_at,
                acquisition_method,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update inventory item
    pub async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            SET r.quantity = $quantity, r.equipped = $equipped
            RETURN i.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("equipped", equipped);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an item from inventory
    pub async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    // =========================================================================
    // Character-Location Relationships
    // =========================================================================

    /// Set character's home location
    pub async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()> {
        // Remove existing home first
        self.remove_home_location(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:HOME_LOCATION {description: $description}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("description", description.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove character's home location
    pub async fn remove_home_location(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:HOME_LOCATION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set character's work location
    pub async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()> {
        // Remove existing work first
        self.remove_work_location(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:WORKS_AT {role: $role, schedule: $schedule}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("role", role)
        .param("schedule", schedule.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove character's work location
    pub async fn remove_work_location(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:WORKS_AT]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add a frequented location
    pub async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:FREQUENTS {
                frequency: $frequency,
                time_of_day: $time_of_day,
                day_of_week: $day_of_week,
                reason: $reason,
                since: $since
            }]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("frequency", frequency.to_string())
        .param("time_of_day", time_of_day)
        .param("day_of_week", day_of_week.unwrap_or_default())
        .param("reason", reason.unwrap_or_default())
        .param("since", Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a frequented location
    pub async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:FREQUENTS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an avoided location
    pub async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:AVOIDS {reason: $reason}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("reason", reason);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an avoided location
    pub async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:AVOIDS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get NPCs who might be at a location
    pub async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<&str>,
    ) -> Result<Vec<Character>> {
        // Build query based on whether time_of_day filter is provided
        let cypher = if time_of_day.is_some() {
            "MATCH (c:Character)-[r]->(l:Location {id: $location_id})
            WHERE (type(r) = 'HOME_LOCATION')
               OR (type(r) = 'WORKS_AT' AND (r.schedule IS NULL OR r.schedule = '' OR r.schedule = $time_of_day))
               OR (type(r) = 'FREQUENTS' AND (r.time_of_day = 'Any' OR r.time_of_day = $time_of_day))
            RETURN DISTINCT c"
        } else {
            "MATCH (c:Character)-[r]->(l:Location {id: $location_id})
            WHERE type(r) IN ['HOME_LOCATION', 'WORKS_AT', 'FREQUENTS']
            RETURN DISTINCT c"
        };

        let q = query(cypher)
            .param("location_id", location_id.to_string())
            .param("time_of_day", time_of_day.unwrap_or(""));

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    // =========================================================================
    // Character-Region Relationships (Phase 23C)
    // =========================================================================

    /// Set character's home region
    pub async fn set_home_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        // Remove existing home region first
        self.remove_home_region(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:HOME_REGION]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set home region for character {}: {}",
            character_id,
            region_id
        );
        Ok(())
    }

    /// Remove character's home region
    pub async fn remove_home_region(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:HOME_REGION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set character's work region with shift (day, night, always)
    pub async fn set_work_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        shift: RegionShift,
    ) -> Result<()> {
        // Remove existing work region first
        self.remove_work_region(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:WORKS_AT_REGION {shift: $shift}]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("shift", shift.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set work region for character {}: {} ({:?})",
            character_id,
            region_id,
            shift
        );
        Ok(())
    }

    /// Remove character's work region
    pub async fn remove_work_region(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:WORKS_AT_REGION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add a frequented region
    pub async fn add_frequented_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        frequency: RegionFrequency,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:FREQUENTS_REGION {
                frequency: $frequency,
                since: $since
            }]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("frequency", frequency.to_string())
        .param("since", Utc::now().to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added frequented region for character {}: {} ({:?})",
            character_id,
            region_id,
            frequency
        );
        Ok(())
    }

    /// Remove a frequented region
    pub async fn remove_frequented_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:FREQUENTS_REGION]->(reg:Region {id: $region_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an avoided region
    pub async fn add_avoided_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        reason: String,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:AVOIDS_REGION {reason: $reason}]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("reason", reason);

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added avoided region for character {}: {}",
            character_id,
            region_id
        );
        Ok(())
    }

    /// Remove an avoided region
    pub async fn remove_avoided_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:AVOIDS_REGION]->(reg:Region {id: $region_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// List all region relationships for a character
    pub async fn list_region_relationships(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<RegionRelationship>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r]->(reg:Region)
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN type(r) as rel_type, reg.id as region_id, reg.name as region_name,
                   r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut relationships = Vec::new();

        while let Some(row) = result.next().await? {
            let rel_type: String = row.get("rel_type")?;
            let region_id_str: String = row.get("region_id")?;
            let region_name: String = row.get("region_name")?;

            let region_id = RegionId::from_uuid(uuid::Uuid::parse_str(&region_id_str)?);

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift_str: String = row.get("shift").unwrap_or_default();
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let freq_str: String = row.get("frequency").unwrap_or_default();
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => {
                    let reason: String = row.get("reason").unwrap_or_default();
                    RegionRelationshipType::Avoids { reason }
                }
                _ => continue,
            };

            relationships.push(RegionRelationship {
                region_id,
                region_name,
                relationship_type,
            });
        }

        Ok(relationships)
    }

    /// Get all NPCs with any relationship to a region
    pub async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        let q = query(
            "MATCH (c:Character)-[r]->(reg:Region {id: $region_id})
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN c, type(r) as rel_type, r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("region_id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            // Extract relationship data first (before consuming row for character)
            let rel_type: String = row.get("rel_type")?;
            let shift_str: String = row.get("shift").unwrap_or_default();
            let freq_str: String = row.get("frequency").unwrap_or_default();
            let reason: String = row.get("reason").unwrap_or_default();

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => RegionRelationshipType::Avoids { reason },
                _ => continue,
            };

            let character = row_to_character(row)?;
            npcs.push((character, relationship_type));
        }

        Ok(npcs)
    }

    // =========================================================================
    // NPC Disposition & Relationship Methods
    // =========================================================================

    /// Get an NPC's disposition state toward a specific PC
    pub async fn get_disposition_toward_pc(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>> {
        let q = query(
            "MATCH (npc:Character {id: $npc_id})-[r:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            RETURN r.disposition as disposition, r.relationship as relationship, r.sentiment as sentiment,
                   r.updated_at as updated_at, r.disposition_reason as disposition_reason, r.relationship_points as relationship_points",
        )
        .param("npc_id", npc_id.to_string())
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let disposition_str: String = row
                .get("disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            let relationship_str: String = row
                .get("relationship")
                .unwrap_or_else(|_| "Stranger".to_string());
            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Some(NpcDispositionState {
                npc_id,
                pc_id,
                disposition: disposition_str.parse().unwrap_or(DispositionLevel::Neutral),
                relationship: relationship_str
                    .parse()
                    .unwrap_or(RelationshipLevel::Stranger),
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            }))
        } else {
            Ok(None)
        }
    }

    /// Set/update an NPC's disposition state toward a specific PC
    pub async fn set_disposition_toward_pc(
        &self,
        disposition_state: &NpcDispositionState,
    ) -> Result<()> {
        let q = query(
            "MATCH (npc:Character {id: $npc_id}), (pc:PlayerCharacter {id: $pc_id})
            MERGE (npc)-[r:DISPOSITION_TOWARD]->(pc)
            SET r.disposition = $disposition,
                r.relationship = $relationship,
                r.sentiment = $sentiment,
                r.updated_at = $updated_at,
                r.disposition_reason = $disposition_reason,
                r.relationship_points = $relationship_points
            RETURN npc.id as id",
        )
        .param("npc_id", disposition_state.npc_id.to_string())
        .param("pc_id", disposition_state.pc_id.to_string())
        .param("disposition", disposition_state.disposition.to_string())
        .param("relationship", disposition_state.relationship.to_string())
        .param("sentiment", disposition_state.sentiment as f64)
        .param("updated_at", disposition_state.updated_at.to_rfc3339())
        .param(
            "disposition_reason",
            disposition_state
                .disposition_reason
                .clone()
                .unwrap_or_default(),
        )
        .param(
            "relationship_points",
            disposition_state.relationship_points as i64,
        );

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set disposition for NPC {} toward PC {}: {:?}",
            disposition_state.npc_id,
            disposition_state.pc_id,
            disposition_state.disposition
        );
        Ok(())
    }

    /// Get disposition states for multiple NPCs toward a PC (for scene context)
    pub async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        if npc_ids.is_empty() {
            return Ok(vec![]);
        }

        let npc_id_strings: Vec<String> = npc_ids.iter().map(|id| id.to_string()).collect();

        let q = query(
            "MATCH (npc:Character)-[r:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            WHERE npc.id IN $npc_ids
            RETURN npc.id as npc_id, r.disposition as disposition, r.relationship as relationship,
                   r.sentiment as sentiment, r.updated_at as updated_at,
                   r.disposition_reason as disposition_reason, r.relationship_points as relationship_points",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_ids", npc_id_strings);

        let mut result = self.connection.graph().execute(q).await?;
        let mut dispositions = Vec::new();

        while let Some(row) = result.next().await? {
            let npc_id_str: String = row.get("npc_id")?;
            let disposition_str: String = row
                .get("disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            let relationship_str: String = row
                .get("relationship")
                .unwrap_or_else(|_| "Stranger".to_string());
            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            let npc_uuid = uuid::Uuid::parse_str(&npc_id_str)?;
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            dispositions.push(NpcDispositionState {
                npc_id: CharacterId::from_uuid(npc_uuid),
                pc_id,
                disposition: disposition_str.parse().unwrap_or(DispositionLevel::Neutral),
                relationship: relationship_str
                    .parse()
                    .unwrap_or(RelationshipLevel::Stranger),
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            });
        }

        Ok(dispositions)
    }

    /// Get all NPCs who have a relationship with a PC (for DM panel)
    pub async fn get_all_npc_dispositions_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        let q = query(
            "MATCH (npc:Character)-[r:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            RETURN npc.id as npc_id, r.disposition as disposition, r.relationship as relationship,
                   r.sentiment as sentiment, r.updated_at as updated_at,
                   r.disposition_reason as disposition_reason, r.relationship_points as relationship_points
            ORDER BY r.updated_at DESC",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut dispositions = Vec::new();

        while let Some(row) = result.next().await? {
            let npc_id_str: String = row.get("npc_id")?;
            let disposition_str: String = row
                .get("disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            let relationship_str: String = row
                .get("relationship")
                .unwrap_or_else(|_| "Stranger".to_string());
            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            let npc_uuid = uuid::Uuid::parse_str(&npc_id_str)?;
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            dispositions.push(NpcDispositionState {
                npc_id: CharacterId::from_uuid(npc_uuid),
                pc_id,
                disposition: disposition_str.parse().unwrap_or(DispositionLevel::Neutral),
                relationship: relationship_str
                    .parse()
                    .unwrap_or(RelationshipLevel::Stranger),
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            });
        }

        Ok(dispositions)
    }

    /// Get the NPC's default/global disposition (from Character node)
    pub async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel> {
        let q = query(
            "MATCH (c:Character {id: $id})
            RETURN c.default_disposition as default_disposition",
        )
        .param("id", npc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let disposition_str: String = row
                .get("default_disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            Ok(disposition_str.parse().unwrap_or(DispositionLevel::Neutral))
        } else {
            Ok(DispositionLevel::Neutral)
        }
    }

    /// Set the NPC's default/global disposition (on Character node)
    pub async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})
            SET c.default_disposition = $disposition
            RETURN c.id as id",
        )
        .param("id", npc_id.to_string())
        .param("disposition", disposition.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set default disposition for NPC {}: {:?}",
            npc_id,
            disposition
        );
        Ok(())
    }
}

// =============================================================================
// Row Conversion Helpers
// =============================================================================

fn row_to_character(row: Row) -> Result<Character> {
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
    let stats: StatBlock = serde_json::from_str::<StatBlockStored>(&stats_json)?.into();
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

// =============================================================================
// Persistence serde models
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatBlockStored {
    pub stats: std::collections::HashMap<String, i32>,
    pub current_hp: Option<i32>,
    pub max_hp: Option<i32>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchetypeChangeStored {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub timestamp: String,
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
        let timestamp = chrono::DateTime::parse_from_rfc3339(&value.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Self {
            from: value.from.parse().unwrap_or(CampbellArchetype::Ally),
            to: value.to.parse().unwrap_or(CampbellArchetype::Ally),
            reason: value.reason,
            timestamp,
        }
    }
}

// =============================================================================
// CharacterRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl CharacterRepositoryPort for Neo4jCharacterRepository {
    async fn create(&self, character: &Character) -> Result<()> {
        Neo4jCharacterRepository::create(self, character).await
    }

    async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        Neo4jCharacterRepository::get(self, id).await
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Character>> {
        Neo4jCharacterRepository::list_by_world(self, world_id).await
    }

    async fn update(&self, character: &Character) -> Result<()> {
        Neo4jCharacterRepository::update(self, character).await
    }

    async fn delete(&self, id: CharacterId) -> Result<()> {
        Neo4jCharacterRepository::delete(self, id).await
    }

    async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>> {
        Neo4jCharacterRepository::get_by_scene(self, scene_id).await
    }

    // Wants
    async fn create_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()> {
        Neo4jCharacterRepository::create_want(self, character_id, want, priority).await
    }

    async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>> {
        Neo4jCharacterRepository::get_wants(self, character_id).await
    }

    async fn update_want(&self, want: &Want) -> Result<()> {
        Neo4jCharacterRepository::update_want(self, want).await
    }

    async fn delete_want(&self, want_id: WantId) -> Result<()> {
        Neo4jCharacterRepository::delete_want(self, want_id).await
    }

    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_want_target(self, want_id, target_id, target_type).await
    }

    async fn remove_want_target(&self, want_id: WantId) -> Result<()> {
        Neo4jCharacterRepository::remove_want_target(self, want_id).await
    }

    // Actantial Views
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_actantial_view(self, subject_id, role, target_id, view).await
    }

    async fn add_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_actantial_view_to_pc(self, subject_id, role, target_id, view)
            .await
    }

    async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, ActantialTarget, ActantialView)>> {
        Neo4jCharacterRepository::get_actantial_views(self, character_id).await
    }

    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_actantial_view(self, subject_id, role, target_id, want_id)
            .await
    }

    async fn remove_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        want_id: WantId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_actantial_view_to_pc(
            self, subject_id, role, target_id, want_id,
        )
        .await
    }

    async fn get_want_target(&self, want_id: WantId) -> Result<Option<WantTarget>> {
        Neo4jCharacterRepository::get_want_target(self, want_id).await
    }

    // Inventory
    async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_inventory_item(
            self,
            character_id,
            item_id,
            quantity,
            equipped,
            acquisition_method,
        )
        .await
    }

    async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>> {
        Neo4jCharacterRepository::get_inventory(self, character_id).await
    }

    async fn get_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>> {
        Neo4jCharacterRepository::get_inventory_item(self, character_id, item_id).await
    }

    async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()> {
        Neo4jCharacterRepository::update_inventory_item(
            self,
            character_id,
            item_id,
            quantity,
            equipped,
        )
        .await
    }

    async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_inventory_item(self, character_id, item_id).await
    }

    // Character-Location Relationships
    async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_home_location(self, character_id, location_id, description)
            .await
    }

    async fn remove_home_location(&self, character_id: CharacterId) -> Result<()> {
        Neo4jCharacterRepository::remove_home_location(self, character_id).await
    }

    async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_work_location(self, character_id, location_id, role, schedule)
            .await
    }

    async fn remove_work_location(&self, character_id: CharacterId) -> Result<()> {
        Neo4jCharacterRepository::remove_work_location(self, character_id).await
    }

    async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_frequented_location(
            self,
            character_id,
            location_id,
            frequency,
            time_of_day,
            day_of_week,
            reason,
        )
        .await
    }

    async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_frequented_location(self, character_id, location_id).await
    }

    async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()> {
        Neo4jCharacterRepository::add_avoided_location(self, character_id, location_id, reason)
            .await
    }

    async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        Neo4jCharacterRepository::remove_avoided_location(self, character_id, location_id).await
    }

    async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<&str>,
    ) -> Result<Vec<Character>> {
        Neo4jCharacterRepository::get_npcs_at_location(self, location_id, time_of_day).await
    }

    // Disposition & Relationship
    async fn get_disposition_toward_pc(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>> {
        Neo4jCharacterRepository::get_disposition_toward_pc(self, npc_id, pc_id).await
    }

    async fn set_disposition_toward_pc(
        &self,
        disposition_state: &NpcDispositionState,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_disposition_toward_pc(self, disposition_state).await
    }

    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        Neo4jCharacterRepository::get_scene_dispositions(self, npc_ids, pc_id).await
    }

    async fn get_all_npc_dispositions_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        Neo4jCharacterRepository::get_all_npc_dispositions_for_pc(self, pc_id).await
    }

    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel> {
        Neo4jCharacterRepository::get_default_disposition(self, npc_id).await
    }

    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_default_disposition(self, npc_id, disposition).await
    }

    // Character-Region Relationships
    async fn get_region_relationships(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<RegionRelationship>> {
        Neo4jCharacterRepository::list_region_relationships(self, character_id).await
    }

    async fn set_home_region(&self, character_id: CharacterId, region_id: RegionId) -> Result<()> {
        Neo4jCharacterRepository::set_home_region(self, character_id, region_id).await
    }

    async fn set_work_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        shift: RegionShift,
    ) -> Result<()> {
        Neo4jCharacterRepository::set_work_region(self, character_id, region_id, shift).await
    }

    async fn remove_region_relationship(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        relationship_type: &str,
    ) -> Result<()> {
        match relationship_type.to_lowercase().as_str() {
            "home" => Neo4jCharacterRepository::remove_home_region(self, character_id).await,
            "work" => Neo4jCharacterRepository::remove_work_region(self, character_id).await,
            "frequents" => {
                Neo4jCharacterRepository::remove_frequented_region(self, character_id, region_id)
                    .await
            }
            "avoids" => {
                Neo4jCharacterRepository::remove_avoided_region(self, character_id, region_id).await
            }
            _ => Err(anyhow::anyhow!(
                "Unknown relationship type: {}. Must be one of: home, work, frequents, avoids",
                relationship_type
            )),
        }
    }
}
