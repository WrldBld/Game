//! CRUD implementation for NarrativeEventCrudPort

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::common::row_to_narrative_event;
use super::stored_types::{StoredEventOutcome, StoredNarrativeTrigger};
use super::Neo4jNarrativeEventRepository;
use wrldbldr_domain::entities::NarrativeEvent;
use wrldbldr_domain::{NarrativeEventId, WorldId};
use wrldbldr_engine_ports::outbound::NarrativeEventCrudPort;

#[async_trait]
impl NarrativeEventCrudPort for Neo4jNarrativeEventRepository {
    /// Create a new narrative event
    ///
    /// NOTE: This only creates the node. Scene/location/act associations and featured NPCs
    /// are now stored as graph edges and must be created separately using the edge methods:
    /// - `tie_to_scene()` for TIED_TO_SCENE edge
    /// - `tie_to_location()` for TIED_TO_LOCATION edge
    /// - `assign_to_act()` for BELONGS_TO_ACT edge
    /// - `add_featured_npc()` for FEATURES_NPC edges
    /// - Chain membership is managed via EventChainRepositoryPort
    async fn create(&self, event: &NarrativeEvent) -> Result<()> {
        let stored_triggers: Vec<StoredNarrativeTrigger> =
            event.trigger_conditions.iter().map(|t| t.into()).collect();
        let triggers_json = serde_json::to_string(&stored_triggers)?;
        let stored_outcomes: Vec<StoredEventOutcome> =
            event.outcomes.iter().map(|o| o.into()).collect();
        let outcomes_json = serde_json::to_string(&stored_outcomes)?;
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (e:NarrativeEvent {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                tags_json: $tags_json,
                triggers_json: $triggers_json,
                trigger_logic: $trigger_logic,
                scene_direction: $scene_direction,
                suggested_opening: $suggested_opening,
                outcomes_json: $outcomes_json,
                default_outcome: $default_outcome,
                is_active: $is_active,
                is_triggered: $is_triggered,
                triggered_at: $triggered_at,
                selected_outcome: $selected_outcome,
                is_repeatable: $is_repeatable,
                trigger_count: $trigger_count,
                delay_turns: $delay_turns,
                expires_after_turns: $expires_after_turns,
                priority: $priority,
                is_favorite: $is_favorite,
                created_at: $created_at,
                updated_at: $updated_at
            })
            CREATE (w)-[:HAS_NARRATIVE_EVENT]->(e)
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("name", event.name.clone())
        .param("description", event.description.clone())
        .param("tags_json", tags_json)
        .param("triggers_json", triggers_json)
        .param("trigger_logic", format!("{:?}", event.trigger_logic))
        .param("scene_direction", event.scene_direction.clone())
        .param(
            "suggested_opening",
            event.suggested_opening.clone().unwrap_or_default(),
        )
        .param("outcomes_json", outcomes_json)
        .param(
            "default_outcome",
            event.default_outcome.clone().unwrap_or_default(),
        )
        .param("is_active", event.is_active)
        .param("is_triggered", event.is_triggered)
        .param(
            "triggered_at",
            event
                .triggered_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        )
        .param(
            "selected_outcome",
            event.selected_outcome.clone().unwrap_or_default(),
        )
        .param("is_repeatable", event.is_repeatable)
        .param("trigger_count", event.trigger_count as i64)
        .param("delay_turns", event.delay_turns as i64)
        .param(
            "expires_after_turns",
            event.expires_after_turns.map(|t| t as i64).unwrap_or(-1),
        )
        .param("priority", event.priority as i64)
        .param("is_favorite", event.is_favorite)
        .param("created_at", event.created_at.to_rfc3339())
        .param("updated_at", event.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created narrative event: {}", event.name);

        Ok(())
    }

    /// Get a narrative event by ID
    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            RETURN e",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_narrative_event(row, self.clock.now())?))
        } else {
            Ok(None)
        }
    }

    /// Update a narrative event
    ///
    /// NOTE: This only updates node properties. Scene/location/act associations and featured NPCs
    /// are now stored as graph edges and must be managed separately using the edge methods.
    async fn update(&self, event: &NarrativeEvent) -> Result<bool> {
        let stored_triggers: Vec<StoredNarrativeTrigger> =
            event.trigger_conditions.iter().map(|t| t.into()).collect();
        let triggers_json = serde_json::to_string(&stored_triggers)?;
        let stored_outcomes: Vec<StoredEventOutcome> =
            event.outcomes.iter().map(|o| o.into()).collect();
        let outcomes_json = serde_json::to_string(&stored_outcomes)?;
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.name = $name,
                e.description = $description,
                e.tags_json = $tags_json,
                e.triggers_json = $triggers_json,
                e.trigger_logic = $trigger_logic,
                e.scene_direction = $scene_direction,
                e.suggested_opening = $suggested_opening,
                e.outcomes_json = $outcomes_json,
                e.default_outcome = $default_outcome,
                e.is_active = $is_active,
                e.is_triggered = $is_triggered,
                e.triggered_at = $triggered_at,
                e.selected_outcome = $selected_outcome,
                e.is_repeatable = $is_repeatable,
                e.trigger_count = $trigger_count,
                e.delay_turns = $delay_turns,
                e.expires_after_turns = $expires_after_turns,
                e.priority = $priority,
                e.is_favorite = $is_favorite,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("name", event.name.clone())
        .param("description", event.description.clone())
        .param("tags_json", tags_json)
        .param("triggers_json", triggers_json)
        .param("trigger_logic", format!("{:?}", event.trigger_logic))
        .param("scene_direction", event.scene_direction.clone())
        .param(
            "suggested_opening",
            event.suggested_opening.clone().unwrap_or_default(),
        )
        .param("outcomes_json", outcomes_json)
        .param(
            "default_outcome",
            event.default_outcome.clone().unwrap_or_default(),
        )
        .param("is_active", event.is_active)
        .param("is_triggered", event.is_triggered)
        .param(
            "triggered_at",
            event
                .triggered_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        )
        .param(
            "selected_outcome",
            event.selected_outcome.clone().unwrap_or_default(),
        )
        .param("is_repeatable", event.is_repeatable)
        .param("trigger_count", event.trigger_count as i64)
        .param("delay_turns", event.delay_turns as i64)
        .param(
            "expires_after_turns",
            event.expires_after_turns.map(|t| t as i64).unwrap_or(-1),
        )
        .param("priority", event.priority as i64)
        .param("is_favorite", event.is_favorite)
        .param("updated_at", self.clock.now_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// List all narrative events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            RETURN e
            ORDER BY e.is_favorite DESC, e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    /// List active narrative events for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_active = true
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    /// List favorite narrative events for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_favorite = true
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    /// List untriggered active events (for LLM context)
    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            WHERE e.is_active = true AND e.is_triggered = false
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_favorite = NOT e.is_favorite,
                e.updated_at = $updated_at
            RETURN e.is_favorite as is_favorite",
        )
        .param("id", id.to_string())
        .param("updated_at", self.clock.now_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let is_favorite: bool = row.get("is_favorite")?;
            Ok(is_favorite)
        } else {
            Ok(false)
        }
    }

    /// Set active status
    async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_active = $is_active,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("is_active", is_active)
        .param("updated_at", self.clock.now_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Mark event as triggered
    async fn mark_triggered(
        &self,
        id: NarrativeEventId,
        outcome_name: Option<String>,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_triggered = true,
                e.triggered_at = $triggered_at,
                e.selected_outcome = $selected_outcome,
                e.trigger_count = e.trigger_count + 1,
                e.is_active = CASE WHEN e.is_repeatable THEN e.is_active ELSE false END,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("triggered_at", self.clock.now_rfc3339())
        .param("selected_outcome", outcome_name.unwrap_or_default())
        .param("updated_at", self.clock.now_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Reset triggered status (for repeatable events)
    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_triggered = false,
                e.triggered_at = null,
                e.selected_outcome = null,
                e.updated_at = $updated_at
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("updated_at", self.clock.now_rfc3339());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete a narrative event
    async fn delete(&self, id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            DETACH DELETE e
            RETURN count(*) as deleted",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }
}
