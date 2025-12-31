//! StoryEventDialoguePort implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::common::row_to_story_event;
use super::Neo4jStoryEventRepository;
use wrldbldr_domain::entities::{StoryEvent, StoryEventType};
use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::outbound::StoryEventDialoguePort;

#[async_trait]
impl StoryEventDialoguePort for Neo4jStoryEventRepository {
    /// Get recent dialogue exchanges with a specific NPC
    ///
    /// Returns DialogueExchange events involving the specified NPC,
    /// ordered by timestamp descending (most recent first).
    ///
    /// The query filters by event_type containing "DialogueExchange"
    /// and matches the npc_id field within the event_type_json.
    async fn get_dialogues_with_npc(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        limit: u32,
    ) -> Result<Vec<StoryEvent>> {
        // Query for DialogueExchange events that involve this NPC
        // The event_type_json contains npc_id as a field
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE e.event_type_json CONTAINS 'DialogueExchange'
              AND e.event_type_json CONTAINS $npc_id
            RETURN e
            ORDER BY e.timestamp DESC
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            let event = row_to_story_event(row)?;
            // Double-check it's actually a DialogueExchange with this NPC
            if let StoryEventType::DialogueExchange {
                npc_id: event_npc_id,
                ..
            } = &event.event_type
            {
                if *event_npc_id == npc_id {
                    events.push(event);
                }
            }
        }

        Ok(events)
    }

    /// Update or create a SPOKE_TO edge between a PlayerCharacter and an NPC
    ///
    /// This edge tracks conversation history metadata for the Staging System.
    async fn update_spoke_to_edge(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        topic: Option<String>,
    ) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
             MATCH (npc:Character {id: $npc_id})
             MERGE (pc)-[r:SPOKE_TO]->(npc)
             SET r.last_dialogue_at = datetime(),
                 r.last_topic = $topic,
                 r.conversation_count = COALESCE(r.conversation_count, 0) + 1
             RETURN r.conversation_count as count",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("topic", topic.unwrap_or_default());

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated SPOKE_TO edge: PC {} -> NPC {}", pc_id, npc_id);

        Ok(())
    }
}
