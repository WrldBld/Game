//! Neo4j scene repository implementation.
//!
//! # Graph-First Design
//!
//! This repository uses Neo4j edges for all relationships:
//! - Location: `(Scene)-[:AT_LOCATION]->(Location)`
//! - Region: `(Scene)-[:IN_REGION]->(Region)` (for scenes tied to a region)
//! - Featured characters: `(Scene)-[:FEATURES_CHARACTER {role, entrance_cue}]->(Character)`
//! - Current scene: `(World)-[:CURRENT_SCENE]->(Scene)`
//!
//! Entry conditions remain as JSON (acceptable per ADR - complex nested non-relational)

use async_trait::async_trait;
use neo4rs::{query, Graph, Row};
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{RepoError, SceneRepo};

// =============================================================================
// Stored Types for JSON serialization
// =============================================================================

/// Stored representation of TimeContext for Neo4j persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum TimeContextStored {
    Unspecified,
    TimeOfDay(TimeOfDayStored),
    During(String),
    Custom(String),
}

impl From<TimeContext> for TimeContextStored {
    fn from(value: TimeContext) -> Self {
        match value {
            TimeContext::Unspecified => Self::Unspecified,
            TimeContext::TimeOfDay(t) => Self::TimeOfDay(t.into()),
            TimeContext::During(s) => Self::During(s),
            TimeContext::Custom(s) => Self::Custom(s),
        }
    }
}

