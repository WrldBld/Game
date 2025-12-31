//! Featured NPC management implementation for NarrativeEventNpcPort
//!
//! Handles FEATURES_NPC edge relationships and chain membership queries.

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use uuid::Uuid;

use super::Neo4jNarrativeEventRepository;
use wrldbldr_domain::entities::{EventChainMembership, FeaturedNpc};
use wrldbldr_domain::{CharacterId, EventChainId, NarrativeEventId};
use wrldbldr_engine_ports::outbound::NarrativeEventNpcPort;

#[async_trait]
impl NarrativeEventNpcPort for Neo4jNarrativeEventRepository {
    // =========================================================================
    // FEATURES_NPC Edge Methods
    // =========================================================================

    /// Add a featured NPC to the event (creates FEATURES_NPC edge)
    async fn add_featured_npc(
        &self,
        event_id: NarrativeEventId,
        featured_npc: FeaturedNpc,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            MATCH (c:Character {id: $character_id})
            MERGE (e)-[r:FEATURES_NPC]->(c)
            SET r.role = $role
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", featured_npc.character_id.to_string())
        .param("role", featured_npc.role.unwrap_or_default());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get all featured NPCs for an event
    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:FEATURES_NPC]->(c:Character)
            RETURN c.id as character_id, r.role as role
            ORDER BY c.name",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            let character_id_str: String = row.get("character_id")?;
            let role: String = row.get("role").unwrap_or_default();

            npcs.push(FeaturedNpc {
                character_id: CharacterId::from(Uuid::parse_str(&character_id_str)?),
                role: if role.is_empty() { None } else { Some(role) },
            });
        }

        Ok(npcs)
    }

    /// Remove a featured NPC from the event (deletes FEATURES_NPC edge)
    async fn remove_featured_npc(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:FEATURES_NPC]->(c:Character {id: $character_id})
            DELETE r
            RETURN count(*) as deleted",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    /// Update featured NPC role
    async fn update_featured_npc_role(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
        role: Option<String>,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:FEATURES_NPC]->(c:Character {id: $character_id})
            SET r.role = $role
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", character_id.to_string())
        .param("role", role.unwrap_or_default());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    // =========================================================================
    // Chain Membership Query Methods
    // =========================================================================

    /// Get chain membership info for an event (queries CONTAINS_EVENT edges)
    async fn get_chain_memberships(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Vec<EventChainMembership>> {
        let q = query(
            "MATCH (chain:EventChain)-[r:CONTAINS_EVENT]->(e:NarrativeEvent {id: $event_id})
            RETURN chain.id as chain_id, r.position as position, r.is_completed as is_completed
            ORDER BY chain.name",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut memberships = Vec::new();

        while let Some(row) = result.next().await? {
            let chain_id_str: String = row.get("chain_id")?;
            let position: i64 = row.get("position").unwrap_or(0);
            let is_completed: bool = row.get("is_completed").unwrap_or(false);

            memberships.push(EventChainMembership {
                chain_id: EventChainId::from(Uuid::parse_str(&chain_id_str)?),
                position: position as u32,
                is_completed,
            });
        }

        Ok(memberships)
    }
}
