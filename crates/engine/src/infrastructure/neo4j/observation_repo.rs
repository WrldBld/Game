//! Neo4j observation repository implementation.
//!
//! Tracks PC observations of NPC locations (fog of war for investigation gameplay).
//! Observations are stored as edges: `(PlayerCharacter)-[:OBSERVED_NPC {...}]->(Character)`

use async_trait::async_trait;
use neo4rs::{query, Graph};
use wrldbldr_domain::*;
use wrldbldr_domain::common::{parse_datetime_or, StringExt};

use crate::infrastructure::ports::{ClockPort, ObservationRepo, RepoError};

pub struct Neo4jObservationRepo {
    graph: Graph,
    clock: std::sync::Arc<dyn ClockPort>,
}

impl Neo4jObservationRepo {
    pub fn new(graph: Graph, clock: std::sync::Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }
}

#[async_trait]
impl ObservationRepo for Neo4jObservationRepo {
    /// Get all observations for a PC
    async fn get_observations(&self, pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->(npc:Character)
            RETURN r.location_id as location_id, r.region_id as region_id,
                   r.game_time as game_time, r.observation_type as observation_type,
                   coalesce(r.is_revealed_to_player, true) as is_revealed_to_player,
                   r.notes as notes, r.created_at as created_at,
                   npc.id as npc_id
            ORDER BY r.game_time DESC",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self.graph.execute(q).await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut observations = Vec::new();
        let now = self.clock.now();

        while let Some(row) = result.next().await.map_err(|e| RepoError::Database(e.to_string()))? {
            let npc_id_str: String = row.get("npc_id").map_err(|e| RepoError::Database(e.to_string()))?;
            let location_id_str: String = row.get("location_id").map_err(|e| RepoError::Database(e.to_string()))?;
            let region_id_str: String = row.get("region_id").map_err(|e| RepoError::Database(e.to_string()))?;
            let game_time_str: String = row.get("game_time").map_err(|e| RepoError::Database(e.to_string()))?;
            let observation_type_str: String = row.get("observation_type").map_err(|e| RepoError::Database(e.to_string()))?;
            let is_revealed_to_player: bool = row.get("is_revealed_to_player").unwrap_or(true);
            let notes: String = row.get("notes").unwrap_or_default();
            let created_at_str: String = row.get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;

            let observation = NpcObservation {
                pc_id,
                npc_id: CharacterId::from_uuid(uuid::Uuid::parse_str(&npc_id_str)
                    .map_err(|e| RepoError::Database(e.to_string()))?),
                location_id: LocationId::from_uuid(uuid::Uuid::parse_str(&location_id_str)
                    .map_err(|e| RepoError::Database(e.to_string()))?),
                region_id: RegionId::from_uuid(uuid::Uuid::parse_str(&region_id_str)
                    .map_err(|e| RepoError::Database(e.to_string()))?),
                game_time: parse_datetime_or(&game_time_str, now),
                observation_type: observation_type_str
                    .parse()
                    .unwrap_or(ObservationType::Direct),
                is_revealed_to_player,
                notes: notes.into_option(),
                created_at: parse_datetime_or(&created_at_str, now),
            };

            observations.push(observation);
        }

        Ok(observations)
    }

    /// Save an observation (upsert - updates if exists)
    async fn save_observation(&self, observation: &NpcObservation) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id}), (npc:Character {id: $npc_id})
            MERGE (pc)-[r:OBSERVED_NPC]->(npc)
            SET r.location_id = $location_id,
                r.region_id = $region_id,
                r.game_time = $game_time,
                r.observation_type = $observation_type,
                r.is_revealed_to_player = $is_revealed_to_player,
                r.notes = $notes,
                r.created_at = $created_at",
        )
        .param("pc_id", observation.pc_id.to_string())
        .param("npc_id", observation.npc_id.to_string())
        .param("location_id", observation.location_id.to_string())
        .param("region_id", observation.region_id.to_string())
        .param("game_time", observation.game_time.to_rfc3339())
        .param("observation_type", observation.observation_type.to_string())
        .param("is_revealed_to_player", observation.is_revealed_to_player)
        .param("notes", observation.notes.clone().unwrap_or_default())
        .param("created_at", observation.created_at.to_rfc3339());

        self.graph.run(q).await.map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    /// Check if a PC has observed a specific NPC
    async fn has_observed(&self, pc_id: PlayerCharacterId, target_id: CharacterId) -> Result<bool, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->(npc:Character {id: $npc_id})
            RETURN count(r) > 0 as has_observed",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", target_id.to_string());

        let mut result = self.graph.execute(q).await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result.next().await.map_err(|e| RepoError::Database(e.to_string()))? {
            let has_observed: bool = row.get("has_observed").unwrap_or(false);
            Ok(has_observed)
        } else {
            Ok(false)
        }
    }
}