impl From<TimeContextStored> for TimeContext {
    fn from(value: TimeContextStored) -> Self {
        match value {
            TimeContextStored::Unspecified => Self::Unspecified,
            TimeContextStored::TimeOfDay(t) => Self::TimeOfDay(t.into()),
            TimeContextStored::During(s) => Self::During(s),
            TimeContextStored::Custom(s) => Self::Custom(s),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
enum TimeOfDayStored {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl From<TimeOfDay> for TimeOfDayStored {
    fn from(value: TimeOfDay) -> Self {
        match value {
            TimeOfDay::Morning => Self::Morning,
            TimeOfDay::Afternoon => Self::Afternoon,
            TimeOfDay::Evening => Self::Evening,
            TimeOfDay::Night => Self::Night,
        }
    }
}

impl From<TimeOfDayStored> for TimeOfDay {
    fn from(value: TimeOfDayStored) -> Self {
        match value {
            TimeOfDayStored::Morning => Self::Morning,
            TimeOfDayStored::Afternoon => Self::Afternoon,
            TimeOfDayStored::Evening => Self::Evening,
            TimeOfDayStored::Night => Self::Night,
        }
    }
}

/// Stored representation of SceneCondition for Neo4j persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum SceneConditionStored {
    CompletedScene(String),
    HasItem(String),
    KnowsCharacter(String),
    FlagSet(String),
    Custom(String),
}

impl From<SceneCondition> for SceneConditionStored {
    fn from(value: SceneCondition) -> Self {
        match value {
            SceneCondition::CompletedScene(id) => Self::CompletedScene(id.to_string()),
            SceneCondition::HasItem(id) => Self::HasItem(id.to_string()),
            SceneCondition::KnowsCharacter(id) => Self::KnowsCharacter(id.to_string()),
            SceneCondition::FlagSet(s) => Self::FlagSet(s),
            SceneCondition::Custom(s) => Self::Custom(s),
        }
    }
}

impl SceneConditionStored {
    fn try_into_condition(self) -> Result<SceneCondition, RepoError> {
        Ok(match self {
            SceneConditionStored::CompletedScene(id) => SceneCondition::CompletedScene(
                SceneId::from(uuid::Uuid::parse_str(&id).map_err(|e| RepoError::Database(e.to_string()))?),
            ),
            SceneConditionStored::HasItem(id) => SceneCondition::HasItem(
                ItemId::from(uuid::Uuid::parse_str(&id).map_err(|e| RepoError::Database(e.to_string()))?),
            ),
            SceneConditionStored::KnowsCharacter(id) => SceneCondition::KnowsCharacter(
                CharacterId::from(uuid::Uuid::parse_str(&id).map_err(|e| RepoError::Database(e.to_string()))?),
            ),
            SceneConditionStored::FlagSet(s) => SceneCondition::FlagSet(s),
            SceneConditionStored::Custom(s) => SceneCondition::Custom(s),
        })
    }
}

// =============================================================================
// Repository Implementation
// =============================================================================

/// Repository for Scene operations.
pub struct Neo4jSceneRepo {
    graph: Graph,
}

impl Neo4jSceneRepo {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Convert a Neo4j row to a Scene entity.
    fn row_to_scene(&self, row: Row) -> Result<Scene, RepoError> {
        let node: neo4rs::Node = row
            .get("s")
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let id: SceneId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
        let act_id: ActId =
            parse_typed_id(&node, "act_id").map_err(|e| RepoError::Database(e.to_string()))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let directorial_notes: String = node.get_string_or("directorial_notes", "");
        let order_num: i64 = node.get_i64_or("order_num", 0);

        // location_id is stored directly - may be placeholder if using AT_LOCATION edge
        let location_id = match node.get_optional_string("location_id") {
            Some(s) => LocationId::from(
                uuid::Uuid::parse_str(&s).map_err(|e| RepoError::Database(e.to_string()))?,
            ),
            None => LocationId::new(), // Placeholder
        };

        // JSON fields
        let time_context: TimeContext = node
            .get_json::<TimeContextStored>("time_context")
            .map(Into::into)
            .unwrap_or(TimeContext::Unspecified);

        let entry_conditions: Vec<SceneCondition> = node
            .get_json::<Vec<SceneConditionStored>>("entry_conditions")
            .map(|stored| {
                stored
                    .into_iter()
                    .filter_map(|sc| sc.try_into_condition().ok())
                    .collect()
            })
            .unwrap_or_default();

        let featured_characters: Vec<CharacterId> = node
            .get_json_or_default::<Vec<String>>("featured_characters")
            .into_iter()
            .filter_map(|s| uuid::Uuid::parse_str(&s).ok().map(CharacterId::from))
            .collect();

        let backdrop_override = node.get_optional_string("backdrop_override");

        Ok(Scene {
            id,
            act_id,
            name,
            location_id,
            time_context,
            backdrop_override,
            entry_conditions,
            featured_characters,
            directorial_notes,
            order: order_num as u32,
        })
    }
}

#[async_trait]
impl SceneRepo for Neo4jSceneRepo {
    async fn get(&self, id: SceneId) -> Result<Option<Scene>, RepoError> {
        let q = query("MATCH (s:Scene {id: $id}) RETURN s").param("id", id.to_string());

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
            Ok(Some(self.row_to_scene(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, scene: &Scene) -> Result<(), RepoError> {
        let time_context_json = serde_json::to_string(&TimeContextStored::from(scene.time_context.clone()))
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let entry_conditions_json = serde_json::to_string(
            &scene
                .entry_conditions
                .iter()
                .cloned()
                .map(SceneConditionStored::from)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let featured_characters_json = serde_json::to_string(
            &scene
                .featured_characters
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| RepoError::Serialization(e.to_string()))?;

        // MERGE for upsert behavior
        let q = query(
            "MATCH (a:Act {id: $act_id})
            MERGE (s:Scene {id: $id})
            SET s.act_id = $act_id,
                s.name = $name,
                s.location_id = $location_id,
                s.time_context = $time_context,
                s.backdrop_override = $backdrop_override,
                s.entry_conditions = $entry_conditions,
                s.featured_characters = $featured_characters,
                s.directorial_notes = $directorial_notes,
                s.order_num = $order_num
            MERGE (a)-[:CONTAINS_SCENE]->(s)
            WITH s
            MATCH (l:Location {id: $location_id})
            MERGE (s)-[:AT_LOCATION]->(l)
            RETURN s.id as id",
        )
        .param("id", scene.id.to_string())
        .param("act_id", scene.act_id.to_string())
        .param("name", scene.name.clone())
        .param("location_id", scene.location_id.to_string())
        .param("time_context", time_context_json)
        .param(
            "backdrop_override",
            scene.backdrop_override.clone().unwrap_or_default(),
        )
        .param("entry_conditions", entry_conditions_json)
        .param("featured_characters", featured_characters_json)
        .param("directorial_notes", scene.directorial_notes.clone())
        .param("order_num", scene.order as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        // Update FEATURES_CHARACTER edges atomically:
        // Delete existing edges and create new ones in a single query using UNWIND
        let char_ids: Vec<String> = scene
            .featured_characters
            .iter()
            .map(|id| id.to_string())
            .collect();

        let features_q = query(
            "MATCH (s:Scene {id: $scene_id})
            OPTIONAL MATCH (s)-[old:FEATURES_CHARACTER]->()
            DELETE old
            WITH s
            UNWIND CASE WHEN $char_ids = [] THEN [null] ELSE $char_ids END AS char_id
            WITH s, char_id WHERE char_id IS NOT NULL
            MATCH (c:Character {id: char_id})
            CREATE (s)-[:FEATURES_CHARACTER {role: 'Secondary', entrance_cue: ''}]->(c)
            RETURN count(*) as created",
        )
        .param("scene_id", scene.id.to_string())
        .param("char_ids", char_ids);

        self.graph
            .run(features_q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Saved scene: {}", scene.name);
        Ok(())
    }

    async fn delete(&self, id: SceneId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (s:Scene {id: $id})
            DETACH DELETE s",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted scene: {}", id);
        Ok(())
    }

    async fn get_current(&self, world_id: WorldId) -> Result<Option<Scene>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CURRENT_SCENE]->(s:Scene)
            RETURN s",
        )
        .param("world_id", world_id.to_string());

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
            Ok(Some(self.row_to_scene(row)?))
        } else {
            Ok(None)
        }
    }

    async fn set_current(&self, world_id: WorldId, scene_id: SceneId) -> Result<(), RepoError> {
        // Atomically remove any existing CURRENT_SCENE edge and create new one
        let q = query(
            "MATCH (w:World {id: $world_id})
            OPTIONAL MATCH (w)-[old:CURRENT_SCENE]->()
            DELETE old
            WITH w
            MATCH (s:Scene {id: $scene_id})
            CREATE (w)-[:CURRENT_SCENE]->(s)
            RETURN s.id as scene_id",
        )
        .param("world_id", world_id.to_string())
        .param("scene_id", scene_id.to_string());

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
                "set_current failed: World {} or Scene {} not found",
                world_id,
                scene_id
            );
            return Err(RepoError::NotFound);
        }

        tracing::debug!("Set current scene {} for world {}", scene_id, world_id);
        Ok(())
    }

    async fn list_for_region(&self, region_id: RegionId) -> Result<Vec<Scene>, RepoError> {
        // Scenes associated with a region via IN_REGION edge or via location
        let q = query(
            "MATCH (s:Scene)-[:IN_REGION|AT_LOCATION*1..2]->(r:Region {id: $region_id})
            RETURN s
            ORDER BY s.order_num",
        )
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut scenes = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            scenes.push(self.row_to_scene(row)?);
        }

        Ok(scenes)
    }

    async fn get_featured_characters(&self, scene_id: SceneId) -> Result<Vec<CharacterId>, RepoError> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[:FEATURES_CHARACTER]->(c:Character)
            RETURN c.id as character_id",
        )
        .param("scene_id", scene_id.to_string());

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
            let char_id_str: String = row
                .get("character_id")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let char_id = CharacterId::from(
                uuid::Uuid::parse_str(&char_id_str)
                    .map_err(|e| RepoError::Database(e.to_string()))?,
            );
            characters.push(char_id);
        }

        Ok(characters)
    }

    async fn set_featured_characters(
        &self,
        scene_id: SceneId,
        characters: &[CharacterId],
    ) -> Result<(), RepoError> {
        // Delete existing edges and create new ones atomically using UNWIND
        let char_ids: Vec<String> = characters.iter().map(|id| id.to_string()).collect();

        let q = query(
            "MATCH (s:Scene {id: $scene_id})
            OPTIONAL MATCH (s)-[old:FEATURES_CHARACTER]->()
            DELETE old
            WITH s
            UNWIND CASE WHEN $char_ids = [] THEN [null] ELSE $char_ids END AS char_id
            WITH s, char_id WHERE char_id IS NOT NULL
            MATCH (c:Character {id: char_id})
            CREATE (s)-[:FEATURES_CHARACTER {role: 'Secondary', entrance_cue: ''}]->(c)
            RETURN count(*) as created",
        )
        .param("scene_id", scene_id.to_string())
        .param("char_ids", char_ids);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!(
            "Set {} featured characters for scene {}",
            characters.len(),
            scene_id
        );
        Ok(())
    }

    async fn has_completed_scene(
        &self,
        pc_id: PlayerCharacterId,
        scene_id: SceneId,
    ) -> Result<bool, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:COMPLETED_SCENE]->(s:Scene {id: $scene_id})
            RETURN count(s) > 0 as completed",
        )
        .param("pc_id", pc_id.to_string())
        .param("scene_id", scene_id.to_string());

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
            let completed: bool = row
                .get("completed")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            Ok(completed)
        } else {
            Ok(false)
        }
    }

    async fn mark_scene_completed(
        &self,
        pc_id: PlayerCharacterId,
        scene_id: SceneId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
            MATCH (s:Scene {id: $scene_id})
            MERGE (pc)-[r:COMPLETED_SCENE]->(s)
            ON CREATE SET r.completed_at = datetime()
            RETURN pc.id as pc_id",
        )
        .param("pc_id", pc_id.to_string())
        .param("scene_id", scene_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Marked scene {} as completed for PC {}", scene_id, pc_id);
        Ok(())
    }

    async fn get_completed_scenes(&self, pc_id: PlayerCharacterId) -> Result<Vec<SceneId>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:COMPLETED_SCENE]->(s:Scene)
            RETURN s.id as scene_id",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut scenes = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let scene_id_str: String = row
                .get("scene_id")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let scene_id = SceneId::from(
                uuid::Uuid::parse_str(&scene_id_str)
                    .map_err(|e| RepoError::Database(e.to_string()))?,
            );
            scenes.push(scene_id);
        }

        Ok(scenes)
    }
}
