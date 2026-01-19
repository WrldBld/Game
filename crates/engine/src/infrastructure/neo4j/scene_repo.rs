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

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
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
            SceneConditionStored::CompletedScene(id) => {
                SceneCondition::CompletedScene(SceneId::from(
                    uuid::Uuid::parse_str(&id).map_err(|e| RepoError::database("query", e))?,
                ))
            }
            SceneConditionStored::HasItem(id) => SceneCondition::HasItem(ItemId::from(
                uuid::Uuid::parse_str(&id).map_err(|e| RepoError::database("query", e))?,
            )),
            SceneConditionStored::KnowsCharacter(id) => {
                SceneCondition::KnowsCharacter(CharacterId::from(
                    uuid::Uuid::parse_str(&id).map_err(|e| RepoError::database("query", e))?,
                ))
            }
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
    graph: Neo4jGraph,
}

impl Neo4jSceneRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    /// Convert a Neo4j row to a Scene entity.
    ///
    /// Note: location_id and featured_characters are now managed via graph edges,
    /// not stored on the node. Use `get_location()` and `get_featured_characters()`
    /// to retrieve these relationships.
    fn row_to_scene(&self, row: Row) -> Result<Scene, RepoError> {
        let node: neo4rs::Node = row.get("s").map_err(|e| RepoError::database("query", e))?;

        let id: SceneId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let act_id: ActId =
            parse_typed_id(&node, "act_id").map_err(|e| RepoError::database("query", e))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::database("query", e))?;
        let name = SceneName::new(name).map_err(|e| RepoError::database("query", e.to_string()))?;
        let directorial_notes: String = node.get_string_or("directorial_notes", "");
        let order_num: i64 = node.get_i64_or("order_num", 0);

        // JSON fields
        let time_context: TimeContext = node
            .get_json::<TimeContextStored>("time_context")
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid time_context JSON for Scene {}: {}", id, e),
                )
            })?
            .into();

        let entry_conditions: Vec<SceneCondition> = node
            .get_json::<Vec<SceneConditionStored>>("entry_conditions")
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid entry_conditions JSON for Scene {}: {}", id, e),
                )
            })?
            .into_iter()
            .map(|sc| {
                sc.try_into_condition().map_err(|e| {
                    RepoError::database("parse", format!("Invalid entry condition: {:?}", e))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let backdrop_override = node.get_optional_string("backdrop_override");

        let notes = Description::new(directorial_notes)
            .map_err(|e| RepoError::database("query", e.to_string()))?;

        let mut scene = Scene::new(act_id, name)
            .with_id(id)
            .with_time(time_context)
            .with_directorial_notes(notes)
            .with_order(order_num as u32)
            .with_entry_conditions(entry_conditions);

        if let Some(backdrop) = backdrop_override {
            scene = scene.with_backdrop_override(backdrop);
        }

        Ok(scene)
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
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_scene(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, scene: &Scene) -> Result<(), RepoError> {
        let time_context_json =
            serde_json::to_string(&TimeContextStored::from(scene.time_context().clone()))
                .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let entry_conditions_json = serde_json::to_string(
            &scene
                .entry_conditions()
                .iter()
                .cloned()
                .map(SceneConditionStored::from)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| RepoError::Serialization(e.to_string()))?;

        // MERGE for upsert behavior
        // Note: location_id and featured_characters are now managed via graph edges,
        // not stored as node properties. Use set_location() and set_featured_characters()
        // to manage these relationships.
        let q = query(
            "MATCH (a:Act {id: $act_id})
            MERGE (s:Scene {id: $id})
            SET s.act_id = $act_id,
                s.name = $name,
                s.time_context = $time_context,
                s.backdrop_override = $backdrop_override,
                s.entry_conditions = $entry_conditions,
                s.directorial_notes = $directorial_notes,
                s.order_num = $order_num
            MERGE (a)-[:CONTAINS_SCENE]->(s)
            RETURN s.id as id",
        )
        .param("id", scene.id().to_string())
        .param("act_id", scene.act_id().to_string())
        .param("name", scene.name().to_string())
        .param("time_context", time_context_json)
        .param(
            "backdrop_override",
            scene
                .backdrop_override()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        )
        .param("entry_conditions", entry_conditions_json)
        .param("directorial_notes", scene.directorial_notes().to_string())
        .param("order_num", scene.order() as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved scene: {}", scene.name());
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
            .map_err(|e| RepoError::database("query", e))?;

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
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
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
            .map_err(|e| RepoError::database("query", e))?;

        // Verify the operation succeeded
        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            tracing::warn!(
                world_id = %world_id,
                scene_id = %scene_id,
                "set_current failed: World or Scene not found"
            );
            return Err(RepoError::not_found(
                "Scene",
                format!("world:{}/scene:{}", world_id, scene_id),
            ));
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
            .map_err(|e| RepoError::database("query", e))?;

        let mut scenes = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            scenes.push(self.row_to_scene(row)?);
        }

        Ok(scenes)
    }

    async fn list_for_act(&self, act_id: ActId) -> Result<Vec<Scene>, RepoError> {
        let q = query(
            "MATCH (a:Act {id: $act_id})-[:CONTAINS_SCENE]->(s:Scene)
            RETURN s
            ORDER BY s.order_num",
        )
        .param("act_id", act_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut scenes = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            scenes.push(self.row_to_scene(row)?);
        }

        Ok(scenes)
    }

    async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>, RepoError> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[:AT_LOCATION]->(l:Location)
            RETURN l.id as location_id",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let location_id_str: String = row
                .get("location_id")
                .map_err(|e| RepoError::database("query", e))?;
            let location_id = LocationId::from(
                uuid::Uuid::parse_str(&location_id_str)
                    .map_err(|e| RepoError::database("query", e))?,
            );
            Ok(Some(location_id))
        } else {
            Ok(None)
        }
    }

    async fn set_location(
        &self,
        scene_id: SceneId,
        location_id: LocationId,
    ) -> Result<(), RepoError> {
        // Delete existing AT_LOCATION edge and create new one
        let q = query(
            "MATCH (s:Scene {id: $scene_id})
            OPTIONAL MATCH (s)-[old:AT_LOCATION]->()
            DELETE old
            WITH s
            MATCH (l:Location {id: $location_id})
            CREATE (s)-[:AT_LOCATION]->(l)
            RETURN s.id as scene_id",
        )
        .param("scene_id", scene_id.to_string())
        .param("location_id", location_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Verify the operation succeeded
        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            tracing::warn!(
                scene_id = %scene_id,
                location_id = %location_id,
                "set_location failed: Scene or Location not found"
            );
            return Err(RepoError::not_found(
                "Scene",
                format!("scene:{}/location:{}", scene_id, location_id),
            ));
        }

        tracing::debug!("Set location {} for scene {}", location_id, scene_id);
        Ok(())
    }

    async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<SceneCharacter>, RepoError> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})-[r:FEATURES_CHARACTER]->(c:Character)
            RETURN c.id as character_id, r.role as role, r.entrance_cue as entrance_cue",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut characters = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let char_id_str: String = row
                .get("character_id")
                .map_err(|e| RepoError::database("query", e))?;
            let char_id = CharacterId::from(
                uuid::Uuid::parse_str(&char_id_str).map_err(|e| RepoError::database("query", e))?,
            );

            // Parse role from string - fail-fast on invalid values
            let role_str: String = row.get("role").map_err(|e| {
                RepoError::database(
                    "query",
                    format!("Missing role for FEATURES_CHARACTER relationship: {}", e),
                )
            })?;
            let role: SceneCharacterRole = match role_str.parse::<SceneCharacterRole>() {
                Ok(r) => r,
                Err(_) => {
                    return Err(RepoError::database(
                        "parse",
                        format!(
                            "Invalid SceneCharacterRole for Scene {}: '{}'",
                            scene_id, role_str
                        ),
                    ))
                }
            };

            // Parse entrance_cue, treating empty string as None
            let entrance_cue: Option<String> = row.get("entrance_cue").ok().and_then(
                |s: String| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                },
            );

            characters.push(SceneCharacter::from_parts(char_id, role, entrance_cue));
        }

        Ok(characters)
    }

    async fn set_featured_characters(
        &self,
        scene_id: SceneId,
        characters: &[SceneCharacter],
    ) -> Result<(), RepoError> {
        // Build parallel arrays for UNWIND
        let char_ids: Vec<String> = characters
            .iter()
            .map(|sc| sc.character_id.to_string())
            .collect();
        let roles: Vec<String> = characters.iter().map(|sc| sc.role.to_string()).collect();
        let entrance_cues: Vec<String> = characters
            .iter()
            .map(|sc| sc.entrance_cue.as_deref().unwrap_or_default().to_string())
            .collect();

        // Delete existing edges first
        let delete_q = query(
            "MATCH (s:Scene {id: $scene_id})-[old:FEATURES_CHARACTER]->()
            DELETE old",
        )
        .param("scene_id", scene_id.to_string());

        self.graph
            .run(delete_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Create new edges if there are any characters
        if !char_ids.is_empty() {
            let create_q = query(
                "MATCH (s:Scene {id: $scene_id})
                UNWIND range(0, size($char_ids) - 1) AS i
                MATCH (c:Character {id: $char_ids[i]})
                CREATE (s)-[:FEATURES_CHARACTER {role: $roles[i], entrance_cue: $entrance_cues[i]}]->(c)
                RETURN count(*) as created",
            )
            .param("scene_id", scene_id.to_string())
            .param("char_ids", char_ids)
            .param("roles", roles)
            .param("entrance_cues", entrance_cues);

            self.graph
                .run(create_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

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
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let completed: bool = row
                .get("completed")
                .map_err(|e| RepoError::database("query", e))?;
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
        // First, verify both nodes exist and atomically create/check the relationship
        let q = query(
            "OPTIONAL MATCH (pc:PlayerCharacter {id: $pc_id})
            OPTIONAL MATCH (s:Scene {id: $scene_id})
            WITH pc, s,
                 pc IS NOT NULL AS pc_exists,
                 s IS NOT NULL AS scene_exists
            WHERE pc_exists AND scene_exists
            MERGE (pc)-[r:COMPLETED_SCENE]->(s)
            ON CREATE SET r.completed_at = datetime()
            RETURN pc_exists, scene_exists,
                   CASE WHEN r.completed_at = datetime() THEN 'created' ELSE 'existed' END as status",
        )
        .param("pc_id", pc_id.to_string())
        .param("scene_id", scene_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Check if we got a result - if not, one or both nodes don't exist
        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let pc_exists: bool = row
                .get("pc_exists")
                .map_err(|e| RepoError::database("query", e))?;
            let scene_exists: bool = row
                .get("scene_exists")
                .map_err(|e| RepoError::database("query", e))?;

            if !pc_exists {
                tracing::warn!(
                    pc_id = %pc_id,
                    "mark_scene_completed failed: PlayerCharacter not found"
                );
                return Err(RepoError::not_found("PlayerCharacter", pc_id.to_string()));
            }
            if !scene_exists {
                tracing::warn!(scene_id = %scene_id, "mark_scene_completed failed: Scene not found");
                return Err(RepoError::not_found("Scene", scene_id.to_string()));
            }

            tracing::debug!("Marked scene {} as completed for PC {}", scene_id, pc_id);
            Ok(())
        } else {
            // No rows returned means one or both nodes don't exist
            // We need to check which one is missing for a better error message
            let check_q = query(
                "OPTIONAL MATCH (pc:PlayerCharacter {id: $pc_id})
                OPTIONAL MATCH (s:Scene {id: $scene_id})
                RETURN pc IS NOT NULL AS pc_exists, s IS NOT NULL AS scene_exists",
            )
            .param("pc_id", pc_id.to_string())
            .param("scene_id", scene_id.to_string());

            let mut check_result = self
                .graph
                .execute(check_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;

            if let Some(check_row) = check_result
                .next()
                .await
                .map_err(|e| RepoError::database("query", e))?
            {
                let pc_exists: bool = check_row.get("pc_exists").unwrap_or(false);
                let scene_exists: bool = check_row.get("scene_exists").unwrap_or(false);

                if !pc_exists {
                    tracing::warn!(
                        pc_id = %pc_id,
                        "mark_scene_completed failed: PlayerCharacter not found"
                    );
                    return Err(RepoError::not_found("PlayerCharacter", pc_id.to_string()));
                } else if !scene_exists {
                    tracing::warn!(scene_id = %scene_id, "mark_scene_completed failed: Scene not found");
                    return Err(RepoError::not_found("Scene", scene_id.to_string()));
                }
            }

            Err(RepoError::not_found(
                "SceneCompletion",
                format!("pc:{}/scene:{}", pc_id, scene_id),
            ))
        }
    }

    async fn get_completed_scenes(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<SceneId>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:COMPLETED_SCENE]->(s:Scene)
            RETURN s.id as scene_id",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut scenes = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let scene_id_str: String = row
                .get("scene_id")
                .map_err(|e| RepoError::database("query", e))?;
            let scene_id = SceneId::from(
                uuid::Uuid::parse_str(&scene_id_str)
                    .map_err(|e| RepoError::database("query", e))?,
            );
            scenes.push(scene_id);
        }

        Ok(scenes)
    }
}
