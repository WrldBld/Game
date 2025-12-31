//! CharacterCrudPort implementation

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_domain::entities::Character;
use wrldbldr_domain::{CharacterId, SceneId, WorldId};
use wrldbldr_engine_ports::outbound::CharacterCrudPort;

use super::common::row_to_character;
use super::stored_types::{ArchetypeChangeStored, StatBlockStored};
use super::Neo4jCharacterRepository;

impl Neo4jCharacterRepository {
    /// Create a new character
    pub(crate) async fn create_impl(&self, character: &Character) -> Result<()> {
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
    pub(crate) async fn get_impl(&self, id: CharacterId) -> Result<Option<Character>> {
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
    pub(crate) async fn list_by_world_impl(&self, world_id: WorldId) -> Result<Vec<Character>> {
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
    pub(crate) async fn get_by_scene_impl(&self, scene_id: SceneId) -> Result<Vec<Character>> {
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
    pub(crate) async fn update_impl(&self, character: &Character) -> Result<()> {
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
    pub(crate) async fn delete_impl(&self, id: CharacterId) -> Result<()> {
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
}

#[async_trait]
impl CharacterCrudPort for Neo4jCharacterRepository {
    async fn create(&self, character: &Character) -> Result<()> {
        self.create_impl(character).await
    }

    async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        self.get_impl(id).await
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Character>> {
        self.list_by_world_impl(world_id).await
    }

    async fn update(&self, character: &Character) -> Result<()> {
        self.update_impl(character).await
    }

    async fn delete(&self, id: CharacterId) -> Result<()> {
        self.delete_impl(id).await
    }

    async fn get_by_scene(&self, scene_id: SceneId) -> Result<Vec<Character>> {
        self.get_by_scene_impl(scene_id).await
    }
}
