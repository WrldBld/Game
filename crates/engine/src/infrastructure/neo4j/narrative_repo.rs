//! Neo4j narrative repository implementation.
//!
//! Handles NarrativeEvents, EventChains, and StoryEvents.

use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use neo4rs::{query, Graph, Node, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wrldbldr_domain::*;

use super::helpers::{parse_optional_typed_id, parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, ConversationTurnRecord, NarrativeRepo, RepoError};

pub struct Neo4jNarrativeRepo {
    graph: Graph,
    clock: std::sync::Arc<dyn ClockPort>,
}

impl Neo4jNarrativeRepo {
    pub fn new(graph: Graph, clock: std::sync::Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }
}

#[async_trait]
impl NarrativeRepo for Neo4jNarrativeRepo {
    // =========================================================================
    // NarrativeEvent operations
    // =========================================================================

    async fn get_event(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>, RepoError> {
        let q = query("MATCH (e:NarrativeEvent {id: $id}) RETURN e").param("id", id.to_string());

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
            Ok(Some(row_to_narrative_event(row, self.clock.now())?))
        } else {
            Ok(None)
        }
    }

    async fn save_event(&self, event: &NarrativeEvent) -> Result<(), RepoError> {
        let stored_triggers: Vec<StoredNarrativeTrigger> =
            event.trigger_conditions.iter().map(|t| t.into()).collect();
        let triggers_json = serde_json::to_string(&stored_triggers)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let stored_outcomes: Vec<StoredEventOutcome> =
            event.outcomes.iter().map(|o| o.into()).collect();
        let outcomes_json = serde_json::to_string(&stored_outcomes)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(&event.tags)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MERGE (e:NarrativeEvent {id: $id})
            ON CREATE SET
                e.world_id = $world_id,
                e.name = $name,
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
                e.created_at = $created_at,
                e.updated_at = $updated_at
            ON MATCH SET
                e.name = $name,
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
            WITH e
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:HAS_NARRATIVE_EVENT]->(e)",
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

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn list_events_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<NarrativeEvent>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
            RETURN e
            ORDER BY e.is_favorite DESC, e.priority DESC, e.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut events = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            events.push(row_to_narrative_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    async fn delete_event(&self, id: NarrativeEventId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            DETACH DELETE e",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted narrative event: {}", id);
        Ok(())
    }

    // =========================================================================
    // EventChain operations
    // =========================================================================

    async fn get_chain(&self, id: EventChainId) -> Result<Option<EventChain>, RepoError> {
        let q = query("MATCH (c:EventChain {id: $id}) RETURN c").param("id", id.to_string());

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
            Ok(Some(row_to_event_chain(row, self.clock.now())?))
        } else {
            Ok(None)
        }
    }

    async fn save_chain(&self, chain: &EventChain) -> Result<(), RepoError> {
        let events_json: Vec<String> = chain.events.iter().map(|id| id.to_string()).collect();
        let completed_json: Vec<String> = chain
            .completed_events
            .iter()
            .map(|id| id.to_string())
            .collect();
        let tags_json = serde_json::to_string(&chain.tags)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MERGE (c:EventChain {id: $id})
            ON CREATE SET
                c.world_id = $world_id,
                c.name = $name,
                c.description = $description,
                c.events = $events,
                c.is_active = $is_active,
                c.current_position = $current_position,
                c.completed_events = $completed_events,
                c.act_id = $act_id,
                c.tags_json = $tags_json,
                c.color = $color,
                c.is_favorite = $is_favorite,
                c.created_at = $created_at,
                c.updated_at = $updated_at
            ON MATCH SET
                c.name = $name,
                c.description = $description,
                c.events = $events,
                c.is_active = $is_active,
                c.current_position = $current_position,
                c.completed_events = $completed_events,
                c.act_id = $act_id,
                c.tags_json = $tags_json,
                c.color = $color,
                c.is_favorite = $is_favorite,
                c.updated_at = $updated_at
            WITH c
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:HAS_EVENT_CHAIN]->(c)",
        )
        .param("id", chain.id.to_string())
        .param("world_id", chain.world_id.to_string())
        .param("name", chain.name.clone())
        .param("description", chain.description.clone())
        .param("events", events_json)
        .param("is_active", chain.is_active)
        .param("current_position", chain.current_position as i64)
        .param("completed_events", completed_json)
        .param(
            "act_id",
            chain.act_id.map(|a| a.to_string()).unwrap_or_default(),
        )
        .param("tags_json", tags_json)
        .param("color", chain.color.clone().unwrap_or_default())
        .param("is_favorite", chain.is_favorite)
        .param("created_at", chain.created_at.to_rfc3339())
        .param("updated_at", chain.updated_at.to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn delete_chain(&self, id: EventChainId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:EventChain {id: $id})
            DETACH DELETE c",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted event chain: {}", id);
        Ok(())
    }

    async fn list_chains_for_world(&self, world_id: WorldId) -> Result<Vec<EventChain>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_EVENT_CHAIN]->(c:EventChain)
            RETURN c
            ORDER BY c.is_favorite DESC, c.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut chains = Vec::new();
        let now = self.clock.now();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            chains.push(row_to_event_chain(row, now)?);
        }

        Ok(chains)
    }

    // =========================================================================
    // StoryEvent operations
    // =========================================================================

    async fn get_story_event(&self, id: StoryEventId) -> Result<Option<StoryEvent>, RepoError> {
        let q = query("MATCH (e:StoryEvent {id: $id}) RETURN e").param("id", id.to_string());

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
            Ok(Some(row_to_story_event(row, self.clock.now())?))
        } else {
            Ok(None)
        }
    }

    async fn save_story_event(&self, event: &StoryEvent) -> Result<(), RepoError> {
        let stored_event_type: StoredStoryEventType = (&event.event_type).into();
        let event_type_json = serde_json::to_string(&stored_event_type)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(&event.tags)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MERGE (e:StoryEvent {id: $id})
            ON CREATE SET
                e.world_id = $world_id,
                e.event_type_json = $event_type_json,
                e.timestamp = $timestamp,
                e.game_time = $game_time,
                e.summary = $summary,
                e.is_hidden = $is_hidden,
                e.tags_json = $tags_json
            ON MATCH SET
                e.event_type_json = $event_type_json,
                e.timestamp = $timestamp,
                e.game_time = $game_time,
                e.summary = $summary,
                e.is_hidden = $is_hidden,
                e.tags_json = $tags_json
            WITH e
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:HAS_STORY_EVENT]->(e)",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("event_type_json", event_type_json)
        .param("timestamp", event.timestamp.to_rfc3339())
        .param("game_time", event.game_time.clone().unwrap_or_default())
        .param("summary", event.summary.clone())
        .param("is_hidden", event.is_hidden)
        .param("tags_json", tags_json);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn delete_story_event(&self, id: StoryEventId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            DETACH DELETE e",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted story event: {}", id);
        Ok(())
    }

    async fn list_story_events(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<StoryEvent>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE e.is_hidden = false
            RETURN e
            ORDER BY e.timestamp DESC
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("limit", limit as i64);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut events = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            events.push(row_to_story_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    // =========================================================================
    // Trigger queries
    // =========================================================================

    async fn get_triggers_for_region(
        &self,
        world_id: WorldId,
        region_id: RegionId,
    ) -> Result<Vec<NarrativeEvent>, RepoError> {
        const FALLBACK_LIMIT: i64 = 500;

        // First, try to find events with a direct TIED_TO_LOCATION edge (preferred path)
        // This uses an indexed relationship lookup which is fast.
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)-[:TIED_TO_LOCATION]->(r:Region {id: $region_id})
            WHERE e.is_active = true
              AND e.is_triggered = false
            RETURN e
            ORDER BY e.priority DESC",
        )
        .param("world_id", world_id.to_string())
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut events = Vec::new();
        let now = self.clock.now();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            events.push(row_to_narrative_event(row, now)?);
        }

        // If we found events via edge, return them
        if !events.is_empty() {
            return Ok(events);
        }

        // Fallback: check trigger_conditions JSON (slower - requires fetching all events)
        // This handles legacy events that don't have TIED_TO_LOCATION edges
        // TODO: Migration to add TIED_TO_LOCATION edges for all location-based triggers
        let q_fallback = query(
            "MATCH (w:World {id: $world_id})-[:HAS_NARRATIVE_EVENT]->(e:NarrativeEvent)
                        WHERE e.is_active = true
                            AND e.is_triggered = false
                        RETURN e
                        ORDER BY e.priority DESC
                        LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("limit", FALLBACK_LIMIT);

        let mut result = self
            .graph
            .execute(q_fallback)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let region_id_str = region_id.to_string();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let event = row_to_narrative_event(row, now)?;
            // Filter events that have a trigger condition for this region/location
            let has_region_trigger =
                event
                    .trigger_conditions
                    .iter()
                    .any(|t| match &t.trigger_type {
                        NarrativeTriggerType::PlayerEntersLocation { location_id, .. } => {
                            location_id.to_string() == region_id_str
                        }
                        NarrativeTriggerType::TimeAtLocation { location_id, .. } => {
                            location_id.to_string() == region_id_str
                        }
                        _ => false,
                    });
            if has_region_trigger {
                events.push(event);
            }
        }

        Ok(events)
    }

    // =========================================================================
    // Dialogue history
    // =========================================================================

    async fn get_dialogues_with_npc(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<StoryEvent>, RepoError> {
        // Find StoryEvents of type DialogueExchange that involve both the PC and NPC
        // We check the event_type_json for npc_id match
        let q = query(
            "MATCH (e:StoryEvent)
            WHERE e.event_type_json CONTAINS $npc_id_str
              AND e.event_type_json CONTAINS 'DialogueExchange'
            WITH e
            MATCH (pc:PlayerCharacter {id: $pc_id})-[:INVOLVED_IN]->(e)
            RETURN e
            ORDER BY e.timestamp DESC
            LIMIT $limit",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id_str", npc_id.to_string())
        .param("limit", limit as i64);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut events = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            events.push(row_to_story_event(row, self.clock.now())?);
        }

        Ok(events)
    }

    async fn update_spoke_to(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        timestamp: DateTime<Utc>,
        last_topic: Option<String>,
    ) -> Result<(), RepoError> {
        // Create or update SPOKE_TO relationship between PC and NPC
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
            MATCH (npc:Character {id: $npc_id})
            MERGE (pc)-[r:SPOKE_TO]->(npc)
            ON CREATE SET
                r.first_dialogue_at = $timestamp,
                r.last_dialogue_at = $timestamp,
                r.last_topic = $last_topic,
                r.conversation_count = 1
            ON MATCH SET
                r.last_dialogue_at = $timestamp,
                r.last_topic = COALESCE($last_topic, r.last_topic),
                r.conversation_count = COALESCE(r.conversation_count, 0) + 1",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("timestamp", timestamp.to_rfc3339())
        .param("last_topic", last_topic.unwrap_or_default());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn record_dialogue_context(
        &self,
        world_id: WorldId,
        story_event_id: StoryEventId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        player_dialogue: String,
        npc_dialogue: String,
        topics: Vec<String>,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
        game_time: Option<GameTime>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), RepoError> {
        let scene_id_str = scene_id.map(|id| id.to_string()).unwrap_or_default();
        let location_id_str = location_id.map(|id| id.to_string()).unwrap_or_default();
        let region_id_str = region_id.map(|id| id.to_string()).unwrap_or_default();
        let topic_hint = topics.first().cloned().unwrap_or_default();

        let mut conversation_id = None;

        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:PARTICIPATED_IN]->(c:Conversation {is_active: true})<-[:PARTICIPATED_IN]-(npc:Character {id: $npc_id})
            OPTIONAL MATCH (c)-[:IN_SCENE]->(scene:Scene)
            OPTIONAL MATCH (c)-[:AT_LOCATION]->(location:Location)
            OPTIONAL MATCH (c)-[:AT_REGION]->(region:Region)
            WHERE ($scene_id = '' OR scene.id = $scene_id)
              AND ($location_id = '' OR location.id = $location_id)
              AND ($region_id = '' OR region.id = $region_id)
            RETURN c.id as id
            ORDER BY c.last_updated_at DESC
            LIMIT 1",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("scene_id", scene_id_str.clone())
        .param("location_id", location_id_str.clone())
        .param("region_id", region_id_str.clone());

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
            let id: String = row.get("id").map_err(|e| RepoError::Database(e.to_string()))?;
            conversation_id = Some(id);
        }

        let conversation_id = if let Some(existing_id) = conversation_id {
            let q = query(
                "MATCH (c:Conversation {id: $id})
                SET c.last_updated_at = $timestamp,
                    c.topic_hint = CASE
                        WHEN c.topic_hint IS NULL OR c.topic_hint = '' THEN $topic_hint
                        ELSE c.topic_hint
                    END",
            )
            .param("id", existing_id.clone())
            .param("timestamp", timestamp.to_rfc3339())
            .param("topic_hint", topic_hint.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;

            existing_id
        } else {
            let new_id = Uuid::new_v4().to_string();
            let q = query(
                "MATCH (w:World {id: $world_id})
                MATCH (pc:PlayerCharacter {id: $pc_id})
                MATCH (npc:Character {id: $npc_id})
                CREATE (c:Conversation {
                    id: $conversation_id,
                    world_id: $world_id,
                    started_at: $timestamp,
                    ended_at: '',
                    topic_hint: $topic_hint,
                    is_active: true,
                    last_updated_at: $timestamp
                })
                MERGE (w)-[:HAS_CONVERSATION]->(c)
                MERGE (pc)-[:PARTICIPATED_IN]->(c)
                MERGE (npc)-[:PARTICIPATED_IN]->(c)",
            )
            .param("world_id", world_id.to_string())
            .param("pc_id", pc_id.to_string())
            .param("npc_id", npc_id.to_string())
            .param("conversation_id", new_id.clone())
            .param("timestamp", timestamp.to_rfc3339())
            .param("topic_hint", topic_hint.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;

            new_id
        };

        if !scene_id_str.is_empty() {
            let q = query(
                "MATCH (c:Conversation {id: $conversation_id})
                MATCH (scene:Scene {id: $scene_id})
                MERGE (c)-[:IN_SCENE]->(scene)",
            )
            .param("conversation_id", conversation_id.clone())
            .param("scene_id", scene_id_str.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        if !location_id_str.is_empty() {
            let q = query(
                "MATCH (c:Conversation {id: $conversation_id})
                MATCH (location:Location {id: $location_id})
                MERGE (c)-[:AT_LOCATION]->(location)",
            )
            .param("conversation_id", conversation_id.clone())
            .param("location_id", location_id_str.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        if !region_id_str.is_empty() {
            let q = query(
                "MATCH (c:Conversation {id: $conversation_id})
                MATCH (region:Region {id: $region_id})
                MERGE (c)-[:AT_REGION]->(region)",
            )
            .param("conversation_id", conversation_id.clone())
            .param("region_id", region_id_str.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        let mut time_node_id = None;
        let mut turn_game_time = timestamp.to_rfc3339();

        if let Some(gt) = game_time {
            let time_id = Uuid::new_v4().to_string();
            let current = gt.current();
            let day = gt.day_ordinal() as i64;
            let hour = current.hour() as i64;
            let minute = current.minute() as i64;
            let period = gt.time_of_day().display_name().to_string();
            let label = format!("Day {}, {} ({:02}:{:02})", day, period, hour, minute);
            turn_game_time = current.to_rfc3339();

            let q = query(
                "MERGE (t:GameTime {world_id: $world_id, day: $day, hour: $hour, minute: $minute})
                ON CREATE SET
                    t.id = $id,
                    t.period = $period,
                    t.label = $label
                ON MATCH SET
                    t.period = $period,
                    t.label = $label
                RETURN t.id as id",
            )
            .param("world_id", world_id.to_string())
            .param("day", day)
            .param("hour", hour)
            .param("minute", minute)
            .param("id", time_id.clone())
            .param("period", period)
            .param("label", label);

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
                let id: String =
                    row.get("id").map_err(|e| RepoError::Database(e.to_string()))?;
                time_node_id = Some(id);
            }
        }

        if let Some(ref time_id) = time_node_id {
            let q = query(
                "MATCH (c:Conversation {id: $conversation_id})
                MATCH (t:GameTime {id: $time_id})
                MERGE (c)-[:OCCURRED_AT]->(t)",
            )
            .param("conversation_id", conversation_id.clone())
            .param("time_id", time_id.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        let last_order = {
            let q = query(
                "MATCH (c:Conversation {id: $conversation_id})-[:HAS_TURN]->(t:DialogueTurn)
                RETURN COALESCE(max(t.order), 0) as last_order",
            )
            .param("conversation_id", conversation_id.clone());

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
                row.get("last_order")
                    .map_err(|e| RepoError::Database(e.to_string()))?
            } else {
                0
            }
        };

        let player_order = last_order + 1;
        let npc_order = last_order + 2;
        let player_turn_id = Uuid::new_v4().to_string();
        let npc_turn_id = Uuid::new_v4().to_string();

        let q = query(
            "MATCH (c:Conversation {id: $conversation_id})
            CREATE (player:DialogueTurn {
                id: $player_turn_id,
                conversation_id: $conversation_id,
                speaker_id: $pc_id,
                speaker_type: 'pc',
                text: $player_text,
                order: $player_order,
                is_dm_override: false,
                is_llm_generated: false,
                game_time: $game_time
            })
            CREATE (npc:DialogueTurn {
                id: $npc_turn_id,
                conversation_id: $conversation_id,
                speaker_id: $npc_id,
                speaker_type: 'npc',
                text: $npc_text,
                order: $npc_order,
                is_dm_override: false,
                is_llm_generated: true,
                game_time: $game_time
            })
            MERGE (c)-[:HAS_TURN {order: $player_order}]->(player)
            MERGE (c)-[:HAS_TURN {order: $npc_order}]->(npc)",
        )
        .param("conversation_id", conversation_id.clone())
        .param("player_turn_id", player_turn_id.clone())
        .param("npc_turn_id", npc_turn_id.clone())
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("player_text", player_dialogue)
        .param("npc_text", npc_dialogue)
        .param("player_order", player_order)
        .param("npc_order", npc_order)
        .param("game_time", turn_game_time);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if !scene_id_str.is_empty() {
            let q = query(
                "MATCH (scene:Scene {id: $scene_id})
                MATCH (player:DialogueTurn {id: $player_turn_id})
                MATCH (npc:DialogueTurn {id: $npc_turn_id})
                MERGE (player)-[:OCCURRED_IN_SCENE]->(scene)
                MERGE (npc)-[:OCCURRED_IN_SCENE]->(scene)",
            )
            .param("scene_id", scene_id_str.clone())
            .param("player_turn_id", player_turn_id.clone())
            .param("npc_turn_id", npc_turn_id.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        if let Some(ref time_id) = time_node_id {
            let q = query(
                "MATCH (t:GameTime {id: $time_id})
                MATCH (player:DialogueTurn {id: $player_turn_id})
                MATCH (npc:DialogueTurn {id: $npc_turn_id})
                MERGE (player)-[:OCCURRED_AT]->(t)
                MERGE (npc)-[:OCCURRED_AT]->(t)",
            )
            .param("time_id", time_id.clone())
            .param("player_turn_id", player_turn_id.clone())
            .param("npc_turn_id", npc_turn_id.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        let q = query(
            "MATCH (e:StoryEvent {id: $story_event_id})
            MATCH (c:Conversation {id: $conversation_id})
            MERGE (e)-[:PART_OF_CONVERSATION]->(c)",
        )
        .param("story_event_id", story_event_id.to_string())
        .param("conversation_id", conversation_id.clone());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if !scene_id_str.is_empty() {
            let q = query(
                "MATCH (e:StoryEvent {id: $story_event_id})
                MATCH (scene:Scene {id: $scene_id})
                MERGE (e)-[:OCCURRED_IN_SCENE]->(scene)",
            )
            .param("story_event_id", story_event_id.to_string())
            .param("scene_id", scene_id_str.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        if !location_id_str.is_empty() {
            let q = query(
                "MATCH (e:StoryEvent {id: $story_event_id})
                MATCH (location:Location {id: $location_id})
                MERGE (e)-[:OCCURRED_AT]->(location)",
            )
            .param("story_event_id", story_event_id.to_string())
            .param("location_id", location_id_str);

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        if let Some(ref time_id) = time_node_id {
            let q = query(
                "MATCH (e:StoryEvent {id: $story_event_id})
                MATCH (t:GameTime {id: $time_id})
                MERGE (e)-[:OCCURRED_AT]->(t)",
            )
            .param("story_event_id", story_event_id.to_string())
            .param("time_id", time_id.clone());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn set_event_active(&self, id: NarrativeEventId, active: bool) -> Result<(), RepoError> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $id})
            SET e.is_active = $active
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("active", active);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!(
            event_id = %id,
            active = active,
            "Set narrative event active status"
        );
        Ok(())
    }

    async fn get_completed_events(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<NarrativeEventId>, RepoError> {
        // Get all completed event IDs from event chains in this world
        let q = query(
            "MATCH (c:EventChain {world_id: $world_id})
            WHERE c.completed_events IS NOT NULL
            RETURN c.completed_events AS completed",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut completed_events = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let completed_strs: Vec<String> = row.get("completed").unwrap_or_default();
            for id_str in completed_strs {
                if let Ok(id) = id_str.parse::<uuid::Uuid>() {
                    completed_events.push(NarrativeEventId::from(id));
                }
            }
        }

        // Deduplicate (in case same event is in multiple chains)
        let mut seen = std::collections::HashSet::new();
        completed_events.retain(|id| seen.insert(*id));

        Ok(completed_events)
    }

    async fn get_conversation_turns(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<ConversationTurnRecord>, RepoError> {
        // Find the active conversation between PC and NPC, then get its turns
        // Join with PC/NPC nodes to get speaker names
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:PARTICIPATED_IN]->(c:Conversation {is_active: true})<-[:PARTICIPATED_IN]-(npc:Character {id: $npc_id})
            MATCH (c)-[:HAS_TURN]->(t:DialogueTurn)
            RETURN
                t.order AS order,
                t.text AS text,
                t.speaker_type AS speaker_type,
                CASE t.speaker_type
                    WHEN 'pc' THEN pc.name
                    WHEN 'npc' THEN npc.name
                    ELSE 'Unknown'
                END AS speaker_name
            ORDER BY t.order ASC
            LIMIT $limit",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("limit", limit as i64);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut turns = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let speaker: String = row
                .get("speaker_name")
                .unwrap_or_else(|_| "Unknown".to_string());
            let text: String = row.get("text").unwrap_or_default();
            let order: i64 = row.get("order").unwrap_or(0);

            turns.push(ConversationTurnRecord {
                speaker,
                text,
                order,
            });
        }

        Ok(turns)
    }

    async fn get_active_conversation_id(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<Uuid>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:PARTICIPATED_IN]->(c:Conversation {is_active: true})<-[:PARTICIPATED_IN]-(npc:Character {id: $npc_id})
            RETURN c.id AS conversation_id
            ORDER BY c.last_updated_at DESC
            LIMIT 1",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string());

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
            let id_str: String = row
                .get("conversation_id")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let uuid = Uuid::parse_str(&id_str).map_err(|e| RepoError::Database(e.to_string()))?;
            Ok(Some(uuid))
        } else {
            Ok(None)
        }
    }
}

// =============================================================================
// Row conversion helpers
// =============================================================================

fn row_to_narrative_event(row: Row, fallback: DateTime<Utc>) -> Result<NarrativeEvent, RepoError> {
    let node: Node = row
        .get("e")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id: NarrativeEventId =
        parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let name: String = node
        .get("name")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let description = node.get_string_or("description", "");
    let scene_direction = node.get_string_or("scene_direction", "");
    let suggested_opening = node.get_optional_string("suggested_opening");
    let default_outcome = node.get_optional_string("default_outcome");
    let selected_outcome = node.get_optional_string("selected_outcome");

    let tags: Vec<String> = node.get_json_or_default("tags_json");
    let stored_triggers: Vec<StoredNarrativeTrigger> = node.get_json_or_default("triggers_json");
    let stored_outcomes: Vec<StoredEventOutcome> = node.get_json_or_default("outcomes_json");
    let trigger_logic_str = node.get_string_or("trigger_logic", "All");

    let is_active = node.get_bool_or("is_active", true);
    let is_triggered = node.get_bool_or("is_triggered", false);
    let is_repeatable = node.get_bool_or("is_repeatable", false);
    let is_favorite = node.get_bool_or("is_favorite", false);

    let trigger_count = node.get_i64_or("trigger_count", 0) as u32;
    let delay_turns = node.get_i64_or("delay_turns", 0) as u32;
    let priority = node.get_i64_or("priority", 0) as i32;
    let expires_after_turns = node.get_positive_i64("expires_after_turns");

    let created_at = node.get_datetime_or("created_at", fallback);
    let updated_at = node.get_datetime_or("updated_at", fallback);

    let triggered_at: Option<DateTime<Utc>> = node
        .get_optional_string("triggered_at")
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let trigger_conditions: Vec<NarrativeTrigger> =
        stored_triggers.into_iter().map(|t| t.into()).collect();
    let outcomes: Vec<EventOutcome> = stored_outcomes.into_iter().map(|o| o.into()).collect();

    let trigger_logic = match trigger_logic_str.as_str() {
        "Any" => TriggerLogic::Any,
        s if s.starts_with("AtLeast(") => {
            let n: u32 = s
                .trim_start_matches("AtLeast(")
                .trim_end_matches(')')
                .parse()
                .unwrap_or(1);
            TriggerLogic::AtLeast(n)
        }
        _ => TriggerLogic::All,
    };

    Ok(NarrativeEvent {
        id,
        world_id,
        name,
        description,
        tags,
        trigger_conditions,
        trigger_logic,
        scene_direction,
        suggested_opening,
        outcomes,
        default_outcome,
        is_active,
        is_triggered,
        triggered_at,
        selected_outcome,
        is_repeatable,
        trigger_count,
        delay_turns,
        expires_after_turns,
        priority,
        is_favorite,
        created_at,
        updated_at,
    })
}

fn row_to_event_chain(row: Row, fallback: DateTime<Utc>) -> Result<EventChain, RepoError> {
    let node: Node = row
        .get("c")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id: EventChainId =
        parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let name: String = node
        .get("name")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let description: String = node.get_string_or("description", "");
    let events_strs: Vec<String> = node.get("events").unwrap_or_default();
    let is_active: bool = node.get_bool_or("is_active", true);
    let current_position: i64 = node.get_i64_or("current_position", 0);
    let completed_strs: Vec<String> = node.get("completed_events").unwrap_or_default();
    let act_id: Option<ActId> =
        parse_optional_typed_id(&node, "act_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let tags: Vec<String> = node.get_json_or_default("tags_json");
    let color: Option<String> = node.get_optional_string("color");
    let is_favorite: bool = node.get_bool_or("is_favorite", false);
    let created_at: DateTime<Utc> = node.get_datetime_or("created_at", fallback);
    let updated_at: DateTime<Utc> = node.get_datetime_or("updated_at", fallback);

    let events: Vec<NarrativeEventId> = events_strs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(NarrativeEventId::from))
        .collect();

    let completed_events: Vec<NarrativeEventId> = completed_strs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok().map(NarrativeEventId::from))
        .collect();

    Ok(EventChain {
        id,
        world_id,
        name,
        description,
        events,
        is_active,
        current_position: current_position as u32,
        completed_events,
        act_id,
        tags,
        color,
        is_favorite,
        created_at,
        updated_at,
    })
}

fn row_to_story_event(row: Row, fallback: DateTime<Utc>) -> Result<StoryEvent, RepoError> {
    let node: Node = row
        .get("e")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id: StoryEventId =
        parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let event_type_json: String = node
        .get("event_type_json")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let timestamp = node.get_datetime_or("timestamp", fallback);
    let game_time = node.get_optional_string("game_time");
    let summary: String = node
        .get("summary")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let is_hidden = node.get_bool_or("is_hidden", false);
    let tags: Vec<String> = node.get_json_or_default("tags_json");

    let stored_event_type: StoredStoryEventType = serde_json::from_str(&event_type_json)
        .map_err(|e| RepoError::Serialization(e.to_string()))?;
    let event_type: StoryEventType = stored_event_type.into();

    Ok(StoryEvent {
        id,
        world_id,
        event_type,
        timestamp,
        game_time,
        summary,
        is_hidden,
        tags,
    })
}

// =============================================================================
// Stored types for JSON serialization
// =============================================================================

/// Parse a UUID string, returning nil UUID on error
fn parse_uuid_or_nil(s: &str, field: &str) -> Uuid {
    match Uuid::parse_str(s) {
        Ok(uuid) => uuid,
        Err(e) => {
            tracing::warn!(field = %field, input = %s, error = %e, "Failed to parse UUID, using nil");
            Uuid::nil()
        }
    }
}

// ---------------------------------------------------------------------------
// NarrativeTrigger stored types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredNarrativeTrigger {
    trigger_type: StoredNarrativeTriggerType,
    description: String,
    is_required: bool,
    trigger_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredNarrativeTriggerType {
    NpcAction {
        npc_id: String,
        npc_name: String,
        action_keywords: Vec<String>,
        action_description: String,
    },
    PlayerEntersLocation {
        location_id: String,
        location_name: String,
    },
    TimeAtLocation {
        location_id: String,
        location_name: String,
        time_context: String,
    },
    DialogueTopic {
        keywords: Vec<String>,
        with_npc: Option<String>,
        npc_name: Option<String>,
    },
    ChallengeCompleted {
        challenge_id: String,
        challenge_name: String,
        requires_success: Option<bool>,
    },
    RelationshipThreshold {
        character_id: String,
        character_name: String,
        with_character: String,
        with_character_name: String,
        min_sentiment: Option<f32>,
        max_sentiment: Option<f32>,
    },
    HasItem {
        item_name: String,
        quantity: Option<u32>,
    },
    MissingItem {
        item_name: String,
    },
    EventCompleted {
        event_id: String,
        event_name: String,
        outcome_name: Option<String>,
    },
    TurnCount {
        turns: u32,
        since_event: Option<String>,
    },
    FlagSet {
        flag_name: String,
    },
    FlagNotSet {
        flag_name: String,
    },
    StatThreshold {
        character_id: String,
        stat_name: String,
        min_value: Option<i32>,
        max_value: Option<i32>,
    },
    CombatResult {
        victory: Option<bool>,
        involved_npc: Option<String>,
    },
    Custom {
        description: String,
        llm_evaluation: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredEventOutcome {
    name: String,
    label: String,
    description: String,
    condition: Option<StoredOutcomeCondition>,
    effects: Vec<StoredEventEffect>,
    chain_events: Vec<StoredChainedEvent>,
    timeline_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredOutcomeCondition {
    DmChoice,
    ChallengeResult {
        challenge_id: Option<String>,
        success_required: bool,
    },
    CombatResult {
        victory_required: bool,
    },
    DialogueChoice {
        keywords: Vec<String>,
    },
    PlayerAction {
        action_keywords: Vec<String>,
    },
    HasItem {
        item_name: String,
    },
    Custom {
        description: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredEventEffect {
    ModifyRelationship {
        from_character: String,
        from_name: String,
        to_character: String,
        to_name: String,
        sentiment_change: f32,
        reason: String,
    },
    GiveItem {
        item_name: String,
        item_description: Option<String>,
        quantity: u32,
    },
    TakeItem {
        item_name: String,
        quantity: u32,
    },
    RevealInformation {
        info_type: String,
        title: String,
        content: String,
        persist_to_journal: bool,
    },
    SetFlag {
        flag_name: String,
        value: bool,
    },
    EnableChallenge {
        challenge_id: String,
        challenge_name: String,
    },
    DisableChallenge {
        challenge_id: String,
        challenge_name: String,
    },
    EnableEvent {
        event_id: String,
        event_name: String,
    },
    DisableEvent {
        event_id: String,
        event_name: String,
    },
    TriggerScene {
        scene_id: String,
        scene_name: String,
    },
    StartCombat {
        participants: Vec<String>,
        participant_names: Vec<String>,
        combat_description: String,
    },
    ModifyStat {
        character_id: String,
        character_name: String,
        stat_name: String,
        modifier: i32,
    },
    AddReward {
        reward_type: String,
        amount: i32,
        description: String,
    },
    Custom {
        description: String,
        requires_dm_action: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredChainedEvent {
    event_id: String,
    event_name: String,
    delay_turns: u32,
    additional_trigger: Option<Box<StoredNarrativeTriggerType>>,
    chain_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// StoryEvent stored types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredStoryEventType {
    LocationChange {
        from_location: Option<String>,
        to_location: String,
        character_id: String,
        travel_method: Option<String>,
    },
    DialogueExchange {
        npc_id: String,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics_discussed: Vec<String>,
        tone: Option<String>,
    },
    CombatEvent {
        combat_type: StoredCombatEventType,
        participants: Vec<String>,
        enemies: Vec<String>,
        outcome: Option<StoredCombatOutcome>,
        location_id: String,
        rounds: Option<u32>,
    },
    ChallengeAttempted {
        challenge_id: Option<String>,
        challenge_name: String,
        character_id: String,
        skill_used: Option<String>,
        difficulty: Option<String>,
        roll_result: Option<i32>,
        modifier: Option<i32>,
        outcome: StoredChallengeEventOutcome,
    },
    ItemAcquired {
        item_name: String,
        item_description: Option<String>,
        character_id: String,
        source: StoredItemSource,
        quantity: u32,
    },
    ItemTransferred {
        item_name: String,
        from_character: Option<String>,
        to_character: String,
        quantity: u32,
        reason: Option<String>,
    },
    ItemUsed {
        item_name: String,
        character_id: String,
        target: Option<String>,
        effect: String,
        consumed: bool,
    },
    RelationshipChanged {
        from_character: String,
        to_character: String,
        previous_sentiment: Option<f32>,
        new_sentiment: f32,
        sentiment_change: f32,
        reason: String,
    },
    SceneTransition {
        from_scene: Option<String>,
        to_scene: String,
        from_scene_name: Option<String>,
        to_scene_name: String,
        trigger_reason: String,
    },
    InformationRevealed {
        info_type: StoredInfoType,
        title: String,
        content: String,
        source: Option<String>,
        importance: StoredInfoImportance,
        persist_to_journal: bool,
    },
    NpcAction {
        npc_id: String,
        npc_name: String,
        action_type: String,
        description: String,
        dm_approved: bool,
        dm_modified: bool,
    },
    DmMarker {
        title: String,
        note: String,
        importance: StoredMarkerImportance,
        marker_type: StoredDmMarkerType,
    },
    NarrativeEventTriggered {
        narrative_event_id: String,
        narrative_event_name: String,
        outcome_branch: Option<String>,
        effects_applied: Vec<String>,
    },
    StatModified {
        character_id: String,
        stat_name: String,
        previous_value: i32,
        new_value: i32,
        reason: String,
    },
    FlagChanged {
        flag_name: String,
        new_value: bool,
        reason: String,
    },
    SessionStarted {
        session_number: u32,
        session_name: Option<String>,
        players_present: Vec<String>,
    },
    SessionEnded {
        duration_minutes: u32,
        summary: String,
    },
    Custom {
        event_subtype: String,
        title: String,
        description: String,
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredCombatEventType {
    Started,
    RoundCompleted,
    CharacterDefeated,
    CharacterFled,
    Ended,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredCombatOutcome {
    Victory,
    Defeat,
    Fled,
    Negotiated,
    Draw,
    Interrupted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredChallengeEventOutcome {
    CriticalSuccess,
    Success,
    PartialSuccess,
    Failure,
    CriticalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type")]
enum StoredItemSource {
    Found { location: String },
    Purchased { from: String, cost: Option<String> },
    Gifted { from: String },
    Looted { from: String },
    Crafted,
    Reward { for_what: String },
    Stolen { from: String },
    Custom { description: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredInfoType {
    Lore,
    Quest,
    Character,
    Location,
    Item,
    Secret,
    Rumor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredInfoImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredMarkerImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StoredDmMarkerType {
    Note,
    PlotPoint,
    CharacterMoment,
    WorldEvent,
    PlayerDecision,
    Foreshadowing,
    Callback,
    Custom,
}

// =============================================================================
// Domain -> Stored conversions
// =============================================================================

impl From<&NarrativeTrigger> for StoredNarrativeTrigger {
    fn from(t: &NarrativeTrigger) -> Self {
        Self {
            trigger_type: StoredNarrativeTriggerType::from(&t.trigger_type),
            description: t.description.clone(),
            is_required: t.is_required,
            trigger_id: t.trigger_id.clone(),
        }
    }
}

impl From<&NarrativeTriggerType> for StoredNarrativeTriggerType {
    fn from(t: &NarrativeTriggerType) -> Self {
        match t {
            NarrativeTriggerType::NpcAction {
                npc_id,
                npc_name,
                action_keywords,
                action_description,
            } => StoredNarrativeTriggerType::NpcAction {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                action_keywords: action_keywords.clone(),
                action_description: action_description.clone(),
            },
            NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name,
            } => StoredNarrativeTriggerType::PlayerEntersLocation {
                location_id: location_id.to_string(),
                location_name: location_name.clone(),
            },
            NarrativeTriggerType::TimeAtLocation {
                location_id,
                location_name,
                time_context,
            } => StoredNarrativeTriggerType::TimeAtLocation {
                location_id: location_id.to_string(),
                location_name: location_name.clone(),
                time_context: time_context.clone(),
            },
            NarrativeTriggerType::DialogueTopic {
                keywords,
                with_npc,
                npc_name,
            } => StoredNarrativeTriggerType::DialogueTopic {
                keywords: keywords.clone(),
                with_npc: with_npc.as_ref().map(|id| id.to_string()),
                npc_name: npc_name.clone(),
            },
            NarrativeTriggerType::ChallengeCompleted {
                challenge_id,
                challenge_name,
                requires_success,
            } => StoredNarrativeTriggerType::ChallengeCompleted {
                challenge_id: challenge_id.to_string(),
                challenge_name: challenge_name.clone(),
                requires_success: *requires_success,
            },
            NarrativeTriggerType::RelationshipThreshold {
                character_id,
                character_name,
                with_character,
                with_character_name,
                min_sentiment,
                max_sentiment,
            } => StoredNarrativeTriggerType::RelationshipThreshold {
                character_id: character_id.to_string(),
                character_name: character_name.clone(),
                with_character: with_character.to_string(),
                with_character_name: with_character_name.clone(),
                min_sentiment: *min_sentiment,
                max_sentiment: *max_sentiment,
            },
            NarrativeTriggerType::HasItem {
                item_name,
                quantity,
            } => StoredNarrativeTriggerType::HasItem {
                item_name: item_name.clone(),
                quantity: *quantity,
            },
            NarrativeTriggerType::MissingItem { item_name } => {
                StoredNarrativeTriggerType::MissingItem {
                    item_name: item_name.clone(),
                }
            }
            NarrativeTriggerType::EventCompleted {
                event_id,
                event_name,
                outcome_name,
            } => StoredNarrativeTriggerType::EventCompleted {
                event_id: event_id.to_string(),
                event_name: event_name.clone(),
                outcome_name: outcome_name.clone(),
            },
            NarrativeTriggerType::TurnCount { turns, since_event } => {
                StoredNarrativeTriggerType::TurnCount {
                    turns: *turns,
                    since_event: since_event.as_ref().map(|id| id.to_string()),
                }
            }
            NarrativeTriggerType::FlagSet { flag_name } => StoredNarrativeTriggerType::FlagSet {
                flag_name: flag_name.clone(),
            },
            NarrativeTriggerType::FlagNotSet { flag_name } => {
                StoredNarrativeTriggerType::FlagNotSet {
                    flag_name: flag_name.clone(),
                }
            }
            NarrativeTriggerType::StatThreshold {
                character_id,
                stat_name,
                min_value,
                max_value,
            } => StoredNarrativeTriggerType::StatThreshold {
                character_id: character_id.to_string(),
                stat_name: stat_name.clone(),
                min_value: *min_value,
                max_value: *max_value,
            },
            NarrativeTriggerType::CombatResult {
                victory,
                involved_npc,
            } => StoredNarrativeTriggerType::CombatResult {
                victory: *victory,
                involved_npc: involved_npc.as_ref().map(|id| id.to_string()),
            },
            NarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            } => StoredNarrativeTriggerType::Custom {
                description: description.clone(),
                llm_evaluation: *llm_evaluation,
            },
        }
    }
}

impl From<&EventOutcome> for StoredEventOutcome {
    fn from(o: &EventOutcome) -> Self {
        Self {
            name: o.name.clone(),
            label: o.label.clone(),
            description: o.description.clone(),
            condition: o.condition.as_ref().map(StoredOutcomeCondition::from),
            effects: o.effects.iter().map(StoredEventEffect::from).collect(),
            chain_events: o
                .chain_events
                .iter()
                .map(StoredChainedEvent::from)
                .collect(),
            timeline_summary: o.timeline_summary.clone(),
        }
    }
}

impl From<&OutcomeCondition> for StoredOutcomeCondition {
    fn from(c: &OutcomeCondition) -> Self {
        match c {
            OutcomeCondition::DmChoice => StoredOutcomeCondition::DmChoice,
            OutcomeCondition::ChallengeResult {
                challenge_id,
                success_required,
            } => StoredOutcomeCondition::ChallengeResult {
                challenge_id: challenge_id.as_ref().map(|id| id.to_string()),
                success_required: *success_required,
            },
            OutcomeCondition::CombatResult { victory_required } => {
                StoredOutcomeCondition::CombatResult {
                    victory_required: *victory_required,
                }
            }
            OutcomeCondition::DialogueChoice { keywords } => {
                StoredOutcomeCondition::DialogueChoice {
                    keywords: keywords.clone(),
                }
            }
            OutcomeCondition::PlayerAction { action_keywords } => {
                StoredOutcomeCondition::PlayerAction {
                    action_keywords: action_keywords.clone(),
                }
            }
            OutcomeCondition::HasItem { item_name } => StoredOutcomeCondition::HasItem {
                item_name: item_name.clone(),
            },
            OutcomeCondition::Custom { description } => StoredOutcomeCondition::Custom {
                description: description.clone(),
            },
        }
    }
}

impl From<&EventEffect> for StoredEventEffect {
    fn from(e: &EventEffect) -> Self {
        match e {
            EventEffect::ModifyRelationship {
                from_character,
                from_name,
                to_character,
                to_name,
                sentiment_change,
                reason,
            } => StoredEventEffect::ModifyRelationship {
                from_character: from_character.to_string(),
                from_name: from_name.clone(),
                to_character: to_character.to_string(),
                to_name: to_name.clone(),
                sentiment_change: *sentiment_change,
                reason: reason.clone(),
            },
            EventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            } => StoredEventEffect::GiveItem {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
                quantity: *quantity,
            },
            EventEffect::TakeItem {
                item_name,
                quantity,
            } => StoredEventEffect::TakeItem {
                item_name: item_name.clone(),
                quantity: *quantity,
            },
            EventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            } => StoredEventEffect::RevealInformation {
                info_type: info_type.clone(),
                title: title.clone(),
                content: content.clone(),
                persist_to_journal: *persist_to_journal,
            },
            EventEffect::SetFlag { flag_name, value } => StoredEventEffect::SetFlag {
                flag_name: flag_name.clone(),
                value: *value,
            },
            EventEffect::EnableChallenge {
                challenge_id,
                challenge_name,
            } => StoredEventEffect::EnableChallenge {
                challenge_id: challenge_id.to_string(),
                challenge_name: challenge_name.clone(),
            },
            EventEffect::DisableChallenge {
                challenge_id,
                challenge_name,
            } => StoredEventEffect::DisableChallenge {
                challenge_id: challenge_id.to_string(),
                challenge_name: challenge_name.clone(),
            },
            EventEffect::EnableEvent {
                event_id,
                event_name,
            } => StoredEventEffect::EnableEvent {
                event_id: event_id.to_string(),
                event_name: event_name.clone(),
            },
            EventEffect::DisableEvent {
                event_id,
                event_name,
            } => StoredEventEffect::DisableEvent {
                event_id: event_id.to_string(),
                event_name: event_name.clone(),
            },
            EventEffect::TriggerScene {
                scene_id,
                scene_name,
            } => StoredEventEffect::TriggerScene {
                scene_id: scene_id.to_string(),
                scene_name: scene_name.clone(),
            },
            EventEffect::StartCombat {
                participants,
                participant_names,
                combat_description,
            } => StoredEventEffect::StartCombat {
                participants: participants.iter().map(|id| id.to_string()).collect(),
                participant_names: participant_names.clone(),
                combat_description: combat_description.clone(),
            },
            EventEffect::ModifyStat {
                character_id,
                character_name,
                stat_name,
                modifier,
            } => StoredEventEffect::ModifyStat {
                character_id: character_id.to_string(),
                character_name: character_name.clone(),
                stat_name: stat_name.clone(),
                modifier: *modifier,
            },
            EventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => StoredEventEffect::AddReward {
                reward_type: reward_type.clone(),
                amount: *amount,
                description: description.clone(),
            },
            EventEffect::Custom {
                description,
                requires_dm_action,
            } => StoredEventEffect::Custom {
                description: description.clone(),
                requires_dm_action: *requires_dm_action,
            },
        }
    }
}

impl From<&ChainedEvent> for StoredChainedEvent {
    fn from(c: &ChainedEvent) -> Self {
        Self {
            event_id: c.event_id.to_string(),
            event_name: c.event_name.clone(),
            delay_turns: c.delay_turns,
            additional_trigger: c
                .additional_trigger
                .as_ref()
                .map(|t| Box::new(StoredNarrativeTriggerType::from(t))),
            chain_reason: c.chain_reason.clone(),
        }
    }
}

impl From<&StoryEventType> for StoredStoryEventType {
    fn from(e: &StoryEventType) -> Self {
        match e {
            StoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoredStoryEventType::LocationChange {
                from_location: from_location.map(|id| id.to_string()),
                to_location: to_location.to_string(),
                character_id: character_id.to_string(),
                travel_method: travel_method.clone(),
            },
            StoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => StoredStoryEventType::DialogueExchange {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                player_dialogue: player_dialogue.clone(),
                npc_response: npc_response.clone(),
                topics_discussed: topics_discussed.clone(),
                tone: tone.clone(),
            },
            StoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => StoredStoryEventType::CombatEvent {
                combat_type: (*combat_type).into(),
                participants: participants.iter().map(|id| id.to_string()).collect(),
                enemies: enemies.clone(),
                outcome: outcome.map(|o| o.into()),
                location_id: location_id.to_string(),
                rounds: *rounds,
            },
            StoryEventType::ChallengeAttempted {
                challenge_id,
                challenge_name,
                character_id,
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome,
            } => StoredStoryEventType::ChallengeAttempted {
                challenge_id: challenge_id.map(|id| id.to_string()),
                challenge_name: challenge_name.clone(),
                character_id: character_id.to_string(),
                skill_used: skill_used.clone(),
                difficulty: difficulty.clone(),
                roll_result: *roll_result,
                modifier: *modifier,
                outcome: (*outcome).into(),
            },
            StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoredStoryEventType::ItemAcquired {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
                character_id: character_id.to_string(),
                source: source.into(),
                quantity: *quantity,
            },
            StoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoredStoryEventType::ItemTransferred {
                item_name: item_name.clone(),
                from_character: from_character.map(|id| id.to_string()),
                to_character: to_character.to_string(),
                quantity: *quantity,
                reason: reason.clone(),
            },
            StoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => StoredStoryEventType::ItemUsed {
                item_name: item_name.clone(),
                character_id: character_id.to_string(),
                target: target.clone(),
                effect: effect.clone(),
                consumed: *consumed,
            },
            StoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => StoredStoryEventType::RelationshipChanged {
                from_character: from_character.to_string(),
                to_character: to_character.to_string(),
                previous_sentiment: *previous_sentiment,
                new_sentiment: *new_sentiment,
                sentiment_change: *sentiment_change,
                reason: reason.clone(),
            },
            StoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => StoredStoryEventType::SceneTransition {
                from_scene: from_scene.map(|id| id.to_string()),
                to_scene: to_scene.to_string(),
                from_scene_name: from_scene_name.clone(),
                to_scene_name: to_scene_name.clone(),
                trigger_reason: trigger_reason.clone(),
            },
            StoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => StoredStoryEventType::InformationRevealed {
                info_type: (*info_type).into(),
                title: title.clone(),
                content: content.clone(),
                source: source.map(|id| id.to_string()),
                importance: (*importance).into(),
                persist_to_journal: *persist_to_journal,
            },
            StoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoredStoryEventType::NpcAction {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                action_type: action_type.clone(),
                description: description.clone(),
                dm_approved: *dm_approved,
                dm_modified: *dm_modified,
            },
            StoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => StoredStoryEventType::DmMarker {
                title: title.clone(),
                note: note.clone(),
                importance: (*importance).into(),
                marker_type: (*marker_type).into(),
            },
            StoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoredStoryEventType::NarrativeEventTriggered {
                narrative_event_id: narrative_event_id.to_string(),
                narrative_event_name: narrative_event_name.clone(),
                outcome_branch: outcome_branch.clone(),
                effects_applied: effects_applied.clone(),
            },
            StoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => StoredStoryEventType::StatModified {
                character_id: character_id.to_string(),
                stat_name: stat_name.clone(),
                previous_value: *previous_value,
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => StoredStoryEventType::FlagChanged {
                flag_name: flag_name.clone(),
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoredStoryEventType::SessionStarted {
                session_number: *session_number,
                session_name: session_name.clone(),
                players_present: players_present.clone(),
            },
            StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoredStoryEventType::SessionEnded {
                duration_minutes: *duration_minutes,
                summary: summary.clone(),
            },
            StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoredStoryEventType::Custom {
                event_subtype: event_subtype.clone(),
                title: title.clone(),
                description: description.clone(),
                data: data.clone(),
            },
        }
    }
}

impl From<CombatEventType> for StoredCombatEventType {
    fn from(c: CombatEventType) -> Self {
        match c {
            CombatEventType::Started => StoredCombatEventType::Started,
            CombatEventType::RoundCompleted => StoredCombatEventType::RoundCompleted,
            CombatEventType::CharacterDefeated => StoredCombatEventType::CharacterDefeated,
            CombatEventType::CharacterFled => StoredCombatEventType::CharacterFled,
            CombatEventType::Ended => StoredCombatEventType::Ended,
        }
    }
}

impl From<CombatOutcome> for StoredCombatOutcome {
    fn from(c: CombatOutcome) -> Self {
        match c {
            CombatOutcome::Victory => StoredCombatOutcome::Victory,
            CombatOutcome::Defeat => StoredCombatOutcome::Defeat,
            CombatOutcome::Fled => StoredCombatOutcome::Fled,
            CombatOutcome::Negotiated => StoredCombatOutcome::Negotiated,
            CombatOutcome::Draw => StoredCombatOutcome::Draw,
            CombatOutcome::Interrupted => StoredCombatOutcome::Interrupted,
        }
    }
}

impl From<ChallengeEventOutcome> for StoredChallengeEventOutcome {
    fn from(c: ChallengeEventOutcome) -> Self {
        match c {
            ChallengeEventOutcome::CriticalSuccess => StoredChallengeEventOutcome::CriticalSuccess,
            ChallengeEventOutcome::Success => StoredChallengeEventOutcome::Success,
            ChallengeEventOutcome::PartialSuccess => StoredChallengeEventOutcome::PartialSuccess,
            ChallengeEventOutcome::Failure => StoredChallengeEventOutcome::Failure,
            ChallengeEventOutcome::CriticalFailure => StoredChallengeEventOutcome::CriticalFailure,
        }
    }
}

impl From<&ItemSource> for StoredItemSource {
    fn from(s: &ItemSource) -> Self {
        match s {
            ItemSource::Found { location } => StoredItemSource::Found {
                location: location.clone(),
            },
            ItemSource::Purchased { from, cost } => StoredItemSource::Purchased {
                from: from.clone(),
                cost: cost.clone(),
            },
            ItemSource::Gifted { from } => StoredItemSource::Gifted {
                from: from.to_string(),
            },
            ItemSource::Looted { from } => StoredItemSource::Looted { from: from.clone() },
            ItemSource::Crafted => StoredItemSource::Crafted,
            ItemSource::Reward { for_what } => StoredItemSource::Reward {
                for_what: for_what.clone(),
            },
            ItemSource::Stolen { from } => StoredItemSource::Stolen { from: from.clone() },
            ItemSource::Custom { description } => StoredItemSource::Custom {
                description: description.clone(),
            },
        }
    }
}

impl From<InfoType> for StoredInfoType {
    fn from(i: InfoType) -> Self {
        match i {
            InfoType::Lore => StoredInfoType::Lore,
            InfoType::Quest => StoredInfoType::Quest,
            InfoType::Character => StoredInfoType::Character,
            InfoType::Location => StoredInfoType::Location,
            InfoType::Item => StoredInfoType::Item,
            InfoType::Secret => StoredInfoType::Secret,
            InfoType::Rumor => StoredInfoType::Rumor,
        }
    }
}

impl From<StoryEventInfoImportance> for StoredInfoImportance {
    fn from(i: StoryEventInfoImportance) -> Self {
        match i {
            StoryEventInfoImportance::Minor => StoredInfoImportance::Minor,
            StoryEventInfoImportance::Notable => StoredInfoImportance::Notable,
            StoryEventInfoImportance::Major => StoredInfoImportance::Major,
            StoryEventInfoImportance::Critical => StoredInfoImportance::Critical,
        }
    }
}

impl From<MarkerImportance> for StoredMarkerImportance {
    fn from(m: MarkerImportance) -> Self {
        match m {
            MarkerImportance::Minor => StoredMarkerImportance::Minor,
            MarkerImportance::Notable => StoredMarkerImportance::Notable,
            MarkerImportance::Major => StoredMarkerImportance::Major,
            MarkerImportance::Critical => StoredMarkerImportance::Critical,
        }
    }
}

impl From<DmMarkerType> for StoredDmMarkerType {
    fn from(d: DmMarkerType) -> Self {
        match d {
            DmMarkerType::Note => StoredDmMarkerType::Note,
            DmMarkerType::PlotPoint => StoredDmMarkerType::PlotPoint,
            DmMarkerType::CharacterMoment => StoredDmMarkerType::CharacterMoment,
            DmMarkerType::WorldEvent => StoredDmMarkerType::WorldEvent,
            DmMarkerType::PlayerDecision => StoredDmMarkerType::PlayerDecision,
            DmMarkerType::Foreshadowing => StoredDmMarkerType::Foreshadowing,
            DmMarkerType::Callback => StoredDmMarkerType::Callback,
            DmMarkerType::Custom => StoredDmMarkerType::Custom,
        }
    }
}

// =============================================================================
// Stored -> Domain conversions
// =============================================================================

impl From<StoredNarrativeTrigger> for NarrativeTrigger {
    fn from(s: StoredNarrativeTrigger) -> Self {
        Self {
            trigger_type: NarrativeTriggerType::from(s.trigger_type),
            description: s.description,
            is_required: s.is_required,
            trigger_id: s.trigger_id,
        }
    }
}

impl From<StoredNarrativeTriggerType> for NarrativeTriggerType {
    fn from(s: StoredNarrativeTriggerType) -> Self {
        match s {
            StoredNarrativeTriggerType::NpcAction {
                npc_id,
                npc_name,
                action_keywords,
                action_description,
            } => NarrativeTriggerType::NpcAction {
                npc_id: CharacterId::from(parse_uuid_or_nil(&npc_id, "npc_id")),
                npc_name,
                action_keywords,
                action_description,
            },
            StoredNarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name,
            } => NarrativeTriggerType::PlayerEntersLocation {
                location_id: LocationId::from(parse_uuid_or_nil(&location_id, "location_id")),
                location_name,
            },
            StoredNarrativeTriggerType::TimeAtLocation {
                location_id,
                location_name,
                time_context,
            } => NarrativeTriggerType::TimeAtLocation {
                location_id: LocationId::from(parse_uuid_or_nil(&location_id, "location_id")),
                location_name,
                time_context,
            },
            StoredNarrativeTriggerType::DialogueTopic {
                keywords,
                with_npc,
                npc_name,
            } => NarrativeTriggerType::DialogueTopic {
                keywords,
                with_npc: with_npc.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                npc_name,
            },
            StoredNarrativeTriggerType::ChallengeCompleted {
                challenge_id,
                challenge_name,
                requires_success,
            } => NarrativeTriggerType::ChallengeCompleted {
                challenge_id: ChallengeId::from(parse_uuid_or_nil(&challenge_id, "challenge_id")),
                challenge_name,
                requires_success,
            },
            StoredNarrativeTriggerType::RelationshipThreshold {
                character_id,
                character_name,
                with_character,
                with_character_name,
                min_sentiment,
                max_sentiment,
            } => NarrativeTriggerType::RelationshipThreshold {
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                character_name,
                with_character: CharacterId::from(parse_uuid_or_nil(
                    &with_character,
                    "with_character",
                )),
                with_character_name,
                min_sentiment,
                max_sentiment,
            },
            StoredNarrativeTriggerType::HasItem {
                item_name,
                quantity,
            } => NarrativeTriggerType::HasItem {
                item_name,
                quantity,
            },
            StoredNarrativeTriggerType::MissingItem { item_name } => {
                NarrativeTriggerType::MissingItem { item_name }
            }
            StoredNarrativeTriggerType::EventCompleted {
                event_id,
                event_name,
                outcome_name,
            } => NarrativeTriggerType::EventCompleted {
                event_id: NarrativeEventId::from(parse_uuid_or_nil(&event_id, "event_id")),
                event_name,
                outcome_name,
            },
            StoredNarrativeTriggerType::TurnCount { turns, since_event } => {
                NarrativeTriggerType::TurnCount {
                    turns,
                    since_event: since_event
                        .and_then(|id| Uuid::parse_str(&id).ok().map(NarrativeEventId::from)),
                }
            }
            StoredNarrativeTriggerType::FlagSet { flag_name } => {
                NarrativeTriggerType::FlagSet { flag_name }
            }
            StoredNarrativeTriggerType::FlagNotSet { flag_name } => {
                NarrativeTriggerType::FlagNotSet { flag_name }
            }
            StoredNarrativeTriggerType::StatThreshold {
                character_id,
                stat_name,
                min_value,
                max_value,
            } => NarrativeTriggerType::StatThreshold {
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                stat_name,
                min_value,
                max_value,
            },
            StoredNarrativeTriggerType::CombatResult {
                victory,
                involved_npc,
            } => NarrativeTriggerType::CombatResult {
                victory,
                involved_npc: involved_npc
                    .and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
            },
            StoredNarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            } => NarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            },
        }
    }
}

impl From<StoredEventOutcome> for EventOutcome {
    fn from(s: StoredEventOutcome) -> Self {
        Self {
            name: s.name,
            label: s.label,
            description: s.description,
            condition: s.condition.map(OutcomeCondition::from),
            effects: s.effects.into_iter().map(EventEffect::from).collect(),
            chain_events: s.chain_events.into_iter().map(ChainedEvent::from).collect(),
            timeline_summary: s.timeline_summary,
        }
    }
}

impl From<StoredOutcomeCondition> for OutcomeCondition {
    fn from(s: StoredOutcomeCondition) -> Self {
        match s {
            StoredOutcomeCondition::DmChoice => OutcomeCondition::DmChoice,
            StoredOutcomeCondition::ChallengeResult {
                challenge_id,
                success_required,
            } => OutcomeCondition::ChallengeResult {
                challenge_id: challenge_id
                    .and_then(|id| Uuid::parse_str(&id).ok().map(ChallengeId::from)),
                success_required,
            },
            StoredOutcomeCondition::CombatResult { victory_required } => {
                OutcomeCondition::CombatResult { victory_required }
            }
            StoredOutcomeCondition::DialogueChoice { keywords } => {
                OutcomeCondition::DialogueChoice { keywords }
            }
            StoredOutcomeCondition::PlayerAction { action_keywords } => {
                OutcomeCondition::PlayerAction { action_keywords }
            }
            StoredOutcomeCondition::HasItem { item_name } => {
                OutcomeCondition::HasItem { item_name }
            }
            StoredOutcomeCondition::Custom { description } => {
                OutcomeCondition::Custom { description }
            }
        }
    }
}

impl From<StoredEventEffect> for EventEffect {
    fn from(s: StoredEventEffect) -> Self {
        match s {
            StoredEventEffect::ModifyRelationship {
                from_character,
                from_name,
                to_character,
                to_name,
                sentiment_change,
                reason,
            } => EventEffect::ModifyRelationship {
                from_character: CharacterId::from(parse_uuid_or_nil(
                    &from_character,
                    "from_character",
                )),
                from_name,
                to_character: CharacterId::from(parse_uuid_or_nil(&to_character, "to_character")),
                to_name,
                sentiment_change,
                reason,
            },
            StoredEventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            } => EventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            },
            StoredEventEffect::TakeItem {
                item_name,
                quantity,
            } => EventEffect::TakeItem {
                item_name,
                quantity,
            },
            StoredEventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            } => EventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            },
            StoredEventEffect::SetFlag { flag_name, value } => {
                EventEffect::SetFlag { flag_name, value }
            }
            StoredEventEffect::EnableChallenge {
                challenge_id,
                challenge_name,
            } => EventEffect::EnableChallenge {
                challenge_id: ChallengeId::from(parse_uuid_or_nil(&challenge_id, "challenge_id")),
                challenge_name,
            },
            StoredEventEffect::DisableChallenge {
                challenge_id,
                challenge_name,
            } => EventEffect::DisableChallenge {
                challenge_id: ChallengeId::from(parse_uuid_or_nil(&challenge_id, "challenge_id")),
                challenge_name,
            },
            StoredEventEffect::EnableEvent {
                event_id,
                event_name,
            } => EventEffect::EnableEvent {
                event_id: NarrativeEventId::from(parse_uuid_or_nil(&event_id, "event_id")),
                event_name,
            },
            StoredEventEffect::DisableEvent {
                event_id,
                event_name,
            } => EventEffect::DisableEvent {
                event_id: NarrativeEventId::from(parse_uuid_or_nil(&event_id, "event_id")),
                event_name,
            },
            StoredEventEffect::TriggerScene {
                scene_id,
                scene_name,
            } => EventEffect::TriggerScene {
                scene_id: SceneId::from(parse_uuid_or_nil(&scene_id, "scene_id")),
                scene_name,
            },
            StoredEventEffect::StartCombat {
                participants,
                participant_names,
                combat_description,
            } => EventEffect::StartCombat {
                participants: participants
                    .into_iter()
                    .filter_map(|id| Uuid::parse_str(&id).ok().map(CharacterId::from))
                    .collect(),
                participant_names,
                combat_description,
            },
            StoredEventEffect::ModifyStat {
                character_id,
                character_name,
                stat_name,
                modifier,
            } => EventEffect::ModifyStat {
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                character_name,
                stat_name,
                modifier,
            },
            StoredEventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => EventEffect::AddReward {
                reward_type,
                amount,
                description,
            },
            StoredEventEffect::Custom {
                description,
                requires_dm_action,
            } => EventEffect::Custom {
                description,
                requires_dm_action,
            },
        }
    }
}

impl From<StoredChainedEvent> for ChainedEvent {
    fn from(s: StoredChainedEvent) -> Self {
        Self {
            event_id: NarrativeEventId::from(parse_uuid_or_nil(&s.event_id, "event_id")),
            event_name: s.event_name,
            delay_turns: s.delay_turns,
            additional_trigger: s.additional_trigger.map(|t| NarrativeTriggerType::from(*t)),
            chain_reason: s.chain_reason,
        }
    }
}

impl From<StoredStoryEventType> for StoryEventType {
    fn from(s: StoredStoryEventType) -> Self {
        match s {
            StoredStoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoryEventType::LocationChange {
                from_location: from_location
                    .and_then(|id| Uuid::parse_str(&id).ok().map(LocationId::from)),
                to_location: LocationId::from(parse_uuid_or_nil(&to_location, "to_location")),
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                travel_method,
            },
            StoredStoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => StoryEventType::DialogueExchange {
                npc_id: CharacterId::from(parse_uuid_or_nil(&npc_id, "npc_id")),
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            },
            StoredStoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => StoryEventType::CombatEvent {
                combat_type: combat_type.into(),
                participants: participants
                    .into_iter()
                    .filter_map(|id| Uuid::parse_str(&id).ok().map(CharacterId::from))
                    .collect(),
                enemies,
                outcome: outcome.map(|o| o.into()),
                location_id: LocationId::from(parse_uuid_or_nil(&location_id, "location_id")),
                rounds,
            },
            StoredStoryEventType::ChallengeAttempted {
                challenge_id,
                challenge_name,
                character_id,
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome,
            } => StoryEventType::ChallengeAttempted {
                challenge_id: challenge_id
                    .and_then(|id| Uuid::parse_str(&id).ok().map(ChallengeId::from)),
                challenge_name,
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome: outcome.into(),
            },
            StoredStoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                source: source.into(),
                quantity,
            },
            StoredStoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoryEventType::ItemTransferred {
                item_name,
                from_character: from_character
                    .and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                to_character: CharacterId::from(parse_uuid_or_nil(&to_character, "to_character")),
                quantity,
                reason,
            },
            StoredStoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => StoryEventType::ItemUsed {
                item_name,
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                target,
                effect,
                consumed,
            },
            StoredStoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => StoryEventType::RelationshipChanged {
                from_character: CharacterId::from(parse_uuid_or_nil(
                    &from_character,
                    "from_character",
                )),
                to_character: CharacterId::from(parse_uuid_or_nil(&to_character, "to_character")),
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            },
            StoredStoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => StoryEventType::SceneTransition {
                from_scene: from_scene.and_then(|id| Uuid::parse_str(&id).ok().map(SceneId::from)),
                to_scene: SceneId::from(parse_uuid_or_nil(&to_scene, "to_scene")),
                from_scene_name,
                to_scene_name,
                trigger_reason,
            },
            StoredStoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => StoryEventType::InformationRevealed {
                info_type: info_type.into(),
                title,
                content,
                source: source.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                importance: importance.into(),
                persist_to_journal,
            },
            StoredStoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoryEventType::NpcAction {
                npc_id: CharacterId::from(parse_uuid_or_nil(&npc_id, "npc_id")),
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            },
            StoredStoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => StoryEventType::DmMarker {
                title,
                note,
                importance: importance.into(),
                marker_type: marker_type.into(),
            },
            StoredStoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoryEventType::NarrativeEventTriggered {
                narrative_event_id: NarrativeEventId::from(parse_uuid_or_nil(
                    &narrative_event_id,
                    "narrative_event_id",
                )),
                narrative_event_name,
                outcome_branch,
                effects_applied,
            },
            StoredStoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => StoryEventType::StatModified {
                character_id: CharacterId::from(parse_uuid_or_nil(&character_id, "character_id")),
                stat_name,
                previous_value,
                new_value,
                reason,
            },
            StoredStoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            },
            StoredStoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            },
            StoredStoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            },
            StoredStoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            },
        }
    }
}

impl From<StoredCombatEventType> for CombatEventType {
    fn from(s: StoredCombatEventType) -> Self {
        match s {
            StoredCombatEventType::Started => CombatEventType::Started,
            StoredCombatEventType::RoundCompleted => CombatEventType::RoundCompleted,
            StoredCombatEventType::CharacterDefeated => CombatEventType::CharacterDefeated,
            StoredCombatEventType::CharacterFled => CombatEventType::CharacterFled,
            StoredCombatEventType::Ended => CombatEventType::Ended,
        }
    }
}

impl From<StoredCombatOutcome> for CombatOutcome {
    fn from(s: StoredCombatOutcome) -> Self {
        match s {
            StoredCombatOutcome::Victory => CombatOutcome::Victory,
            StoredCombatOutcome::Defeat => CombatOutcome::Defeat,
            StoredCombatOutcome::Fled => CombatOutcome::Fled,
            StoredCombatOutcome::Negotiated => CombatOutcome::Negotiated,
            StoredCombatOutcome::Draw => CombatOutcome::Draw,
            StoredCombatOutcome::Interrupted => CombatOutcome::Interrupted,
        }
    }
}

impl From<StoredChallengeEventOutcome> for ChallengeEventOutcome {
    fn from(s: StoredChallengeEventOutcome) -> Self {
        match s {
            StoredChallengeEventOutcome::CriticalSuccess => ChallengeEventOutcome::CriticalSuccess,
            StoredChallengeEventOutcome::Success => ChallengeEventOutcome::Success,
            StoredChallengeEventOutcome::PartialSuccess => ChallengeEventOutcome::PartialSuccess,
            StoredChallengeEventOutcome::Failure => ChallengeEventOutcome::Failure,
            StoredChallengeEventOutcome::CriticalFailure => ChallengeEventOutcome::CriticalFailure,
        }
    }
}

impl From<StoredItemSource> for ItemSource {
    fn from(s: StoredItemSource) -> Self {
        match s {
            StoredItemSource::Found { location } => ItemSource::Found { location },
            StoredItemSource::Purchased { from, cost } => ItemSource::Purchased { from, cost },
            StoredItemSource::Gifted { from } => ItemSource::Gifted {
                from: CharacterId::from(parse_uuid_or_nil(&from, "from")),
            },
            StoredItemSource::Looted { from } => ItemSource::Looted { from },
            StoredItemSource::Crafted => ItemSource::Crafted,
            StoredItemSource::Reward { for_what } => ItemSource::Reward { for_what },
            StoredItemSource::Stolen { from } => ItemSource::Stolen { from },
            StoredItemSource::Custom { description } => ItemSource::Custom { description },
        }
    }
}

impl From<StoredInfoType> for InfoType {
    fn from(s: StoredInfoType) -> Self {
        match s {
            StoredInfoType::Lore => InfoType::Lore,
            StoredInfoType::Quest => InfoType::Quest,
            StoredInfoType::Character => InfoType::Character,
            StoredInfoType::Location => InfoType::Location,
            StoredInfoType::Item => InfoType::Item,
            StoredInfoType::Secret => InfoType::Secret,
            StoredInfoType::Rumor => InfoType::Rumor,
        }
    }
}

impl From<StoredInfoImportance> for StoryEventInfoImportance {
    fn from(s: StoredInfoImportance) -> Self {
        match s {
            StoredInfoImportance::Minor => StoryEventInfoImportance::Minor,
            StoredInfoImportance::Notable => StoryEventInfoImportance::Notable,
            StoredInfoImportance::Major => StoryEventInfoImportance::Major,
            StoredInfoImportance::Critical => StoryEventInfoImportance::Critical,
        }
    }
}

impl From<StoredMarkerImportance> for MarkerImportance {
    fn from(s: StoredMarkerImportance) -> Self {
        match s {
            StoredMarkerImportance::Minor => MarkerImportance::Minor,
            StoredMarkerImportance::Notable => MarkerImportance::Notable,
            StoredMarkerImportance::Major => MarkerImportance::Major,
            StoredMarkerImportance::Critical => MarkerImportance::Critical,
        }
    }
}

impl From<StoredDmMarkerType> for DmMarkerType {
    fn from(s: StoredDmMarkerType) -> Self {
        match s {
            StoredDmMarkerType::Note => DmMarkerType::Note,
            StoredDmMarkerType::PlotPoint => DmMarkerType::PlotPoint,
            StoredDmMarkerType::CharacterMoment => DmMarkerType::CharacterMoment,
            StoredDmMarkerType::WorldEvent => DmMarkerType::WorldEvent,
            StoredDmMarkerType::PlayerDecision => DmMarkerType::PlayerDecision,
            StoredDmMarkerType::Foreshadowing => DmMarkerType::Foreshadowing,
            StoredDmMarkerType::Callback => DmMarkerType::Callback,
            StoredDmMarkerType::Custom => DmMarkerType::Custom,
        }
    }
}
